//! End-to-end API tests against the spawned binary. `#[ignore]` by default —
//! run with `cargo test -p raspi-dashboard-e2e -- --ignored`.

use raspi_dashboard_e2e::Stack;

#[tokio::test]
#[ignore]
async fn status_is_unauthenticated_and_reports_upstreams() {
    let s = Stack::start().await.unwrap();
    let r = s.get("/status").await;
    assert!(r.status().is_success());
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["service"], "raspi-dashboard");
    assert_eq!(body["gatus_healthy"], true);
    assert_eq!(body["beszel_healthy"], true);
    // Sample scan is dated 2026-05-31 → some non-negative age.
    assert!(body["trivy_scan_age"].is_number());
}

#[tokio::test]
#[ignore]
async fn health_normalizes_gatus() {
    let s = Stack::start().await.unwrap();
    let body: serde_json::Value = s.get("/api/health").await.json().await.unwrap();
    let eps = body["endpoints"].as_array().unwrap();
    assert_eq!(eps.len(), 2);
    let pihole = &eps[0];
    assert_eq!(pihole["name"], "Pi-hole");
    assert_eq!(pihole["up"], true);
    assert_eq!(pihole["uptime"], 1.0);
}

#[tokio::test]
#[ignore]
async fn metrics_passes_beszel_info_through() {
    let s = Stack::start().await.unwrap();
    let body: serde_json::Value = s.get("/api/metrics").await.json().await.unwrap();
    let sys = &body["systems"][0];
    assert_eq!(sys["name"], "raspi");
    assert_eq!(sys["info"]["cpu"], 12.5);
}

#[tokio::test]
#[ignore]
async fn cve_reports_totals_and_staleness() {
    let s = Stack::start().await.unwrap();
    let body: serde_json::Value = s.get("/api/cve").await.json().await.unwrap();
    assert_eq!(body["total_critical"], 1);
    assert_eq!(body["total_high"], 2);
    assert!(body["stale"].is_boolean());
    assert_eq!(body["images"][0]["vulns"][0]["id"], "CVE-1");
}

#[tokio::test]
#[ignore]
async fn cve_scan_touches_request_file() {
    let s = Stack::start().await.unwrap();
    let r = s.post("/api/cve/scan").await;
    assert_eq!(r.status(), reqwest::StatusCode::ACCEPTED);
    // The backend wrote the scan-request file the trivy .path unit watches.
    let req = s.trivy_dir.join("scan-request");
    assert!(req.exists(), "scan-request not created");
}
