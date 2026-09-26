# TBD Reforger Platform

The monorepo of the TBD Arma Reforger milsim community: the website with its
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator), the
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) [mod](/documentation_v2/glossary/g_to_m.md#mod)
that game servers run, the [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent)
that controls those servers, the contracts and map data they share, and the developer tools behind
`cargo xtask`. Start here, then read [CLAUDE.md](/CLAUDE.md) for the project laws and the
[documentation entry](/documentation_v2/README.md) for everything deeper.

## Contents

```text
./
├── .ai/                         the ticket registry (`tickets/`), agent run artifacts and the factory wave marker
├── .cargo/                      the `cargo xtask` alias
├── .claude/                     Claude Code project settings: the `xtask ai guard` hook
├── .cursor/                     the Cursor agent rules (`rules/*.mdc`) and MCP server entry (`mcp.json`)
├── .editorconfig                the editor formatting rules `cargo xtask ci verify-editorconfig` checks
├── .editorconfig-checker.json   the settings of that check
├── .gitattributes               the Git LFS patterns for the terrain datasets
├── .github/                     the GitHub Actions workflows: CI, contracts, editor gates, mod gates, schema
├── .gitignore                   keeps the deploy settings, build output, export scratch and local reference copies out of git
├── .world-boot-warning-baseline the per-mission warning budget of `cargo xtask mod world-boot`
├── AGENTS.md                    a symlink to CLAUDE.md for agents that read AGENTS.md
├── apps/                        the products: website, mod suite, fleet host agent, ticketboard
├── assets_v2/                   terrain datasets and the world-object glyph set, served at `/map-assets`
├── Cargo.lock                   the workspace lockfile
├── Cargo.toml                   the Cargo workspace: the six app crates and four tooling crates
├── CLAUDE.md                    the agent entry file: project laws, directory atlas, canonical commands
├── contracts_v2/                JSON Schemas, rules, catalogs and fixtures of every cross-boundary shape
├── documentation_v2/            all documentation: feature docs, runbooks, standards, glossary, archive
├── rust-toolchain.toml          the pinned Rust toolchain with rustfmt, clippy and the wasm32 target
└── tools_v2/                    the developer tools: `xtask`, ticket engine, developer tools, verification core
```

## How it works

Mission makers build a [mission](/documentation_v2/glossary/g_to_m.md#mission) in the Mission Creator,
a page of the website's single-page app. The [API](/documentation_v2/glossary/a_to_f.md#api) stores
it, compiles it into an immutable [artifact](/documentation_v2/glossary/a_to_f.md#artifact) and
deploys it to a game server. The mod on that server fetches the deployed mission and runs the
session; the fleet host agent beside it carries out the server commands the API queues. The
[event](/documentation_v2/glossary/a_to_f.md#event) schedule, [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) slotting, leaderboards and doctrine
pages sit on the same API. `contracts_v2/` defines every shape these programs exchange, and
`tools_v2/` builds, checks and deploys all of it.

```text
browser ── apps/website/frontend (Mission Creator)
                       │
                       │ /api/v1, SSE, /map-assets
                       ▼
           apps/website/api_v2 ──▶ Postgres; serves assets_v2/terrains at /map-assets
                       ▲
                       │ HTTPS, outbound from the game side
           ┌───────────┴───────────┐
  apps/mod/tbd-framework   apps/fleet_host_agent
  (dedicated server:       (game host: claims
   fetches the mission)     fleet commands)

contracts_v2/  the shapes all of these exchange
tools_v2/      cargo xtask: build, gates, deploy
```

## Getting started

The [local development runbook](/documentation_v2/runbooks/local_development.md) brings the database,
the API and the single-page app up step by step; the canonical commands, in the order they run, are
section 3 of [CLAUDE.md](/CLAUDE.md). `cargo xtask help` lists the build, CI and database tasks, and
`cargo xtask --help` the full command tree.

## Boundaries

- Depends on: the Rust toolchain in `rust-toolchain.toml` with Trunk for the web app; Postgres in a
  container for local development; Git LFS for terrain data; Discord OAuth for sign-in; Arma
  Reforger [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) and the Enfusion MCP server for mod work.
- Used by: community members through the website; the dedicated game servers that load the mod;
  the game hosts that run the fleet host agent; operators who deploy with `cargo xtask deploy`.
- Rules: the project laws in [CLAUDE.md](/CLAUDE.md), held by review and by the gates
  `cargo xtask ci ci-local` runs; every folder in `apps/`, `tools_v2/`, `contracts_v2/`, `assets_v2/`
  and `documentation_v2/` carries a README.md that lists its children
  (`cargo xtask verify readme-coverage`); the code trees hold no Markdown but README.md
  (`cargo xtask verify markdown-placement`); every link, path and cited command resolves (`cargo xtask verify link-check`).

## Related documentation

- [Documentation](/documentation_v2/README.md) — the map of every document and the authority ladder.
- [CLAUDE.md](/CLAUDE.md) — project laws, directory atlas and canonical commands.
- [Glossary](/documentation_v2/glossary/README.md) — the project's terms.
- [Applications](/apps/README.md) — the products in `apps/`.
