# Frontend

Svelte 5 (runes) + SvelteKit with `adapter-static` — a static SPA the backend
embeds and serves. Consumes halo-design tokens via `src/lib/styles/halo.css`
(copied verbatim) and `--halo-*` vars in scoped `<style>` blocks. See the
`raspi-design` skill for the brand delta and `coding-style:svelte` for runes/style.

## Validation

Run `yarn validate` after changes — typecheck + lint + format in one shot.

Individual scripts:

- `yarn lint` / `yarn lint:fix` (eslint, house `eslint-config/svelte` preset)
- `yarn format` / `yarn format:fix` (prettier)
- `yarn typecheck` (`svelte-kit sync && svelte-check`)

Use yarn (not npm). Dev server proxies `/api` + `/status` to the backend at
`:3007`. Icons: SVG sources in `static/`, rasterized to PNG via
`./scripts/gen-icons.sh`.
