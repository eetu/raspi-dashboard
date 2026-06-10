//! beszel upstream — beszel embeds PocketBase. We authenticate a dedicated
//! read-only user (`auth-with-password`), cache the token, and read the
//! `systems` collection. The token is reused until it's rejected — either an
//! explicit 401 or a silent guest-downgrade (PocketBase returns 200 + an empty
//! list rather than 401 for a stale token), then re-minted once.
//!
//! NOTE (verify at deploy): the exact `systems` record shape — especially the
//! `info` sub-object's keys — is pinned to beszel's PocketBase schema and must
//! be confirmed against the live instance (plan open question). Parsing is
//! therefore lenient: unknown/absent fields degrade to `None` and the raw
//! `info` blob is passed through so the UI can render whatever beszel provides
//! without a backend change.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct RecordList {
    #[serde(default)]
    items: Vec<Value>,
}

/// Normalised per-system metrics row. `info` is beszel's raw latest-stats blob,
/// passed through untouched; the typed fields are best-effort extractions.
#[derive(Debug, Serialize)]
pub struct SystemMetrics {
    /// PocketBase record id — the key beszel's UI uses in its /system/{id} route.
    pub id: String,
    pub name: String,
    pub status: String,
    pub host: Option<String>,
    pub info: Value,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub systems: Vec<SystemMetrics>,
    /// Public beszel base URL for deep-links, or null when not configured.
    pub beszel_url: Option<String>,
}

/// Authenticate and return a fresh token, caching it in state.
async fn login(state: &AppState) -> AppResult<String> {
    let (user, pass) = match (&state.cfg.beszel_user, &state.cfg.beszel_password) {
        (Some(u), Some(p)) => (u, p),
        _ => {
            return Err(AppError::ServiceUnavailable(
                "beszel credentials not configured".into(),
            ))
        }
    };
    let url = format!(
        "{}/api/collections/users/auth-with-password",
        state.cfg.beszel_url
    );
    let resp = state
        .http
        .post(&url)
        .json(&serde_json::json!({ "identity": user, "password": pass }))
        .send()
        .await?;
    if !resp.status().is_success() {
        // Don't echo the body — it can include the identity. Status only.
        return Err(AppError::Upstream(format!(
            "beszel auth returned {}",
            resp.status()
        )));
    }
    let auth: AuthResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Upstream(format!("beszel auth parse: {e}")))?;
    *state.beszel_token.lock().await = Some(auth.token.clone());
    Ok(auth.token)
}

/// Token from cache, minting one if absent.
async fn token(state: &AppState) -> AppResult<String> {
    if let Some(t) = state.beszel_token.lock().await.clone() {
        return Ok(t);
    }
    login(state).await
}

async fn get_records(state: &AppState, token: &str) -> AppResult<reqwest::Response> {
    let url = format!(
        "{}/api/collections/systems/records?perPage=200",
        state.cfg.beszel_url
    );
    Ok(state.http.get(&url).bearer_auth(token).send().await?)
}

pub async fn fetch(state: &AppState) -> AppResult<MetricsResponse> {
    if !state.cfg.beszel_configured() {
        return Err(AppError::ServiceUnavailable(
            "beszel credentials not configured".into(),
        ));
    }

    // Attempt with the cached token (minting one if absent).
    let had_cached = state.beszel_token.lock().await.is_some();
    let tok = token(state).await?;
    let mut systems = read_systems(state, &tok).await?;

    // A stale token is NOT rejected with 401 on a PocketBase collection list —
    // it's silently downgraded to an unauthenticated guest, and beszel's
    // `systems` list rule (auth required) then returns 200 with an empty set.
    // So an empty result from a *reused* token is ambiguous: re-auth once and
    // retry before trusting it. (A token that does 401 is already handled inside
    // read_systems.) This is what was leaving the dashboard stuck on "No systems
    // reported." after beszel restarted / the token aged out, until a process
    // restart cleared the cache. We only retry when the token was cached: a
    // freshly-minted token returning empty means genuinely zero systems.
    if systems.is_empty() && had_cached {
        *state.beszel_token.lock().await = None;
        let fresh = login(state).await?;
        systems = read_systems(state, &fresh).await?;
    }

    Ok(MetricsResponse {
        systems,
        beszel_url: state.cfg.beszel_public_url.clone(),
    })
}

