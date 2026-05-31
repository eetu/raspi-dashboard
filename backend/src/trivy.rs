//! trivy upstream — a file handshake over the shared `/var/lib/trivy` mount
//! (../raspi Phase A). We read `last-scan.json` and trigger an on-demand scan by
//! touching `scan-request` (watched by the trivy-cve-scan.path systemd unit).

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The report written by ../raspi's trivy-cve-scan.sh. Shape is owned there;
/// these structs mirror it.
#[derive(Debug, Deserialize, Serialize)]
pub struct ScanReport {
    pub scanned_at: String,
    pub images: Vec<ImageReport>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImageReport {
    pub image: String,
    pub critical: u32,
    pub high: u32,
    pub vulns: Vec<Vuln>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Vuln {
    pub id: String,
    pub pkg: String,
    pub severity: String,
    pub title: String,
}

/// What `/api/cve` returns: the report plus derived staleness so the UI can flag
/// a missed scan without re-parsing the timestamp.
#[derive(Debug, Serialize)]
pub struct CveResponse {
    pub scanned_at: String,
    pub images: Vec<ImageReport>,
    pub stale: bool,
    pub age_hours: Option<i64>,
    pub total_critical: u32,
    pub total_high: u32,
}

pub async fn read(state: &AppState) -> AppResult<CveResponse> {
    let path = &state.cfg.trivy_scan_file;
    let bytes = match tokio::fs::read(path).await {
        Ok(b) => b,
        // No scan has ever run — distinct from a parse failure. The UI shows
        // "no scan yet; press Scan now".
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::ServiceUnavailable(
                "no scan report yet — trigger a scan".into(),
            ));
        }
        Err(e) => return Err(AppError::Internal(e.into())),
    };
    let report: ScanReport = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::Upstream(format!("last-scan.json parse: {e}")))?;
    Ok(to_response(report, state.cfg.trivy_stale_hours, Utc::now()))
}

fn to_response(report: ScanReport, stale_hours: i64, now: DateTime<Utc>) -> CveResponse {
    let age_hours = DateTime::parse_from_rfc3339(&report.scanned_at)
        .ok()
        .map(|t| (now - t.with_timezone(&Utc)).num_hours());
    let stale = age_hours.map(|h| h >= stale_hours).unwrap_or(true);
    let total_critical = report.images.iter().map(|i| i.critical).sum();
    let total_high = report.images.iter().map(|i| i.high).sum();
    CveResponse {
        scanned_at: report.scanned_at,
        images: report.images,
        stale,
        age_hours,
        total_critical,
        total_high,
    }
}

/// Touch the scan-request file → the trivy-cve-scan.path unit starts a scan.
/// Writing (not just utime) guarantees an IN_CLOSE_WRITE so PathChanged fires
/// even on filesystems with coarse mtime resolution.
pub async fn request_scan(state: &AppState) -> AppResult<()> {
    let path = &state.cfg.trivy_scan_request;
    write_request(path)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("touch scan-request: {e}")))
}

async fn write_request(path: &Path) -> std::io::Result<()> {
    // A timestamp body is enough to register as a content change; the watcher
    // only cares that the file was written + closed.
    let stamp = Utc::now().to_rfc3339();
    tokio::fs::write(path, stamp).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ScanReport {
        ScanReport {
            scanned_at: "2026-05-31T00:00:00Z".into(),
            images: vec![
                ImageReport {
                    image: "a".into(),
                    critical: 2,
                    high: 3,
                    vulns: vec![],
                },
                ImageReport {
                    image: "b".into(),
                    critical: 0,
                    high: 1,
                    vulns: vec![],
                },
            ],
        }
    }

    #[test]
    fn totals_sum_across_images() {
        let now = DateTime::parse_from_rfc3339("2026-05-31T01:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let r = to_response(sample(), 96, now);
        assert_eq!(r.total_critical, 2);
        assert_eq!(r.total_high, 4);
        assert_eq!(r.age_hours, Some(1));
        assert!(!r.stale);
    }

    #[test]
    fn old_scan_is_stale() {
        let now = DateTime::parse_from_rfc3339("2026-06-10T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let r = to_response(sample(), 96, now);
        assert!(r.stale);
        assert_eq!(r.age_hours, Some(10 * 24));
    }

    #[test]
    fn unparseable_timestamp_is_stale() {
        let mut s = sample();
        s.scanned_at = "not-a-date".into();
        let r = to_response(s, 96, Utc::now());
        assert!(r.stale);
        assert_eq!(r.age_hours, None);
    }

    #[tokio::test]
    async fn request_scan_writes_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scan-request");
        write_request(&path).await.unwrap();
        assert!(path.exists());
        assert!(!tokio::fs::read_to_string(&path).await.unwrap().is_empty());
    }
}
