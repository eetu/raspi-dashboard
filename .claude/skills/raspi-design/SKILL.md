---
name: raspi-design
description: The per-app visual identity for raspi-dashboard — layers a glyph, wordmark, and voice on top of the shared halo-design tokens. Use when building or restyling any UI in this repo (panels, the wordmark, empty/error states). Inherits everything from halo-design; only the four deltas below are app-specific.
---

# raspi-design

Thin layer over **halo-design** (the family tokens). Do not redefine colors or
type — import `src/lib/styles/halo.css` and consume `--halo-*` in scoped
`<style>` blocks. This skill owns only the four per-app deltas.

## 1. Glyph

A **shield enclosing a heartbeat pulse** — the two questions the page answers
(is it *secure*, is it *healthy*). Single-stroke, `currentColor` for the shield
and `--halo-accent` for the pulse line, 1.8 stroke-width, round caps/joins, on a
24×24 viewBox. Canonical implementation: `src/lib/components/Wordmark.svelte`.
Matches the family's thin-line glyph style.

## 2. Wordmark

Text: **`raspidash`** (one word, lowercase) — `raspi` in `--halo-text-main`,
`dash` in `--halo-accent` (the `.accent` span). Font: `--halo-font-heading`
(Space Grotesk). The glyph sits to its left with a `0.5em` gap. A muted Pulp
Fiction riff (`.riff`) trails the name and collapses below 520px — see Voice.

## 3. Layout

`max-width: 880px`, centered. Persistent header (wordmark, riff baked in) and a tab
bar live in `+layout.svelte`; tabs are real routes so refresh/deep-link work:

- **Dashboard** (`/`) — service health grid + host/container metrics.
- **CVE** (`/cve`) — the scan panel.

The active tab is underlined in `--halo-accent` (compare `page.url.pathname`
from `$app/state`; build hrefs with `resolve()` from `$app/paths`). Within a tab,
content is stacked `.halo-card` panels — each a `Panel.svelte` card with a
heading and an optional right-aligned action (e.g. CVE's "Scan now" — the one
accent-filled button). Status uses dots: `--halo-connected` (up) /
`--halo-disconnected` (down); severity uses `--halo-disconnected` (critical) and
`--halo-accent` (high).

## 4. Voice

The wordmark carries a **Pulp Fiction riff**, baked into `Wordmark.svelte` and
collapsing on narrow screens — same as the family (halo *"i shot marvin in the
halo."*, chat *"royale with chat."*, scribe *"the path of the righteous
scribe."*, ocular *"a quiet eye on the ocular."*). raspi's line trails the name:
*"pretty freaking far from ok."* (Marsellus' answer to "are you ok?").
Empty/degraded states stay plain and factual ("no scan yet — press Scan now",
"beszel unavailable"), never cute. Numbers over adjectives.
