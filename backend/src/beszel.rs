//! beszel upstream — beszel embeds PocketBase. We authenticate a dedicated
//! read-only user (`auth-with-password`), cache the token, and read the
//! `systems` collection. The token is reused until a request 401s, then
//! re-minted once.
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
    pub name: String,
    pub status: String,
    pub host: Option<String>,
    pub info: Value,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub systems: Vec<SystemMetrics>,
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
    let mut tok = token(state).await?;
    let mut resp = get_records(state, &tok).await?;
    // One re-auth on 401: the cached token may have expired.
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        *state.beszel_token.lock().await = None;
        tok = login(state).await?;
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
    Ok(MetricsResponse {
        systems: list.items.into_iter().map(normalize_system).collect(),
    })
}

fn normalize_system(rec: Value) -> SystemMetrics {
    let str_field = |k: &str| {
        rec.get(k)
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default()
    };
    SystemMetrics {
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
