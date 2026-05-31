pub mod auth;
pub mod beszel;
pub mod config;
pub mod error;
pub mod gatus;
pub mod routes;
pub mod state;
pub mod trivy;

use std::time::Duration;

use tower_http::set_header::SetResponseHeaderLayer;
use tracing_subscriber::EnvFilter;

use config::Config;
use state::AppState;

/// Content-Security-Policy applied to every response. The dashboard fans in
/// JSON only (no third-party images/media), so everything stays same-origin
/// except the Google Fonts hosts halo-design uses. HSTS / X-Frame-Options /
/// X-Content-Type-Options are Traefik's job, not the binary's.
const CSP: &str = "default-src 'self'; \
     script-src 'self'; \
     style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
     font-src 'self' data: https://fonts.gstatic.com; \
     img-src 'self' data:; \
     connect-src 'self'; \
     frame-ancestors 'none'; \
     base-uri 'self'; \
     object-src 'none'; \
     form-action 'self'";

pub async fn run_server() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("info,raspi_dashboard_backend=debug")
        }))
        .init();

    let cfg = Config::from_env()?;
    if cfg.dev_auth {
        tracing::warn!("DEV_AUTH=1 — forward-auth gate bypassed; do not use in prod");
    }
    if !cfg.beszel_configured() {
        tracing::warn!("beszel credentials unset — /api/metrics will report unavailable");
    }

    // Short timeouts: all upstreams are loopback JSON. A hung gatus/beszel must
    // not wedge a request — it should degrade to an error the UI can show.
    let http = reqwest::Client::builder()
        .user_agent(concat!("raspi-dashboard/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()?;

    let state = AppState::new(cfg, http);
    let bind = state.cfg.bind.clone();

    let csp_value = axum::http::HeaderValue::from_static(CSP);
    let app = routes::router(state).layer(SetResponseHeaderLayer::if_not_present(
        axum::http::header::CONTENT_SECURITY_POLICY,
        csp_value,
    ));

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "raspi-dashboard listening");
    axum::serve(listener, app).await?;
    Ok(())
}
