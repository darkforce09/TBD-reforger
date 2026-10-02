# Xtask shared plumbing

The helpers every xtask command group shares: finding the checkout root, the one spelling of each
repository location xtask reads, the one reader of the deploy settings file, the ssh transport to
the deploy host, running host binaries from inside the development container, the cargo target
directory policy, and the `PATH` guard tests use. The command groups use them
without owning [ticket](/documentation/glossary/n_to_z.md#ticket) storage or map processing, which stay in `ticket_engine` and
`developer_tools`.

## Contents

```text
tools/xtask/src/core/
├── cargo_target_directory.rs  the shared target directory, the build output subfolder names, the glibc stamp guard, and the target checks
├── deploy_environment/        the deploy settings grammar, the deploy host and the remote folder defaults
├── deploy_environment.rs      `DeployEnvironment`: loads `deploy.env` under one precedence rule; errors
├── host_execution.rs          `Host`: runs host binaries through the container bridge, or directly on the host
├── mod.rs                     the module tree
├── repository_layout.rs       repository-relative locations, the ticket_engine ones re-exported, and `documentation`
├── repository_root.rs         `find_repo_root` for commands, `test_repo_root` for tests
├── secure_shell_transport.rs  `SshBase` and `ssh_argv`, the shared ssh transport to the deploy host
├── test_environment.rs        `PathGuard`: prepends a folder to `PATH` and keeps the system tools reachable
└── tests/                     unit tests for the deploy settings, the host bridge, the `PATH` construction and the build output folders
```

## How it works

A command starts from `repository_root::find_repo_root`, which walks up from the working
directory to the folder holding `.ai/tickets/ROOT` (`ticket_engine::repository::find_repo_root`),
and joins the constants of `repository_layout` onto it: the deploy files and systemd units, the
dedicated-server profiles, the MCP transcript fixtures, the ticket locations re-exported from
`ticket_engine::repository` (`TICKETS_DIR`, `WAVE_LOCK`, `WORKTREES_DIR` and others), and, in the
`documentation` submodule, every document or documentation root a command names or walks: the
runbooks that refusals cite, the [API](/documentation/glossary/a_to_f.md#api) readiness register, and the roots and exemptions of the
documentation gates.

`deploy_environment` is the only reader of `deploy/deploy.env`, the file that names
the deploy host (`TBD_SSH_HOST`) and nothing else in the repository does. `deploy_environment_path`
takes the file from `DEPLOY_ENV` when that is set (made absolute against the working directory),
else from `DEPLOY_ENV` in `repository_layout`. `DeployEnvironment` answers each key under one rule
for every command: the file decides every key it assigns, an empty assignment counting as unset,
and the process environment fills only the keys the file never assigns; a command-line flag, where
a command has one, beats both. `load_required` refuses a missing file, `load_if_present` allows
one, and both refuse an unreadable file and a line that breaks the grammar with `<path>:<line>`. A
refused value reports `<path>:<line>: <KEY>: <problem>` (or `<KEY> (process environment): …`), a
missing one `<KEY> is not set: add it to <path>`. `deploy_host` parses `TBD_SSH_HOST`, and
`DeployHostFolder` defaults the remote folders under `/home/<user>`; the folder's README gives the
grammar.

`secure_shell_transport` is the one way a command reaches the deploy host over ssh: `SshBase`
reads `TBD_SSH_PASS` (first) or `TBD_SSH_IDENTITY_FILE` and builds `ssh`, `ssh -i <file>` or
`sshpass -e ssh`, and `ssh_argv` appends the destination and the remote words. The password
reaches `sshpass` only through the spawned process's `SSHPASS` variable, and `SshBase`'s `Debug`
redacts it.

`host_execution::Host` exists because the development container cannot run host-linked binaries
(Steam, [Workbench](/documentation/glossary/n_to_z.md#workbench), `ArmaReforgerServer`). `Host::detect` asks whether this process is in a
container (`/run/.containerenv` or `/.dockerenv`) and which bridge is on `PATH`
(`distrobox-host-exec`, then `host-spawn`); a command goes through the bridge only when both
hold, and runs directly otherwise. With no bridge in a container, `run` prints a refusal and
returns 127 and `capture` returns nothing. Cargo is never routed through the bridge.

`cargo_target_directory` holds the build cache policy the `mk` recipes apply:

- `resolve_target_dir`: `CARGO_TARGET_DIR` when set, else the `target/` of the primary checkout
  (from `git rev-parse --git-common-dir`), so every linked worktree shares one warm cache; the
  development API builds into the current checkout's own `target/dev-api` instead.
- `build_output_subfolder`: every tool's private build output is one purpose subfolder of
  `target/` (`dev-api`, `ci`, `dev-mcpd` and the wave gate's `gate-*` folders, each its own
  `CARGO_TARGET_DIR` with its own cargo lock, and `db-selftest`, the database selftest's compose
  project); no name is one cargo writes inside a target directory.
  `is_retired_root_level_build_folder` names the root-level `target-dev-api`, `target-ci`,
  `target-dev-mcpd`, `target-mk-db-selftest`, `target-gate-*` and `dist-gate-*` folders no tool
  writes, which `cargo xtask platform wave reclaim` deletes.
- `abi_guard`: stamps a target directory with `glibc<version>-<container|host>` in
  `.tbd-build-abi` on first use and refuses a build whose stamp names the other glibc, since the
  two share no binaries.
- `verify_cargo_target` and `reclaim_target_ci`: the bodies of `cargo xtask mk verify-cargo-target`
  and `cargo xtask mk reclaim-target-ci` (which deletes `target/ci` and the retired `target-ci`).

`test_environment::PathGuard` prepends a stub folder to `PATH` for its lifetime and always keeps
`/usr/bin` and `/bin` on it; tests hold `ENV_LOCK` while they change `PATH`.

## Boundaries

- Depends on: `ticket_engine::repository` (the root marker and the ticket locations);
  `verification_core` (`proc::Run`, `Verdict`, `NotRun`); `crate::commands::build::recipes` for
  the steps the target checks inspect; `libc` for the glibc version; `git`.
- Used by:
  - `repository_root` and `repository_layout`: nearly every command group and verification in
    `tools/xtask/src/`;
  - `deploy_environment`: `deploy website` and `deploy staging`
    (`tools/xtask/src/commands/deploy/`), `mod bootstrap-staging` and `setup client-addons`
    (`tools/xtask/src/commands/setup/`), `mod remote-logs`, `debug direct-join` and
    `debug a2s-probe` (`tools/xtask/src/commands/debug/`);
  - `secure_shell_transport`: `deploy staging`
    (`tools/xtask/src/commands/deploy/staging/remote.rs`) and the `staging` harness
    (`tools/xtask/src/commands/staging/remote_observers/host_shell.rs`);
  - `host_execution`: the `db` group (`tools/xtask/src/commands/db/operations.rs`), the
    playtest server of the `mod` group
    (`tools/xtask/src/commands/mod_ops/playtest_server/host.rs`) and the platform [wave](/documentation/glossary/n_to_z.md#wave) driver
    (`tools/xtask/src/commands/platform/wave_execution/host.rs`);
  - `cargo_target_directory`: the `mk` recipes (`tools/xtask/src/commands/build/recipes.rs`),
    the wave driver's flush and gate folders (`tools/xtask/src/commands/platform/wave_execution/`)
    and its reclaim sweep (`tools/xtask/src/commands/platform/wave_execution/reclaim/`);
  - `test_environment`: the tests of the `db`, `debug`, `mcp`, `mod` and `setup` groups.
- Rules:
  - A production source outside the three layout modules (this one, and those of `ticket_engine`
    and `developer_tools`) never writes a string literal that starts with a scripts, docs, `.ai/`
    or `documentation/` path (`only_a_layout_module_spells_a_repository_path` in
    `tools/xtask/src/tests/tooling_prose_rules.rs`), and every committed location in
    `repository_layout` exists (`every_committed_location_exists_in_the_checkout` in
    `tools/xtask/src/tests/repository_layout_tests.rs`).
  - The file decides every key it assigns, even empty, over the process environment
    (`an_empty_assignment_in_the_file_beats_the_process_environment` in
    `tests/deploy_environment/tests.rs`), and the committed example loads and masks no optional
    key (`the_committed_example_loads_and_masks_nothing`).
  - The bridge is never used outside a container (`bridge_is_never_used_on_the_metal` in
    `tests/host_execution/tests.rs`).
  - A new `PATH` always keeps the system tools (`prepended_path_onto_stub_only_path_keeps_system_bins_reachable`
    in `tests/test_environment/tests.rs`).
  - The target directory tests live with the recipes that apply the policy, in
    `tools/xtask/src/commands/build/tests/recipes.rs` (`abi_guard_refuses_a_foreign_stamp`).
