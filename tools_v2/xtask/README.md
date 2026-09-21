# Workspace task runner

`cargo xtask` is the repository command router. It owns repository operations and verification orchestration; ticket storage and lock semantics belong to `ticket-engine`, process and verification primitives belong to `verification-core`, and engine-backed asset processing belongs to `developer-tools`.

## Source layout

- `src/main.rs` converts dispatch results into process exit codes. `src/cli/` owns the top-level command parser, argument preprocessing, and routing.
- `src/commands/` contains command declarations, handlers, and operational implementations grouped by database, deployment, setup, fetching, mod operations, MCP, debugging, reproduction, build, CI, generation, schema operations, map operations, agent context, tickets, waves, and platform execution.
- `src/commands/platform/` owns agent execution, worktree orchestration, receipts, and platform-wave execution. `src/commands/wave/` delegates lock compilation and validation to `ticket-engine`; it is separate from the platform-wave lifecycle.
- `src/core/` contains repository-root discovery, host execution, Cargo target-directory handling, and shared test-environment utilities.
- `src/verifications/` groups checks by architecture, licensing, prohibited languages, mod scripts, deployment, registries, database seeds, CI, schemas, and map assets. Map-asset checks delegate engine work to `developer-tools`.
- `src/verifications/schemas/` separates contract citations, content budgets, object enums and type inventory, specification consistency, kit references, wire-field readers, glyphs, and schema validation.

Command names are independent of implementation filenames. Ticket-number CLI spellings remain supported even where the corresponding verification file has a domain name.

## Development commands

Run these commands from the repository:

```bash
cargo xtask help
cargo xtask --help
cargo xtask ticket check
cargo xtask ticket sync
cargo xtask db up
cargo xtask db test-it
cargo xtask ci ci-local
cargo xtask mk leptos-gates
```

`ci-local` runs the CI composite. The browser/editor gates run separately through `mk leptos-gates`; the composite does not include them. Operational commands require their configured services, tools, and credentials. Use a command's help and supported dry-run mode before changing external state.

Focused crate validation:

```bash
cargo test --locked -p xtask -- --test-threads=1
cargo check --locked -p xtask --all-targets
cargo fmt -p xtask --check
```

## Tests and boundaries

Unit tests live in separate files under sibling `tests/` directories, connected through `#[cfg(test)]` and `#[path = "tests/…"]` declarations. `src/tests/` contains entrypoint and structural checks. Production files must remain below 500 lines, test files below 1,000 lines, and `main.rs` below 150 lines. Inline test modules are prohibited.

Structural tests enforce file limits, test placement, and dependency boundaries. `verification-core` and `ticket-engine` must remain free of workspace dependencies. Heavy map-engine ownership stays in `developer-tools`; adding engine dependencies to this router defeats that boundary.

Receipt fixtures live in `../ticket-engine/tests/fixtures/execution_receipts`; blueprint fixtures live in `../developer-tools/test_fixtures/blueprint`. Tests resolve repository fixtures from the active checkout. Source-inspection tests must follow the actual execution path and continue rejecting missing inputs or bypassed checks when modules move.
