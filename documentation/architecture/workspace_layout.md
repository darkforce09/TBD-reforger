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
├── apps/            the products: website, Enfusion mod suite, fleet host agent, ticketboard
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
editor and checker settings (`.editorconfig`, `.editorconfig-checker.json`), the Git LFS patterns
(`.gitattributes`), the ignore rules (`.gitignore`), the per-mission warning budget of the mod
world-boot gate (`.world-boot-warning-baseline`), the root `README.md`, and `CLAUDE.md` with
`AGENTS.md` as a symlink to it. Build output is never tracked: `target/` and every `target-*/`
folder are ignored.

## Workspace members

One Cargo workspace (resolver 3) holds every Rust crate. Members inherit edition 2024 and
rust-version 1.95 from `[workspace.package]`, except the frontend, which declares edition 2021.

| Folder | Package | What it is |
|---|---|---|
| [`apps/website/api_v2/`](/apps/website/api_v2/README.md) | `website-api` | the Axum and sqlx REST API and SSE hub, with the `api` server and the registry import and staging fixture tools |
| [`apps/website/frontend/`](/apps/website/frontend/README.md) | `website-frontend` | the Leptos single-page app, compiled to WebAssembly and served by Trunk |
| [`apps/website/map-engine/`](/apps/website/map-engine/README.md) | `website-map-engine` | map graphics, spatial computation, terrain formats, streaming and the mission domain |
| [`apps/website/graphics-engine/`](/apps/website/graphics-engine/README.md) | `website-graphics-engine` | GPU rendering primitives with no map concept |
| [`apps/website/offline-service-worker/`](/apps/website/offline-service-worker/README.md) | `website-offline-service-worker` | the WebAssembly service worker behind offline packs |
| [`apps/fleet_host_agent/`](/apps/fleet_host_agent/README.md) | `fleet-host-agent` | the agent beside each game-server instance that carries out fleet commands |
| [`apps/ticketboard/`](/apps/ticketboard/README.md) | `ticketboard` | the egui desktop viewer of the ticket registry |
| [`tools/xtask/`](/tools/xtask/README.md) | `xtask` | the `cargo xtask` command router: builds, gates, deploys, repository verifications |
| [`tools/verification_core/`](/tools/verification_core/README.md) | `verification_core` | fail-closed verdicts, pattern scans, process isolation and the verification lock |
| [`tools/ticket_engine/`](/tools/ticket_engine/README.md) | `ticket_engine` | ticket storage, validation, queue and roadmap sync |
| [`tools/developer_tools/`](/tools/developer_tools/README.md) | `developer_tools` | the heavy executables: script index, browser gates, MCP broker, world export, map assets, capture |

The tool packages and folders are snake_case; the website packages keep their hyphenated
`website-*` names until the restructure renames them. The mod suite under `apps/mod/` is not
Cargo code: its three Enfusion addons are built by Workbench and checked by `cargo xtask mod compile`.

## Where things live

```text
code ─────────── apps/<product>/            products, one folder each
                 tools/<tool>/              repository tooling
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
  documentation root plus its code path without `src/`; mirrors of code under `apps/` also leave
  out `apps/`, `src/v2/` and `Scripts/Game/TBD/` until the restructure moves those trees, as the
  [documentation standards](/documentation/standards/documentation_standards.md) set out.
- **Agent configuration.** `CLAUDE.md` holds the project laws, the atlas and the canonical
  commands; `.cursor/rules/` the Cursor rules; `.claude/settings.json` the Claude Code settings.

## What changes next

The [restructure program](/documentation/restructure/README.md) runs in stages, each one commit
with its [relocation manifest](/documentation/restructure/manifests/README.md). Stage S1 renamed
the four top-level folders to `assets/`, `contracts/`, `documentation/` and `tools/`, gave the tool
crates snake_case names and archived the finished documentation program and the earlier layout
proposals. The stages after it:

- S2 moves the API, the frontend and the service worker directly under apps/, parks the map and
  graphics engines in a legacy folder, gives every package a snake_case name and adds a top-level
  deploy folder.
- S3 splits the frontend's `src/v2/` layer in place.
- S4 to S11 build the tiered crates under a new crates folder, from the foundations through the
  mission, world, streaming, rendering, API, frontend and tool crates, and delete the legacy
  engines and the ticket engine.
- S12 closes the program; this document then describes its end state.

Each stage that moves code also moves the code's documentation mirror. The end state is the
[target file tree](/documentation/restructure/target_file_tree.md); the archived
[architecture blueprint draft](/documentation/archive/restructure_research/00_architecture_blueprint_draft.md)
is where the design started.

## Related documentation

- [Restructure program](/documentation/restructure/README.md) — the plan, target tree and progress.
- [Documentation entry](/documentation/README.md) — the map of every document.
- [Where does X go?](/documentation/standards/where_does_x_go.md) — where a new file belongs.
