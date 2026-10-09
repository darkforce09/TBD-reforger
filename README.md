# TBD Reforger Platform

The monorepo of the TBD Arma Reforger milsim community: the website with its
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) [mod](/documentation/glossary/g_to_m.md#mod)
that game servers run, the [game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent)
that controls those servers, the contracts and map data they share, and the developer tools behind
`cargo xtask`. Start here, then read [CLAUDE.md](/CLAUDE.md) for the project laws and the
[documentation entry](/documentation/README.md) for everything deeper.

## Contents

```text
./
├── .ai/                         the ticket registry (`tickets/`), agent run artifacts and the factory wave marker
├── .cargo/                      the `cargo xtask` alias
├── .claude/                     Claude Code project settings: the `xtask ai guard` hook
├── .cursor/                     the Cursor agent rules (`rules/*.mdc`) and MCP server entry (`mcp.json`)
├── .dockerignore                the build context of the API release image: the workspace crates and the contracts they embed
├── .editorconfig                the editor formatting rules `cargo xtask ci verify-editorconfig` checks
├── .editorconfig-checker.json   the settings of that check
├── .gitattributes               the Git LFS patterns for the terrain datasets
├── .github/                     the GitHub Actions workflows: CI, contracts, editor gates, mod gates, schema
├── .gitignore                   keeps the deploy settings, build output, export scratch and local reference copies out of git
├── .world-boot-warning-baseline the per-mission warning budget of `cargo xtask mod world-boot`
├── AGENTS.md                    a symlink to CLAUDE.md for agents that read AGENTS.md
├── assets/                      terrain datasets and the world-object glyph set, served at `/map-assets`
├── Cargo.lock                   the workspace lockfile
├── Cargo.toml                   the Cargo workspace: the product crates, the applications among them, and the tool crates
├── CLAUDE.md                    the agent entry file: project laws, directory atlas, canonical commands
├── clippy.toml                  the clippy settings every workspace crate reads: tests may call `unwrap()`
├── contracts/                   JSON Schemas, rules, catalogs and fixtures of every cross-boundary shape
├── crates/                      the tiered product crates by category: API server, single-page app, service worker, game server host agent and their libraries
├── deploy/                      the release Dockerfile, compose files, Caddy site, deploy settings template, systemd units
├── documentation/               all documentation: feature docs, runbooks, standards, glossary, archive
├── mod/                         the Enfusion mod suite the game servers run; no Rust crate
├── rust-toolchain.toml          the pinned Rust toolchain with rustfmt, clippy and the wasm32 target
└── tools/                       the developer tools: `xtask`, the ticket crates and the ticketboard desktop viewer, `developer_tools`, the tool foundations
```

## How it works

Mission makers build a [mission](/documentation/glossary/g_to_m.md#mission) in the Mission Creator,
a page of the website's single-page app. The [API](/documentation/glossary/a_to_f.md#api) stores
it, compiles it into an immutable [artifact](/documentation/glossary/a_to_f.md#artifact) and
deploys it to a game server. The mod on that server fetches the deployed mission and runs the
session; the game server host agent beside it carries out the server commands the API queues. The
[event](/documentation/glossary/a_to_f.md#event) schedule, [ORBAT](/documentation/glossary/n_to_z.md#orbat) slotting, leaderboards and doctrine
pages sit on the same API. `contracts/` defines every shape these programs exchange, and
`tools/` builds, checks and deploys all of it.

```text
browser ── crates/frontend/shell/frontend_application (Mission Creator)
                       │
                       │ /api/v1, SSE, /map-assets
                       ▼
           crates/api/api_server ──▶ Postgres; serves assets/terrains at /map-assets
                       ▲
                       │ HTTPS, outbound from the game side
           ┌───────────┴───────────┐
  mod/tbd-framework        crates/fleet/game_server_host_agent
  (dedicated server:       (game host: claims
   fetches the mission)     fleet commands)

contracts/  the shapes all of these exchange
tools/      cargo xtask: build, gates, deploy
```

## Getting started

The [local development runbook](/documentation/runbooks/local_development.md) brings the database,
the API and the single-page app up step by step; the canonical commands, in the order they run, are
section 3 of [CLAUDE.md](/CLAUDE.md). `cargo xtask help` lists the build, CI and database tasks, and
`cargo xtask --help` the full command tree.

## Boundaries

- Depends on: the Rust toolchain in `rust-toolchain.toml` with Trunk for the web app; Postgres in a
  container for local development; Git LFS for terrain data; Discord OAuth for sign-in; Arma
  Reforger [Workbench](/documentation/glossary/n_to_z.md#workbench) and the Enfusion MCP server for mod work.
- Used by: community members through the website; the dedicated game servers that load the mod;
  the game hosts that run the game server host agent; operators who deploy with `cargo xtask deploy`.
- Rules: the project laws in [CLAUDE.md](/CLAUDE.md), held by review and by the gates
  `cargo xtask ci ci-local` runs; every folder below the root but the test, generated-output and
  hidden ones carries a README.md that lists its children
  (`cargo xtask verify readme-coverage`); the code trees hold no Markdown but README.md
  (`cargo xtask verify markdown-placement`); every link, path and cited command resolves (`cargo xtask verify link-check`).

## Related documentation

- [Documentation](/documentation/README.md) — the map of every document and the authority ladder.
- [CLAUDE.md](/CLAUDE.md) — project laws, directory atlas and canonical commands.
- [Glossary](/documentation/glossary/README.md) — the project's terms.
- [Crates](/crates/README.md) — the product crates by category, the applications among them.
- [Game mod](/mod/README.md) — the Enfusion mod suite in `mod/`.
