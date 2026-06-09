# raspi-dashboard — repo overview

One-page LAN status dashboard — "is everything ok, and is anything vulnerable?"
Fans in gatus (service health), beszel (host/container metrics), and trivy (CVE
scan). Stateless, read-only. Sibling in eetu's homebrew family ([halo](../halo),
[chat](../chat), [scribe](../scribe), [ocular](../ocular)) — shares the design
system; the family's reference Rust(axum)+SvelteKit app.

## Layout

```
backend/         Rust axum 0.8 — stateless fan-in (gatus/beszel/trivy), serves the SPA
frontend/        Svelte 5 + SvelteKit (adapter-static), halo-design tokens
e2e/             spawned-binary integration tests with wiremock'd upstreams
.claude/skills/  raspi-design skill (visual language, brand)
Dockerfile       multi-stage: node build + Rust cross-compile (arm64) → scratch
SECURITY.md      forward-auth trust model, CSP, fail-closed notes
```

Per-area instructions in `backend/CLAUDE.md` and `frontend/CLAUDE.md`.
Cargo workspace = `backend` + `e2e`.

## Conventions

- **Auth at the edge.** Behind oauth2-proxy (Traefik forward-auth). `/status` is
  unauthenticated (gatus liveness); all `/api/*` require `X-Auth-Request-User` /
  `-Email`. `DEV_AUTH=1` bypasses on localhost.
- **Stateless.** No database, no session key — a pure read-only aggregator.
  - gatus: REST over loopback → endpoint statuses.
  - beszel: PocketBase REST, read-only creds optional (unset →
    `/api/metrics` degrades gracefully).
  - trivy: shared `/var/lib/trivy/` mount — read `last-scan.json`, request a
    scan by touching `scan-request`.
- **CVE scan handoff.** `POST /api/cve/scan` touches `scan-request` → 202; the
  scan runs out-of-band (systemd `.path` unit). Client polls `/api/cve` until
  `scanned_at` advances. Reports older than `TRIVY_STALE_HOURS` (default 96 —
  Pi scans Mon+Thu) are flagged stale.

## Working on this repo

- Backend `:3007` (`DASHBOARD_BIND`): `cp backend/.env.example backend/.env` then
  `cargo run -p raspi-dashboard-backend` (set `DEV_AUTH=1` for local).
- Frontend dev `:5173`: `cd frontend && yarn install && yarn dev`; Vite proxies
  `/api` + `/status` to `:3007`.
- Point dev at Pi upstreams: `ssh -L 3001:127.0.0.1:3001 -L 8091:127.0.0.1:8091 pi`.
- e2e: `cargo test -p raspi-dashboard-e2e -- --ignored`.
- Key env: `GATUS_URL`, `BESZEL_URL`/`_USER`/`_PASSWORD`/`_PUBLIC_URL`,
  `TRIVY_SCAN_FILE`/`_REQUEST`/`_STALE_HOURS`, `STATIC_DIR`. See `backend/src/config.rs`.

## Out of scope (for now)

- Authentication (forward-auth at the edge only)
- Writable endpoints / state mutation, per-user prefs
- Multi-host dashboards (single Pi)

If a feature crosses into those areas, raise it before implementing.
