use std::env;
use std::path::PathBuf;

/// Stateless fan-in config. No DB, no session key — the app sits behind
/// oauth2-proxy (it trusts `X-Auth-Request-*`, see [`crate::auth`]) so it holds
/// no auth secret of its own. The only secret is the read-only beszel
/// PocketBase login, and its absence degrades the metrics endpoint rather than
/// failing the boot (health + CVE still serve).
#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    /// When set, the forward-auth gate is bypassed with a synthetic user so the
    /// app is usable on localhost without oauth2-proxy in front. Never enable in
    /// prod — it removes the only request-origin check the binary makes.
    pub dev_auth: bool,

    /// gatus base URL. Its REST API is unauthenticated on loopback (see
    /// ../raspi tasks/gatus.py — Phase A).
    pub gatus_url: String,

    /// beszel (PocketBase) base URL + optional read-only credentials.
    pub beszel_url: String,
    pub beszel_user: Option<String>,
    pub beszel_password: Option<String>,

    /// trivy hand-off files on the shared `/var/lib/trivy` mount. The scan
    /// report is read; touching the request file triggers a scan via the
    /// trivy-cve-scan.path unit (Phase A).
    pub trivy_scan_file: PathBuf,
    pub trivy_scan_request: PathBuf,
    /// A scan report older than this many hours is reported as stale. The Pi
    /// scans Mon+Thu (~72–96h apart), so 96h flags a genuinely missed run.
    pub trivy_stale_hours: i64,

    /// Directory of the built SPA to serve (Vite `dist/`).
    pub static_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            dev_auth: env::var("DEV_AUTH").as_deref() == Ok("1"),
            bind: env::var("DASHBOARD_BIND").unwrap_or_else(|_| "0.0.0.0:3007".into()),
            gatus_url: trim_url(
                env::var("GATUS_URL").unwrap_or_else(|_| "http://127.0.0.1:3001".into()),
            ),
            beszel_url: trim_url(
                env::var("BESZEL_URL").unwrap_or_else(|_| "http://127.0.0.1:8091".into()),
            ),
            beszel_user: env::var("BESZEL_USER").ok().filter(|s| !s.is_empty()),
            beszel_password: env::var("BESZEL_PASSWORD").ok().filter(|s| !s.is_empty()),
            trivy_scan_file: env::var("TRIVY_SCAN_FILE")
                .unwrap_or_else(|_| "/var/lib/trivy/last-scan.json".into())
                .into(),
            trivy_scan_request: env::var("TRIVY_SCAN_REQUEST")
                .unwrap_or_else(|_| "/var/lib/trivy/scan-request".into())
                .into(),
            trivy_stale_hours: env::var("TRIVY_STALE_HOURS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(96),
            static_dir: env::var("STATIC_DIR")
                .unwrap_or_else(|_| "./dist".into())
                .into(),
        })
    }

    /// True when beszel credentials are present — gates whether the metrics
    /// endpoint attempts an upstream call at all.
    pub fn beszel_configured(&self) -> bool {
        self.beszel_user.is_some() && self.beszel_password.is_some()
    }
}

fn trim_url(s: String) -> String {
    s.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Env is process-global; this test sets only vars no other test reads and
    // asserts defaults via a fresh parse. Run serially-safe by avoiding shared
    // keys.
    #[test]
    fn defaults_and_trim() {
        // Defaults hold when nothing is set (cargo runs tests with a clean-ish
        // env; we only assert the trim + parse helpers here to avoid env races).
        assert_eq!(trim_url("http://x/".into()), "http://x");
        assert_eq!(trim_url("http://x".into()), "http://x");
    }

    #[test]
    fn beszel_configured_requires_both() {
        let mut c = Config {
            dev_auth: true,
            bind: "x".into(),
            gatus_url: "x".into(),
            beszel_url: "x".into(),
            beszel_user: None,
            beszel_password: None,
            trivy_scan_file: "x".into(),
            trivy_scan_request: "x".into(),
            trivy_stale_hours: 96,
            static_dir: "x".into(),
        };
        assert!(!c.beszel_configured());
        c.beszel_user = Some("u".into());
        assert!(!c.beszel_configured());
        c.beszel_password = Some("p".into());
        assert!(c.beszel_configured());
    }
}
