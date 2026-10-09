# Crates

The workspace's product crates, one folder per category: `crates/<category>/<name>` (the
frontend's `crates/frontend/<layer>/<name>`), where the package name equals the folder name. The
applications are crates here like any other: the API server, the single-page app and its offline
service worker, and the game server host agent. The library crates under them hold the domains,
the map engine and the shared foundations; the tools under `tools/` link the library crates, and
no member links an application.

## Contents

```text
crates/
├── api/  the API's crates, such as the typed identifiers, and the server `api_server` that assembles them
├── ballistics/  the mortar ballistics: the catalog and flight model, the solver, the fire-mission planner
├── contracts/   shapes and policies two programs share, such as the offline cache policy
├── fleet/       the game server host agent beside each game server, which carries out the API's fleet commands
├── foundation/  dependency-free building blocks, such as the HTTP URL guard
├── frontend/  the single-page app's crates, one folder per layer, such as the session, the mission review record and the app shell
├── geometry/  plain geometry, map coordinates, camera arithmetic and spatial indexes, such as the BVH
├── graphics/  map-agnostic renderer building blocks, such as the render primitives
├── line_of_sight/  visibility over the bare ground, inside one building and through the placed world
├── map_overlay/  what the map draws on the terrain and in what order, such as the draw lanes and unit symbology
├── map_rendering/  the map's typed GPU layers and its renderer, such as the symbology layers
├── mission/   the mission domain's shared crates, such as the wire-safety scans
├── mission_editing/  the Mission Creator's editing layer over the mission document, such as the editing session
├── paper_doll/  the Arsenal's 3D paper doll: its scene and picking, and its renderer
├── streaming/  the streamed world: the chunk scheduler and draw buffers, the browser loaders and the map host
├── terrain/  the ground the map reads, such as the elevation model and the satellite container reader
├── world_formats/  the files a terrain's map data is stored in and their readers, such as the chunks
└── world_objects/  what stands on the ground, such as the vegetation and the building interiors
```

## How it works

The website's [API](/documentation/glossary/a_to_f.md#api) is the hub. Members use the single-page
app in a browser; the offline service worker beside it serves the offline packs from the cache. On
each game host the dedicated server runs the [mod](/documentation/glossary/g_to_m.md#mod) in
`mod/`, which talks to the API's `/api/v1/game-runtime/`, `/api/v1/ingest/` and
`/api/v1/fleet-executor/` routes. Beside the server, the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) polls the API's
`/api/v1/fleet-executor/` routes over outbound HTTPS, carries out each
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) (process control,
[RCON](/documentation/glossary/n_to_z.md#rcon) commands, a switch of the server's
[mission header](/documentation/glossary/g_to_m.md#mission-header)) and reports every step to the
command ledger.

```text
browser ── frontend_application ──▶ api_server ◀── HTTPS ── game_server_host_agent ─┐ controls
   │                                  ▲  ▲                                          ▼
   └ offline_service_worker           │  └── game-runtime, ingest ── dedicated server + mod
                                      │
                                   Postgres
```

The applications and where they sit:

| Application | Crate | Binaries |
|---|---|---|
| API server: the router and composition root over the domain, kernel and worker crates, with the integration suites in its `tests/` | [`crates/api/api_server`](/crates/api/api_server/README.md) | `api-server`, `import-item-registry` |
| Single-page app: the Leptos entry and the app frame over the frontend crates and the map crates | [`crates/frontend/shell/frontend_application`](/crates/frontend/shell/frontend_application/README.md) | `frontend_application` (built by Trunk) |
| Offline service worker: the offline pack caches and ranged reads from the cache | [`crates/frontend/shell/offline_service_worker`](/crates/frontend/shell/offline_service_worker/README.md) | `offline_service_worker` (built by Trunk) |
| Game server host agent: process control, RCON reads and mission header switches on a game host | [`crates/fleet/game_server_host_agent`](/crates/fleet/game_server_host_agent/README.md) | `game_server_host_agent` |

The ticketboard desktop viewer is a tool crate,
[`tools/tickets/ticketboard_desktop`](/tools/tickets/ticketboard_desktop/README.md).

Every manifest under `crates/` is a workspace member through the root `Cargo.toml`, which lists
one glob per category: `crates/<category>/*` for each category folder above (`crates/fleet/*`
among them) and `crates/frontend/*/*` for the frontend's layer folders (`foundation`, `features`,
`pages`, `workspaces`, `shell`). A manifest in a category no glob lists fails the crate-tier law.
Each crate declares `[package.metadata.layout]`: its `category` (the parent folder, such as
`crates/foundation` or `crates/frontend/shell`), its `tier` and its `targets`. A crate's tier is 0
with no workspace dependency and otherwise 1 plus the highest tier it depends on, so edges point
strictly down. Foundation crates depend on foundation crates only, most on none; contracts crates
depend on foundation crates only. Each library crate keeps the same anatomy: a `lib.rs` of at most
80 lines holding only the module header, `mod` lines and `pub use` lines, a `prelude` module, an
`error.rs` when its public API is fallible, and a README with a Contents block; a crate of
binaries only is exempt.

## Getting started

Run from the repository root:

```bash
cargo test -p http_url_guard -p offline_cache_policy   # two library crates' unit tests
cargo test -p game_server_host_agent                   # the game server host agent's tests
cargo xtask mk rust-api                                # the API server on port 8080; stays in the foreground
cargo xtask mk leptos                                  # the single-page app on 127.0.0.1:3000, in a second terminal
cargo xtask verify crate-tiers                         # membership, layout, tiers, category edges
cargo xtask verify crate-anatomy                       # lib.rs, prelude, error, README, manifest
```

## Boundaries

- Depends on: external crates from the root `[workspace.dependencies]` only; at run time
  Postgres, Discord and the Arma Reforger dedicated server.
- Used by: the members' browsers and the game hosts at run time; the tools under `tools/`, which
  link the library crates (never an application) and build, test, check and deploy the
  applications.
- Rules: every manifest here is a workspace member with a layout declaration whose category equals
  its parent folder and whose package name equals its folder name, and every dependency edge
  points to a lower tier along the category matrix (`cargo xtask verify crate-tiers`); no member
  depends on an application package, in any table (rule 7 of the crate-tier law); the products
  share data only over the API and through the schemas in `contracts/`; every library crate keeps
  the anatomy above (`cargo xtask verify crate-anatomy`).

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the crate-tier and
  crate-anatomy laws in full, and the dependency directions between the workspace crates.
- [Workspace layout](/documentation/architecture/workspace_layout.md) — every workspace member and
  where it sits.
- [Local development](/documentation/runbooks/local_development.md) — the full local setup of the
  API server and the single-page app.
