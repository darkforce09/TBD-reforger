# Developer tooling

Every developer tool in the repository: the `cargo xtask` command router, the heavy offline tools
it calls, the two dependency-light libraries it builds on, and the npm package that pins the
Enfusion MCP server. Developers, AI agents, the CI workflows and the host's timers run them; the
products they build, check and deploy live in `apps/`.

## Contents

```text
tools/
├── developer_tools/            browser gates, Enfusion archives, blueprints and map pipelines
├── enfusion_mcp_node_package/  the npm package that pins the `enfusion-mcp` server
├── ticket_engine/              ticket storage, validation, sync outputs, the wave lock and metrics
├── verification_core/          fail-closed verdicts, pattern scans, process isolation, the verification lock
└── xtask/                      the `cargo xtask` command router, its verifications and CI tasks
```

## How it works

`cargo xtask` is the alias `run --package xtask --` in `.cargo/config.toml`. The `xtask` binary
dispatches every command group, runs the repository's verifications and CI tasks, and executes the
platform's agent, worktree and wave orchestration. It keeps the work that belongs to a library in
that library and passes it the checkout root:

- `ticket_engine` owns the [ticket](/documentation/glossary/n_to_z.md#ticket) files: typed storage and
  operations, validation, the sync outputs (`queue.json`, the roadmap markers and the gap-analysis
  ticket column), the [wave](/documentation/glossary/n_to_z.md#wave) lock and its history, run
  receipts and estimates. The `ticket` and `wave` command groups of xtask delegate to it, and
  [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) (`apps/ticketboard/`) reads the
  registry through it; process invocation and worktree cleanup stay with xtask.
- `verification_core` holds the fail-closed primitives the gates share: verdicts and findings,
  pattern scans, process-group isolation with deadlines, and the `flock` on the shared
  verification lock.
- `developer_tools` holds the heavy work in six executables (`enf`, `gate`, `mcpd`, `world`, `map`,
  `capture`) and a library: Enfusion archive and script tooling, the headless browser gates, the
  blueprint compiler, the world export and raster pipelines, and engine-backed map verification.
  It is the one tooling crate that links `map_engine`.
- `enfusion_mcp_node_package` is data, not a crate: `npm ci` there installs the pinned server that
  `mcpd` and `cargo xtask mcp` start. It sits outside every crate root, so no crate-scoped walk
  reads its `node_modules/`.

```text
xtask ──▶ developer_tools ──▶ map_engine (legacy/map_engine)
  │               │
  │               └── mcpd starts ──▶ enfusion_mcp_node_package (node_modules, after npm ci)
  ├────▶ ticket_engine ◀── ticketboard (apps/ticketboard)
  └────▶ verification_core
```

Each crate spells the repository paths it uses in one layout module:
`tools/xtask/src/core/repository_layout.rs`, `tools/developer_tools/src/repository_layout.rs`
and `tools/ticket_engine/src/repository.rs`.

## Getting started

Run these from the repository root:

```bash
cargo xtask --help                                 # every command group
cargo test --locked -p xtask -- --test-threads=1   # the router's tests, the tooling rules included
cargo test -p ticket_engine                        # the ticket library's tests
cargo test -p verification_core                    # the verification primitives' tests
cargo test -p developer_tools                      # the heavy tools' unit tests; no browser or game install
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
  links `ticket_engine`.
- Rules: each held by a test in `tools/xtask/src/tests/`:
  - `developer_tools` never depends on `xtask`, and `xtask` never depends on
    `map_engine` or `graphics_engine` directly
    (`tooling_dependency_direction_is_enforced` in `tooling_dependency_boundaries.rs`);
  - `ticket_engine` and `verification_core` depend on no workspace crate
    (`foundational_engines_have_no_workspace_dependencies`), and ticket logic has one owner, the
    xtask `ticket` and `wave` groups delegating to `ticket_engine`
    (`ticket_implementations_have_one_owner`);
  - in production source only the three layout modules spell a repository path literal, such as
    one under `.ai/` or `documentation/`; no tracked tooling file carries a retired spelling, a
    script file name or a history word, and no document or production source a ticket id; every
    Rust file named in the prose exists (`tooling_prose_rules.rs`);
  - the four crates' source files stay within their line limits
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
