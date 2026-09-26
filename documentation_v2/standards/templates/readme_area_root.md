**Status:** live

# README template: area root

**When to use:** the top of a code tree, or a folder that groups several products without being one
(`apps/`, `apps/website/`, `apps/mod/`, `tools_v2/`, `contracts_v2/`, `assets_v2/`). The
[README standard](/documentation_v2/standards/readme_standard.md) defines every rule this template
follows; the area root kind adds Getting started.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. An area root that
also holds files of a later kind, such as deploy files, adds that kind's sections after Getting
started; the worked sample shows one.

````markdown
# <Name of the area, in plain words: no path, no backticks>

<One to three sentences: what the area is for and which products it holds.>

## Contents

```text
<repository path of the folder>/
├── <product folder>/  <the product in one phrase, with its crate or package name>
└── <file>             <what it is for: a lowercase phrase, no closing period>
```

## How it works

<How the products fit together: who calls whom, over which boundary, and what they share. An ASCII
diagram in a text block helps here. Name each product's part in one clause; its own README holds the
detail.>

## Getting started

<The few commands, run from the repository root, that bring the area up locally, in the order they
must run, each with what to expect; say which stay in the foreground and what a later command waits
for. Link the runbook for the full procedure.>

## Boundaries

- Depends on: <the other areas, services and data the products need>
- Used by: <the areas, tools and clients outside that use the products>
- Rules: <the invariants particular to the area, each with the gate or test that holds it; no
  repository-wide law>

## Related documentation

- [<document title>](/documentation_v2/<path to the document>) — <what it covers>
````

## Worked sample

Written from `apps/website/`. It is also the standard's example of a folder that is two kinds: an
area root that holds the API's release `Dockerfile` and the staging compose file, so Configuration
and Installed by follow Getting started. The sample sits in a fenced block, so no gate reads it as a
README; the folder's own README.md is written from the same code and may differ.

````markdown
# Website platform

The community's web platform: the REST API, the single-page app, and the two engines they share for
maps, [missions](/documentation_v2/glossary/g_to_m.md#mission) and GPU rendering. The folder also holds
the API's release image and the staging compose stack.

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

The four crates are members of the root Cargo workspace. The browser runs `website-frontend`,
compiled to WebAssembly; it calls `website-api` over `/api/v1` and Server-Sent Events and streams
terrain from `/map-assets`. Both link `website-map-engine`: the API takes only its `scenario` tier,
which compiles and validates missions, and the app takes the `world`, `io`, `store` and `editing`
tiers, and `render` in its browser build, which the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) draws and edits with.
`website-graphics-engine` sits below the map engine.

```text
browser ── website-frontend ── /api/v1, SSE, /map-assets ──▶ website-api ──▶ Postgres
                  │                                               │
                  └──────────▶ website-map-engine ◀───────────────┘ (scenario tier)
                                        │
                               website-graphics-engine
```

## Getting started

Copy `api_v2/.env.example` to `api_v2/.env`, then run these from the repository root, in this
order:

```bash
cargo xtask db up        # Postgres 18 on host port 5434, in the background
cargo xtask mk rust-api  # the API on port 8080: applies the migrations, stays in the foreground
cargo xtask db seed      # a second terminal, once the API logs `migrations applied`
cargo xtask mk leptos    # the app on 127.0.0.1:3000, a release build; stays in the foreground
```

The seeds fill tables the migrations create, and only the API applies the migrations; psql carries
on past a failed statement, so seeding a fresh database before the API's first boot loads nothing
and still exits 0. The app's server proxies `/api` and `/map-assets` to the API. In development,
`/api/v1/auth/dev-login?role=admin` signs in without Discord.

## Configuration

- `Dockerfile` builds `website-api --bin api` in a trimmed workspace (the API, the two engines,
  `contracts_v2/definitions/` and `contracts_v2/rules/kit-aliases.json`) from the repository root.
  The runtime image runs as uid 65534, listens on `PORT` (8080), needs `DATABASE_URL` and
  `JWT_SECRET`, and does not serve the app.
- `docker-compose.staging.yml` defines `postgres` (Postgres 18, container `tbd_staging_db`, bound to
  loopback on `TBD_POSTGRES_HOST_PORT`, 5432 by default, password `POSTGRES_PASSWORD`) and, under the
  `api` profile, `api`: the image built from the `Dockerfile`, bound to loopback port 8080 and set up
  from the shell's `APP_ENV`, `JWT_SECRET`, `FRONTEND_URL`, `ALLOWED_ORIGINS`, `SERVICE_TOKEN`,
  `TRUSTED_PROXIES` and `DISCORD_*` values, with the terrain and glyph trees mounted read-only and the
  uploads on a named volume.

## Installed by

- `cargo xtask deploy website` starts the compose file's `postgres` service on the host that
  `TBD_SSH_HOST` names, with `TBD_POSTGRES_HOST_PORT` taken from
  `tools_v2/xtask/deploy/deploy.env` (`TBD_SKIP_COMPOSE=1` skips the step). The API itself runs
  there as the `tbd-website-api` systemd unit, not from the image.
- `cargo xtask deploy staging` runs
  `docker compose -f apps/website/docker-compose.staging.yml up -d --build` on that host.
- The image is built by that compose file with `--profile api`, or by hand with
  `docker build -f apps/website/Dockerfile .` from the repository root.

## Boundaries

- Depends on: `contracts_v2/`, whose schemas and kit-alias rules the API and the map engine embed;
  `assets_v2/terrains/` and `assets_v2/glyphs/`, served under `/map-assets`; and Postgres 18.
- Used by: the game servers, through the mod's API bridge in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the fleet host agent in `apps/fleet_host_agent/`;
  the developer tools in `tools_v2/developer-tools/`, which link `website-map-engine` and drive the
  app in a headless browser; and the build, database and deploy commands of `tools_v2/xtask/`.
- Rules: the graphics engine imports nothing from the map engine and names no map concept; inside the
  map engine only the frame-packet boundary names the graphics engine's `frame` module, and nothing
  names its device, pipelines or shaders (`cargo xtask verify engine-layers` checks these).

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — the full local setup.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — deploying to the home
  server.
- [Frontend documentation](/documentation_v2/website/frontend/README.md) and
  [API documentation](/documentation_v2/website/api_v2/README.md) — the feature docs of each side.
````
