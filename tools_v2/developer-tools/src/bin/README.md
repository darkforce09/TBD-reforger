# Developer tool executables

The six executables of the `developer-tools` crate: the
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) script oracle, the headless browser gates, the
enfusion-mcp broker, the world-export and map-image pipelines, and [Mission
Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) captures. Developers run them by hand, and
xtask recipes and CI tasks run them by name.

## Contents

```text
tools_v2/developer-tools/src/bin/
├── capture.rs  the `capture` binary: Mission Creator screenshots, zoom sweeps and crops
├── enf.rs      the `enf` binary: symbol indexes, lookups and checks over Enfusion scripts
├── gate.rs     the `gate` binary: the headless browser gates of the single-page app
├── map.rs      the `map` binary: satellite, cartographic, label, water and glyph map assets
├── mcpd.rs     the `mcpd` binary: the persistent enfusion-mcp broker and its offline stub
└── world.rs    the `world` binary: the world-export pipeline and its verification gates
```

## How it works

Each file is a three-line `main` that calls one entry function of the `developer_tools` library and
returns its `ExitCode`; the `[[bin]]` tables of `tools_v2/developer-tools/Cargo.toml` name the
binaries. Argument parsing, help text and error reporting live in the owning module: five binaries
parse with clap's derive API, and `mcpd` reads its few flags itself.

```text
enf.rs      ──▶ enfusion_tooling::cli::entrypoint
gate.rs     ──▶ browser_testing::cli::run
capture.rs  ──▶ browser_testing::capture_cli::run
mcpd.rs     ──▶ enfusion_tooling::mcp_broker::run
world.rs    ──▶ world_export_pipeline::cli::entrypoint
map.rs      ──▶ map_raster_pipeline::cli::entrypoint
```

Default paths such as `apps/website/frontend/dist` and `.ai/artifacts/enf-index` are relative to the
working directory, so the commands run from the repository root.

## Commands

Each runs as `cargo run -q -p developer-tools --bin <name> -- <arguments>`. `--help` on a binary or
a subcommand prints its usage, and a clap usage error exits 2.

### enf

- Synopsis: `enf <COMMAND>`: `index`, `carve`, `apidoc`, `citations`, `extract`, `dump-entry`,
  `source`, `lookup`, `capability`, `dirs`.
- Does: builds TSV symbol indexes over Enfusion `.c` sources (`index crf|vanilla --root <dir>`),
  gets the vanilla scripts out of the game's paks (`extract`, `carve`) or the cached Script API
  pages (`apidoc`, `source`), and answers against the indexes (`lookup`, `dirs`); `citations` checks
  that every `@idx` marker under `documentation_v2/` resolves, and `capability` that every framework
  file has a verdict.
- Exit codes: 0 done; 1 a check failed (an unresolved citation, an unknown symbol, an untriaged
  file) or a step produced nothing; 2 an error or a usage error.
- Example: `cargo run -q -p developer-tools --bin enf -- citations`

### gate

- Synopsis: `gate <COMMAND>`: `v-suite`, `s-routes`, `smoke`, `editor-suite`, `doctor`, `r-auth`,
  `render-check`, `serve`.
- Does: drives headless Chromium over the DevTools protocol against the built app: the DOM oracle
  (`v-suite verify|accept`), the route-table drift check (`s-routes`), one Mission Creator smoke or
  the whole suite, the session-refresh gate (`r-auth`), a render check with an optional probe script
  and screenshot, and a static server with the cross-origin isolation headers; `doctor` is the
  preflight the others rely on, checked against `tools_v2/developer-tools/gate-env.json`.
- Exit codes: 0 green; 1 a gate failed; 2 usage; 3 a driver error.
- Example: `cargo run -q -p developer-tools --bin gate -- doctor`

### mcpd

- Synopsis: `mcpd --socket <path> [--pidfile <path>]` (the socket may come from `MCP_SOCK`, and the
  pid file defaults to `<socket>.pid`); `mcpd --stub`, or `MCP_STUB=1` without `--socket`, runs the
  offline stub instead. `--help` is not a flag: without a socket it prints `mcp-daemon: --socket
  required` and exits 2.
