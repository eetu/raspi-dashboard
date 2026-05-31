# raspi-dashboard — build plan

One LAN-only page answering "is everything ok, and is anything vulnerable?" —
live service health (gatus), host/container metrics (beszel), press-to-scan CVE
report (trivy). 4th app in the homebrew family; **first Svelte app**.

## How to run this plan

A clean session with the `eetu-skills` marketplace installed. **Use the skills —
don't re-derive:**

- `homebrew:sibling-app` — the orchestrator (scaffold order, repo layout, seam,
  Dockerfile/CI/hooks, edge trust model, raspi deploy wiring). Start here.
- `homebrew:spa-frontend` — frontend contract; **this app fills its Svelte
  section** (see Decisions).
- `homebrew:rust-axum` — backend, security, tests.
- `homebrew:halo-design` — look; create the per-app `raspi-design` skill.
- `coding-style:ts-style`, `coding-style:fish` — conventions; fish gotchas when
  running commands.

Memory files (auto-load): `claude-skills-marketplace`, `homebrew-app-family`.
Live reference repo for exact current values: `../scribe`. IaC repo: `../raspi`.

## Decisions (locked)

- **Frontend = Svelte** (trial; React is the family default — this is the pilot).
  Stateless fan-in app = low risk. **First build step: `find-docs` SvelteKit
  `adapter-static` SPA/fallback mode** (verify current adapter story; don't trust
  memory). Then fill `spa-frontend`'s Svelte section with the verified stack
  (versions, adapter, router) as the React section is specified, and commit that
  skill update back to `~/dev/claude-skills`.
- **Backend = Rust axum**, **stateless** (no SQLite — pure fan-in). Skip `db.rs`.
- **Behind oauth2-proxy forward-auth** (LAN-only, `public: False`). No own login,
  no Kanidm client. Trust `X-Auth-Request-*` (validate present, never log).
- Port **3007** (verified free). `MemoryMax≈96M`, `MALLOC_ARENA_MAX=2`.
- Image `ghcr.io/eetu/raspi-dashboard:main`.

## Data flow

```
browser ─https─▶ traefik (oauth2-proxy gate) ─▶ dashboard backend :3007 (loopback)
   poll/SSE ┌──────────────┬──────────────────┬─────────────────┐
            ▼ REST unauth  ▼ PocketBase token  ▼ file handshake
       gatus :3001     beszel :8091        /var/lib/trivy/last-scan.json
   /api/v1/.../statuses  PB REST           + touch scan-request → .path unit
```

Backend normalizes the three, serves `/api/*` + an SSE `/api/stream`. v1
poll-first; realtime later.

## ../raspi review findings (still true — a fresh session won't know)

Verified against `../raspi` this session:

- **Phase 0 (status→gatus, metrics→beszel rename) is ALREADY DONE.** ROUTES,
  OIDC clients, kanidm_oidc landing-origin PATCH all already renamed. Skip it.
  Only leftover: hand-prune stale `status`/`metrics` Cloudflare A records.
- **There is NO `OAUTH2_GATED_HOSTS` var.** Gating = the `_gated_hosts` list in
  `tasks/traefik.py` (built inline). To gate a host, append its route name there.
- **`_SUBDOMAIN_NAMES` entries are DICT names** (e.g. `RASPI_DASHBOARD`), not
  subdomain strings.
- **`trivy-cve-scan.service` has NO `MemoryMax`** (`tasks/trivy.py`). Must add it
  in Phase A — the plan's "spike stays in trivy's own unit" depends on it.
- Quadlets are **rootful** (`/etc/containers/systemd`) → container runs as root,
  can write host-shared mounts without extra privilege. No gymnastics for
  `scan-request`.
- `deploy.py` order: include `tasks/raspi_dashboard.py` **after `tasks/beszel.py`**
  (line ~38). secrets.py runs early (line 7) so the env file is ready.

## Phase A — ../raspi prep (do first; deploy + verify before app work)

1. **gatus loopback API must be unauthenticated.** Today `tasks/gatus.py` emits
   `security.oidc` (gates the whole server, incl. the loopback API) when the
   gatus OIDC secret exists. Fix:
   - Drop the `security.oidc` block from `tasks/gatus.py`.
   - Gate `gatus.{domain}` at the edge instead: append `"gatus"` to `_gated_hosts`
     in `tasks/traefik.py`.
   - **Ripple (required):** gatus's subdomain is currently registered public via
     `KANIDM_OIDC_CLIENTS["gatus"]["public"]`. If you also remove that client, the
     subdomain vanishes from DNS — so add `GATUS` to `_SUBDOMAIN_NAMES` and
     `"public": True` to the `GATUS` dict (in `all.py` + `all.example.py`).
     Simplest: keep the gatus Kanidm client entry but stop gatus *consuming* the
     OIDC secret (so DNS via the client stays, server stays open). Decide at build;
     verify `127.0.0.1:3001/api/v1/endpoints/statuses` returns 200 unauth after.
   - Verify humans still gated on `gatus.{domain}` via oauth2-proxy.
2. **trivy → JSON + path trigger** (`tasks/trivy.py`):
   - cve-scan writes structured `/var/lib/trivy/last-scan.json`
     (`{scanned_at, images:[{image, critical, high, vulns:[{id,pkg,severity,title}]}]}`).
     Keep the ntfy push as optional digest.
   - **Add `MemoryMax` (+ `MemorySwapMax`) to `trivy-cve-scan.service`.**
   - New `trivy-cve-scan.path` unit: `PathChanged=/var/lib/trivy/scan-request` →
     starts `trivy-cve-scan.service`.
