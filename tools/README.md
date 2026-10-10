# Developer tooling

Every developer tool in the repository: the `cargo xtask` command line, the tool crates its
command groups dispatch onto (grouped by category), the heavy offline tools, and the npm package
that pins the Enfusion MCP server. Developers, AI agents, the
CI workflows and the host's timers run them; the products they build, check and deploy are the
crates under `crates/` and the game mod in `mod/`.

## Contents

```text
tools/
├── browser_testing/            the headless browser gate crates: the DevTools protocol client, and the gate suites with the `gate` and `capture` command lines
├── checks/                     the repository verification crates: repository checks, mod script checks and documentation checks
├── commands/                   the crates behind the `cargo xtask` command groups: ci, platform, mod, db, deploy, staging, schema, relocation and the rest
├── developer_tools/            the eight one-line tool binaries: enf, gate, mcpd, world, map, capture, the staging relay and load
├── enfusion/                   the Enfusion crates: the `.pak` reader, the script index, the MCP broker
├── enfusion_mcp_node_package/  the npm package that pins the `enfusion-mcp` server
├── foundation/                 verdicts and the verification lock, child processes, the repository laws, the layout, the deploy settings, the ticket manager client, the test locks
├── map_assets/                 the map asset crates: the blueprint compiler, the world export, raster and verification pipelines
├── staging/                    the staging crates: the load plan, the load generator, the acknowledgement-dropping relay
└── xtask/                      the `cargo xtask` command line and dispatch, and the groups without a crate of their own
```

## How it works

`cargo xtask` is the alias `run --package xtask --` in `.cargo/config.toml`. The `xtask` binary
parses the command line and dispatches every command group; the verifications, the CI tasks, the
deploys and the platform's agent, worktree and wave orchestration live in the crates below, and
xtask keeps only the `ai`, `fetch`, `map`, `refactor`, `schema` and `verify` groups as modules of
its own. It passes each crate the checkout root:

