# Developer tooling

Every developer tool in the repository: the `cargo xtask` command line, the tool crates its
command groups dispatch onto (grouped by category), the heavy offline tools, and the npm package
that pins the
Enfusion MCP server. Developers, AI agents, the CI workflows and the host's timers run them; the
products they build, check and deploy live in `apps/`.

## Contents

```text
tools/
├── browser_testing/            the headless browser gate crates: the DevTools protocol client and the gate suites
├── checks/                     the repository verification crates: repository checks, mod script checks and documentation checks
├── commands/                   the crates behind the `cargo xtask` command groups: ci, platform, mod, db, deploy, staging, schema, relocation and the rest
├── developer_tools/            the heavy executables, blueprints, the world export and map pipelines, the ballistics and mortar gate suites
├── enfusion/                   the Enfusion crates: the `.pak` reader, the script index, the MCP broker
├── enfusion_mcp_node_package/  the npm package that pins the `enfusion-mcp` server
├── foundation/                 verdicts and the verification lock, child processes, the repository laws, the layout, the deploy settings, the test locks
├── map_assets/                 the folder of the map asset pipeline and verification crates (README only, no crate yet)
├── staging/                    the staging crates: the load plan, the load generator, the acknowledgement-dropping relay
├── tickets/                    the ticket registry crates: model, metrics, wave lock, registry, the ticketboard's headless model
└── xtask/                      the `cargo xtask` command line and dispatch, and the groups without a crate of their own
```

## How it works

`cargo xtask` is the alias `run --package xtask --` in `.cargo/config.toml`. The `xtask` binary
parses the command line and dispatches every command group; the verifications, the CI tasks, the
deploys and the platform's agent, worktree and wave orchestration live in the crates below, and
xtask keeps only the `ai`, `fetch`, `map`, `refactor`, `schema`, `ticket`, `verify` and `wave`
groups as modules of its own. It passes each crate the checkout root:

