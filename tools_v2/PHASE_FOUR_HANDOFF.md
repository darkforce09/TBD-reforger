# Tools V2 Phase Four Handoff

Status: complete. Decomposition, full CI and the separate DOM oracle gate all pass.

## Ownership and layout

- `xtask/src/main.rs` is a 32-line entrypoint. `cli/` owns argument preprocessing and dispatch; each command domain owns its Clap declarations and handlers.
- `xtask/src/commands/` owns database, deployment, setup, fetching, mod operations, MCP, debugging, reproduction, build/CI, generation, platform execution, mission flattening, and thin ticket/wave adapters.
- `xtask/src/core/` owns repository discovery, host execution, Cargo target directories, and test-environment helpers.
- `xtask/src/verifications/` categorizes architecture, CI, database, deployment, language, licensing, map-asset, mod-script, registry, and schema checks. Ticket-number filenames are replaced by domain names; CLI spellings are preserved.
- Schema verification separates contract citations, content budgets, object enums, type inventory, specification consistency, kit references, wire readers, glyphs, and contract-family validation. CI schema parity remains an independently invoked check with negative tests.
- `developer-tools/src/browser_testing/` owns Chromium/CDP lifecycle, serving/proxying, diagnostics, DOM verification, route drift, capture, and editor scenarios. Smoke helpers separate input, fixtures, assertions, and boot scenarios; all 21 suite entries retain their order and criteria.
- `developer-tools/src/enfusion_tooling/` owns extraction, indexing, symbol scans, API documentation, capability checks, citations, and the MCP broker.
- `developer-tools/src/map_raster_pipeline/` owns aerial orthophotos, satellite archives, cartography, inland water, and labels.
- `developer-tools/src/world_export_pipeline/` owns export preparation, prefab catalogs, object partitioning, chunk emission, vegetation density, roads, forest smoothing, mathematical verification, and Enfusion texture decoding.
- `developer-tools/src/blueprint/` groups architectural analysis, mesh decoding, voxel processing, BVH work, and archive emission. Public blueprint/map-verification signatures and shared PAK ownership are preserved.
- Executables remain `enf`, `gate`, `mcpd`, `world`, `map`, and `capture`. Their entrypoints delegate into the subsystem library.

## Baseline repairs

- Objective, mission-loader/validator, results-reporter, and player-identity source checks use the live mod directories. Missing source remains an error.
- Wire identifier baselines are `seats = 11` and `area = 11`, with owner explanations and unexpected-reader negative tests.
- Host-bridge testing compares mount namespaces; matching glibc versions do not imply the same namespace.
- Objective simulation uses the existing host bridge for the host C++ compiler.
- Compile-gate tests serialize process environment changes through the existing test lock and restore `HOME` on panic; assertions and expected failure statuses are preserved.
- Ticket-engine canonical serialization repairs 68 noncanonical ticket files discovered by the corpus assertion. Typed before/after equality is checked; priorities and all other ticket semantics are retained. Canonical omission of T-945's empty `depends_on` list preserves its default empty value.
- Frontend menu-helper imports use the definitions' conditional compilation, restoring native builds.
- Four pre-existing untracked design documents receive only their missing final newline to satisfy EditorConfig.

## Verification evidence

Evidence and retained executables are under `/tmp/tbd-tools-phase-four/` on the validation machine.

