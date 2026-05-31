//! Integration harness: spawns the real `raspi-dashboard-backend` binary with
//! `DEV_AUTH=1`, points its upstreams at wiremock mock servers + a tempdir
//! trivy report, polls `/status` until up, and exposes a `reqwest` client. The
//! child is killed on `Drop`.
//!
//! Tests are `#[ignore]` (they spawn a process + bind a port); run them with
//! `cargo test -p raspi-dashboard-e2e -- --ignored`.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;

use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct Stack {
    child: Child,
    pub base: String,
    pub client: reqwest::Client,
    pub trivy_dir: PathBuf,
    // Held so the temp dirs / mock servers outlive the running binary.
    _trivy_tmp: TempDir,
    _static_dir: TempDir,
    _gatus: MockServer,
    _beszel: MockServer,
}

impl Stack {
    pub async fn start() -> anyhow::Result<Self> {
        let gatus = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/endpoints/statuses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_gatus()))
            .mount(&gatus)
            .await;

        let beszel = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/collections/users/auth-with-password"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "test-token",
                "record": { "id": "u1" }
            })))
            .mount(&beszel)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/collections/systems/records"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_beszel()))
            .mount(&beszel)
            .await;

        // Trivy report on disk. Keep the TempDir guard alive in the struct, or
        // the dir (and last-scan.json) is removed the moment start() returns.
        let trivy_tmp = tempfile::tempdir()?;
        let trivy_path = trivy_tmp.path().to_path_buf();
        std::fs::write(
            trivy_path.join("last-scan.json"),
            serde_json::to_vec(&sample_scan())?,
        )?;

        // Minimal static dir so ServeDir has something to serve.
        let static_dir = tempfile::tempdir()?;
        std::fs::write(static_dir.path().join("index.html"), "<html></html>")?;

        let port = free_port()?;
        let base = format!("http://127.0.0.1:{port}");

        let child = Command::new(bin_path())
            .env("DEV_AUTH", "1")
            .env("DASHBOARD_BIND", format!("127.0.0.1:{port}"))
            .env("GATUS_URL", gatus.uri())
            .env("BESZEL_URL", beszel.uri())
            .env("BESZEL_USER", "ro")
            .env("BESZEL_PASSWORD", "ro-pass")
            .env("TRIVY_SCAN_FILE", trivy_path.join("last-scan.json"))
            .env("TRIVY_SCAN_REQUEST", trivy_path.join("scan-request"))
            .env("STATIC_DIR", static_dir.path())
            .env("RUST_LOG", "warn")
            .spawn()?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Poll /status until the server is accepting.
        let mut up = false;
        for _ in 0..100 {
            if let Ok(r) = client.get(format!("{base}/status")).send().await {
                if r.status().is_success() {
                    up = true;
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let stack = Stack {
            child,
            base,
            client,
            trivy_dir: trivy_path,
            _trivy_tmp: trivy_tmp,
            _static_dir: static_dir,
            _gatus: gatus,
            _beszel: beszel,
        };
        if !up {
            anyhow::bail!("backend did not come up within 10s");
        }
        Ok(stack)
    }

    pub async fn get(&self, route: &str) -> reqwest::Response {
        self.client
            .get(format!("{}{route}", self.base))
            .send()
            .await
            .expect("request failed")
    }

    pub async fn post(&self, route: &str) -> reqwest::Response {
        self.client
            .post(format!("{}{route}", self.base))
            .send()
            .await
            .expect("request failed")
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> anyhow::Result<u16> {
    let l = TcpListener::bind("127.0.0.1:0")?;
    Ok(l.local_addr()?.port())
}

/// The product binary sits next to the test binary's `deps/` dir.
fn bin_path() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // remove test exe name
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("raspi-dashboard-backend")
}

fn sample_gatus() -> serde_json::Value {
    serde_json::json!([
        {"name":"Pi-hole","group":"core","results":[
            {"success":true,"timestamp":"2026-05-31T00:00:00Z","duration":5000000},
            {"success":true,"timestamp":"2026-05-31T00:01:00Z","duration":6000000}
        ]},
        {"name":"NAS","group":"infra","results":[
            {"success":false,"timestamp":"2026-05-31T00:00:00Z","duration":0}
        ]}
    ])
}

fn sample_beszel() -> serde_json::Value {
    serde_json::json!({
        "page": 1, "perPage": 200, "totalItems": 1, "items": [
            {"id":"s1","name":"raspi","status":"up","host":"127.0.0.1",
             "info":{"cpu":12.5,"mp":40.0,"dp":55.0}}
        ]
    })
}

fn sample_scan() -> serde_json::Value {
    serde_json::json!({
        "scanned_at": "2026-05-31T00:00:00Z",
        "images": [
            {"image":"ghcr.io/x:1","critical":1,"high":2,"vulns":[
                {"id":"CVE-1","pkg":"openssl 3.0","severity":"CRITICAL","title":"bad"}
            ]}
        ]
    })
}
