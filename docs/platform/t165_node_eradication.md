# T-165 — Node/JS eradication: every .mjs tool → Rust

**Status:** ACTIVE (T-165.0–.4 shipped; next T-165.5) · **Executor:** claude-code (operator-approved
plan) · **Sequel to:** T-161/T-162 (Python eradication). **Plan of record:** the operator-approved
T-165 plan (session plan file); this hub tracks slice state + evidence.

## In one sentence

Port all repo-authored Node tooling (90 .mjs, 13.8k LOC — schema toolchain, map-asset pipeline,
T-159 CDP gate harness, MCP broker, misc verifies) to Rust; the only Node left standing is the
third-party `enfusion-mcp` runtime for Workbench (the floor), and no CI job needs Node at all.

## Locked decisions

| ID | Decision |
|----|----------|
| **N1** | `xtask` stays sync (schema gates, citations, codegen, small verifies); new async crate **`tools/tbd-tools`** owns CDP harness (`gate`), MCP broker (`mcpd`), map/world pipelines. |
| **N2** | `freeze.js`/`dom.js` are browser-INJECTED payloads — they survive as **verbatim `const &str`** in Rust; never re-implemented natively (frozen V-golden byte-parity by construction). |
| **N3** | Image policy: no external processes. Pure-Rust encoders everywhere except the **one lossy-WebP leg** (map-view pyramid; `webp` crate = cargo-vendored libwebp C). Lossless legs = `image-webp` (pure Rust). Committed assets untouched (no gate byte-compares WebP). |
| **N4** | Per-slice `.mjs` deletion is gated on the **reverse-dependency edge list** (createRequire borrowers of tbd-schema node_modules + cross-script path spawns). tbd-schema npm dies at the END of T-165.9 (last pngjs borrower). |
| **N5** | T-165.8 world artifacts: one-time migration — Rust double-build determinism + **decompressed-content byte-identity** vs committed + exact census (1623/1,216,109/315/888/36/625) → re-commit Rust-encoded artifacts → E6 stays raw-byte thereafter (node-zlib vs flate2 gz bytes differ by construction). |
| **N6** | mcp lane: broker+stub port only (`xtask mcp` client already Rust); selftest contract frozen — byte-exact stdout + exit codes 0/1/2/3/4, 19/19. |
| **N7** | quicktype → typify for the 4 tractable schemas; `loadout.rs` becomes hand-maintained (quicktype output provably lossy — empty `Wear{}`/`Equipment{}`; zero consumers) with serde round-trip tests vs fixtures. |

## Slice ladder

| Slice | Scope | Status |
|-------|-------|--------|
| **T-165.0** | `tools/tbd-tools` scaffold (workspace member; tokio + tokio-tungstenite pinned to lock-resolved versions) + dead-set deletion | **shipped** |
| **T-165.1** | Text/JSON gates → `cargo xtask schema …` (citations, t090-specs, n6, n10, map-object-enums, type-inventory, terrain-manifest, flatten-orbat-slots); parity proven side-by-side (8× verdict+exit MATCH, negative probes, count parity); Makefile + ci.yml + contracts.yml citation steps → cargo; 7 .mjs deleted (verify-type-inventory.mjs kept — spawned by census-types/validate-export until .8) | **shipped** |
| **T-165.2** | validate suite → `cargo xtask schema validate` + `validate-file` (Registry-resolved cross-file $refs, FK walkers, ENF-4 `$defs` pointer validators). Parity: **130/130 PASS both runners, label-set diff empty, negative probe rc=1 both**. ci.yml schema job + schema.yml + deploy-staging V1 → cargo (schema CI is Node-free); validate.mjs + validate-file.mjs deleted | **shipped** |
| **T-165.3** | codegen → `cargo xtask schema codegen` (typify; run-to-run hash-idempotent; regenerated 4 contracts — `regress`-validated patterns, an upgrade over quicktype). `loadout.rs` hand-frozen: faithful versioned oneOf + patternProperties wear/equipment maps + double-Option null-vs-absent, value-level round-trip tests vs BOTH fixtures. `registry_import.rs` adapted; contracts.yml codegen-drift job Node-free; codegen.mjs deleted; quicktype devDep dropped | **shipped** |
| **T-165.4** | Golden S-gates + terrain/DEM + labels → Rust. .4a tbd-tools libs (geometry/density/forest) · .4b S2–S14 golden gate (12/12 parity; S13 TBDD encode byte-identity; S14 js_num Value-equality) · .4c glyphs GL-G1..G6 + height-labels with the wasm-era branch RESTORED natively (declutter + ASL oracle — Node permanently skips these) · .4 close: locations G3–G7 + town-labels G1–G5+fade + road-names G3–G7 gates on `map-engine-core::world`, terrain-alignment (png-crate u16 decode + core `sample_elevation_meters`, **byte-identical** output incl. `js_fixed3` toFixed tie semantics; ±probe), t152 aggregator → `runCargo`, `verify-terrain`/`-strict` Makefile npm lines → xtask (**Makefile npm surface = 0**), 7 .mjs deleted (verify-{locations,height-labels,town-labels,road-names,terrain-alignment} + height-labels-export + dem-sample libs). KEPT per edge list: raw-u16-to-dem-png (spawner ports @ .8), locations-export lib (export-locations), verify-type-inventory (census-types), tbd-schema npm (ajv/pngjs borrowers → .9) | **shipped** |
| T-165.5 | CDP harness core (`cdp`/`serve`/`inject`/`diff_node` + `gate v-suite`) — 25/25 byte-count parity vs Node side-by-side | queued |
| T-165.6 | Smokes suite registry → Rust; Node driver deleted; `make leptos-gates` = cargo | queued |
| T-165.7 | MCP broker + `--stub` → Rust; runner-resolution fix in mcp-call.sh/mcp-daemon.sh; selftest 19/19 | queued |
| T-165.8 | World-export pipeline (decode-topo/edds + bcdec_rs; vendor wasm/C deleted) + E6 migration (N5) | queued |
| T-165.9 | Image pipeline pure Rust (MVG→resvg; image-webp/webp per N3; tile pyramid; seam tools; verifiers); tbd-schema npm deleted | queued |
| T-165.10 | Bash logic + closure: 0 tracked .mjs; `verify no-node` gate; zero setup-node in CI; doc sync | queued |

## T-165.0 — dead-set deletion evidence (deleted, not ported; ~650 LOC off the port surface)

| File | Evidence it was dead |
|------|---------------------|
| `driver/gate_v.mjs` | live-React-oracle V diff — React deleted at T-159.29.3; superseded by the frozen `gate_v_suite.mjs` |
| `driver/spike_upload_140mb.mjs` | self-labeled transport-spike evidence (T-159.24 verify log records the result); not a suite gate |
| `manifests/extract-react.mjs` | first read is `apps/website/frontend/src/router.tsx` — tree deleted; the 5 S-manifests are frozen goldens (t159_gates README §Status) |
| `scripts/website/differential.mjs` + `run-differential.sh` + `differential_seed.sql` | T-145 Go-vs-Rust differential — the Go backend it boots no longer exists |
| `scripts/website/tools/scrape-eden-wiki.mjs` | one-shot scraper; outputs committed under `.ai/artifacts/eden-wiki/` |
| `scripts/verify-monorepo-migration.sh` (+ `make verify-migration`) | **already red**: V15 runs `go build ./...` in `apps/website` (zero `.go` files since T-145); migration long complete |

Recovery: all in git history @ tag T-164 and earlier.
