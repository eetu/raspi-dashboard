# raspi-dashboard

One LAN-only page answering **"is everything ok, and is anything vulnerable?"** —
live service health (gatus), host/container metrics (beszel), and a press-to-scan
CVE report (trivy) for the Raspberry Pi. The 4th app in the homebrew family, and
the **first Svelte one**.

A single Rust (axum) binary that embeds a SvelteKit SPA, ships as one arm64
container to `ghcr.io/eetu/raspi-dashboard`, and is deployed onto the Pi by
[`../raspi`](../raspi) behind Traefik + oauth2-proxy. Stateless — pure fan-in, no
database.

## Layout

```
backend/    Rust axum service — config, upstream clients (gatus/beszel/trivy),
            forward-auth gate, CSP, SPA serving. No DB.
frontend/   SvelteKit SPA (adapter-static, runes), halo-design tokens.
e2e/        Spawned-binary integration harness (wiremock'd upstreams).
Dockerfile  Multi-stage xx cross-compile → scratch.
```

## Endpoints

| Route | Auth | Purpose |
|---|---|---|
| `GET /status` | none | Liveness for gatus: `{service, version, gatus_healthy, beszel_healthy, trivy_scan_age}` |
| `GET /api/health` | forward-auth | Normalised gatus endpoint statuses |
| `GET /api/metrics` | forward-auth | beszel systems + latest stats |
| `GET /api/cve` | forward-auth | Parsed `last-scan.json` + staleness + totals |
| `POST /api/cve/scan` | forward-auth | Touch `scan-request` → 202; client polls `/api/cve` |

See [SECURITY.md](SECURITY.md) for the trust model (oauth2-proxy edge,
`X-Auth-Request-*` header trust, read-only beszel user).

## Develop

```fish
# Backend (loopback :3007). DEV_AUTH bypasses the forward-auth gate.
cp backend/.env.example backend/.env
cargo run -p raspi-dashboard-backend

# Frontend (Vite dev server, proxies /api + /status → :3007)
cd frontend; and yarn install; and yarn dev
```

Point dev at the Pi's loopback upstreams with an SSH tunnel:
`ssh -L 3001:127.0.0.1:3001 -L 8091:127.0.0.1:8091 pi`.

## Test

```fish
cargo test --workspace                              # unit
cargo test -p raspi-dashboard-e2e -- --ignored      # spawned-binary integration
cd frontend; and yarn validate                      # typecheck + lint + format
```

`./install-hooks.sh` wires the pre-commit clippy/lint gate.

## Deploy

Built + pushed by CI on a push to `main` (`ghcr.io/eetu/raspi-dashboard:main`).
Wired into the Pi by `../raspi` (`RASPI_DASHBOARD` dict in `group_data/all.py`,
`tasks/raspi_dashboard.py`). Create the `raspi-dashboard` Bitwarden item (a
read-only beszel user's login) before deploy, then `uv run pyinfra inventory.py
deploy.py`. Lands at `https://dashboard.{domain}`.
