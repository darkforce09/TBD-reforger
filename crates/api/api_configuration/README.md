# API configuration

The `api_configuration` crate: the runtime configuration of the
[API](/documentation/glossary/a_to_f.md#api), read once from the environment at boot, and the
process-wide shutdown signal the server raises when it is asked to stop.

## Contents

```text
crates/api/api_configuration/
├── Cargo.toml  the package: `dotenvy`, `tokio` (sync), `api_identifiers`, `repository_root`, layout tier 2
└── src/        the configuration and process lifecycle modules, the error and the prelude
```

## How it works

`Config::load` loads a `.env` when one is found, reads every setting from the environment with
its development default, and checks the result: a required variable that is empty, or a value
that is set but unusable (a relative upload, terrain or glyph folder outside development, a
malformed trusted proxy entry), fails the boot with a `ConfigError` naming the variable, never at
first use. The development defaults of the four directory settings (`UPLOAD_DIR`,
`EQUIPMENT_DATA_DIR`, `MAP_ASSETS_DIR`, `GLYPH_ASSETS_DIR`) are folders of the checkout, joined
onto the checkout root `repository_root` finds above the working directory, so the API resolves
them from any folder inside the checkout; with no root above, the boot stops with
`ConfigError::CheckoutRootNotFound` naming the setting, never with a path relative to the working
directory. Outside development no default applies. The
trusted proxy entries are parsed into `ProxyNet` networks, which the API's rate limiter matches a
peer address against.

`process_shutdown` is one `ShutdownSignal` for the whole process: the `api-server` binary begins it when
SIGINT or SIGTERM arrives, and every open event stream waits on it and ends, so the graceful drain
finishes.

## Getting started

Run from the repository root:

```bash
cargo test -p api_configuration   # the boot checks, the proxy parser and the shutdown signal
```

A new setting is a `Config` field read in `Config::load`, added together with the code that
reads it, and a row in the configuration table of the [API README](/crates/api/api_server/README.md).

## Configuration

The variables `Config::load` reads are listed in the [API README](/crates/api/api_server/README.md) and the
[environment variables](/documentation/crates/api/api_server/environment_variables.md) page. No feature.

## Public surface

- `configuration`: `Config` (`load`, `for_tests`, `require_discord_bot_token`, the field per
  setting), `ConfigError`, and `proxy_network::{ProxyNet, parse_trusted_proxies}`.
- `process_lifecycle`: `ShutdownSignal` and `process_shutdown`.
- `Error` and `Result`; `prelude`.

## Boundaries

- Depends on: `api_identifiers` (`DiscordClientId`, `DiscordGuildId`), `repository_root` (the
  checkout-root walk of the development directories), `dotenvy`, `thiserror` and `tokio`.
- Used by: `api_database`, whose pool settings refuse through `ConfigError`; the API application
  (`crates/api/api_server`): its binaries, composition root, router, middleware, observability and domains,
  and its integration suites through `Config::for_tests`.
- Rules: `Config` holds no setting nothing reads; a set but unusable value fails boot instead of
  falling back to a default; a development default never resolves against the working directory.

## Related documentation

- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
- [Environment variables](/documentation/crates/api/api_server/environment_variables.md) — every variable the
  API reads.