- Tooling and ticketboard unit tests: 1,310 passed; seven existing ignored tests remain unchanged. Additional compile-fail and documentation tests pass. See `final-tooling-tests.log` and the final 626-test xtask rerun in `final-xtask-tests.log`.
- Workspace `cargo check --workspace --locked`: passes; see `final-workspace-check.log`.
- Tooling Clippy with `-D warnings`: passes; see `tooling-clippy.log`.
- Tooling documentation builds: pass; existing CLI documentation produces rustdoc warnings. See `tooling-docs.log`.
- Schema composite: passes, including positive/negative fixtures, size pins, wire readers, independent CI parity, map data, and citations. See `schema-gates.log`.
- All 165 CLI help/error/missing-input/nested-directory/dry-run comparisons match; see `cli-parity.json`.
- All ten related domain verifications pass on the live tree, including their negative proofs; see `domain-verification-results.json`.
- Five operational self-tests match the retained executable exactly in stdout, stderr, and status: MCP log verdicts, server log verdicts, spawn determinism, world boot, and MCP call paths. See `operational-parity.json`.
- Ticket sync, wave repack/check, and ticket execution dry runs match the retained executable on a disposable repository. Sync and repack are byte-idempotent. See `disposable-parity.json`.
- All 92 fixture baseline entries are checked: the MCP fixture README has updated documentation; all 91 remaining files are byte-identical. See `fixture-hash-comparison.json`.
- Dependency metadata is unchanged across all four tooling crates. See `dependency-parity.json`.
- Structural audit covers 602 Rust files in xtask/developer-tools: zero limit violations; largest production file is 493 lines and largest test file is 962 lines. All inline test modules are extracted; structural regression tests enforce limits, placement, exemptions, and dependency direction. All 83 obsolete tooling size exemptions are removed. The independent CI-schema parity and faction-seed checks read both linked wave-gate implementations. Ten new regression tests cover live execution, missing/disconnected/hollowed implementations, conditional bypasses, and newline normalization.
- Browser doctor, all 21 editor scenarios and all 25 DOM oracle routes pass (`cargo xtask mk leptos-gates`). The editor API must be running first — `cargo xtask ci editor-api-boot` — or the hydrate and mutations scenarios refuse to report.

## DOM oracle closure

The oracle renders every route against the committed fixture corpus, so that corpus is the entire
data surface a captured page can see. Requests it did not answer used to receive an empty JSON
object with HTTP 200, which let a page render a stable empty or error screen — and a stable screen
is exactly what the capture's settle loop accepts and `accept` would then write into a golden.

`browser_testing/dom_oracle/fixture_router.rs` makes that resolution total. A request is answered by
a committed fixture, is one of the two token endpoints the corpus deliberately does not own, or is
recorded as unanswered; anything outside `/api/v1/` still reaches the local static server. A route
with any unanswered request fails after the DOM settles, naming every URL and the corpus file that
would have served it. `accept` captures through the same function, so an unfed capture cannot be
written to a golden.

Fixture names carry the request method, which also reaches the committed `POST__` entry the previous
`GET__`-only rule left unreachable. A `.sse.txt` fixture is served as `text/event-stream` through
`cdp::Page::fulfill_raw`: a Server-Sent Events body is delimited by a literal blank line and does not
survive `serde_json`. `LIVE_SSE_FRAME` in the frontend's R-api tests embeds that same file, so the
bytes the gate serves and the bytes the DTO pins are one file.

Five fixtures close the corpus gaps: the CMS announcement list (published rows and a draft), the
caller's leave requests, the administrative leave queue, the server status stream, and the mission
detail the approvals drawer reads for its first queue row. Each carries an R-api round-trip test.

Twenty-one routes carry goldens re-sourced from the live Leptos render, one route at a time, each
with its own recorded justification in `manifest.json`; `notfound`, `eventmgr`, `callback` and
`login` still match their React captures unchanged. Every React capture is preserved alongside as
`<slug>.react.dom.json`. The divergences are fixture-fed data replacing React's unfed or mock states
and intended UI work that postdates the 2026-07-17 freeze; no route diverged because of a defect in
the Leptos pages. Two routes carried earlier accepted deltas (T-173 P4 on `missions`, T-159.25 on
`content`), and their justifications are carried forward rather than replaced.

## Working-tree handling

Work stays on `main`. Pre-existing mod/documentation changes are preserved. Source moves retain the pre-existing mod-compile comments. During tracked-file checks, ten pre-existing unrelated deletions are temporarily reflected in the index; their original index entries are retained in `preexisting-deleted-index.bin` and restored byte-for-byte after validation. The retained CI result reflects those intentional working-tree deletions; rerunning tracked-file checks with the restored index may require staging those deletions again. No production service deployment, database migration, or asset-format migration is performed.
