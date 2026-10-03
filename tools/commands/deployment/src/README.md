# Deployment source

The website and staging deploy drivers, the clap subcommand and dispatch of the `deploy` group,
the paths no deploy ships, the remote toolchain line, the staging compose-path check, and the
errors they report.

## Contents

```text
tools/commands/deployment/src/
├── deploy_command.rs                  the `DeployCmd` clap enum: `website`, `db` and `staging`
├── deploy_dispatch.rs                 `run`: routes each `DeployCmd` to its driver
├── deployment_checks/                 the staging-compose-paths gate
├── deployment_checks.rs               declares the deployment source gate
├── development_machine_only_paths.rs  the rsync excludes both deploys share: build folders and local tool state
├── error.rs                           `Error` and `Result`
├── lib.rs                             the crate root: module header, `mod` lines and the re-exports
├── prelude.rs                         `DeployCmd`, `run`, `Error` and `Result` for glob import
├── remote_rust_toolchain.rs           the PATH line every remote step runs before it calls cargo or trunk
├── staging/                           the staging deploy: settings, render, payloads, pipeline and boot verdict
├── staging.rs                         `deploy staging`: `Paths`, the flag parser and the mode order
├── tests/                             unit tests for the shared excludes, staging flags and website
├── website/                           the website deploy's pure steps: rsync argv, remote shells, probe, unit
└── website.rs                         `deploy website`: deploy.env, the refusals and the step runner
```

## How it works

- `deploy_dispatch::run` hands `website` and `staging` their raw argv and `db` to
  `database_operations::container_database::run`.
- `staging::fleet_instances` and `staging::payloads` are public: the xtask binary's `staging` and
  `debug` groups read the fleet's instances, ports, folders and profile commands from them.
- Every remote build payload starts with `remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH`.

## Boundaries

- Depends on: `database_operations`, `deploy_settings`, `process_runner`, `repository_layout`,
  `verification_core`, `newtype_ids`, `clap`, `regex`, `serde_json` and `thiserror`.
- Used by: the crate root's re-exports and public modules, read by the `deploy`, `verify` and `ci`
  groups of `xtask` and by the `staging_procedures` and `remote_debugging` crates.
