# API executables

The two binaries of the `api` crate: the `api` server behind the web platform, and
`import-registry`, which loads the item [registry](/documentation/glossary/n_to_z.md#registry) that
Workbench exports into Postgres. The staging host tool `staging-fixtures` is its own crate,
[`staging_fixtures`](/tools/staging/staging_fixtures/README.md).

## Contents

```text
apps/api/src/bin/
├── api.rs              the `api` binary: the HTTP server, its migrations and background workers
└── import_registry.rs  the `import-registry` binary: loads registry envelopes into Postgres
```

## How it works

The `[[bin]]` tables of `apps/api/Cargo.toml` name the two binaries. Each `main` is
a Tokio entry point that calls into the `api` library. Both return an `anyhow::Result`, so a
failure prints `Error: <cause>` and exits 1. Neither uses an argument-parsing crate: `api` takes no
arguments and reads everything from the environment, and `import-registry` walks its four flags by
hand.

```text
api.rs              ──▶ Config::load ─▶ database::connect
                        ─▶ database::migrate, unless SKIP_MIGRATE is set
                        ─▶ composition::application_state ─▶ api_background_workers::spawn_all
                        ─▶ router::router ─▶ axum::serve until SIGINT or SIGTERM
                        ─▶ process_shutdown begins: open SSE streams close, the drain ends
import_registry.rs  ──▶ database::connect ─▶ database::migrate
                        ─▶ registry_import::import_items and import_compat
```

## Commands

Run `api` and `import-registry` from `apps/api/` as
`cargo run --bin <name> -- <arguments>`; both read `.env` there.

### api

- Synopsis: `api`, with no arguments; the environment and `.env` configure it.
- Does: sets the log filter from `RUST_LOG` (`info` when unset), loads the configuration, connects
  to Postgres, applies the pending migrations unless `SKIP_MIGRATE` is set and logs
  `migrations applied`, arms the
  [background workers](/documentation/glossary/a_to_f.md#background-workers), and serves every route
  on `0.0.0.0:$PORT`. It stays in the foreground until SIGINT or SIGTERM. The signal begins
  `api_configuration::process_lifecycle::process_shutdown`: the server stops accepting connections, every open
  [SSE](/documentation/glossary/n_to_z.md#sse) stream (the audit log feed, the server status
  streams) ends its body at once with no further event, and the requests in flight drain, so the
  process exits without waiting for the service manager's kill timeout. A client of the audit
  log feed reconnects with `Last-Event-ID` and misses no row. It needs Postgres running.
- Exit codes: 0 after a signal and a clean drain; 1 when the configuration, the database
  connection, a migration or the port bind fails.
- Example: `cargo xtask mk rust-api`, which runs `cargo run --bin api` in `apps/api/`
  with its own target directory, `target/dev-api` in the checkout.

### import-registry

- Synopsis: `import-registry [--items <path>] [--compat <path>] [--modpack <uuid>] [--prune]`
- Does: reads `DATABASE_URL`, connects, applies the pending migrations, then imports the item
  envelope (`--items`) and the compatibility-edge envelope (`--compat`), each checked against its
  schema in `contracts/definitions/`, into the registry tables of the envelope's `modpackId`,
  or of `--modpack` when given. Re-running an envelope updates rows in place; `--prune` also
  deletes that modpack's rows the envelope does not hold. It prints the total, unique, inserted,
  updated and pruned counts of each envelope. At least one of `--items` and `--compat` is
  required.
- Exit codes: 0 when every envelope imported; 1 on an unknown flag, a flag without its value, a
  malformed `--modpack`, neither envelope given, an unset `DATABASE_URL`, an unreadable file, or
  a failed import.
- Example: `cargo xtask db registry-import`, which imports
  `contracts/catalogs/registry-items.workbench.json` and
  `contracts/catalogs/registry-compat.workbench.json`.

## Boundaries

- Depends on: the `api` library's `composition` and `router`, with
  `api_configuration::configuration`, `api_configuration::process_lifecycle`, `api_database` and
  `api_background_workers` for `api`; `api_database` and
  `api_missions::services::registry_import` for `import-registry`.
- Used by: `cargo xtask mk rust-api` and `cargo xtask db registry-import`
  (`tools/commands/ci_task_catalog/src/build_lane/recipes/shell_word.rs` and
  `tools/commands/database_operations/src/local_database.rs`); the `editor-api-boot` task of `cargo xtask ci`;
  the release image built by `deploy/Dockerfile`, whose entry point is `api`; the systemd
  unit `deploy/systemd/tbd-website-api.service`, which runs the release `api`;
  `apps/api/tests/audit_replay_shutdown.rs`, which starts the `api` binary and stops
  it with SIGTERM.
- Rules: `api.rs` is the one file that arms `api_background_workers`
  (`background_workers_used_only_by_the_binary` in
  `apps/api/src/tests/architecture_rules.rs`); the binary names are stable, because the
  xtask recipes, the Dockerfile and the systemd unit call them by name; a new binary adds its
  `[[bin]]` table and its file in the same change.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — running the
  [API](/documentation/glossary/a_to_f.md#api) and importing the registry locally.
- [Website deployment](/documentation/runbooks/website_deployment.md) — building and running
  the release `api` on the home server.
