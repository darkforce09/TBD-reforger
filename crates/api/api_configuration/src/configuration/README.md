# Runtime configuration

The [API](/documentation/glossary/a_to_f.md#api)'s settings, read once from the environment at boot
into `Config`, with the checks that stop a misconfigured process before it serves a request.

## Contents

```text
crates/api/api_configuration/src/configuration/
├── development_directories.rs  the four directory settings' development defaults, joined onto the checkout root
├── mod.rs                      `Config` and `ConfigError`: the settings `Config::load` reads, and their checks
├── proxy_network.rs            `ProxyNet`: one `TRUSTED_PROXIES` address or CIDR block, and peer matching
└── tests/                      unit tests for the boot checks, the development directories and the proxy parser
```

## How it works

`Config::load` loads the first `.env` found in the working directory or one of its parents, when
there is one, then reads the process environment; a variable already exported wins over the
file. It fills the defaults and hands the result to a validation step that fails boot with a
`ConfigError` naming the variable:
`DATABASE_URL` or `JWT_SECRET` empty; outside development, a blank `DISCORD_CLIENT_ID`,
`DISCORD_CLIENT_SECRET` or `DISCORD_REDIRECT_URL`, an `UPLOAD_DIR`, `MAP_ASSETS_DIR` or
`GLYPH_ASSETS_DIR` that is unset or not absolute, or an `EQUIPMENT_DATA_DIR` or
`EQUIPMENT_EXPORT_SOURCE_DIR` that is set and not absolute; in every environment, any of those
directories with surrounding whitespace, a `DISCORD_BOT_TOKEN` holding whitespace, or a
`TRUSTED_PROXIES` entry that `ProxyNet::parse` refuses.

`development_directories.rs` names the development default of `UPLOAD_DIR`
(`assets/scratch/api/uploads`), `EQUIPMENT_DATA_DIR` (`assets/equipment`), `MAP_ASSETS_DIR`
(`assets/terrains`) and `GLYPH_ASSETS_DIR` (`assets/glyphs`), each relative to the checkout root.
In development, `Config::load` joins an unset one onto the root `repository_root` finds by walking
up from the working directory to the root marker, so every folder inside the checkout resolves the
same absolute paths; a walk that finds no root fails the boot with
`ConfigError::CheckoutRootNotFound`, naming the setting and the folder the walk started from. A set
value is kept as written, and outside development no walk runs. `JWT_ACCESS_TTL_MIN` and `MISSION_VERSION_MAX_BODY_BYTES` fall back to their defaults
when they do not parse. The full list of settings, with defaults, is the crate's
[Configuration](/crates/api/api_server/README.md#configuration) section.

`DISCORD_BOT_TOKEN` is read only through `Config::require_discord_bot_token`, which turns an unset
token into a named error at the point of use. The four `TBD_DB_POOL_*` settings are not in
`Config`: `api_database::connection_pool` reads them when the pool opens.
`OBSERVABILITY_TOKEN` is optional: left empty, `/metrics` answers 401 and `/healthz` serves only its
public view. `Config::for_tests` builds a development configuration for tests and harnesses, with
blank Discord credentials, a fixed test observability token, the upload and equipment directories
under a per-process temporary directory, and the checkout's read-only terrain and glyph trees,
found from this crate's manifest folder.

`TRUSTED_PROXIES` is a comma-separated list of addresses and CIDR blocks. `ProxyNet::parse`
refuses a block with host bits set rather than masking it, and `ProxyNet::contains` compares
IPv4-mapped IPv6 addresses in their IPv4 form. Outside this folder, the rate limiter's client
resolution in `api_http_layer::middleware` is the only reader.

## Boundaries

- Depends on: `dotenvy`, which loads `.env`; `api_identifiers` for the Discord client and guild
  ids; `repository_root`, whose checkout-root walk anchors the development directories.
- Used by:
  - `crates/api/api_server/src/bin/api_server.rs`, which calls `Config::load`;
  - `api_server::composition`, `api_server::router` and `api_state`: the composition root builds the services and
    `AppState::new` the rest of the state from a `Config`, the router reads the
    file mounts, the body limit and the development flag, the middleware reads the proxy list,
    `api_http_layer::observability::observability_auth` reads the observability token, and
    `api_database::connection_pool` reports through `ConfigError`;
  - the domains, through `AppState::cfg`; `api_identity_and_access`, `api_missions` and `api_operations` also
    name `Config` directly, in the OAuth host guard, session authorization, the
    [mission](/documentation/glossary/g_to_m.md#mission) write lock,
    [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) requests and the
    reservation scope;
  - the integration suites under `crates/api/api_server/tests/`, through `Config::for_tests`.
- Rules: a variable is added together with the code that reads it, so `Config` holds no setting
  that nothing uses; a value that is set but unusable fails boot instead of falling back to a
  default, except the two numeric settings named above; a development default is never relative
  to the working directory (`tests/development_directories.rs`); no test configuration writes into
  the checkout (`test_configs_keep_runtime_storage_out_of_the_checkout` in
  `tests/configuration.rs`).

## Related documentation

- [API environment variables](/documentation/crates/api/api_server/environment_variables.md) — every
  variable the API reads, with its default, when it is required and what an unusable value does.