/// GET the systems collection with `token`, re-authing once on an explicit 401,
/// and parse the records into normalised rows.
async fn read_systems(state: &AppState, token: &str) -> AppResult<Vec<SystemMetrics>> {
    let mut resp = get_records(state, token).await?;
    // One re-auth on 401: the cached token may have expired.
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        *state.beszel_token.lock().await = None;
        let tok = login(state).await?;
        resp = get_records(state, &tok).await?;
    }
    if !resp.status().is_success() {
        return Err(AppError::Upstream(format!(
            "beszel systems returned {}",
            resp.status()
        )));
    }
    let list: RecordList = resp
        .json()
        .await
        .map_err(|e| AppError::Upstream(format!("beszel systems parse: {e}")))?;
    Ok(list.items.into_iter().map(normalize_system).collect())
}

fn normalize_system(rec: Value) -> SystemMetrics {
    let str_field = |k: &str| {
        rec.get(k)
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default()
    };
    SystemMetrics {
        id: str_field("id"),
        name: str_field("name"),
        status: str_field("status"),
        host: rec
            .get("host")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        info: rec.get("info").cloned().unwrap_or(Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn config_for(beszel_url: String) -> Config {
        Config {
            dev_auth: true,
            bind: "127.0.0.1:0".into(),
            gatus_url: "http://unused".into(),
            beszel_url,
            beszel_user: Some("ro".into()),
            beszel_password: Some("ro-pass".into()),
            beszel_public_url: None,
            trivy_scan_file: "x".into(),
            trivy_scan_request: "x".into(),
            trivy_stale_hours: 96,
            static_dir: "x".into(),
        }
    }

    fn systems_body(items: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "page": 1, "perPage": 200, "totalItems": 0, "items": items })
    }

    // Regression: a stale cached token isn't 401'd by PocketBase — the systems
    // list silently comes back empty (guest-downgraded). fetch() must re-auth on
    // that empty-from-cached case, not surface "No systems reported.". Here the
    // stale token returns [] and only a fresh login unlocks the real row.
    #[tokio::test]
    async fn empty_from_stale_cached_token_triggers_reauth() {
        let beszel = MockServer::start().await;

        // Login mints a *fresh* token.
        Mock::given(method("POST"))
            .and(path("/api/collections/users/auth-with-password"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "token": "fresh" })),
            )
            .mount(&beszel)
            .await;

        // The stale token is silently downgraded → empty list, HTTP 200.
        Mock::given(method("GET"))
            .and(path("/api/collections/systems/records"))
            .and(header("authorization", "Bearer stale"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(systems_body(serde_json::json!([]))),
            )
            .mount(&beszel)
            .await;

        // The fresh token sees the real systems.
        Mock::given(method("GET"))
            .and(path("/api/collections/systems/records"))
            .and(header("authorization", "Bearer fresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(systems_body(
                serde_json::json!([{ "id": "s1", "name": "raspi", "status": "up" }]),
            )))
            .mount(&beszel)
            .await;

        let state = AppState::new(config_for(beszel.uri()), reqwest::Client::new());
        // Seed a token that was valid before but no longer is.
        *state.beszel_token.lock().await = Some("stale".into());

        let resp = fetch(&state).await.expect("fetch");
        assert_eq!(
            resp.systems.len(),
            1,
            "should re-auth past the empty result"
        );
        assert_eq!(resp.systems[0].name, "raspi");
        // Cache now holds the re-minted token.
        assert_eq!(state.beszel_token.lock().await.as_deref(), Some("fresh"));
    }

    #[test]
    fn normalizes_a_systems_record_passing_info_through() {
        let rec = serde_json::json!({
            "id": "abc",
            "name": "raspi",
            "status": "up",
            "host": "127.0.0.1",
            "info": { "cpu": 12.5, "mp": 40.1, "dp": 55.0 }
        });
        let m = normalize_system(rec);
        assert_eq!(m.id, "abc");
        assert_eq!(m.name, "raspi");
        assert_eq!(m.status, "up");
        assert_eq!(m.host.as_deref(), Some("127.0.0.1"));
        assert_eq!(m.info["cpu"], serde_json::json!(12.5));
    }

    #[test]
    fn missing_fields_degrade_gracefully() {
        let m = normalize_system(serde_json::json!({ "name": "x" }));
        assert_eq!(m.name, "x");
        assert_eq!(m.status, "");
        assert!(m.host.is_none());
        assert!(m.info.is_null());
    }
}