- The [ticket crates](/tools/tickets/README.md) own the
  [ticket](/documentation/glossary/n_to_z.md#ticket) files: `ticket_model` the typed ticket, its
  canonical TOML encoding and the corpus store; `ticket_metrics` the run receipts and estimates;
  `ticket_wave_lock` the [wave](/documentation/glossary/n_to_z.md#wave) lock and its history; and
  `ticket_registry` the typed operations, validation and the sync outputs (`queue.json`, the
  roadmap markers and the gap-analysis ticket column). The `ticket` and `wave` command groups of
  xtask delegate to them, and [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)
  (`apps/ticketboard/`) reads the registry through `ticket_model` and paints the models of
  `ticketboard_model` (tools/tickets); process invocation and worktree
  cleanup stay with xtask.
- The [foundation crates](/tools/foundation/README.md) hold what every tool builds on:
  `verification_core` the fail-closed primitives the gates share (verdicts and findings, pattern
  scans, the run report and the `flock` on the shared verification lock), `process_runner` child
  processes with process-group isolation and deadlines, the container-to-host bridge and the ssh
  transport, `repository_laws` the structural engineering laws as pure checks,
  `repository_layout` the checkout-root walk and the shared locations, `deploy_settings` the
  reader of `deploy/deploy.env`, and `tool_test_support` the locks and the checkout root the tool
  tests share (a dev-dependency only).
- The [command crates](/tools/commands/README.md) hold the work of xtask command groups:
  `api_readiness_checks` the API readiness judge behind `cargo xtask verify api-readiness`;
  `workstation_setup` the `cargo xtask setup` commands; `enfusion_mcp` the Enfusion MCP client
  behind `cargo xtask mcp`;
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
  checks and the tooling rules over every tool crate, `mod_script_checks` the Enfusion mod
  script checks and the Workbench spawn runs, and `documentation_checks` the README coverage,
  Markdown placement and link-check gates.
- The [Enfusion crates](/tools/enfusion/README.md) hold the libraries that read and drive the
  Enfusion engine's formats and tools: `enfusion_pak` the `.pak` archive reader, `enfusion_script_index`
  the script oracle the `enf` binary runs and the vanilla page mirrors behind `cargo xtask fetch`,
  and `enfusion_mcp_broker` the `mcpd` broker over one enfusion-mcp server, whose client behind
  `cargo xtask mcp` is the command crate `enfusion_mcp`.
- The [browser testing crates](/tools/browser_testing/README.md) hold the headless browser gates of
  the single-page app: `chrome_devtools_protocol` the client that launches Chromium and drives its
  pages, and `browser_gate_suites` the static server, the DOM oracle, the route drift check, the
  Mission Creator smokes, the data viewer gate, the capture rig and the doctor, which the `gate`
  and `capture` binaries of `developer_tools` run.
- The [staging crates](/tools/staging/README.md) hold the engines of the staging verification
  receipts: `staging_load_plan` the member load's plan, report and their JSON codec, which the
  `staging_procedures` load procedure builds and judges; `staging_load_generator` the load itself,
  which the procedure runs as `developer_tools`' `staging-load` child process; and
  `acknowledgement_dropping_relay` the fault-injecting relay behind
  `acknowledgement-dropping-relay` on the staging host.
- `developer_tools` holds the heavy work in eight executables (`enf`, `gate`, `mcpd`, `world`,
  `map`, `capture`, `acknowledgement-dropping-relay`, `staging-load`) and a library: the headless browser gates, the
  blueprint compiler, the world export and raster pipelines, and engine-backed map verification.
  It is the one tooling crate that links `map_engine`.
- `enfusion_mcp_node_package` is data, not a crate: `npm ci` there installs the pinned server that
  `mcpd` and `cargo xtask mcp` start. It sits outside every crate root, so no crate-scoped walk
  reads its `node_modules/`.

```text
xtask ──▶ developer_tools ──▶ map_engine (legacy/map_engine)
  │               │
  │               ├── mcpd ──▶ enfusion_mcp_broker (tools/enfusion) ──starts──▶ enfusion_mcp_node_package
  │               ├── enf, map pipelines ──▶ enfusion_script_index, enfusion_pak (tools/enfusion)
  │               ├── gate, capture ──▶ browser_gate_suites ──▶ chrome_devtools_protocol (tools/browser_testing)
  │               └── staging-load, acknowledgement-dropping-relay ──▶ staging_load_generator, acknowledgement_dropping_relay (tools/staging)
  ├────▶ staging_procedures (tools/commands), the staging group ──▶ staging_load_plan (tools/staging), the plan crate under staging_load_generator
  ├────▶ ticket crates (tools/tickets) ◀── ticketboard_model (tools/tickets) ◀── ticketboard (apps/ticketboard)
  ├────▶ command crates (tools/commands); enfusion_mcp: mcp; schema_tooling ──▶ developer_tools (INSTANCE_KINDS)
  ├────▶ ci_task_catalog (tools/commands): ci, help, mk ──▶ check crates, command crates, developer_tools (map asset checks)
  ├────▶ platform_execution (tools/commands): platform ──▶ ci_task_catalog, ticket crates
  ├────▶ mod_operations (tools/commands): mod ──▶ platform_execution, command crates, mod_script_checks
  ├────▶ enfusion_script_index (tools/enfusion): cargo xtask fetch
  ├────▶ check crates (tools/checks)
  └────▶ process_runner, repository_laws ──▶ verification_core   (tools/foundation)
```

The repository paths more than one tool names, and the one walk that finds the checkout root, live
in `tools/foundation/repository_layout`. Each tool spells the paths only it uses in its own layout
module: `tools/developer_tools/src/map_pipeline_layout.rs`,
`tools/enfusion/enfusion_script_index/src/script_index_layout.rs`,
`tools/browser_testing/browser_gate_suites/src/gate_layout.rs` and `tools/tickets/ticket_model/src/repository.rs`.

## Getting started

Run these from the repository root:

```bash
cargo xtask --help                                 # every command group
cargo test --locked -p xtask -- --test-threads=1   # the router's tests
cargo test -p repository_checks -p mod_script_checks -p documentation_checks   # the check crates' tests, the tooling rules included
cargo test -p ticket_model -p ticket_metrics -p ticket_wave_lock -p ticket_registry   # the ticket crates' tests
cargo test -p verification_core -p process_runner -p repository_laws   # the foundation crates' tests
cargo test -p developer_tools                      # the heavy tools' unit tests; no browser or game install
cargo test -p chrome_devtools_protocol -p browser_gate_suites   # the browser gate crates' tests; no browser
cargo xtask mod dev-bootstrap                      # a mod workstation: npm ci for the MCP server, Workbench
```

Each crate's README lists its own commands and checks.

## Boundaries

- Depends on: `legacy/map_engine/`, through `developer_tools` alone; the checkout's data
  (`.ai/tickets/`, `contracts/`, `assets/`, `documentation/`); and the external tools
  individual commands run: git, Docker or Podman, Postgres, Chromium, Trunk, npm and Node.js, ssh
  and rsync, the Arma Reforger tools.
- Used by: developers and AI agents at the command line; the GitHub workflows in `.github/`; the
  backup and backup-drill units in `deploy/systemd/`, which run
  `cargo xtask deploy db backup` and `cargo xtask deploy db drill`; and `apps/ticketboard/`, which
  links `ticket_model`.
- Rules: each held by a test in `tools/checks/repository_checks/src/tests/`, over every tool crate
  found by folder (each `tools/<name>` and `tools/<category>/<name>` holding a `Cargo.toml`):
  - `developer_tools` never depends on `xtask`, and `xtask` never depends on
    `map_engine` or `graphics_engine` directly
    (`tooling_dependency_direction_is_enforced` in `tooling_dependency_boundaries.rs`);
  - a `tools/foundation` crate depends only on lower `tools/foundation` crates
    (`foundation_crates_depend_only_on_lower_foundation_crates`); a `tools/tickets` crate depends
    only on `tools/foundation` crates, on `time_source`, `content_digest` and `newtype_ids`, and on
    ticket crates of a lower tier
    (`ticket_crates_depend_only_on_foundations_and_lower_ticket_crates`); and ticket logic has one
    owner, the xtask `ticket` group delegating to `ticket_registry` and the `wave` group to
    `ticket_wave_lock` (`ticket_implementations_have_one_owner`);
  - in production source only a layout module — a file of the `repository_layout` crate, or a
    file whose first line declares it one (`//! The repository locations only …`) — spells a
    repository path literal, such as one under `.ai/` or `documentation/`; no tracked tooling file carries a retired spelling, a
    script file name or a history word, and no document or production source a ticket id; every
    Rust file named in the prose exists (`tooling_prose_rules.rs`);
  - every tooling crate's source files stay within their line limits
    (`tooling_source_files_stay_below_their_structural_limits`).

## Related documentation

- [Tooling architecture](/documentation/tools/tooling_architecture.md) — the crates, their
  dependency direction, the invariants and the verification surface.
- [Tooling documentation](/documentation/tools/README.md) — the index of the deeper
  documents on each crate.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the MCP daemon and
  call path.
- [Factory waves](/documentation/runbooks/factory_waves/README.md) — running waves of tickets
  with the xtask platform commands.
