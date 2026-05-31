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
(Space Grotesk). The glyph sits to its left with a `0.5em` gap.

## 3. Layout

Single scrolling column, `max-width: 880px`, centered. Three stacked
`.halo-card` panels in fixed order: **service health → host/container metrics →
CVE scan**. Each panel is a `Panel.svelte` card with a heading and an optional
right-aligned action (e.g. CVE's "Scan now" button — the one accent-filled
button on the page). Status uses dots: `--halo-connected` (up) /
`--halo-disconnected` (down); severity uses `--halo-disconnected` (critical) and
`--halo-accent` (high).

## 4. Voice

Terse, operational, lowercase tagline. The page's one line of copy:
*"is everything ok, and is anything vulnerable?"*. Empty/degraded states are
plain and factual ("no scan yet — press Scan now", "beszel unavailable"), never
cute. Numbers over adjectives.
