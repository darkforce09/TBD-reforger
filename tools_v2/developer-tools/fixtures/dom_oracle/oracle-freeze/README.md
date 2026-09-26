# DOM oracle goldens

The frozen reference captures that `gate v-suite` compares the single-page app against: for each of
the 25 routes it checks, the page's normalized DOM tree and a screenshot, plus a manifest that
records where each golden came from.

## Contents

```text
tools_v2/developer-tools/fixtures/dom_oracle/oracle-freeze/
├── *.png                                 the 1440 × 900 screenshot taken with each route's golden
├── *.react.dom.json                      a route's first reference capture, kept once `accept` replaced it
├── {audit,content,orbat,*[!t]}.dom.json  each route's golden, the DOM tree `verify` diffs against
└── manifest.json                         the freeze record: source build, viewport, one row per route
```

## How it works

`gate v-suite verify` serves `apps/website/frontend/dist`, opens every route of the route list in
`tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs` with its API calls answered from
`apps/website/frontend/tests/fixtures/api/`, serializes the app root until two captures in a row are
identical, and diffs that tree against `<slug>.dom.json`. A missing golden or any difference fails
the route; the screenshots and the manifest are never compared.

`gate v-suite accept --only <slug> --note "<reason>"` is the one writer. It copies the route's
golden to `<slug>.react.dom.json` when no such copy exists yet, refuses a capture under 256 bytes,
then overwrites `<slug>.dom.json` and `<slug>.png` and updates the route's manifest row. The gate
has no mode that re-captures every route at once, because the goldens cannot be rebuilt from any
build the repository makes.

## Format

- Encoding: a golden is one line of UTF-8 JSON: a tree of nodes, each with `tag`, `attrs`, a fixed
  set of computed `style` properties and its children, starting at the app root
  (`#root>:first-child`). The screenshots are PNG. Files are named by the route's slug, such as
  `dashboard`, `orbat` or `wikislug`. A `*.react.dom.json` file keeps the same tree shape.
- Schema: `manifest.json` holds `frozenFrom`, `distSha256`, `viewport` (`1440x900`), `excluded` (the
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) route, whose check is the editor
  smoke suite), `scope`, and `routes`: one row per slug with `path`, `authed`, `bytes` and `sha256`,
  and, once a route is accepted, `goldenSource` (`leptos`) and `acceptedDelta` (the `--note` text).
- Adding a file: a route joins by adding it to the route list in `routes.rs` and to the manifest,
  then running `accept` for its slug against a fresh `cargo xtask mk leptos` build.

## Producers and consumers

- Producers: `gate v-suite accept`, the `run_modes` step of
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/`.
- Consumers: `gate v-suite verify`, which `cargo xtask mk leptos-gates` runs after the editor suite.

## Boundaries

- Depends on: the serializer `tools_v2/developer-tools/src/browser_testing/fixture_injection.rs`
  injects, which wrote every golden and must stay byte-stable; the API fixtures in
  `apps/website/frontend/tests/fixtures/api/`, which set the data each page shows.
- Used by: the DOM oracle gate in `tools_v2/developer-tools/src/browser_testing/dom_oracle/`.
- Rules: goldens change only through `accept`, one route at a time and with a note; `accept` refuses
  a capture below `MIN_ACCEPT_DOM_JS_LEN` (256 bytes) so an empty page never becomes a golden; a
  slug renamed in `routes.rs` renames its three files and its manifest row in the same change.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running `gate v-suite` and the other
  browser gates.
