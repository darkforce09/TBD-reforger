# Website API

The `api` crate: the Axum REST [API](/documentation/glossary/a_to_f.md#api) and
[SSE](/documentation/glossary/n_to_z.md#sse) streams behind the web platform, assembled from the API
crates under `crates/api/`. It serves `/api/v1` to
the single-page app, the game servers and the
[fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent), applies the Postgres schema
migrations of `crates/api/api_database/` at boot, and serves uploads and terrain assets.

## Contents

```text
apps/api/
├── .env.example         the template the gitignored `.env` is copied from, with development values
├── Cargo.toml           the `api` package: the library and its two binaries
├── src/                 the thin application: the router, the composition root, the binaries, the layout rules
└── tests/               integration suites against real Postgres, with their shared support
```

## How it works

`src/bin/api.rs` loads the configuration, opens the Postgres pool, applies the migrations unless
`SKIP_MIGRATE` is set, arms the [background workers](/documentation/glossary/a_to_f.md#background-workers)
and serves the router on `0.0.0.0:$PORT` until SIGINT or SIGTERM. The router nests the eight
domains' route tables under `/api/v1` and wraps everything in one middleware chain, outermost
first: request id, access log, Prometheus metrics, panic recovery, CORS, body limit, rate limit.
The terrain and glyph mounts under `/map-assets` sit below the rate limit. The application holds no
domain logic: each domain is an API crate (`crates/api/api_<domain>/`) that owns its handlers,
services, models and route table, the kernel crates below them depend on no domain, and the
background workers are their own crate (`crates/api/api_background_workers/`), which only the
`api` binary arms.

A route's access tier is the extractor its handler takes: a signed-in member, a member of at
least a given [role](/documentation/glossary/n_to_z.md#role), or a per-server
[machine credential](/documentation/glossary/g_to_m.md#machine-credential), with which the
[game runtime](/documentation/glossary/g_to_m.md#game-runtime) (heartbeats, match telemetry, link
confirmation, fleet commands) and the fleet host agent authenticate. `/metrics` and the detailed
`/healthz` take the operator's `OBSERVABILITY_TOKEN` bearer, which no other route accepts.
The [mission](/documentation/glossary/g_to_m.md#mission) compiler and the mortar ballistics come from
the mission crates of `crates/mission/` and the ballistics crates of `crates/ballistics/`; the
crate links no map engine and no graphics crate. Game servers
fetch compiled mission [artifacts](/documentation/glossary/a_to_f.md#artifact) over HTTPS from
`/api/v1/game-runtime/artifacts/{artifactId}`; nothing is staged on disk for them.

The integration suites in `tests/` build one test binary per top-level file. A binary that needs
a database derives its own scratch database from `TEST_DATABASE_URL`, creates and migrates it,
and the suites that drive HTTP build the same router the `api` binary serves. Shared support
lives in `tests/common/` and the `*_support/` folders, which produce no binary of their own.
Every test build compiles `api_failpoints` with its `failpoints` feature, which places named
fault points on commit and external-effect paths; the deploy build compiles them out.

### Verification suites

Besides the per-domain suites, six groups of binaries verify the API as a whole; each case's
name starts with its group's prefix, which the verification register counts.

| Group | Binaries | Shared support |
|---|---|---|
| route acceptance | `route_acceptance_coverage`, one `route_acceptance_<part>` per part (`identity_and_core`, `operations_events`, `operations_reservations`, `operations_ballistics`, `missions_library`, `missions_reviews`, `fleet_and_telemetry`, `administration_center_content`), and `debug_routes_are_development_only` | `route_acceptance_support/`: the route table read from `src/router.rs` and the domain crates' `routes.rs`, the specs and worlds per part, the derived probes and the dimension runner |
| contract parity | `contract_parity_goldens`, `contract_parity_equipment_viewer`, `contract_parity_mod_wire`, `json_rejection_envelopes`, `query_rejection_envelopes`, and each route acceptance part's `contract_parity_<part>_…` case | `contract_parity_support/` (golden index, normalisation, seeded capture, route contracts), `enfscript_source_support/` (the `@contract` tag grammar of the mod scripts), `fixtures/equipment_data_viewer/` |
| properties | `session_authority_properties`, `mission_artifact_properties`, `telemetry_revision_properties`, `fleet_command_properties`, `audit_publication_properties`, `reservation_transaction_properties` | `api_property_evidence`, the recorder every property runs through; `session_authority_support/` |
| controlled races | `controlled_races_identity`, `controlled_races_reservations`, `controlled_races_missions_and_telemetry`, `controlled_races_audit` | `failpoint_and_race_support/`: arming, interleavings, row-lock barriers, persisted-state checks |
| failure injection | `failure_injection_<area>` for `identity`, `operations`, `missions`, `telemetry`, `fleet`, `audit` and `discord`, and `failure_injection_self_checks` | `failpoint_and_race_support/` |
| engineering laws | `engineering_laws` | the `repository_laws` and `verification_core` dev-dependencies |

The design note linked under Related documentation specifies each group.

## Getting started

Copy `apps/api/.env.example` to `apps/api/.env` (a fresh git worktree has
none), then run these from the repository root, in this order:

```bash
cargo xtask db up        # Postgres 18 in the tbd_reforger_db container, host port 5434, detached
cargo xtask mk rust-api  # cargo run --bin api in this folder; migrates, stays in the foreground
cargo xtask db seed      # a second terminal, once the API logs `migrations applied`
cargo xtask db test-it   # the integration suites, each against its own scratch database
cargo xtask mk rust-test # the library and binary unit tests, source rules included; no database
```

`db seed` applies five development seeds in a fixed order to tables that only the API's
migrations create. Each `psql` run stops at the first failed statement, so seeding before the
API's first boot stops at the first seed with psql's exit code 3. `db test-it` needs only
`db up`: it creates the scratch databases, and each suite applies the migrations itself.
`cargo xtask db registry-import` loads the committed Workbench
[registry](/documentation/glossary/n_to_z.md#registry) exports into the local database,
`cargo xtask mk leptos` serves the single-page app on port 3000, proxying `/api` and
`/map-assets` to the API, `cargo xtask db down` stops Postgres and keeps its volume, and
`cargo xtask ci ci-local` replays the whole CI suite once `db up` has run.

With `APP_ENV=development`, the [dev login](/documentation/glossary/a_to_f.md#dev-login)
`GET /api/v1/auth/dev-login?role=<role>` signs in without Discord
(`guest`, `enlisted`, `leader`, `mission_maker` or `admin`; any other value signs in as `admin`)
and answers with a 302 to the app's `/auth/callback`, the session's `access_token` in the URL
fragment; outside development the route answers 404.

## Configuration

`Config::load` in `crates/api/api_configuration/src/configuration/mod.rs` reads the process
environment, then the first `.env` found from the working directory upward; an exported variable wins. `DATABASE_URL` and
`JWT_SECRET` are always required. Outside development, `DISCORD_CLIENT_ID`,
`DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL` and an absolute `UPLOAD_DIR` are required too. A
value that is set but unusable stops the boot for `UPLOAD_DIR`, `EQUIPMENT_DATA_DIR`,
`EQUIPMENT_EXPORT_SOURCE_DIR`, `DISCORD_BOT_TOKEN`, `TRUSTED_PROXIES` and the pool settings.
`.env.example` carries most variables with development values; `TRUSTED_PROXIES`,
`MISSION_VERSION_MAX_BODY_BYTES`, `SKIP_MIGRATE`, `RUST_LOG` and `TEST_DATABASE_URL` are not in it.

The crate declares no Cargo feature. `Cargo.toml` turns `api_failpoints`'s `failpoints` feature
on only through a dev-dependency, so every test build (the unit tests and every `tests/*.rs`
binary) carries the fault-injection registry of `crates/api/api_failpoints/`, and the deploy
build, `cargo build --release -p api --bin api`, which passes no feature flag, compiles every
fault point to nothing. The `engineering_laws` suite holds both halves.

| Variable | Default | Required | Read by |
|---|---|---|---|
| `PORT` | `8080` | no | `Config::load` |
| `APP_ENV` | `production`; `development` enables dev login and the development defaults | no | `Config::load` |
| `FRONTEND_URL` | `http://localhost:5173`; where sign-in redirects | no | `Config::load` |
| `ALLOWED_ORIGINS` | the value of `FRONTEND_URL`; a comma-separated CORS allow-list | no | `Config::load` |
| `TRUSTED_PROXIES` | empty, which trusts no `X-Forwarded-For`; comma-separated addresses and CIDR blocks | no | `Config::load` |
| `SPA_DIST_DIR` | empty; when set, the built app is served with an `index.html` fallback | no | `Config::load` |
| `MAP_ASSETS_DIR` | `../../assets/terrains`, relative to the working directory | no | `Config::load` |
| `GLYPH_ASSETS_DIR` | `../../assets/glyphs`, relative to the working directory | no | `Config::load` |
| `UPLOAD_DIR` | `../../assets/scratch/api/uploads` in development; the systemd unit sets its state directory | outside development, absolute | `Config::load` |
| `EQUIPMENT_DATA_DIR` | `../../assets/equipment` in development, empty otherwise, which leaves the equipment datasets unconfigured; the systemd unit sets its state directory; the imported equipment datasets and their indexes | no; when set outside development, absolute | `Config::load` |
| `EQUIPMENT_EXPORT_SOURCE_DIR` | empty, which disables importing; the Workbench equipment export publication the import worker polls | no; when set outside development, absolute | `Config::load` |
| `DATABASE_URL` | none | yes | `Config::load`; `import-registry`; `staging-fixtures`, from the API env file |
| `TBD_DB_POOL_MAX_CONNECTIONS` | `25` | no | `crates/api/api_database/src/connection_pool.rs` |
| `TBD_DB_POOL_IDLE_TIMEOUT_SECS` | `300` | no | `crates/api/api_database/src/connection_pool.rs` |
| `TBD_DB_POOL_MAX_LIFETIME_SECS` | `1800` | no | `crates/api/api_database/src/connection_pool.rs` |
| `TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS` | `30` | no | `crates/api/api_database/src/connection_pool.rs` |
| `MISSION_VERSION_MAX_BODY_BYTES` | 256 MiB; the body limit of the mission version save route alone | no | `Config::load` |
| `JWT_SECRET` | none; signs the access tokens | yes | `Config::load` |
| `JWT_ACCESS_TTL_MIN` | `15` | no | `Config::load` |
| `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL` | empty | outside development | `Config::load` |
| `DISCORD_GUILD_ID` | empty; the guild whose roles decide members' roles | no | `Config::load`; `staging-fixtures`, from the API env file |
| `DISCORD_BOT_TOKEN` | empty, meaning no bot; a value holding whitespace stops the boot | no | `Config::load` |
| `DISCORD_WEBHOOK_URL` | empty, which disables announcement pushes | no | `Config::load` |
| `OBSERVABILITY_TOKEN` | empty, which answers 401 on `/metrics` and serves only the public `/healthz`; sent as `Authorization: Bearer` | no | `Config::load` |
| `SERVER_STATUS_PUBLISH_INTERVAL_SECS` | `10` | no | `crates/api/api_background_workers/src/server_status_publisher.rs` |
| `LEADERBOARD_REFRESH_INTERVAL_SECS` | `900` | no | `crates/api/api_background_workers/src/leaderboard_refresher.rs` |
| `ROLE_RESYNC_INTERVAL_SECS` | `86400` | no | `crates/api/api_background_workers/src/discord_role_synchronizer.rs` |
| `SKIP_MIGRATE` | unset; any value skips the migrations at boot, for a harness that migrates a shared database itself | no | `src/bin/api.rs` |
| `RUST_LOG` | `info`; the log filter | no | `src/bin/api.rs` |
| `TEST_DATABASE_URL` | none; `cargo xtask db test-it` sets it | for the database suites | `tests/common/database.rs` |

## Public surface

- The library `api` (`src/lib.rs`): `router::router`, the whole application with its middleware
  chain, and `composition::application_state`, the application state with its concrete services.
  Its users are this crate's binaries and integration suites.
- The `api` binary: the server described above.
- The `import-registry` binary: imports Workbench registry envelopes (`--items`, `--compat`) into
  Postgres for the envelope's modpack, which `--modpack` overrides; `--prune` deletes that
  modpack's rows the envelope lacks.
- The HTTP surface: `/api/v1`, `/healthz` (the detailed report with the `OBSERVABILITY_TOKEN`
  bearer), `/metrics` (`OBSERVABILITY_TOKEN` bearer), `/uploads`, `/map-assets` and
  `/map-assets/glyphs`, and the built app as the fallback when `SPA_DIST_DIR` is set.

## Boundaries

- Depends on: the API crates of [crates/api](/crates/api/README.md): the eight domain crates,
  whose route tables the router merges; `api_background_workers`, which the `api` binary arms;
  `api_state`, `api_caller_identity`, `api_discord` and `api_equipment_datasets`, which the
  composition root assembles; `api_http_layer`, whose middleware the router mounts;
  `api_configuration` and `api_database`, which the binaries load and open; and, for the tests
  only, `api_failpoints` (with its `failpoints` feature), `api_property_evidence` and the mission,
  ballistics and contract crates the suites build their requests and expectations from. Through
  them: Postgres 18, Discord's OAuth2 and REST APIs and a channel webhook, the schemas in
  `contracts/definitions/`, and, at run time, the asset trees in `assets/terrains/` and
  `assets/glyphs/`.
- Used by:
  - the single-page app in `apps/frontend/`, whose Trunk server proxies `/api` and
    `/map-assets` to the API on `127.0.0.1:8080` in development;
  - the game servers, through the mod's `apps/mod/tbd-framework/Scripts/Game/TBD/API/`, and the
    fleet host agent in `apps/fleet_host_agent/`;
  - the `mk rust-api`, `db`, `ci` and `deploy website` commands of `tools/xtask/`;
  - the release image that `deploy/Dockerfile` builds, the optional `api` service of
    `deploy/compose.staging.yml`, and the systemd unit
    `deploy/systemd/tbd-website-api.service`.
- Rules: `src/` holds only the thin application; the kernel crates depend on no domain, the
  domain crates depend on one another only along the domain graph, no crate imports another
  domain's handlers, every domain's one route table is merged by `src/router.rs`, and only
  `src/bin/api.rs` names `api_background_workers` (`src/tests/architecture_rules.rs` reads the
  API crates' manifests and sources); an applied migration never changes
  (`tests/migrations_are_immutable.rs`); neither the application nor any API crate depends on
  a GPU crate (the eight crates the wgpu firewall admits) or `frontend`, and
  `api_failpoints`' `failpoints` stays a test-only feature (`tests/engineering_laws.rs`); the
  crate builds with
  the workspace root's `rust-toolchain.toml` and `rustfmt.toml`; the local Postgres 18 service
  `db` (container `tbd_reforger_db`, port 5434) is `deploy/compose.dev.yml`; the filled `.env` is
  never committed.

## Related documentation

- [API overview](/documentation/apps/api/api_overview.md) — the routes of every domain
  and the layers they share.
- [API environment variables](/documentation/apps/api/environment_variables.md)
  — every variable the API reads, with
  its default, requirement and failure mode.
- [API decisions](/documentation/apps/api/decisions.md) — the cross-domain design
  decisions and their consequences.
- [Local development](/documentation/runbooks/local_development.md) — the full local setup,
  Discord sign-in included.
- [Database operations](/documentation/runbooks/database_operations.md) — the integration
  tests, the migration checksum repair, sample data, backups and restores.
- [Website deployment](/documentation/runbooks/website_deployment.md) — building and running
  the API on the home server.
- [API completion and verification](/documentation/apps/api/verification_evidence/completion_plan.md)
  — the acceptance contract and requirement register the API is verified against.
- [Verification completeness](/documentation/apps/api/verification_evidence/verification_completeness.md)
  — the route acceptance, contract parity, property, race, failure injection and engineering-law
  suites, and the failpoint catalogue.
- [API verification evidence](/documentation/apps/api/verification_evidence/README.md)
  — the index of the register, the program records and the design notes.
