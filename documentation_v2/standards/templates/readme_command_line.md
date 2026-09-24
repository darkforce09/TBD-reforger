**Status:** live

# README template: command-line

**When to use:** the entry points of executables, or one command group: a crate's `src/bin/`, or a
command group folder such as `tools_v2/xtask/src/commands/db/`. The
[README standard](/documentation_v2/standards/readme_standard.md) defines every rule this template
follows; the command-line kind adds Commands.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. Commands holds
one `###` heading per command the folder defines, with the same four bullets under each.

````markdown
# <Name of the tools or the command group, in plain words: no path, no backticks>

<One to three sentences: what the commands are for and who runs them.>

## Contents

```text
<repository path of the folder>/
├── <entry file>  <the command it defines, in one phrase: a lowercase phrase, no closing period>
└── tests/        <what the tests cover>
```

## How it works

<How an invocation flows: where the arguments are parsed, where the work runs, how the exit code
is formed, and what the files share. An ASCII diagram in a text block helps here.>

## Commands

<One line on how to run the commands, if they share a launcher.>

### <command name>

- Synopsis: `<the usage line, as the command's own --help prints it>`
- Does: <what it does, naming its subcommands when it has them>
- Exit codes: <each code and what it means, as the code returns them>
- Example: `<one real invocation, run from the repository root>`

## Boundaries

- Depends on: <the modules and crates the entry points call, read from their imports>
- Used by: <the xtask recipes, CI tasks and people that run the commands, found with git grep>
- Rules: <the invariants a change here must keep: stable names, where parsing lives, and the gate
  or test that checks each>

## Related documentation

- [<runbook or document title>](/documentation_v2/<path to the document>) — <what it covers>
````

## Worked sample

Written from `tools_v2/developer-tools/src/bin/`. Each synopsis and exit code comes from the
binary's own `--help` and entry function; the sample sits in a fenced block, so no gate reads it as
a README, and the folder's own README.md is written from the same code and may differ.

````markdown
# Developer tool executables

The six executables of the `developer-tools` crate: Enfusion script and pak tooling, the headless
browser gates, the Enfusion MCP broker, the world-export and map-image pipelines, and Mission
Creator captures.

## Contents

```text
tools_v2/developer-tools/src/bin/
├── capture.rs  the `capture` binary: Mission Creator screenshots, zoom sweeps and crops
├── enf.rs      the `enf` binary: symbol indexes and lookups over Enfusion script sources and paks
├── gate.rs     the `gate` binary: the headless browser gates of the single-page app
├── map.rs      the `map` binary: satellite, cartographic, label and water map assets
├── mcpd.rs     the `mcpd` binary: the persistent Enfusion MCP broker and its offline stub
└── world.rs    the `world` binary: the world-export pipeline and its verification gates
```

## How it works

Each file is a three-line `main` that calls one entry function of the `developer_tools` library
and returns its `ExitCode`; the `[[bin]]` tables of `tools_v2/developer-tools/Cargo.toml` name the
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

## Commands

Each runs as `cargo run -q -p developer-tools --bin <name> -- <arguments>`. `--help` on a binary or
a subcommand prints its usage, and a clap usage error exits 2.

### enf

- Synopsis: `enf <COMMAND>`: `index`, `carve`, `apidoc`, `citations`, `extract`, `dump-entry`,
  `source`, `lookup`, `capability`, `dirs`.
- Does: builds symbol indexes over Enfusion `.c` sources, extracts vanilla scripts from the game's
  paks and the cached Script API docs, and answers lookups against the indexes; `citations` checks
  that every `@idx` citation under `documentation_v2/` resolves.
- Exit codes: 0 done; 1 a check failed (an unresolved citation, an unknown symbol, a script file
  without a capability verdict); 2 an error.
- Example: `cargo run -q -p developer-tools --bin enf -- citations`

### gate

- Synopsis: `gate <COMMAND>`: `equipment-data-viewer`, `v-suite`, `s-routes`, `smoke`,
  `editor-suite`, `doctor`, `r-auth`, `render-check`, `serve`.
