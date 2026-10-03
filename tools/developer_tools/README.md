# Developer tools

The `developer_tools` package: eight executables (`enf`, `gate`, `mcpd`, `world`, `map`,
`capture`, `acknowledgement-dropping-relay`, `staging-load`), each a one-line `main` over the tool
crate that does the heavy offline work around the platform. They index [Enfusion](/documentation/glossary/a_to_f.md#enfusion) scripts and
read the game's archives, run the headless browser gates of the single-page app and the [Mission
Creator](/documentation/glossary/g_to_m.md#mission-creator), build and verify the terrain and map assets under `assets/`, and run the engines of the staging
verification receipts (the member load generator and the acknowledgement-dropping relay, whose
crates live in `tools/staging/`). Developers run the binaries, and `cargo xtask`
recipes and CI tasks run them by name; the package has no library.

## Contents

```text
tools/developer_tools/
├── Cargo.toml      the `developer_tools` package: eight `[[bin]]` targets and the tool crates they call
└── src/            the eight entry points in `src/bin/`
```

## How it works

Each file in `src/bin/` is a `main` that calls one tool crate's command-line entry:
the `enfusion_script_index` crate for `enf`, the `enfusion_mcp_broker` crate for `mcpd`, the `browser_gate_suites` crate's `command_lines` for `gate` and `capture`,
the `world_export_pipeline` crate for `world`, the `map_raster_pipeline` crate for `map`, and the
`acknowledgement_dropping_relay` and `staging_load_generator` crates for
`acknowledgement-dropping-relay` and `staging-load`. The tool crates read the game's `.pak`
archives through the `enfusion_pak` crate and resolve repository paths through
`repository_layout`; their tests live with them, and `cargo xtask ci workspace-member-tests` runs
them. No crate depends on this package, `xtask` included.

Binary formats, schema versions, numeric thresholds, operation order and the emitted bytes are
contracts the unit tests pin against the blueprint compiler's fixtures
(`tools/map_assets/blueprint_compiler/test_fixtures/`) and against synthetic data; the
browser gates compare against the fixtures of `tools/browser_testing/browser_gate_suites/`.

## Getting started

Run these from the repository root.

```bash
cargo build -p developer_tools --bins                 # the eight executables
cargo run -q -p developer_tools --bin map -- --help   # any binary's command list; likewise enf, gate, world, capture
cargo xtask ci workspace-member-tests                 # the CI lane: builds these binaries and tests the tool crates they call
cargo xtask mk leptos-gates                           # the browser gates: builds the app, gate doctor, the editor suite, gate v-suite verify
```

`cargo check -p developer_tools --all-targets` and `cargo fmt -p developer_tools --check` check the
package without running it. The browser gates need Chromium and the environment `gate doctor` checks; neither `cargo xtask ci
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
| `PROFILE`, `ENFUSION_PROFILE_PATH` | none | `tools/map_assets/world_export_pipeline/src/export_preparation/export_profile.rs`: the [Workbench](/documentation/glossary/n_to_z.md#workbench) profile `world copy-export-profile` reads |

## Public surface

- The eight binaries; their commands are in the [executables
  README](/tools/developer_tools/src/bin/README.md). The package has no library.

## Boundaries

- Depends on: the seven tool crates `Cargo.toml` lists, one per binary (the `gate` and `capture`
  binaries share `browser_gate_suites`); through them, the pinned enfusion-mcp npm package in
  `tools/enfusion_mcp_node_package/`, Chromium for the browser gates, and an Arma Reforger install
  or its cached `addons/` for the pak readers.
- Used by: the `cargo xtask` recipes and CI tasks that run a binary by `--bin <name>` (`mk`,
  `ci`, `map`, `mcp`, `mod`, `deploy`, `staging` and `platform`); the CI workflow
  `.github/workflows/ci.yml`, whose `workspace-members` job builds the binaries; and people.
- Rules: the package is binary-only, depends on tool crates alone, never on `xtask` or a member
  under `legacy/`, and no workspace member depends on it (`tooling_dependency_direction_is_enforced`;
  the strangler law); the eight binary names are fixed
  (`the_tooling_tree_holds_its_executables_manifests_and_layout_modules`); files stay under 500
  lines, test files under 1,000, `src/bin/` files under 250 and editor smoke scenarios under 450,
  and tests live in separate `tests/` files
  (`tooling_source_files_stay_below_their_structural_limits` and
  `tooling_test_modules_live_in_separate_files`, all in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`); every tracked file here is held to
  the prose rules of `tools/checks/repository_checks/src/tests/tooling_prose_rules.rs` (no ticket ids, no history
  words, no retired names, no script file names, every `.rs` named exists).

## Related documentation

- [Editor gates](/documentation/runbooks/editor_gates.md) — running `gate` and its wedge modes.
- [Editor capture](/documentation/runbooks/editor_capture.md) — `capture` against a live Mission
  Creator.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the broker that
  `mcpd` runs.
- [Map asset commands](/tools/xtask/src/commands/map/README.md) — the xtask commands that wrap
  `world`.
- [Developer tools documentation](/documentation/tools/developer_tools/README.md) — the
  executables' index.
- [Map raster pipeline](/documentation/tools/map_assets/map_raster_pipeline.md) — the `map`
  lanes in depth.
- [Terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md) —
  the world export flow from Workbench to the committed terrain.
