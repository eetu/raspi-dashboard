# raspi-dashboard backend

Rust axum 0.8 — the family's `rust-axum` pattern (see that skill). Stateless:
**no database, no session key**. Serves the SPA from `STATIC_DIR` + `/api`.

## Module map (`src/`)

- `main.rs` — `#[tokio::main]` → `run_server`.
- `lib.rs` — `run_server`, router + state init.
- `config.rs` — `Config::from_env`, `beszel_configured` check.
- `state.rs` — `AppState` (HTTP clients + config).
- `routes.rs` — `/status`, `/api/health`, `/api/metrics`, `/api/cve`,
  `/api/cve/scan`, SPA fallback.
- `auth.rs` — `AuthUser` extractor: forward-auth headers + `DEV_AUTH` gate.
- `error.rs` — `AppError` + responses.
- `gatus.rs` / `beszel.rs` / `trivy.rs` — the three upstream clients.

## Notes

- `/status` is the only unauthenticated route (gatus liveness); everything under
  `/api/*` requires the forward-auth headers (or `DEV_AUTH=1`).
- beszel creds optional → `/api/metrics` reports unavailable rather than erroring.
- trivy is file-based: read `last-scan.json`, request via the `scan-request` file
  handshake; report staleness computed against `TRIVY_STALE_HOURS`.
- Tests: `tests/` integration with wiremock'd gatus/beszel; the spawned-binary
  e2e crate lives at the workspace `e2e/`.
