# Deploy settings

The `deploy_settings` crate: the one reader of `deploy/deploy.env`, the file that names the deploy
host and the folders on it. Every command that reaches a host loads the file through it and answers
each setting under one precedence rule, so a stale exported variable can never beat what the file
says.

## Contents

```text
tools/foundation/deploy_settings/
├── Cargo.toml  the `deploy_settings` library package: `repository_layout`, `process_runner`, `thiserror`, layout tier 2
└── src/        the loader, the file's grammar, the deploy host, the remote folders and the errors
```

## How it works

```text
deploy_environment_path(root) ── DEPLOY_ENV (absolute against the working directory) or root/deploy/deploy.env
        │
DeployEnvironment::load_required(path) / load_if_present(path)
        │  the file decides every key it assigns (empty = unset); the process environment fills the rest
        ├── value(key) / required(key) / value_or(key, default)
        ├── deploy_host()  ──► DeployHost (user@host or host) ──► DeployHostFolder::resolve
        └── ssh_base()     ──► process_runner::secure_shell_transport::SshBase (TBD_SSH_PASS first, then TBD_SSH_IDENTITY_FILE)
```

A command-line flag, where a command has one, beats both sources; that choice stays with the
command. A refused value is reported as `<path>:<line>: <KEY>: <problem>` (or `<KEY> (process
environment): <problem>`), a missing one as `<KEY> is not set: add it to <path>`, and no message
echoes a value. `src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p deploy_settings   # the grammar, the precedence rule, the host, the folders and the committed example
```

## Configuration

No feature. The crate reads the `DEPLOY_ENV` variable (another settings file), the settings file
itself and the process environment; the keys it names are `TBD_SSH_HOST`, `TBD_SSH_PASS`,
`TBD_SSH_IDENTITY_FILE`, `TBD_REMOTE_DIR`, `TBD_PROFILE_DIR`, `TBD_ADDONS_STAGING` and
`TBD_SERVER_DIR`; the commands read their own keys through `value` and `required`.

## Public surface

- At the crate root: `DeployEnvironment`, `deploy_environment_path`,
  `resolve_deploy_environment_path`, `DEPLOY_ENV_OVERRIDE_VARIABLE`, `DEPLOY_HOST_KEY`,
  `SSH_PASSWORD_KEY`, `SSH_IDENTITY_FILE_KEY`; `DeployHost` and `first_ipv4_address`;
  `DeployHostFolder`; `Error` and `Result` (loading the file), `SettingError` and `SettingOrigin`
  (a setting a command cannot use).
- `prelude`: `DeployEnvironment`, `deploy_environment_path`, `DeployHost`, `DeployHostFolder` and
  `SettingError`.

## Boundaries

- Depends on: `repository_layout` (the settings file and its example), `process_runner` (`SshBase`)
  and `thiserror`.
- Used by: the `deploy`, `setup`, `debug`, `mod` and `staging` command groups of `xtask`.
- Rules: tier 2 of `tools/foundation` (`cargo xtask verify crate-tiers`); the file is parsed and
  never executed; the file decides every key it assigns, even empty, over the process environment
  (`an_empty_assignment_in_the_file_beats_the_process_environment` in
  `src/tests/deploy_environment_tests.rs`), and the committed example loads and masks no optional
  key (`the_committed_example_loads_and_masks_nothing`).

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the foundation crates and their
  tiers.
- [Deployment configuration](/deploy/README.md) — the settings file, its example and the hosts
  it names.
