# Runtime configuration

The [API](/documentation_v2/glossary/a_to_f.md#api)'s settings, read once from the environment at boot
into `Config`, with the checks that stop a misconfigured process before it serves a request.

## Contents

```text
apps/website/api_v2/src/core/configuration/
├── mod.rs            `Config` and `ConfigError`: the settings `Config::load` reads, and their checks
├── proxy_network.rs  `ProxyNet`: one `TRUSTED_PROXIES` address or CIDR block, and peer matching
└── tests/            unit tests for the boot checks and the proxy parser
```

## How it works

`Config::load` loads the first `.env` found in the working directory or one of its parents, when
there is one, then reads the process environment; a variable already exported wins over the
file. It fills the defaults and hands the result to a validation step that fails boot with a
`ConfigError` naming the variable:
`DATABASE_URL` or `JWT_SECRET` empty; outside development, a blank `DISCORD_CLIENT_ID`,
`DISCORD_CLIENT_SECRET` or `DISCORD_REDIRECT_URL`, or an `UPLOAD_DIR` that is unset or not
absolute; in every environment, an `UPLOAD_DIR` with surrounding whitespace, a
`DISCORD_BOT_TOKEN` holding whitespace, or a `TRUSTED_PROXIES` entry that `ProxyNet::parse`
refuses. `JWT_ACCESS_TTL_MIN` and `MISSION_VERSION_MAX_BODY_BYTES` fall back to their defaults
when they do not parse. The full list of settings, with defaults, is the crate's
[Configuration](/apps/website/api_v2/README.md#configuration) section.

`DISCORD_BOT_TOKEN` is read only through `Config::require_discord_bot_token`, which turns an unset
token into a named error at the point of use. The four `TBD_DB_POOL_*` settings are not in
`Config`: `crate::core::database::connection_pool` reads them when the pool opens.
`Config::for_tests` builds a development configuration for tests and harnesses, with blank Discord
credentials and the upload directory under a per-process temporary directory.

`TRUSTED_PROXIES` is a comma-separated list of addresses and CIDR blocks. `ProxyNet::parse`
refuses a block with host bits set rather than masking it, and `ProxyNet::contains` compares
IPv4-mapped IPv6 addresses in their IPv4 form. Outside this folder, the rate limiter's client
resolution in `crate::core::middleware` is the only reader.

## Boundaries

- Depends on: `dotenvy`, which loads `.env`.
- Used by:
  - `apps/website/api_v2/src/bin/api.rs`, which calls `Config::load`;
  - the rest of `core`: `AppState::new` builds its services from a `Config`, the router reads the
    file mounts, the body limit and the development flag, the middleware reads the service token
    and the proxy list, the health probe reads the service token, and
    `crate::core::database::connection_pool` reports through `ConfigError`;
  - the domains, through `AppState::cfg`; `identity_and_access`, `missions` and `operations` also
    name `Config` directly, in the OAuth host guard, session authorization, the
    [mission](/documentation_v2/glossary/g_to_m.md#mission) write lock,
    [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) requests and the
    reservation scope;
  - the integration suites under `apps/website/api_v2/tests/`, through `Config::for_tests`.
- Rules: a variable is added together with the code that reads it, so `Config` holds no setting
  that nothing uses; a value that is set but unusable fails boot instead of falling back to a
  default, except the two numeric settings named above; no test configuration writes into the
  checkout (`test_configs_keep_runtime_storage_out_of_the_checkout` in `tests/configuration.rs`).

## Related documentation

- [API environment variables](/documentation_v2/website/api_v2/environment_variables.md) — every
  variable the API reads, with its default, when it is required and what an unusable value does.
