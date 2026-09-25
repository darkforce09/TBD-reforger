# Website platform

The community's web platform: Discord sign-in, [events](/documentation_v2/glossary.md#event) and
[ORBAT](/documentation_v2/glossary.md#orbat) slotting, the
[mission](/documentation_v2/glossary.md#mission) library and the
[Mission Creator](/documentation_v2/glossary.md#mission-creator), server telemetry, the doctrine
wiki and the admin tools. The folder holds the REST [API](/documentation_v2/glossary.md#api),
the single-page app, the two engines they share for maps, missions and GPU rendering, and the
API's release image and staging stack.

## Contents

```text
apps/website/
├── api_v2/                     the REST API and SSE hub, crate `website-api`
├── docker-compose.staging.yml  the staging stack: Postgres, and the API under the `api` profile
├── Dockerfile                  the API's release image, built from the repository root
├── frontend/                   the single-page app, crate `website-frontend`, built by Trunk
├── graphics-engine/            GPU rendering with no map concept, crate `website-graphics-engine`
├── map-engine/                 world, streaming and mission domain, crate `website-map-engine`
└── shared/                     the URL-guard test table that the API and the app both include
```

## How it works

The four crates are members of the root Cargo workspace. The browser runs `website-frontend`, a
Leptos 0.8 app compiled to WebAssembly; it calls `website-api`, an Axum and sqlx server on
Postgres, over `/api/v1` and [SSE](/documentation_v2/glossary.md#sse), and streams terrain from
`/map-assets`. Both link `website-map-engine`: the API takes only its default `scenario` tier,
which compiles and validates missions, and the app takes the `world`, `io`, `store` and `editing`
tiers, adding `render` and `streaming` in its browser build, which the Mission Creator draws and
edits with. `website-graphics-engine` sits below the map engine, and only the map engine imports
it.

```text
browser ── website-frontend ── /api/v1, SSE, /map-assets ──▶ website-api ──▶ Postgres
                  │                                               │
                  └──────────▶ website-map-engine ◀───────────────┘ (scenario tier)
                                        │
                               website-graphics-engine
```

`shared/` belongs to no crate: the API's and the app's tests both `include!` its table of URL
cases, so their two copies of the link-scheme guard stay in step.

## Getting started

Copy `apps/website/api_v2/.env.example` to `apps/website/api_v2/.env`, then run these from the
repository root, in this order:

```bash
cargo xtask db up        # Postgres 18 on host port 5434, in the background
cargo xtask mk rust-api  # the API on port 8080: applies the migrations, stays in the foreground
cargo xtask db seed      # a second terminal, once the API logs `migrations applied`
cargo xtask mk leptos    # the app on 127.0.0.1:3000, a release build; stays in the foreground
```

`db seed` applies the five development seeds to tables that only the API's migrations create, so
it waits for the API's first boot. The app's Trunk server proxies `/api` and `/map-assets` to the
API. `cargo xtask mk leptos-debug` serves a debug build instead, which is quicker to build but no
guide to frame rate. The map needs the Everon height map and satellite bundle from Git LFS:
`cargo xtask ci lfs-dem` and `cargo xtask ci lfs-sat`. With `APP_ENV=development`,
`/api/v1/auth/dev-login?role=admin` signs in without Discord.

## Configuration

- `Dockerfile` builds `website-api --bin api` from the repository root in a trimmed workspace (the
  API, the two engines, `contracts_v2/definitions/` and `contracts_v2/rules/kit-aliases.json`) on
  Rust 1.95.0, and copies the binary into a Debian bookworm-slim image. The image runs as uid
  65534, listens on `PORT` (8080), needs `DATABASE_URL` and `JWT_SECRET`, creates `/srv/state`
  for uploads, and does not serve the app.
- `docker-compose.staging.yml` defines:
  - `postgres`: Postgres 18, container `tbd_staging_db`, user `tbd`, database `tbd_reforger`,
    password `POSTGRES_PASSWORD`, bound to loopback on `TBD_POSTGRES_HOST_PORT` (5432 by default);
  - `api`, under the `api` profile only: the image built from the `Dockerfile`, container
    `tbd_staging_api`, bound to loopback port 8080, set up from the shell's `APP_ENV`, `JWT_SECRET`,
    `FRONTEND_URL`, `ALLOWED_ORIGINS`, `SERVICE_TOKEN`, `TRUSTED_PROXIES` and `DISCORD_*` values,
    with `assets_v2/terrains/` and `assets_v2/glyphs/` mounted read-only and the uploads on a named
    volume.

## Installed by

- `cargo xtask deploy website` rsyncs the checkout to the host that `TBD_SSH_HOST` names in
  `tools_v2/xtask/deploy/deploy.env`, starts the compose file's `postgres` service there with
  `TBD_POSTGRES_HOST_PORT` from that file (`TBD_SKIP_COMPOSE=1` skips the step), builds the API
  and a release build of the app there, and restarts the API's systemd unit, `tbd-website-api` by
  default; the deployed API does not run from the image.
- `cargo xtask deploy staging` runs
  `docker compose -f apps/website/docker-compose.staging.yml up -d --build` on that host.
- By hand, `docker build -f apps/website/Dockerfile .` from the repository root builds the image,
  as does the compose file with `--profile api`.

## Boundaries

- Depends on: `contracts_v2/`, whose schemas and kit-alias rules the API and the map engine embed;
  `assets_v2/terrains/` and `assets_v2/glyphs/`, served under `/map-assets`; Postgres 18; and
  Discord's OAuth2 and REST APIs.
- Used by: the game servers, through the [mod](/documentation_v2/glossary.md#mod)'s API bridge in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the
  [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) in `apps/fleet_host_agent/`;
  the developer tools in `tools_v2/developer-tools/`, which link `website-map-engine` and drive the
  app in a headless browser; and the build, database and deploy commands of `tools_v2/xtask/`.
- Rules: the graphics engine imports nothing from the map engine and names no map concept; inside
  the map engine only its `frame` module names the graphics engine's frame vocabulary and GPU
  modules; the app never imports the graphics engine (`cargo xtask verify engine-layers` checks
  all three).

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — the full local setup.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — deploying to the home
  server.
- [Website documentation](/documentation_v2/website/README.md) — the index of the platform's
  deeper documents.
- [API documentation](/documentation_v2/website/api_v2/README.md), starting at the
  [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes.
- [Frontend documentation](/documentation_v2/website/frontend/README.md) — the feature docs of each
  page and app.
- [Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md)
  — where the editor is going.