3. Deploy. Confirm: gatus API open on loopback; touching
   `/var/lib/trivy/scan-request` triggers a scan that writes `last-scan.json`.

## Phase B — scaffold the app (`homebrew:sibling-app` scaffold order)

Repo `~/dev/raspi-dashboard` (this dir). Backend `rust-axum` (stateless), Svelte
frontend (`spa-frontend`, after find-docs), tooling (Dockerfile/CI/hooks/
dependabot), `raspi-design` skill, `SECURITY.md`. Substitute `scribe`→
`raspi-dashboard`, `SCRIBE_IMAGE_TAG`→`RASPI_DASHBOARD_IMAGE_TAG`, env prefixes.
The Dockerfile `frontend-build` stage is just `yarn build` — Svelte-agnostic.

## Phase C — backend endpoints (`homebrew:rust-axum`)

- `GET /status` — unauth liveness `{service, version, gatus_healthy, beszel_healthy,
  trivy_scan_age}`.
- `GET /api/health` — normalized gatus statuses (name, group, up, uptime, last
  results).
- `GET /api/metrics` — beszel systems + latest stats (cpu/mem/disk/net/containers).
- `GET /api/cve` — parsed `last-scan.json` + `scanned_at` + staleness.
- `POST /api/cve/scan` — touch `/var/lib/trivy/scan-request`, 202; client polls.
- `GET /api/stream` — SSE multiplex (v2; poll-first in v1).

Upstream clients (reqwest):
- **gatus:** GET `127.0.0.1:3001/api/v1/endpoints/statuses` (unauth after Phase A).
- **beszel:** PocketBase at `127.0.0.1:8091`. Auth a **dedicated read-only PB
  user** (`POST /api/collections/users/auth-with-password` → token, cache,
  refresh on 401). Read `systems`/`system_stats`/`container_stats` records.
  Provision the user via PB superuser API on deploy (memos/beszel-style REST
  bootstrap) — verify a non-superuser can read those collections.
- **trivy:** read `/var/lib/trivy/last-scan.json` (RW mount of `/var/lib/trivy`).

Security per `rust-axum`: CSP layer, forward-auth header trust, env secrets,
constant-time where bearer compares happen. Tests: `wiremock` the gatus + beszel
HTTP upstreams; temp-file for trivy JSON; spawned-binary `Stack::start` harness.

## Phase D — frontend (Svelte)

Status grid (green/red + uptime), metrics cards/sparklines, CVE panel with "Scan
now" + last-scan timestamp + per-image findings. Tokens from `halo-design`
(`--halo-*`) in scoped `<style>`. `raspi-design`: pick a glyph in the family
stroke family (server/pulse/shield motif), wordmark text, voice.

## Phase E — raspi deploy wiring (`sibling-app` → ../raspi checklist)

- `RASPI_DASHBOARD` dict in `all.py` + `all.example.py` (host `127.0.0.1`, port
  3007, `url_prefix: "dashboard"`, image, `public: False`, `MemoryMax`).
- `tasks/raspi_dashboard.py` quadlet — copy `tasks/chat.py` (`Network=host`,
  `MALLOC_ARENA_MAX=2`, AutoUpdate/Pull `:main`, `optional()` + cleanup branch).
  Mounts: `/var/lib/trivy` (RW, for last-scan.json read + scan-request write),
  `/etc/secrets/raspi-dashboard.env`.
- `tasks/traefik.py`: add `("dashboard", RASPI_DASHBOARD, "dashboard")` to
  `ROUTES`; append `"dashboard"` to `_gated_hosts`; add
  `RASPI_DASHBOARD = optional("RASPI_DASHBOARD")` at top.
- `tasks/secrets.py`: write `/etc/secrets/raspi-dashboard.env` (beszel PB
  read-only creds) gated on the dict; add a `vault.py` helper.
- `all.py`: add `RASPI_DASHBOARD` to `_SUBDOMAIN_NAMES`.
- `tasks/network_restrict.py`: add `raspi-dashboard` to `RESTRICTED` (LAN-only).
  Container name must equal the unit name `raspi-dashboard`.
- Skip `RESTIC["paths"]` (stateless).
- `deploy.py`: include `tasks/raspi_dashboard.py` after `tasks/beszel.py`.
- Create the `raspi-dashboard` Bitwarden item (beszel PB creds) before deploy.

## Open questions — verify at build

- gatus serves `/api/v1/endpoints/statuses` unauth once `security` is unset? (yes
  expected) — confirms Phase A.
- beszel/PB exact collection names + record shapes on `v0.18.7`; does a non-
  superuser read role suffice for `system_stats`/`container_stats`?
- SvelteKit `adapter-static` SPA mode current API (find-docs first).
- Backend idle RSS fits 96M with `MALLOC_ARENA_MAX=2`; trivy stays external.

## Deploy order

1. Phase A (../raspi gatus + trivy) → deploy → verify.
2. Build image (Phases B–D) → CI → ghcr.
3. Create BW item; provision beszel PB read-only user.
4. Phase E wiring → deploy → dashboard at `dashboard.{domain}`.