- Does: starts one enfusion-mcp server, initialises it once, and serves tool calls over a Unix
  socket one at a time; it stops after `MCP_DAEMON_IDLE` seconds idle (1800 by default) or
  `MCP_DAEMON_MAX_LIFE` seconds of life (14400). Stays in the foreground.
- Exit codes: 0 stopped by a signal, the idle limit or the lifetime; 1 the socket server failed; 2
  no socket was given, or the server failed to initialise.
- Example: `cargo xtask mcp daemon start`, which builds the binary and launches it.

### world

- Synopsis: `world <COMMAND>`; `--help` lists its eighteen subcommands.
- Does: turns a [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) world export into the committed
  terrain artifacts (object chunks, the catalogue, density grids, the road network, the elevation
  raster) and runs the gates that verify them, `verify-phase`, `validate-exports` and `reclassify`
  among them.
- Exit codes: 0 done; 1 an error or a failed check; 2 usage, or `verify-phase` without a staged
  export.
- Example: `cargo run -q -p developer-tools --bin world -- reclassify --terrain everon`

### map

- Synopsis: `map <COMMAND>`; `--help` lists its twenty-two subcommands.
- Does: builds the map assets a terrain serves (the orthophoto stitch, the satellite container and
  tile pyramids, the cartographic render, land cover, the glyph atlas, map labels, the inland-water
  archives) and verifies each.
- Exit codes: 0 done; 1 an error or a failed check; 2 usage.
- Example: `cargo run -q -p developer-tools --bin map -- verify-unified --terrain everon`

### capture

- Synopsis: `capture shot <out.png> <url> <waitMs> [url waitMs ...] [--canvas] [--hide-overlay]`,
  `capture zoomsweep <out-prefix> <mission-id> <zoom,zoom,...>`, `capture crop <img> <x> <y> <w> <h>
  [scale] [out]`.
- Does: drives the running Mission Creator in headless Chromium to write screenshots (`shot`) and to
  read the canvas at each zoom level (`zoomsweep`), and crops or upscales a region of a screenshot
  (`crop`); `shot` and `zoomsweep` need the database, the API and the app running.
- Exit codes: 0 capture written; 1 the capture produced nothing, or an error; 2 usage, or no valid
  zoom level.
- Example: `cargo run -q -p developer-tools --bin capture -- crop editor.png 0 0 400 300 2 crop.png`

## Boundaries

- Depends on: the `developer_tools` library modules in the diagram, in
  `tools_v2/developer-tools/src/`.
- Used by:
  - `cargo xtask mk gate-doctor` and `cargo xtask mk leptos-gates`, which run `gate doctor`, `gate
    editor-suite` and `gate v-suite verify`;
  - `cargo xtask mcp daemon` and `cargo xtask mcp selftest`, which build and launch `mcpd`;
  - `cargo xtask map export-terrain` and the platform wave gate, which run `world`;
  - the `map-water-everon`, `map-cartographic-everon` and `map-cartographic-verify` tasks of `cargo
    xtask ci`, which run `map`;
  - people, for `enf` and `capture`; `cargo xtask fetch vanilla-api` and `cargo xtask fetch
    vanilla-source` print the `enf` step that follows them.
- Rules: an entry file holds only `main` and its one call, and stays under 250 lines
  (`tooling_source_files_stay_below_their_structural_limits` in
  `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`); the six binary names are stable,
  because xtask recipes and CI tasks call them by `--bin <name>`
  (`the_tooling_tree_holds_its_executables_manifests_and_layout_modules`); a new binary adds its
  `[[bin]]` table and its file in the same change.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running `gate` and its wedge modes.
- [Editor capture](/documentation_v2/runbooks/editor_capture.md) — `capture` against a live Mission
  Creator.
- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the broker that
  `mcpd` runs.
