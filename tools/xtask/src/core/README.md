# Xtask shared plumbing

The helpers every xtask command group shares: finding the checkout root, the one spelling of each
repository location xtask reads, the one reader of the deploy settings file (with the ssh
transport it chooses for the deploy host), the cargo target directory policy, and the `PATH` guard
tests use. Running host binaries from inside the development container and building the ssh argv
are process mechanics and live in the `process_runner` crate. The command groups use these
helpers without owning [ticket](/documentation/glossary/n_to_z.md#ticket) storage or map processing, which stay in `ticket_engine` and
`developer_tools`.

## Contents

```text
tools/xtask/src/core/
├── cargo_target_directory.rs  the shared target directory, the build output subfolder names, the glibc stamp guard, and the target checks
├── deploy_environment/        the deploy settings grammar, the deploy host and the remote folder defaults
├── deploy_environment.rs      `DeployEnvironment`: loads `deploy.env` under one precedence rule; the ssh transport; errors
├── mod.rs                     the module tree
├── repository_layout.rs       repository-relative locations, the ticket_engine ones re-exported, and `documentation`
├── repository_root.rs         `test_repo_root`: the checkout root for tests, under the working-directory lock
├── test_environment.rs        `PathGuard`: prepends a folder to `PATH` and keeps the system tools reachable
└── tests/                     unit tests for the deploy settings, the `PATH` construction and the build output folders
```

## How it works

A command starts from the `repository_layout` crate's `find_repository_root`, which walks up from
the working directory to the folder holding `.ai/tickets/ROOT`, and joins the constants of `repository_layout` onto it: the deploy files and systemd units, the
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

`DeployEnvironment::ssh_base` is the one way a command chooses how ssh reaches the deploy host:
it reads `TBD_SSH_PASS` (first) or `TBD_SSH_IDENTITY_FILE` and hands them to
`process_runner::secure_shell_transport::SshBase::from_settings`, which builds `ssh`,
`ssh -i <file>` or `sshpass -e ssh`; `ssh_argv` there appends the destination and the remote
words. The password reaches `sshpass` only through the spawned process's `SSHPASS` variable, and
`SshBase`'s `Debug` redacts it.

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
  `verification_core` (`Verdict`, `NotRun`); `process_runner` (`Run`, and `SshBase` for
  `deploy_environment`); `crate::commands::build::recipes` for
  the steps the target checks inspect; `libc` for the glibc version; `git`.
- Used by:
  - `repository_layout`: nearly every command group and verification in `tools/xtask/src/`;
    `repository_root`: the unit tests that read fixtures from the checkout;
  - `deploy_environment`: `deploy website` and `deploy staging`
    (`tools/xtask/src/commands/deploy/`), `mod bootstrap-staging` and `setup client-addons`
    (`tools/xtask/src/commands/setup/`), `mod remote-logs`, `debug direct-join` and
    `debug a2s-probe` (`tools/xtask/src/commands/debug/`); its `ssh_base` by the staging harness
    (`tools/xtask/src/commands/staging/staging_settings.rs`);
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
    key (`the_committed_example_loads_and_masks_nothing`); the ssh password setting wins over the
    identity file (`secure_shell_transport_reads_the_password_before_the_identity_file`).
  - A new `PATH` always keeps the system tools (`prepended_path_onto_stub_only_path_keeps_system_bins_reachable`
    in `tests/test_environment/tests.rs`).
  - The target directory tests live with the recipes that apply the policy, in
    `tools/xtask/src/commands/build/tests/recipes.rs` (`abi_guard_refuses_a_foreign_stamp`).