- Does: drives headless Chromium over the DevTools protocol against the built app: the DOM-oracle
  and route-drift gates, the Mission Creator smoke suite, the session-refresh gate, a render check,
  and a static server with the cross-origin isolation headers; `doctor` is the preflight the others
  rely on.
- Exit codes: 0 green; 1 a gate failed; 2 usage; 3 a driver error.
- Example: `cargo run -q -p developer-tools --bin gate -- doctor`

### mcpd

- Synopsis: `mcpd --socket <path> [--pidfile <path>]`; `mcpd --stub`, or `MCP_STUB=1`, runs the
  offline stub instead.
- Does: spawns one enfusion-mcp child, initialises it once, and serves tool calls over a Unix
  socket one at a time; it stops after `MCP_DAEMON_IDLE` seconds idle (1800 by default) or
  `MCP_DAEMON_MAX_LIFE` seconds of life (14400).
- Exit codes: 0 stopped by a signal, the idle limit or the lifetime; 1 the socket server failed;
  2 no socket was given, or the child failed to initialise.
- Example: `cargo xtask mcp daemon start`, which builds the binary and launches it.

### world

- Synopsis: `world <COMMAND>`; `--help` lists its eighteen subcommands.
- Does: turns a Workbench world export into the committed terrain artifacts (object chunks, the
  catalogue, density grids, the road network, the elevation raster) and runs the gates that verify
  them, `verify-phase` and `reclassify` among them.
- Exit codes: 0 done; 1 an error or a failed check; 2 usage, or `verify-phase` without a staged
  export.
- Example: `cargo run -q -p developer-tools --bin world -- reclassify --terrain everon`

### map

- Synopsis: `map <COMMAND>`; `--help` lists its twenty-two subcommands.
- Does: builds the map assets a terrain serves (the satellite container and tile pyramids, the
  cartographic ortho, land cover, the glyph atlas, map labels, the inland-water archives) and
  verifies each.
- Exit codes: 0 done; 1 an error or a failed check; 2 usage.
- Example: `cargo run -q -p developer-tools --bin map -- verify-unified --terrain everon`

### capture

- Synopsis: `capture shot <out.png> <url> <waitMs> [url waitMs ...] [--canvas] [--hide-overlay]`,
  `capture zoomsweep <out-prefix> <mission-id> <zoom,zoom,...>`,
  `capture crop <img> <x> <y> <w> <h> [scale] [out]`.
- Does: drives the running Mission Creator in headless Chromium to write screenshots (`shot`) and
  to read the canvas at each zoom level (`zoomsweep`), and crops or upscales a region of a
  screenshot (`crop`); `shot` and `zoomsweep` need the database, the API and the app running.
- Exit codes: 0 capture written; 1 the capture produced nothing; 2 usage.
- Example: `cargo run -q -p developer-tools --bin capture -- shot editor.png http://127.0.0.1:3000/missions/<id>/edit 8000`

## Boundaries

- Depends on: the `developer_tools` library modules in the diagram, in
  `tools_v2/developer-tools/src/`.
- Used by:
  - `cargo xtask mk gate-doctor` and `cargo xtask mk leptos-gates`, which run `gate doctor`,
    `gate editor-suite` and `gate v-suite verify`;
  - `cargo xtask mcp daemon` and `cargo xtask mcp selftest`, which build and launch `mcpd`;
  - `cargo xtask map export-terrain` and the platform wave gate, which run `world`;
  - the `map-water-everon`, `map-cartographic-everon` and `map-cartographic-verify` tasks of
    `cargo xtask ci`, which run `map`;
  - people, for `enf` and `capture`; `cargo xtask fetch vanilla-api` and
    `cargo xtask fetch vanilla-source` print the `enf` step that follows them.
- Rules: an entry file holds only `main` and its one call, and parsing, help and cleanup stay in
  the owning module; the binary names are stable, because xtask recipes and CI tasks call them by
  `--bin <name>`; a new binary adds its `[[bin]]` table and its file in the same change.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running `gate` and its wedge modes.
- [Editor capture](/documentation_v2/runbooks/editor_capture.md) — `capture` against a live
  Mission Creator.
- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the broker that
  `mcpd` runs.
````
