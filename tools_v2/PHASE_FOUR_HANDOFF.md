# Decomposition record: the two large crates split by responsibility

What landed, and the measurements taken at the landing.

## Layout

- `xtask/src/main.rs` is a 32-line entrypoint. `cli/` owns argument preprocessing and dispatch;
  each command domain owns its Clap declarations and handlers.
- `xtask/src/commands/` owns database, deployment, setup, fetching, mod operations, MCP, debugging,
  reproduction, build and CI, generation, platform execution, mission flattening, and the thin
  ticket and wave adapters.
- `xtask/src/core/` owns repository discovery, host execution, Cargo target directories and the
  test-environment helpers.
- `xtask/src/verifications/` groups architecture, CI, database, deployment, language, licensing,
  map-asset, mod-script, registry and schema checks. Every file is named for the domain it checks.
- Schema verification separates contract citations, content budgets, object enums, type inventory,
  specification consistency, kit references, wire readers, glyphs and contract-family validation.
  The CI schema parity check stays independently invoked, with negative tests.
- `developer-tools/src/browser_testing/` owns the Chromium and CDP lifecycle, serving and proxying,
  diagnostics, DOM verification, route drift, capture and the editor scenarios. The smoke helpers
  separate input, fixtures, assertions and boot scenarios; all 21 suite entries keep their order
  and criteria.
- `developer-tools/src/enfusion_tooling/` owns extraction, indexing, symbol scans, API
  documentation, capability checks, citations and the MCP broker.
- `developer-tools/src/map_raster_pipeline/` owns aerial orthophotos, satellite archives,
  cartography, inland water and labels.
- `developer-tools/src/world_export_pipeline/` owns export preparation, prefab catalogs, object
  partitioning, chunk emission, vegetation density, roads, forest smoothing, mathematical
  verification and Enfusion texture decoding.
- `developer-tools/src/blueprint/` groups architectural analysis, mesh decoding, voxel processing,
  BVH work and archive emission. The public blueprint and map-verification signatures and the
  shared PAK ownership are preserved.
- The executables are `enf`, `gate`, `mcpd`, `world`, `map` and `capture`; each entrypoint
  delegates into the library.

## Repairs made in the same landing

- The objective, mission-loader and validator, results-reporter and player-identity source checks
  read the live mod directories. Missing source is an error.
- The wire identifier baselines are `seats = 11` and `area = 11`, each with its owner explanation
  and an unexpected-reader negative test.
- Host-bridge testing compares mount namespaces: matching glibc versions do not imply one
  namespace.
- Objective simulation uses the host bridge for the host C++ compiler.
- Compile-gate tests serialise process environment changes through the shared test lock and restore
  `HOME` on panic; the assertions and expected failure statuses are unchanged.
- Canonical serialisation repairs 68 noncanonical ticket files the corpus assertion found. Typed
  before-and-after equality is checked, and every ticket semantic is retained: canonical omission of
  an empty `depends_on` list preserves its default empty value.
- The frontend menu-helper imports follow the definitions' conditional compilation, restoring
  native builds.

## Measurements

- Tooling and ticketboard unit tests: 1,310 passed; seven ignored tests unchanged. The additional
  compile-fail and documentation tests pass, and the xtask suite reruns at 626 tests.
- `cargo check --workspace --locked`: passes.
- Tooling clippy with `-D warnings`: passes.
- Tooling documentation builds.
- The schema composite passes, including positive and negative fixtures, size pins, wire readers,
  the independent CI parity check, map data and citations.
- All 165 CLI help, error, missing-input, nested-directory and dry-run comparisons match.
- All ten domain verifications pass on the live tree, including their negative proofs.
- Five operational self-tests match a retained executable exactly in stdout, stderr and status: MCP
  log verdicts, server log verdicts, spawn determinism, world boot and MCP call paths.
- Ticket sync, wave repack and check, and ticket execution dry runs match the retained executable
  on a disposable repository; sync and repack are byte-idempotent.
- All 92 fixture baseline entries are checked: one README carries updated documentation and the
  remaining 91 files are byte-identical.
- Dependency metadata is unchanged across all four tooling crates.
- The structural audit covers 602 Rust files in `xtask` and `developer-tools`: zero limit
  violations, largest production file 493 lines, largest test file 962 lines, every inline test
  module extracted, and all 83 obsolete size exemptions removed. Ten regression tests cover live
  execution, missing, disconnected and hollowed implementations, conditional bypasses and newline
  normalisation.
- Browser doctor, all 21 editor scenarios and all 25 DOM oracle routes pass
  (`cargo xtask mk leptos-gates`). The editor API must be running first —
  `cargo xtask ci editor-api-boot` — or the hydrate and mutations scenarios refuse to report.

## The DOM oracle is total

The oracle renders every route against the committed fixture corpus, so that corpus is the entire
data surface a captured page can see. A request the corpus does not answer must not receive an
empty JSON object with HTTP 200: a page then renders a stable empty or error screen, and a stable
screen is exactly what the capture's settle loop accepts and `accept` would write into a golden.

`browser_testing/dom_oracle/fixture_router.rs` makes that resolution total. A request is answered
by a committed fixture, is one of the two token endpoints the corpus deliberately does not own, or
is recorded as unanswered; anything outside `/api/v1/` still reaches the local static server. A
route with any unanswered request fails after the DOM settles, naming every URL and the corpus file
that would have served it. `accept` captures through the same function, so an unfed capture cannot
reach a golden.

Fixture names carry the request method, which is what reaches the committed `POST__` entry a
`GET__`-only rule leaves unreachable. A `.sse.txt` fixture is served as `text/event-stream` through
`cdp::Page::fulfill_raw`: a Server-Sent Events body is delimited by a literal blank line and does
not survive `serde_json`. `LIVE_SSE_FRAME` in the frontend's R-api tests embeds that same file, so
the bytes the gate serves and the bytes the DTO pins are one file.

Five fixtures close the corpus gaps: the CMS announcement list (published rows and a draft), the
caller's leave requests, the administrative leave queue, the server status stream, and the mission
detail the approvals drawer reads for its first queue row. Each carries an R-api round-trip test.

Twenty-one routes carry goldens sourced from the live Leptos render, one route at a time, each with
its own recorded justification in `manifest.json`; `notfound`, `eventmgr`, `callback` and `login`
match their earlier captures unchanged, and every earlier capture is preserved alongside as
`<slug>.react.dom.json`. The divergences are fixture-fed data replacing unfed or mock states, and
intended UI work that postdates the 2026-07-17 freeze; no route diverges because of a defect in the
Leptos pages. Two routes carry earlier accepted deltas on `missions` and `content`, and their
justifications are carried forward rather than replaced.
