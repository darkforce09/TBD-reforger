# API executables

The two binaries of the `website-api` crate: the `api` server behind the web platform, and
`import-registry`, which loads the item [registry](/documentation_v2/glossary.md#registry) that
Workbench exports into Postgres.

## Contents

```text
apps/website/api_v2/src/bin/
├── api.rs              the `api` binary: the HTTP server, its migrations and background workers
└── import_registry.rs  the `import-registry` binary: loads registry envelopes into Postgres
```

## How it works

The `[[bin]]` tables of `apps/website/api_v2/Cargo.toml` name both binaries. Each `main` is a
Tokio entry point that calls into the `website_api` library and returns an `anyhow::Result`, so a
failure prints `Error: <cause>` and exits 1. Neither uses an argument-parsing crate: `api` takes
no arguments and reads everything from the environment, and `import-registry` walks its four
flags by hand.

```text
api.rs              ──▶ Config::load ─▶ database::connect
                        ─▶ database::migrate, unless SKIP_MIGRATE is set
                        ─▶ AppState::new ─▶ background_workers::spawn_all
                        ─▶ http_router::router ─▶ axum::serve until SIGINT or SIGTERM
import_registry.rs  ──▶ database::connect ─▶ database::migrate
                        ─▶ registry_import::import_items and import_compat
```

## Commands

Run from `apps/website/api_v2/` as `cargo run --bin <name> -- <arguments>`; both read `.env` there.

### api

- Synopsis: `api`, with no arguments; the environment and `.env` configure it.
- Does: sets the log filter from `RUST_LOG` (`info` when unset), loads the configuration, connects
  to Postgres, applies the pending migrations unless `SKIP_MIGRATE` is set and logs
  `migrations applied`, arms the
  [background workers](/documentation_v2/glossary.md#background-workers), and serves every route
  on `0.0.0.0:$PORT`. It stays in the foreground until SIGINT or SIGTERM, then drains the
  requests in flight. It needs Postgres running.
- Exit codes: 0 after a signal and a clean drain; 1 when the configuration, the database
  connection, a migration or the port bind fails.
- Example: `cargo xtask mk rust-api`, which runs `cargo run --bin api` in `apps/website/api_v2/`
  with its own target directory, `target-dev-api` at the repository root.

### import-registry

- Synopsis: `import-registry [--items <path>] [--compat <path>] [--modpack <uuid>] [--prune]`
- Does: reads `DATABASE_URL`, connects, applies the pending migrations, then imports the item
  envelope (`--items`) and the compatibility-edge envelope (`--compat`), each checked against its
  schema in `contracts_v2/definitions/`, into the registry tables of the envelope's `modpackId`,
  or of `--modpack` when given. Re-running an envelope updates rows in place; `--prune` also
  deletes that modpack's rows the envelope does not hold. It prints the total, unique, inserted,
  updated and pruned counts of each envelope. At least one of `--items` and `--compat` is
  required.
- Exit codes: 0 when every envelope imported; 1 on an unknown flag, a flag without its value, a
  malformed `--modpack`, neither envelope given, an unset `DATABASE_URL`, an unreadable file, or
  a failed import.
- Example: `cargo xtask db registry-import`, which imports
  `contracts_v2/catalogs/registry-items.workbench.json` and
  `contracts_v2/catalogs/registry-compat.workbench.json`.

## Boundaries

- Depends on: the `website_api` library: `core::configuration`, `core::database`,
  `core::application_state` and `core::http_router` for `api`, with `background_workers`;
  `core::database` and `missions::services::registry_import` for `import-registry`.
- Used by: `cargo xtask mk rust-api` and `cargo xtask db registry-import`
  (`tools_v2/xtask/src/commands/build/recipes/shell_word.rs` and
  `tools_v2/xtask/src/commands/db/operations.rs`); the `editor-api-boot` task of `cargo xtask ci`;
  the release image built by `apps/website/Dockerfile`, whose entry point is `api`; the systemd
  unit `tools_v2/xtask/deploy/systemd/tbd-website-api.service`, which runs the release `api`.
- Rules: `api.rs` is the one file that arms `background_workers`
  (`background_workers_used_only_by_the_binary` in
  `apps/website/api_v2/src/tests/architecture_rules.rs`); the binary names are stable, because the
  xtask recipes, the Dockerfile and the systemd unit call them by name; a new binary adds its
  `[[bin]]` table and its file in the same change.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — running the
  [API](/documentation_v2/glossary.md#api) and importing the registry locally.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — building and running
  the release `api` on the home server.
