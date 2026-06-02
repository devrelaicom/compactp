# compactp landing page — design spec

**Date:** 2026-06-02
**Branch:** `site-landing-page`
**Status:** approved (brainstorming) — pending implementation plan

## Summary

A single-page static site for the `compactp` parser frontend, hosted on
GitHub Pages at `compactp.midnightntwrk.expert`, deployed from `/site/`
via a GitHub Actions workflow.

Visual identity is inspired by [aino.agency](https://aino.agency):
dense monospace, numbered fact rows, ASCII-rich hero, ambient motion.
Palette is pure terminal — true black, off-white, amber accent.

No build tool. Vanilla HTML, CSS, JavaScript. The "build" step is
uploading `/site/` as a GitHub Pages artifact.

## Goals

- Communicate what `compactp` is in under one viewport (hero + facts strip).
- Provide install + a working usage example without sending visitors to the README.
- Reflect the project's character: low-level, computational, careful.
- Be cheap to maintain — version bumps are hand-edits to one HTML file.

## Non-goals

- Multi-page documentation (the README and `/docs/` already cover that).
- Blog, changelog, or release notes (linked, not hosted here).
- Analytics, service workers, PWA features.
- A compatibility matrix or live parse-tree demo (explicitly dropped in
  brainstorming — see [Sections dropped](#sections-dropped) below).

## Decisions captured during brainstorming

| Decision | Value |
|---|---|
| Scope | One-page docs-lite |
| Aesthetic | ASCII-rich (lean into aino reference) |
| Motion | Hero animation + ambience, with `prefers-reduced-motion` fallback |
| Palette | Pure terminal — `#000` background, `#f0ede0` text, `#ffb454` accent, `#aaa` secondary text, `#777` dim, `#151515` rule (originally `#555` — raised to `#777` during implementation to satisfy WCAG AA 4.5:1 contrast on small dim text) |
| Stack | Vanilla HTML/CSS/JS — no build tool |
| Hosting | GitHub Pages with GH Actions deploy |
| Domain | `compactp.midnightntwrk.expert` |
| Wordmark | Slant ASCII letterform (italic, slash + underscore based) |
| Footer locale | `GCM` (Grand Cayman — Midnight Foundation HQ) |
| Footer clock | Cayman local time (UTC-5, no DST), formatted `WEEKDAY HH:MM:SS EST` |

### Sections dropped

- **Compact compatibility table** — version matrix duplicates the README.
- **Live parse-tree sample** — content-heavy, defers to the README's example.

## File structure

```
/site/
  index.html         single page
  styles.css         hand-written, no preprocessor
  main.js            ~150 lines: clock, hero animation, marquee init
  CNAME              compactp.midnightntwrk.expert
  favicon.svg        single-character monogram "c_" in amber on black
  og.png             1200×630 Open Graph card, committed (not generated in CI)
  robots.txt         User-agent: * / Allow: /
.github/workflows/
  site.yml           build/deploy on push to main touching /site/**
```

No `index.html` content is shared with the Rust source. Version
strings and the Compact-language compatibility line are hardcoded and
bumped manually on release.

## Page anatomy (top to bottom)

| # | Section | Purpose | Approx height |
|---|---|---|---|
| 1 | Top bar | `COMPACTP · PARSE · CST · AST · JSON · v0.1.0-β1 · MIT · GITHUB ↗` | thin strip |
| 2 | Hero | Animated ASCII wordmark + tagline + blinking amber cursor | ~60vh |
| 3 | Facts strip | 4 columns: STATUS / COMPACT / TESTED COMPILER / MSRV | 1 row |
| 4 | Numbered features | `001 … 007` rows — three-column grid | one screen |
| 5 | Install + usage | One `cargo install` block + three representative CLI invocations | one screen |
| 6 | CLI command table | Seven rows: lex / parse / cst / ast / diag / stats / watch | one screen |
| 7 | ASCII marquee band | One horizontal scrolling band of glyphs (ambience) | 1 row |
| 8 | Footer | UTC clock, locale tag, GitHub / crates.io / docs links | bottom |

The page is short enough that nav anchors aren't warranted — visitors
scroll.

## Section detail

### 1 · Top bar

Single thin row, `#777` dim. Five spans space-between:

```
COMPACTP    PARSE · CST · AST · JSON    v0.1.0-β1    MIT    GITHUB ↗
```

Only `GITHUB ↗` is a link. The rest is label text. The version string
is dim — it lives in the facts strip in amber.

### 2 · Hero

The slant-style ASCII spelling of "compactp" sits in a `<pre>` element,
centered horizontally, vertically aligned to ~1/3 from the top of the
hero. Below it: a one-line tagline in `#aaa`, followed by a steadily
blinking amber `_` cursor.

**Wordmark source art** (final reference — `main.js` consumes this
verbatim, splits into a grid, then animates):

```
                                                      __
   _________  ____ ___  ____  ____ ______/ /_____  / /
  / ___/ __ \/ __ `__ \/ __ \/ __ `/ ___/ __/ __ \/ /
 / /__/ /_/ / / / / / / /_/ / /_/ / /__/ /_/ /_/ / /
 \___/\____/_/ /_/ /_/ ____/\__,_/\___/\__/ ____/_/
                       /_/                  /_/
```

**Tagline:** `a parser frontend for the compact language` (`#aaa`), with
the persistent amber `_` cursor immediately after.

### 3 · Facts strip

Four equal columns, separated by 1px `#151515` rules. Top label is
`#777`, value is `#f0ede0`. The version value is amber — it's the one
piece of "data that changes."

```
FACTS              COMPACT           TESTED            MSRV
v0.1.0-β1 (amber)  ≥ 0.23            0.31.0            1.90
```

All four values are hardcoded in `index.html`. On a release bump, edit
them by hand.

### 4 · Numbered features

Three-column grid: `[NUM] [NAME] [TAG]`. NUM is `#777`, NAME is
`#f0ede0`, TAG is `#ffb454`. Seven rows:

```
001  LOSSLESS CONCRETE SYNTAX TREE      ROWAN
002  TYPED ABSTRACT SYNTAX TREE         ZERO-ALLOC
003  RESILIENT PARSING + RECOVERY       ERROR NODES
004  RUSTC-STYLE HUMAN DIAGNOSTICS      ANSI
005  STRUCTURED JSON ENVELOPE           SCHEMA v1
006  LIBRARY API FOR RUST EMBEDDING     LIB
007  WATCH MODE                         FSNOTIFY
```

No expand / collapse. The rows are the whole content.

### 5 · Install + usage

Two stacked `<pre>` blocks, no syntax highlighting. The `$` prompt is
`#777`, the `compactp` token is amber, the rest of each line is
`#f0ede0`.

```
$ cargo install compactp

$ compactp parse program.compact
$ compactp --format json --pretty diag program.compact
$ compactp watch parse src/
```

Below the second block, one dim line: `read full reference → GITHUB ↗`.

### 6 · CLI command table

Two columns: `COMMAND` (`#f0ede0`) and `DESCRIPTION` (`#aaa`). Seven
rows from the README — verbatim:

| Command | Description |
|---|---|
| `lex` | Tokenize Compact source and print tokens with byte offsets |
| `parse` | Parse input and report diagnostics |
| `cst` | Dump the full lossless concrete syntax tree |
| `ast` | Dump the typed abstract syntax tree |
| `diag` | Emit diagnostics only |
| `stats` | Report file size, token / node / error counts, parse time |
| `watch` | Re-run any of the above when files change |

### 7 · ASCII marquee band

Single full-bleed row, ~10vh tall. Glyphs from `+ ? 9 N + . ?` (aino
reference). Scrolls left at ~30px/s via a pure-CSS `translateX`
keyframe loop. Zero semantic meaning — ambience only.

### 8 · Footer

Three-cluster grid, `#aaa` text on `#000`:

```
GCM                       GITHUB ↗                   MIT
TUESDAY 17:01:14 EST      CRATES.IO ↗                INSPIRED BY AINO ↗
                          DOCS ↗
```

- `GCM` — Grand Cayman locale tag.
- Clock — Cayman local time (UTC-5, no DST). Updates once per second.
- Right cluster — license label (`MIT`) on top; below it a small
  attribution link: `INSPIRED BY AINO ↗` pointing to <https://aino.agency>.

## Motion

### Motion inventory

| # | Element | Behavior | JS |
|---|---|---|---|
| 1 | Hero wordmark — load | 40% of cells (random per load) scramble 3–4 frames → settle. ~600ms total. | ~30 lines |
| 2 | Hero wordmark — hover | Mouse-enter starts a loop: pick a fresh ~40% random subset, scramble, repeat. Mouse-leave stops the loop. | ~20 lines |
| 3 | Tagline cursor | Persistent 1Hz blink on the amber `_`. CSS only. | 0 |
| 4 | ASCII marquee band | Pure-CSS `translateX` keyframe, ~30px/s. | 0 |
| 5 | Footer clock | `setInterval` 1s, format Cayman local time. | ~15 lines |

Total JS budget: ~150 lines, one file, no dependencies.

### Hover scramble — interaction detail

Hover scramble runs as a fixed-interval loop (one iteration ≈ 80ms,
≈ 12fps — fast enough to feel alive, slow enough to be readable):

1. On `mouseenter`, start a `setInterval` at 80ms.
2. Each tick: pick a fresh ~40% random subset of cells from the
   wordmark grid; for each, swap to a random ASCII glyph from the
   scramble alphabet.
3. On `mouseleave`, `clearInterval`, then reset all cells to their
   final glyph in one paint.

(`requestAnimationFrame` is deliberately not used — the visual rhythm
is the point, not 60fps smoothness, and an interval keeps the code
shorter and more obviously correct.)

Scramble alphabet: ASCII printable subset, excluding whitespace and
the cell's final glyph (so a scramble step is always a visible change).

### `prefers-reduced-motion: reduce`

| Element | Reduced-motion behavior |
|---|---|
| Hero load scramble | Skipped — wordmark renders at final state on first paint |
| Hero hover scramble | Disabled — no listener attached |
| Tagline cursor | Still blinks (1Hz, low flash, semantic "alive" indicator) |
| Marquee band | `display: none` — cleaner than paused |
| Footer clock | Still ticks (semantic, not decorative) |

Detected at script start via
`window.matchMedia('(prefers-reduced-motion: reduce)').matches`. The
listener is attached for live updates so toggling system settings
takes effect on the next interaction without a reload.

## Accessibility

- **Contrast** — `#f0ede0` on `#000` ~16:1 (WCAG AAA). `#ffb454` on
  `#000` ~10:1 (AAA for normal text, AA for the 8px top-bar text).
- **Screen readers** — wordmark `<pre>` has `aria-hidden="true"`; its
  parent has `role="img"` and `aria-label="compactp"`. The scramble
  glyphs never reach the accessibility tree.
- **Keyboard** — focusable elements: top-bar GitHub link, install
  block link, footer cluster links. Visible focus ring in amber.
- **No-JS fallback** — page renders correctly with JS disabled:
  - Wordmark static at final state (the source art is in the HTML, not
    constructed by JS).
  - Clock placeholder shows `LIVE` (no current-time guess server-side).
  - Marquee static or hidden via `<noscript>` style.
- **Print stylesheet** — hide marquee, hide cursor blink animation,
  invert colours (`#000` text on `#fff` background).

## Deployment

### `.github/workflows/site.yml`

```yaml
name: site
on:
  push:
    branches: [main]
    paths: ['site/**', '.github/workflows/site.yml']
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v3
        with:
          path: site
      - id: deployment
        uses: actions/deploy-pages@v4
```

No Node, no Rust, no caching. Cold runs land in well under 30 seconds.

### GitHub Pages settings (one-time, in repo UI)

1. **Settings → Pages → Source:** `GitHub Actions`.
2. **Settings → Pages → Custom domain:** `compactp.midnightntwrk.expert`.
   GitHub will read the CNAME file from the artifact (`site/CNAME`).
3. **Settings → Pages → Enforce HTTPS:** ✓ — only available after DNS
   propagates and the cert provisions (~15 minutes).

### DNS records to add at `midnightntwrk.expert`

| Type | Name | Value | TTL |
|---|---|---|---|
| `CNAME` | `compactp` | `devrelaicom.github.io.` | 3600 |

If `midnightntwrk.expert` uses CDN flattening (Cloudflare, etc.), an
`ALIAS` / `ANAME` to the same target works.

Verification: GitHub may also require a `TXT` record at
`_github-pages-challenge-devrelaicom.compactp` to prove ownership. The
exact value is shown in the Pages settings UI when the domain is
saved — copy from there.

### What does not ship

- No analytics (no Plausible, GA, Fathom). Add later if needed.
- No service worker / PWA.
- No sitemap (single page).

## Open items (intentionally deferred)

- **Wordmark "production" art** — the source art above is the brainstorm
  reference. Final implementation may tweak character choices for
  visual balance; behavior (cell grid + scramble) is unchanged.
- **OG image production** — `og.png` will be a one-time hand-rendered
  PNG matching the page palette. No CI generation step.
- **Favicon production** — `favicon.svg` is a one-time hand-authored
  SVG monogram.
- **Analytics decision** — if added later, will be evaluated against
  the page's "no third-party JS" stance.

## Acceptance criteria

1. Visiting `https://compactp.midnightntwrk.expert` returns the page over HTTPS.
2. First paint shows the wordmark, facts strip, and at least the
   beginning of the features list (above the fold on a 1366×768 laptop).
3. The wordmark animates on load (~600ms) and again on hover.
4. The footer clock displays Cayman local time and ticks each second.
5. Under `prefers-reduced-motion: reduce`, the wordmark renders static
   and the marquee is hidden.
6. Page passes axe-core with zero serious / critical violations.
7. Lighthouse Performance ≥ 95, Accessibility = 100, Best Practices ≥ 95.
8. Editing any file under `/site/` and pushing to `main` triggers the
   `site` workflow and replaces the live page within 60 seconds.
