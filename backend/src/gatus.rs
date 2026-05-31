//! gatus upstream — `GET {gatus_url}/api/v1/endpoints/statuses`, unauthenticated
//! on loopback (../raspi Phase A). Normalised into the dashboard's flat shape;
//! the TS side hand-mirrors [`HealthEntry`].

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// gatus's raw per-endpoint status object (only the fields we use).
#[derive(Debug, Deserialize)]
struct GatusEndpoint {
    #[serde(default)]
    name: String,
    #[serde(default)]
    group: String,
    #[serde(default)]
    results: Vec<GatusResult>,
}

#[derive(Debug, Deserialize)]
struct GatusResult {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    timestamp: String,
    /// Nanoseconds, per gatus's JSON.
    #[serde(default)]
    duration: i64,
}

/// Normalised health row for one monitored endpoint.
#[derive(Debug, Serialize, PartialEq)]
pub struct HealthEntry {
    pub name: String,
    pub group: String,
    /// Outcome of the most recent check.
    pub up: bool,
    /// Success ratio across the results gatus returned (0.0–1.0). A rolling
    /// recent-window uptime, not all-time.
    pub uptime: f64,
    /// The few most recent checks, newest last (as gatus orders them).
    pub latest: Vec<ResultBrief>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ResultBrief {
    pub success: bool,
    pub timestamp: String,
    pub duration_ms: i64,
}

pub async fn fetch(state: &AppState) -> AppResult<Vec<HealthEntry>> {
    let url = format!("{}/api/v1/endpoints/statuses", state.cfg.gatus_url);
    let resp = state.http.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(AppError::Upstream(format!(
            "gatus statuses returned {}",
            resp.status()
        )));
    }
    let raw: Vec<GatusEndpoint> = resp
        .json()
        .await
        .map_err(|e| AppError::Upstream(format!("gatus statuses parse: {e}")))?;
    Ok(normalize(raw))
}

fn normalize(raw: Vec<GatusEndpoint>) -> Vec<HealthEntry> {
    raw.into_iter().map(normalize_one).collect()
}

fn normalize_one(e: GatusEndpoint) -> HealthEntry {
    let total = e.results.len();
    let ok = e.results.iter().filter(|r| r.success).count();
    let uptime = if total == 0 {
        0.0
    } else {
        ok as f64 / total as f64
    };
    // gatus orders results oldest→newest; the last one is the latest check.
    let up = e.results.last().map(|r| r.success).unwrap_or(false);
    let latest = e
        .results
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|r| ResultBrief {
            success: r.success,
            timestamp: r.timestamp.clone(),
            duration_ms: r.duration / 1_000_000,
        })
        .collect();
    HealthEntry {
        name: e.name,
        group: e.group,
        up,
        uptime,
        latest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_uptime_and_latest_status() {
        let raw: Vec<GatusEndpoint> = serde_json::from_str(
            r#"[
              {"name":"Pi-hole","group":"core","results":[
                {"success":true,"timestamp":"t1","duration":5000000},
                {"success":false,"timestamp":"t2","duration":12000000},
                {"success":true,"timestamp":"t3","duration":7000000}
              ]}
            ]"#,
        )
        .unwrap();
        let out = normalize(raw);
        assert_eq!(out.len(), 1);
        let e = &out[0];
        assert_eq!(e.name, "Pi-hole");
        assert!(e.up); // last result succeeded
        assert!((e.uptime - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(e.latest.len(), 3);
        assert_eq!(e.latest[2].duration_ms, 7); // 7_000_000 ns → 7 ms
    }

    #[test]
    fn empty_results_is_down_not_panic() {
        let raw: Vec<GatusEndpoint> =
            serde_json::from_str(r#"[{"name":"x","group":"","results":[]}]"#).unwrap();
        let out = normalize(raw);
        assert!(!out[0].up);
        assert_eq!(out[0].uptime, 0.0);
    }
}
