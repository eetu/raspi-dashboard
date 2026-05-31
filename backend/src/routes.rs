use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::state::AppState;
use crate::{beszel, gatus, trivy};

pub fn router(state: AppState) -> Router {
    Router::new()
        // Unauthenticated liveness — gatus probes this; keep it auth-free and on
        // a Traefik monitor router that bypasses oauth2-proxy.
        .route("/status", get(status))
        // Everything below requires the forward-auth headers (AuthUser).
        .route("/api/health", get(api_health))
        .route("/api/metrics", get(api_metrics))
        .route("/api/cve", get(api_cve))
        .route("/api/cve/scan", post(api_cve_scan))
        // SPA: serve a built asset if the path maps to a real file under
        // static_dir, otherwise return index.html with 200 so the client router
        // owns the route (a hard refresh on a sub-route works). Done as a handler
        // rather than tower-http's ServeDir, whose not_found_service leaks a 404
        // status onto every client route.
        .fallback(get(serve_spa))
        .with_state(state)
}

async fn serve_spa(State(state): State<AppState>, uri: axum::http::Uri) -> axum::response::Response {
    use axum::http::header::CONTENT_TYPE;
    use axum::response::{Html, IntoResponse};

    let base = &state.cfg.static_dir;
    let rel = uri.path().trim_start_matches('/');

    // Only attempt a file read for a path that stays inside static_dir after
    // normalisation — rejects `..` traversal and absolute escapes.
    if !rel.is_empty() {
        let candidate = base.join(rel);
        if let Ok(canon) = candidate.canonicalize() {
            if let Ok(canon_base) = base.canonicalize() {
                if canon.starts_with(&canon_base) && canon.is_file() {
                    if let Ok(bytes) = tokio::fs::read(&canon).await {
                        let mime = mime_guess::from_path(&canon).first_or_octet_stream();
                        return ([(CONTENT_TYPE, mime.as_ref())], bytes).into_response();
                    }
                }
            }
        }
    }

    // SPA shell for "/" and every unmatched client route.
    match tokio::fs::read_to_string(base.join("index.html")).await {
        Ok(html) => Html(html).into_response(),
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

// ---------- public probe ----------

async fn status(State(state): State<AppState>) -> Json<Value> {
    let (gatus_healthy, beszel_healthy, trivy_scan_age) = tokio::join!(
        async { gatus::fetch(&state).await.is_ok() },
        async { state.cfg.beszel_configured() && beszel::fetch(&state).await.is_ok() },
        async { trivy::read(&state).await.ok().and_then(|r| r.age_hours) },
    );
    Json(json!({
        "service": "raspi-dashboard",
        "version": env!("CARGO_PKG_VERSION"),
        "gatus_healthy": gatus_healthy,
        "beszel_healthy": beszel_healthy,
        "trivy_scan_age": trivy_scan_age,
    }))
}

// ---------- gated api ----------

async fn api_health(_user: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let endpoints = gatus::fetch(&state).await?;
    Ok(Json(json!({ "endpoints": endpoints })))
}

async fn api_metrics(_user: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let metrics = beszel::fetch(&state).await?;
    Ok(Json(json!(metrics)))
}

async fn api_cve(_user: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let report = trivy::read(&state).await?;
    Ok(Json(json!(report)))
}

async fn api_cve_scan(
    _user: AuthUser,
    State(state): State<AppState>,
) -> AppResult<(axum::http::StatusCode, Json<Value>)> {
    trivy::request_scan(&state).await?;
    // 202: the scan runs out-of-band (trivy-cve-scan.path → .service); the
    // client polls /api/cve until scanned_at advances.
    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({ "status": "scan requested" })),
    ))
}
