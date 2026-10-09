# Deployment source

The website and staging deploy drivers, the clap subcommand and dispatch of the `deploy` group,
the paths no deploy ships, the paths the host owns and the `.env` probe both deploys run before
their rsync, the remote toolchain line, and the errors they report.

## Contents

```text
tools/commands/deployment/src/
├── api_environment_file_preflight.rs  the probe of the host's API `.env` both deploys run before their rsync
├── deploy_command.rs                  the `DeployCmd` clap enum: `website`, `db` and `staging`
├── deploy_dispatch.rs                 `run`: routes each `DeployCmd` to its driver
├── development_machine_only_paths.rs  the rsync excludes both deploys share: build folders and local tool state
├── enfusion_mod_paths.rs              the mod folder's entries the deploys' rsyncs leave out and the rsync exclusion of a mod folder entry
├── error.rs                           `Error` and `Result`
├── host_owned_paths.rs                the paths the host keeps in its checkout, which both rsyncs exclude
├── lib.rs                             the crate root: module header, `mod` lines and the re-exports
├── prelude.rs                         `DeployCmd`, `run`, `Error` and `Result` for glob import
├── remote_rust_toolchain.rs           the PATH line every remote step runs before it calls cargo or trunk
├── staging/                           the staging deploy: settings, render, payloads, pipeline and boot verdict
├── staging.rs                         `deploy staging`: `Paths`, the flag parser and the mode order
├── tests/                             unit tests for the shared excludes, the host-owned paths, the `.env` probe and website
├── website/                           the website deploy's pure steps: rsync argv, remote shells, probe, unit
└── website.rs                         `deploy website`: deploy.env, the refusals and the step runner
```

## How it works

- `deploy_dispatch::run` hands `website` and `staging` their raw argv and `db` to
  `database_operations::container_database::run`.
- `staging::fleet_instances` and `staging::payloads` are public: the xtask binary's `staging` and
  `debug` groups read the fleet's instances, ports, folders and profile commands from them.
- Every remote build payload starts with `remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH`.
- Both rsync builders exclude `host_owned_paths::HOST_OWNED_PATHS`, and both deploys hand their
  rsync to `api_environment_file_preflight::rsync_only_when_present`, which runs it only after the
  host's probe exits 0.
- Both rsync builders build their mod folder exclusions with `enfusion_mod_paths`; the addon
  folders and folder names come from `repository_layout::enfusion_mod_folders`, which the staging
  instance files payload (the framework addon link into each instance's addons folder), the
  addon GUID read and the boot verdict's addon check read as well.

## Boundaries

- Depends on: `database_operations`, `deploy_settings`, `process_runner`, `repository_layout`,
  `verification_core`, `newtype_ids`, `clap`, `regex`, `serde_json` and `thiserror`.
- Used by: the crate root's re-exports and public modules, read by the `deploy`, `verify` and `ci`
  groups of `xtask` and by the `staging_procedures` and `remote_debugging` crates.
