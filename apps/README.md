# Game mod

The home of the Arma Reforger [mod](/documentation/glossary/g_to_m.md#mod) suite: the
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) script and data addons that the game servers
run and that [Workbench](/documentation/glossary/n_to_z.md#workbench) opens. No Rust crate lives
here: the website's API server, single-page app and offline service worker and the game server
host agent are crates under [`crates/`](/crates/README.md), and the ticketboard desktop viewer and
every other developer tool are crates under [`tools/`](/tools/README.md).

## Contents

```text
apps/
└── mod/  the Enfusion mod suite: game mod, Workbench export addon, MCP bridge
```

## How it works

A dedicated server loads the game mod, boots its
[mission header](/documentation/glossary/g_to_m.md#mission-header) and fetches the
[mission](/documentation/glossary/g_to_m.md#mission) deployed to it from the website's
[API](/documentation/glossary/a_to_f.md#api): it reads its
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) and
[event](/documentation/glossary/a_to_f.md#event) roster from the `/api/v1/game-runtime/` routes,
reports server status and match results to `/api/v1/ingest/`, and carries out the in-game
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command), such as loading a mission,
through `/api/v1/fleet-executor/`. The two Workbench addons run in the editor only: one exports the
game data the platform ingests, the other lets the Enfusion MCP tools drive Workbench. The
[mod suite README](/apps/mod/README.md) describes the three addons and their dependencies.

```text
dedicated server ── loads ──▶ apps/mod/tbd-framework ── HTTPS ──▶ crates/api/api_server
Workbench ── opens ──▶ apps/mod/tbd-export (+ apps/mod/tbd-emcp) ◀── Net API ── cargo xtask mcp
```

## Getting started

Run these from the repository root:

```bash
cargo xtask mod compile               # compile-checks the mod's scripts in a headless Enfusion
cargo xtask verify enfusion-comments  # the Enfusion comment card over the pinned mod Scripts roots
```

## Boundaries

- Depends on: the vanilla Arma Reforger addon; `contracts/`, the schemas the mod's JSON shapes
  follow; the website's API at run time.
- Used by: the dedicated game servers; Workbench; the xtask `mod`, `mcp`, `setup` and `deploy`
  commands in `tools/xtask/` that build, check, boot and deploy the addons.
- Rules: `apps/` holds the mod and no Rust crate: a `Cargo.toml` placed here is a finding of the
  crate-tier law (`cargo xtask verify crate-tiers`); the mod's scripts keep the file-length ceiling
  and the Enfusion comment card (`cargo xtask verify file-length`,
  `cargo xtask verify enfusion-comments`).

## Related documentation

- [Mod suite](/apps/mod/README.md) — the three addons and their dependencies.
- [Mod documentation](/documentation/apps/mod/README.md) — the mod's design, screens and export
  evidence.
- [Crates](/crates/README.md) — the applications and library crates of the website and the fleet.
- [Documentation](/documentation/README.md) — the map of every deeper document.