- The [foundation crates](/tools/foundation/README.md) hold what every tool builds on:
  `verification_core` the fail-closed primitives the gates share (verdicts and findings, pattern
  scans, the run report and the `flock` on the shared verification lock), `process_runner` child
  processes with process-group isolation and deadlines, the container-to-host bridge and the ssh
  transport, `repository_laws` the structural engineering laws as pure checks,
  `repository_layout` the shared locations, `deploy_settings` the
  reader of `deploy/deploy.env`, `ticket_manager_client` the typed client of the central ticket
  manager's `ttm` command line, which holds the [tickets](/documentation/glossary/n_to_z.md#ticket),
  run receipts and the [wave](/documentation/glossary/n_to_z.md#wave) plan, and
  `tool_test_support` the locks and the checkout root the tool tests share (a dev-dependency
  only).
- The [command crates](/tools/commands/README.md) hold the work of xtask command groups:
  `workstation_setup` the `cargo xtask setup` commands; `enfusion_mcp` the Enfusion MCP client
  behind `cargo xtask mcp`;
  `agent_context_guards` the agent tool-call guard and the filtered command runner behind
  `cargo xtask ai`;
  `repository_relocation` the manifest-driven moves and the retired-spelling verification behind
  `cargo xtask refactor relocate`; `schema_tooling` the contract codegen, the contract schema
  gates and the ORBAT slot flattening behind `cargo xtask schema` and `cargo xtask gen`;
  `ballistics_oracle_tooling` the ballistics catalog trim behind `cargo xtask ballistics`;
  `database_operations` the local database lane, the verified backup, the guarded restore and
  the restore drill behind `cargo xtask db` and `cargo xtask deploy db`, with the database source
  checks; `remote_debugging` the staging server-join probes, the remote log verdict and the
  mission upload reproduction behind `cargo xtask debug`, `cargo xtask repro` and
  `cargo xtask mod remote-logs`; `staging_procedures` the staging acceptance harness, its host
  actions and recorded receipts behind `cargo xtask staging`; and `deployment` the website and
  staging deploys behind `cargo xtask deploy`; and `ci_task_catalog` the CI task table and
  runner, the build lane's recipes, the cargo target pin and the CI workflow checks behind
  `cargo xtask ci`, `cargo xtask help` and `cargo xtask mk`; and `platform_execution` the
  platform factory behind `cargo xtask platform`: the wave driver, slice runs, slice worktrees
  and the preflight; and `mod_operations` the game mod's compile gate, world boot, playtest
  server, equipment export publication and mod wave driver behind `cargo xtask mod`.
- The [check crates](/tools/checks/README.md) hold the repository verifications behind
  `cargo xtask verify`: `repository_checks` the structural, language-ban, licensing and registry
  checks, `mod_script_checks` the Enfusion mod script checks and the Workbench spawn runs, and
  `documentation_checks` the link-check gate.
- The [Enfusion crates](/tools/enfusion/README.md) hold the libraries that read and drive the
  Enfusion engine's formats and tools: `enfusion_pak` the `.pak` archive reader, `enfusion_script_index`
  the script oracle the `enf` binary runs and the vanilla page mirrors behind `cargo xtask fetch`,
  and `enfusion_mcp_broker` the `mcpd` broker over one enfusion-mcp server, whose client behind
  `cargo xtask mcp` is the command crate `enfusion_mcp`.
- The [browser testing crates](/tools/browser_testing/README.md) hold the headless browser gates of
  the single-page app: `chrome_devtools_protocol` the client that launches Chromium and drives its
  pages, and `browser_gate_suites` the static server, the Mission Creator smokes, the data viewer gate, the offline mortar and ballistics agreement gates,
  the capture rig, the doctor and the `gate` and `capture` command lines, which the `gate` and
  `capture` binaries of `developer_tools` call.
- The [staging crates](/tools/staging/README.md) hold the engines of the staging verification
  receipts: `staging_load_plan` the member load's plan, report and their JSON codec, which the
  `staging_procedures` load procedure builds and judges; `staging_load_generator` the load itself,
  which the procedure runs as `developer_tools`' `staging-load` child process; and
  `acknowledgement_dropping_relay` the fault-injecting relay behind
  `acknowledgement-dropping-relay` on the staging host.
- `developer_tools` holds eight executables (`enf`, `gate`, `mcpd`, `world`, `map`, `capture`,
  `acknowledgement-dropping-relay`, `staging-load`), each a one-line `main` over one tool crate;
  it has no library and no crate depends on it. The map asset verification is
  `map_asset_verification` (tools/map_assets), which the CI task catalogue's map asset steps and
  the `cargo xtask schema`, `verify` and `map world-los` commands call. The world-export pipeline
  is `world_export_pipeline` (tools/map_assets), which the `world` binary runs and whose export
  driver and map tile index writer the `cargo xtask map export-terrain` and `tile-index` commands
  call, and the map raster
  pipeline is `map_raster_pipeline` (tools/map_assets), which the `map` binary runs. The
  building-blueprint compiler is `blueprint_compiler` (tools/map_assets), which the `cargo xtask
  map` commands call.
- xtask and `developer_tools` are the two binary packages; both depend only on tool crates (the
  checkout-root finder comes through `repository_layout`'s prelude; `tooling_dependency_direction_is_enforced`), and no tokio, axum, reqwest, resvg or image enters
  xtask's dependency closure (rule 6 of `cargo xtask verify crate-tiers`): the async servers and
  the raster crates run behind the `developer_tools` binaries, which xtask starts as child
  processes.
- `enfusion_mcp_node_package` is data, not a crate: `npm ci` there installs the pinned server that
  `mcpd` and `cargo xtask mcp` start. It sits outside every crate root, so no crate-scoped walk
  reads its `node_modules/`.

```text
xtask ──runs──▶ developer_tools (binaries only)
  │               ├── mcpd ──▶ enfusion_mcp_broker (tools/enfusion) ──starts──▶ enfusion_mcp_node_package
  │               ├── enf, map pipelines ──▶ enfusion_script_index, enfusion_pak (tools/enfusion)
  │               ├── world, map pipelines ──▶ world_export_pipeline (tools/map_assets) ──▶ enfusion_pak, prefab_catalog
  │               ├── map ──▶ map_raster_pipeline (tools/map_assets) ──▶ world_export_pipeline, enfusion_pak
  │               ├── gate, capture ──▶ browser_gate_suites ──▶ chrome_devtools_protocol (tools/browser_testing)
  │               └── staging-load, acknowledgement-dropping-relay ──▶ staging_load_generator, acknowledgement_dropping_relay (tools/staging)
  ├────▶ staging_procedures (tools/commands), the staging group ──▶ staging_load_plan (tools/staging), the plan crate under staging_load_generator
  ├────▶ command crates (tools/commands); enfusion_mcp: mcp; schema_tooling ──▶ prefab_catalog (INSTANCE_KINDS)
  ├────▶ ci_task_catalog (tools/commands): ci, help, mk ──▶ check crates, command crates, map_asset_verification (map asset steps)
  ├────▶ platform_execution (tools/commands): platform ──▶ ci_task_catalog, ticket_manager_client (tools/foundation) ──runs──▶ ttm
  ├────▶ mod_operations (tools/commands): mod ──▶ platform_execution, command crates, mod_script_checks, ticket_manager_client
  ├────▶ enfusion_script_index (tools/enfusion): cargo xtask fetch
  ├────▶ blueprint_compiler (tools/map_assets): map blueprint, BVH and model commands ──▶ enfusion_pak
  ├────▶ map_asset_verification (tools/map_assets): map world-los, the schema and verify map asset gates ──▶ world_export_pipeline
  ├────▶ world_export_pipeline (tools/map_assets): map export-terrain (runs the world binary), map tile-index
  ├────▶ check crates (tools/checks)
  └────▶ process_runner, repository_laws ──▶ verification_core   (tools/foundation)
```

The repository paths more than one tool names live in `tools/foundation/repository_layout`; the
one walk that finds the checkout root is the foundation crate `crates/foundation/repository_root`. Each tool spells the paths only it uses in its own layout
module: `tools/map_assets/map_raster_pipeline/src/decision_record_locations.rs`,
`tools/map_assets/world_export_pipeline/src/export_locations.rs`,
`tools/enfusion/enfusion_script_index/src/script_index_layout.rs` and
`tools/browser_testing/browser_gate_suites/src/gate_layout.rs`.

## Getting started

Run these from the repository root:

```bash
cargo xtask --help                                 # every command group
cargo test -p repository_checks -p mod_script_checks -p documentation_checks   # the check crates' tests
cargo test -p ticket_manager_client               # the ticket manager client's tests
cargo test -p verification_core -p process_runner -p repository_laws   # the foundation crates' tests
cargo xtask mod dev-bootstrap                      # a mod workstation: npm ci for the MCP server, Workbench
```

Each crate's README lists its own commands and checks.

## Boundaries

- Depends on: the library crates under `crates/` whose `targets` is `any`; the checkout's data
  (`.ai/artifacts/`, `contracts/`, `assets/`, `documentation/`); and the external tools
  individual commands run: git, Docker or Podman, Postgres, Chromium, Trunk, npm and Node.js, ssh
  and rsync, the Arma Reforger tools, and the central ticket manager's `ttm`.
- Used by: developers and AI agents at the command line; the GitHub workflows in `.github/`; the
  backup and backup-drill units in `deploy/systemd/`, which run
  `cargo xtask deploy db backup` and `cargo xtask deploy db drill`.
- Rules, over every tool crate (each `tools/<name>` and `tools/<category>/<name>` holding a
  `Cargo.toml`):
  - `developer_tools` never depends on `xtask`, and `xtask` never depends on
    `developer_tools`;
  - a `tools/foundation` crate depends only on lower `tools/foundation` crates and on
    `repository_root`; and only `ticket_manager_client` runs `ttm`;
  - in production source only a layout module — a file of the `repository_layout` crate, or a
    file whose first line declares it one (`//! The repository locations only …`) — spells a
    repository path literal, such as one under `.ai/` or `documentation/`; no tracked tooling
    file carries a retired spelling, a script file name or a history word, and no document or
    production source a ticket id;
  - every tooling crate's source files stay within their line limits
    (`cargo xtask verify file-length` reports them).

## Related documentation

- [Tooling architecture](/documentation/tools/tooling_architecture.md) — the crates, their
  dependency direction, the invariants and the verification surface.
- [Tooling documentation](/documentation/tools/README.md) — the index of the deeper
  documents on each crate.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the MCP daemon and
  call path.
- [Factory waves](/documentation/runbooks/factory_waves/README.md) — running waves of tickets
  with the xtask platform commands.
