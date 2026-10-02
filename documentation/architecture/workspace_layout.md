**Status:** live

# Workspace layout

How the repository is laid out today: the top-level folders, the members of the Cargo workspace,
and where code, contracts, assets, documents and agent configuration live. The
[workspace restructure](/documentation/restructure/README.md) changes this layout stage by
stage; this document follows each stage's commit, and its last section says what changes next.

## Where it lives

- Code: the root [`Cargo.toml`](/Cargo.toml) (the workspace members),
  [`rust-toolchain.toml`](/rust-toolchain.toml) (Rust 1.95.0 with rustfmt, clippy and the
  `wasm32-unknown-unknown` target) and [`.cargo/config.toml`](/.cargo/config.toml), which makes
  `cargo xtask` run the `xtask` package.
- Entry: the [root README](/README.md) for a first look, the
  [directory atlas](/CLAUDE.md#2-monorepo-directory-atlas) for the folders one level deeper.

## Top-level folders

```text
TBD-reforger/
├── apps/            the products: API, single-page app, service worker, Enfusion mod suite,
│                    fleet host agent, ticketboard
├── crates/          the tiered library crates, grouped by category (foundation/, contracts/, geometry/, world_formats/, graphics/)
├── legacy/          the map and graphics engines, parked while their code moves into crates/
├── deploy/          the release Dockerfile, compose files, Caddy site (caddy/), deploy settings,
│                    systemd units
├── tools/           the developer tools: xtask, verification_core, ticket_engine, developer_tools,
│                    and the pinned Enfusion MCP npm package
├── contracts/       JSON Schemas, rules, catalogs and fixtures of every shape that crosses a boundary
├── assets/          terrain datasets (Git LFS), the world-object glyph set, the storage specification
├── documentation/   every document: feature docs, runbooks, standards, glossary, tickets, archive
├── .ai/             the ticket registry (tickets/) and agent artifacts of past waves
├── .github/         the five GitHub Actions workflows
├── .cursor/         the Cursor agent rules
├── .claude/         the Claude Code project settings
└── .cargo/          the cargo alias for xtask
```

The root files are the workspace manifest and lockfile, the toolchain pin, `clippy.toml`, the
build context of the API's release image (`.dockerignore`), the editor and checker settings (`.editorconfig`, `.editorconfig-checker.json`), the Git LFS patterns
(`.gitattributes`), the ignore rules (`.gitignore`), the per-mission warning budget of the mod
world-boot gate (`.world-boot-warning-baseline`), the root `README.md`, and `CLAUDE.md` with
`AGENTS.md` as a symlink to it. Build output is never tracked: `target/` and every `target-*/`
folder are ignored.

## Workspace members

One Cargo workspace (resolver 3) holds every Rust crate. Members inherit edition 2024 and
rust-version 1.95 from `[workspace.package]`, except the frontend, which declares edition 2021.

| Folder | Package | What it is |
|---|---|---|
| [`apps/api/`](/apps/api/README.md) | `api` | the Axum and sqlx REST API and SSE hub, with the `api` server and the registry import and staging fixture tools |
| [`apps/frontend/`](/apps/frontend/README.md) | `frontend` | the Leptos single-page app, compiled to WebAssembly and served by Trunk |
| [`legacy/map_engine/`](/legacy/map_engine/README.md) | `map_engine` | map graphics, spatial computation, terrain formats, streaming and the mission domain |
| [`legacy/graphics_engine/`](/legacy/graphics_engine/README.md) | `graphics_engine` | GPU rendering primitives with no map concept |
| [`apps/offline_service_worker/`](/apps/offline_service_worker/README.md) | `offline_service_worker` | the WebAssembly service worker behind offline packs |
| [`apps/fleet_host_agent/`](/apps/fleet_host_agent/README.md) | `fleet_host_agent` | the agent beside each game-server instance that carries out fleet commands |
| [`apps/ticketboard/`](/apps/ticketboard/README.md) | `ticketboard` | the egui desktop viewer of the ticket registry |
| [`crates/foundation/http_url_guard/`](/crates/foundation/http_url_guard/README.md) | `http_url_guard` | the HTTP(S) URL check the API and the single-page app share |
| [`crates/contracts/offline_cache_policy/`](/crates/contracts/offline_cache_policy/README.md) | `offline_cache_policy` | the offline cache names, request classes and fallback rules the service worker applies |
| [`crates/foundation/newtype_ids/`](/crates/foundation/newtype_ids/README.md) | `newtype_ids` | macros declaring serde-transparent typed ids |
| [`crates/foundation/time_source/`](/crates/foundation/time_source/README.md) | `time_source` | wall-clock and monotonic time sources, RFC 3339 UTC formatting and validation |
| [`crates/foundation/deterministic_random/`](/crates/foundation/deterministic_random/README.md) | `deterministic_random` | the seeded SplitMix64 generator |
| [`crates/foundation/content_digest/`](/crates/foundation/content_digest/README.md) | `content_digest` | SHA-256 and SHA-384 hex digests and framed hashing |
| [`crates/foundation/browser_platform/`](/crates/foundation/browser_platform/README.md) | `browser_platform` | browser console macros and fetch helpers (wasm32 only) |
| [`crates/geometry/geometry_primitives/`](/crates/geometry/geometry_primitives/README.md) | `geometry_primitives` | vector ops, segment geometry, rigid transforms, axis-aligned boxes |
| [`crates/geometry/map_coordinates/`](/crates/geometry/map_coordinates/README.md) | `map_coordinates` | terrain frames, chunk math, rounding, grid references |
| [`crates/geometry/camera_math/`](/crates/geometry/camera_math/README.md) | `camera_math` | the orthographic map camera, the orbit camera, 4x4 matrices |
| [`crates/world_formats/world_file_formats/`](/crates/world_formats/world_file_formats/README.md) | `world_file_formats` | the on-disk world formats and their typed ids |
| [`crates/graphics/render_primitives/`](/crates/graphics/render_primitives/README.md) | `render_primitives` | map-agnostic CPU rendering primitives: instances, geometry, triangulation, cull oracle, text, the WGSL shader |
| [`tools/xtask/`](/tools/xtask/README.md) | `xtask` | the `cargo xtask` command router: builds, gates, deploys, repository verifications |
| [`tools/verification_core/`](/tools/verification_core/README.md) | `verification_core` | fail-closed verdicts, pattern scans, process isolation and the verification lock |
| [`tools/ticket_engine/`](/tools/ticket_engine/README.md) | `ticket_engine` | ticket storage, validation, queue and roadmap sync |
| [`tools/developer_tools/`](/tools/developer_tools/README.md) | `developer_tools` | the heavy executables: script index, browser gates, MCP broker, world export, map assets, capture |

Every package is named after its folder, in snake_case. A crate under `crates/` sits in the
folder of its category and declares its tier in its manifest, and none depends on a crate in
`legacy/` (`cargo xtask verify crate-tiers`,
`cargo xtask verify strangler`). The mod suite under `apps/mod/` is not
Cargo code: its three Enfusion addons are built by Workbench and checked by `cargo xtask mod compile`.

## Where things live

```text
code ─────────── apps/<product>/            products, one folder each
                 crates/<category>/<crate>/ library crates, by category
                 legacy/<engine>/           the two engines while their code moves to crates/
                 tools/<tool>/              repository tooling
deploy ───────── deploy/                    release image, compose files, Caddy, systemd units
shapes ───────── contracts/definitions/     JSON Schemas, the source of generated contract types
                 contracts/fixtures/        golden test data, positive and negative
data ─────────── assets/terrains/           built-in islands, served at /map-assets (Git LFS)
                 assets/glyphs/             world-object glyph atlas and its SVG sources
documents ────── documentation/<code path>/ feature docs mirroring the code
                 documentation/runbooks/    procedures; standards/, glossary/, archive/ beside it
work tracking ── .ai/tickets/               one TOML per ticket, the queue and the templates
```

- **Code.** A product's code and its README sit in its folder under `apps/`; every folder carries
  a README.md built to the [README standard](/documentation/standards/readme_standard.md), and
  the code trees hold no other Markdown. Engine and layer boundaries are in the
  [engine boundary rules](/documentation/standards/engine_boundary_rules.md).
- **Contracts.** Every shape that crosses a network, process or language boundary is a schema in
  `contracts/definitions/`; `cargo xtask ci schema-codegen` generates the Rust contract types
  from it, and the backend models stay the snake_case source of truth of the API.
- **Assets.** Terrain datasets are Git LFS objects matched by `.gitattributes`; export scratch,
  equipment exports and terrain tiles are ignored and rebuilt by the export tools.
- **Documentation.** Every document lives under `documentation/`. A feature doc sits at the
  documentation root plus its code path without `src/`: `documentation/apps/api/` for `apps/api/`,
  `documentation/legacy/map_engine/` for `legacy/map_engine/`. Two mirrors keep a shorter path
  until a stage reshapes their code: the single-page app's documents also leave out `src/v2/`
  (until S3), and the mod's leave out `apps/` and `Scripts/Game/TBD/` and sit in
  `documentation/mod/` (until M1), as the
  [documentation standards](/documentation/standards/documentation_standards.md) set out.
- **Deployment.** `deploy/` holds what runs the platform outside a developer machine: the API's
  release `Dockerfile` (its build context narrowed by the root `.dockerignore`), the development
  and staging compose files, the Caddy site in `deploy/caddy/` (the one folder the staging Caddy
  container mounts, so a host's `deploy.env` stays outside it), the `deploy.env.example` that
  `cargo xtask deploy` reads, and the systemd units and timers in `deploy/systemd/`.
- **Agent configuration.** `CLAUDE.md` holds the project laws, the atlas and the canonical
  commands; `.cursor/rules/` the Cursor rules; `.claude/settings.json` the Claude Code settings.

## What changes next

The [restructure program](/documentation/restructure/README.md) runs in stages, each one commit
with its [relocation manifest](/documentation/restructure/manifests/README.md). Stage S1 renamed
the four top-level folders to `assets/`, `contracts/`, `documentation/` and `tools/`, gave the tool
crates snake_case names and archived the finished documentation program and the earlier layout
proposals. Stage S2 moved the API, the frontend and the service worker directly under `apps/`,
parked the map and graphics engines in `legacy/`, named every package after its folder (the fleet
host agent's binary, systemd units and configuration folder included), gathered the deployment
files in `deploy/` with the Caddy site in its own `deploy/caddy/` folder, moved the recorded API
responses to `contracts/fixtures/api_goldens/` and created the first two crates under `crates/`:
`http_url_guard`, the one URL check the API and the single-page app link, and
`offline_cache_policy`, the cache policy the service worker and the page share. The stages after
it:

- S3 splits the frontend's `src/v2/` layer in place.
- S4 to S11 build the tiered crates under `crates/`, from the foundations through the
  mission, world, streaming, rendering, API, frontend and tool crates, and delete `legacy/` and
  the ticket engine.
- M1 and M2 reshape the mod suite's folders; M1 also brings its documents to the full code path.
- S12 closes the program; this document then describes its end state.

Each stage that moves code also moves the code's documentation mirror. The end state is the
[target file tree](/documentation/restructure/target_file_tree.md); the archived
[architecture blueprint draft](/documentation/archive/restructure_research/00_architecture_blueprint_draft.md)
is where the design started.

## Related documentation

- [Restructure program](/documentation/restructure/README.md) — the plan, target tree and progress.
- [Documentation entry](/documentation/README.md) — the map of every document.
- [Where does X go?](/documentation/standards/where_does_x_go.md) — where a new file belongs.
