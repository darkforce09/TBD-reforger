# Developer tools

The `developer_tools` crate: the `developer_tools` library and eight executables (`enf`, `gate`,
`mcpd`, `world`, `map`, `capture`, `acknowledgement-dropping-relay`, `staging-load`) that do the
heavy offline work
around the platform. They index [Enfusion](/documentation/glossary/a_to_f.md#enfusion) scripts and
read the game's archives, run the headless browser gates of the single-page app and the [Mission
Creator](/documentation/glossary/g_to_m.md#mission-creator), compile building blueprints, build and
verify the terrain and map assets under `assets/`, and run the engines of the staging
verification receipts (the member load generator and the acknowledgement-dropping relay, whose
crates live in `tools/staging/`). Developers run the binaries, and `cargo xtask`
recipes, CI tasks and xtask verifications call them or the library.

## Contents

```text
tools/developer_tools/
├── Cargo.toml      the `developer_tools` package: the `developer_tools` library and eight `[[bin]]` targets
├── src/            the library modules and the eight entry points in `src/bin/`
└── test_fixtures/  blueprint, prefab and world-parity inputs the library's unit tests read
```

## How it works

Each file in `src/bin/` is a `main` that calls one subsystem's command-line entry in the library:
the `enfusion_script_index` crate for `enf`, the `enfusion_mcp_broker` crate for `mcpd`, `browser_testing` (the command lines over the `browser_gate_suites` crate) for `gate` and `capture`,
`world_export_pipeline` for `world`, `map_raster_pipeline` for `map`, and the
`acknowledgement_dropping_relay` and `staging_load_generator` crates for
`acknowledgement-dropping-relay` and `staging-load`. The `blueprint` and `map_verification` modules have no binary;
`cargo xtask map` and the xtask schema and map-asset verifications call their entry functions with
the checkout root. Every library subsystem reads the game's `.pak` archives through the `enfusion_pak` crate, resolves repository paths
through `repository_layout`, and takes its formats, geometry and spatial indexes from
`map_engine`, the one workspace crate it depends on; `xtask` depends on it in turn.

Binary formats, schema versions, numeric thresholds, operation order and the emitted bytes are
contracts the unit tests pin against the inputs in `test_fixtures/` and against synthetic data; the
browser gates compare against the fixtures of `tools/browser_testing/browser_gate_suites/`.

## Getting started

Run these from the repository root.

```bash
cargo build -p developer_tools --bins                 # the eight executables
cargo run -q -p developer_tools --bin map -- --help   # any binary's command list; likewise enf, gate, world, capture
cargo test -p developer_tools                         # the unit tests; no database, browser or game install
cargo xtask ci developer-tools-test                   # the CI lane: cargo test -p developer_tools --lib
cargo xtask mk leptos-gates                           # the browser gates: builds the app, gate doctor, the editor suite, gate v-suite verify
```

`cargo check -p developer_tools --all-targets` and `cargo fmt -p developer_tools --check` check the
crate without running it. The unit tests that need a real game install are ignored by default. The
browser gates need Chromium and the environment `gate doctor` checks; neither `cargo xtask ci
ci-local` nor the CI workflow runs them.

## Configuration

- `tools/browser_testing/browser_gate_suites/gate-env.json`: the pinned Chromium
  (`chromium.playwright_build`, `chromium.version`), the toolchain (`rustc`, `trunk`,
  `wasm_bindgen`) and the limits (`min_mem_available_mib`, `liveness_timeout_secs`). `gate doctor`
  reads it and warns on drift (fails with `--strict`); `cargo xtask ci ci-chrome` reads the
  Chromium version from it.
- Environment variables, all optional:

| Variable | Default | Read by |
|---|---|---|
| `ENFUSION_GAME_PATH` | `$HOME/.cache/enfusion-mcp-root` | `tools/enfusion/enfusion_pak/src/world_source.rs`: the game folder whose `addons/` holds the paks |
| `ENFUSION_MCP_BIN`, `MCP_SOCK`, `MCP_DAEMON_IDLE`, `MCP_DAEMON_MAX_LIFE`, `MCP_CALL_TIMEOUT`, `MCP_DEBUG`, `MCP_STUB`, `STUB_MODE`, `STUB_DAEMON`, `STUB_LINGER` | see the broker's README | `mcpd`, through the [Enfusion MCP broker](/tools/enfusion/enfusion_mcp_broker/README.md): the server to start, the socket, limits, logging and the offline stub |
| `CHROME_HEADLESS_SHELL` | the Chromium the harness finds | `tools/browser_testing/chrome_devtools_protocol/src/chromium_discovery.rs`: the browser executable |
| `PLAYWRIGHT_BROWSERS_PATH` | `~/.cache/ms-playwright` | `tools/browser_testing/chrome_devtools_protocol/src/chromium_discovery.rs`: the Playwright browser folder searched before the default cache |
| `LEPTOS_DIST` | `apps/frontend/dist` | `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/mutations.rs`: the built app `gate r-auth` serves without `--dist` |
| `TOKEN`, `REFRESH` | none; the smoke exits 2 without them | `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/mutations.rs`: dev-login tokens for `gate smoke mutations` |
| `PROFILE`, `ENFUSION_PROFILE_PATH` | none | `src/world_export_pipeline/export_preparation/export_profile.rs`: the [Workbench](/documentation/glossary/n_to_z.md#workbench) profile `world copy-export-profile` reads |

## Public surface

- The eight binaries; their commands are in the [executables
  README](/tools/developer_tools/src/bin/README.md).
- The library modules `tools/xtask/` imports: `blueprint` and `map_verification` entry functions,
  `repository_layout` paths and
  `world_export_pipeline::INSTANCE_KINDS` and `vegetation_density`; the [source tree
  README](/tools/developer_tools/src/README.md) lists them.

## Boundaries

- Depends on: `map_engine` (`legacy/map_engine`, with its `world` and `streaming` features); the world crates under `crates/` it imports directly (spatial indexes,
  world formats, terrain, world objects, line of sight, label layout); the pinned enfusion-mcp npm package in `tools/enfusion_mcp_node_package/`;
  Chromium for the browser gates; an Arma Reforger install or its cached `addons/` for the pak
  readers; and the crates `Cargo.toml` lists.
- Used by: `tools/xtask/` (its `Cargo.toml` depends on this crate, and its `mk`, `ci`, `map`,
  `mcp`, `db`, `mod`, `schema` and `platform` commands call the library or the binaries); the CI
  workflow `.github/workflows/ci.yml`, through `cargo xtask ci developer-tools-test`; and people.
- Rules: the crate never depends on `xtask` and `xtask` depends on it by path
  (`tooling_dependency_direction_is_enforced`); the eight binary names are fixed
  (`the_tooling_tree_holds_its_executables_manifests_and_layout_modules`); files stay under 500
  lines, test files under 1,000, `src/bin/` files under 250 and editor smoke scenarios under 450,
  and tests live in separate `tests/` files
  (`tooling_source_files_stay_below_their_structural_limits` and
  `tooling_test_modules_live_in_separate_files`, all in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`); every tracked file here is held to
  the prose rules of `tools/checks/repository_checks/src/tests/tooling_prose_rules.rs` (no ticket ids, no history
  words, no retired names, no script file names, every `.rs` named exists), with `test_fixtures/`
  exempt from the ticket-id and Rust-file-name rules.

## Related documentation

- [Editor gates](/documentation/runbooks/editor_gates.md) — running `gate` and its wedge modes.
- [Editor capture](/documentation/runbooks/editor_capture.md) — `capture` against a live Mission
  Creator.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the broker that
  `mcpd` runs.
- [Map asset commands](/tools/xtask/src/commands/map/README.md) — the xtask commands that wrap
  `world` and the blueprint compiler.
- [Developer tools documentation](/documentation/tools/developer_tools/README.md) — the
  map raster pipeline and the executables' index.
- [Terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md) —
  the world export flow from Workbench to the committed terrain.
