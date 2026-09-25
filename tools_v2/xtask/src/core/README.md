# Xtask shared plumbing

The helpers every xtask command group shares: finding the checkout root, the one spelling of each
repository location xtask reads, running host binaries from inside the development container, the
cargo target directory policy, and the `PATH` guard tests use. The command groups use them
without owning [ticket](/documentation_v2/glossary.md#ticket) storage or map processing, which stay in `ticket-engine` and
`developer-tools`.

## Contents

```text
tools_v2/xtask/src/core/
├── cargo_target_directory.rs  the shared target directory, the glibc stamp guard, and the target checks
├── host_execution.rs          `Host`: runs host binaries through the container bridge, or directly on the host
├── mod.rs                     the module tree
├── repository_layout.rs       repository-relative locations, the ticket-engine ones re-exported, and `documentation`
├── repository_root.rs         `find_repo_root` for commands, `test_repo_root` for tests
├── test_environment.rs        `PathGuard`: prepends a folder to `PATH` and keeps the system tools reachable
└── tests/                     unit tests for the host bridge and the `PATH` construction
```

## How it works

A command starts from `repository_root::find_repo_root`, which walks up from the working
directory to the folder holding `.ai/tickets/ROOT` (`ticket_engine::repository::find_repo_root`),
and joins the constants of `repository_layout` onto it: the deploy files and systemd units, the
dedicated-server profiles, the MCP transcript fixtures, the ticket locations re-exported from
`ticket_engine::repository` (`TICKETS_DIR`, `WAVE_LOCK`, `WORKTREES_DIR` and others), and, in the
`documentation` submodule, every document or documentation root a command names or walks: the
runbooks that refusals cite, the [API](/documentation_v2/glossary.md#api) readiness register, and the roots and exemptions of the
documentation gates.

`host_execution::Host` exists because the development container cannot run host-linked binaries
(Steam, [Workbench](/documentation_v2/glossary.md#workbench), `ArmaReforgerServer`). `Host::detect` asks whether this process is in a
container (`/run/.containerenv` or `/.dockerenv`) and which bridge is on `PATH`
(`distrobox-host-exec`, then `host-spawn`); a command goes through the bridge only when both
hold, and runs directly otherwise. With no bridge in a container, `run` prints a refusal and
returns 127 and `capture` returns nothing. Cargo is never routed through the bridge.

`cargo_target_directory` holds the build cache policy the `mk` recipes apply:

- `resolve_target_dir`: `CARGO_TARGET_DIR` when set, else the `target/` of the primary checkout
  (from `git rev-parse --git-common-dir`), so every linked worktree shares one warm cache; the
  development API builds into the current checkout's own `target-dev-api` instead.
- `abi_guard`: stamps a target directory with `glibc<version>-<container|host>` in
  `.tbd-build-abi` on first use and refuses a build whose stamp names the other glibc, since the
  two share no binaries.
- `verify_cargo_target` and `reclaim_target_ci`: the bodies of `cargo xtask mk verify-cargo-target`
  and `cargo xtask mk reclaim-target-ci`.

`test_environment::PathGuard` prepends a stub folder to `PATH` for its lifetime and always keeps
`/usr/bin` and `/bin` on it; tests hold `ENV_LOCK` while they change `PATH`.

## Boundaries

- Depends on: `ticket_engine::repository` (the root marker and the ticket locations);
  `verification_core` (`proc::Run`, `Verdict`, `NotRun`); `crate::commands::build::recipes` for
  the steps the target checks inspect; `libc` for the glibc version; `git`.
- Used by:
  - `repository_root` and `repository_layout`: nearly every command group and verification in
    `tools_v2/xtask/src/`;
  - `host_execution`: the `db` group (`tools_v2/xtask/src/commands/db/operations.rs`), the
    playtest server of the `mod` group
    (`tools_v2/xtask/src/commands/mod_ops/playtest_server/host.rs`) and the platform [wave](/documentation_v2/glossary.md#wave) driver
    (`tools_v2/xtask/src/commands/platform/wave_execution/host.rs`);
  - `cargo_target_directory`: the `mk` recipes (`tools_v2/xtask/src/commands/build/recipes.rs`)
    and the wave driver's flush (`tools_v2/xtask/src/commands/platform/wave_execution/flush.rs`);
  - `test_environment`: the tests of the `db`, `debug`, `mcp`, `mod` and `setup` groups.
- Rules:
  - A production source outside the three layout modules (this one, and those of `ticket-engine`
    and `developer-tools`) never writes a string literal that starts with a scripts, docs, `.ai/`
    or `documentation_v2/` path (`only_a_layout_module_spells_a_repository_path` in
    `tools_v2/xtask/src/tests/tooling_prose_rules.rs`), and every committed location in
    `repository_layout` exists (`every_committed_location_exists_in_the_checkout` in
    `tools_v2/xtask/src/tests/repository_layout_tests.rs`).
  - The bridge is never used outside a container (`bridge_is_never_used_on_the_metal` in
    `tests/host_execution/tests.rs`).
  - A new `PATH` always keeps the system tools (`prepended_path_onto_stub_only_path_keeps_system_bins_reachable`
    in `tests/test_environment/tests.rs`).
  - The target directory tests live with the recipes that apply the policy, in
    `tools_v2/xtask/src/commands/build/tests/recipes.rs` (`abi_guard_refuses_a_foreign_stamp`).
