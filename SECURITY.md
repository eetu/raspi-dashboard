# Security model — raspi-dashboard

A LAN-only, read-mostly status page. It fans in three local upstreams (gatus,
beszel, trivy) and serves one page. It holds **no durable state** and **no auth
secret of its own**.

## Trust boundaries

- **Edge auth is oauth2-proxy, not this app.** On the Pi the `dashboard.{domain}`
  host is a Traefik *gated host* (`_gated_hosts` in `../raspi/tasks/traefik.py`):
  every request is forced through oauth2-proxy → Kanidm before it reaches the
  backend. The backend trusts the injected `X-Auth-Request-User` /
  `X-Auth-Request-Email` headers and **requires them on every `/api/*` route**
  (401 if absent) as defense-in-depth against a request that bypassed the proxy
  on the loopback port. These headers are PII and are **never logged**.
- **No own login / session / cookie / Kanidm client.** There is no `SESSION_KEY`,
  no signed cookie, no OIDC flow in the binary. Removing that surface is the
  point of sitting behind the forward-auth edge.
- **`DEV_AUTH=1`** bypasses the header gate with a synthetic user for local dev.
  It is never set in production (the binary logs a warning at boot when it is).

## Unauthenticated surface

- **`GET /status`** is intentionally auth-free (booleans + version only, no
  secrets) so gatus can probe liveness. It is served on a Traefik monitor router
  that bypasses oauth2-proxy; everything else on the host stays gated.

## Upstream credentials

- **gatus**: unauthenticated loopback REST API (`127.0.0.1:3001`) — no creds.
- **beszel**: a **dedicated read-only PocketBase user**, creds from
  `/etc/secrets/raspi-dashboard.env` (written by `../raspi/tasks/secrets.py` from
  the `raspi-dashboard` Bitwarden item). Loaded from env only, **never logged**;
  the auth-error path reports the HTTP status, not the response body (which can
  echo the identity). The token is cached in memory and re-minted on a 401.
  Creds absent → `/api/metrics` reports unavailable; the app still serves.
- **trivy**: a file handshake over the RW `/var/lib/trivy` mount — reads
  `last-scan.json`, and `POST /api/cve/scan` touches `scan-request` (watched by
  the `trivy-cve-scan.path` unit). No network, no creds.

## Hardening

- **CSP** set in-code on every response (same-origin except Google Fonts; no
  inline scripts, `frame-ancestors 'none'`, `object-src 'none'`). HSTS /
  X-Frame-Options / X-Content-Type-Options are Traefik's job.
- **Static file serving** resolves the requested path and rejects anything that
  escapes `STATIC_DIR` after canonicalisation (path-traversal guard); unmatched
  routes return the SPA shell, never an arbitrary file.
- **Container**: runs as a non-root user, `scratch` base (no shell/userland),
  LAN-restricted egress (`../raspi/tasks/network_restrict.py`), `MemoryMax=96M`.
- **Upstream HTTP** uses short connect/read timeouts so a hung loopback upstream
  degrades to an error instead of wedging a request.

## Reporting

Personal single-user project. Open an issue, or just fix it.
