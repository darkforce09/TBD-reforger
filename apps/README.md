# Applications

Every product the TBD Reforger platform ships: the web platform, the
[Enfusion](/documentation_v2/glossary.md#enfusion) [mod](/documentation_v2/glossary.md#mod) that
game servers run, the agent that controls those servers, and the desktop viewer for the
[ticket](/documentation_v2/glossary.md#ticket) registry. Developer tooling lives in `tools_v2/`,
not here.

## Contents

```text
apps/
├── fleet_host_agent/  the game-host agent for fleet commands, crate `fleet-host-agent`
├── mod/               the Enfusion mod suite: game mod, Workbench export addon, MCP bridge
├── ticketboard/       the native desktop viewer of the ticket registry, crate `ticketboard`
└── website/           the web platform: REST API, single-page app, map and graphics engines
```

## How it works

The website's [API](/documentation_v2/glossary.md#api) is the hub. Members use the single-page
app in a browser. On each game host, the dedicated server runs the mod, which reads its
[mission deployment](/documentation_v2/glossary.md#mission-deployment) and
[event](/documentation_v2/glossary.md#event) roster from the API's
`/api/v1/game-runtime/` routes, reports server status and match results to `/api/v1/ingest/`,
and carries out the in-game fleet commands, such as loading a
[mission](/documentation_v2/glossary.md#mission), through `/api/v1/fleet-executor/`. Beside the
server, the [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) polls the API's
`/api/v1/fleet-executor/` routes over outbound HTTPS, carries out each
[fleet command](/documentation_v2/glossary.md#fleet-command) (process control,
[RCON](/documentation_v2/glossary.md#rcon) commands, a switch of the server's
[mission header](/documentation_v2/glossary.md#mission-header)) and reports every step to the
command ledger. [Ticketboard](/documentation_v2/glossary.md#ticketboard) stands apart: it
reads `.ai/tickets/` through the `ticket-engine` crate in `tools_v2/` and talks to none of the
others.

```text
browser ── website/frontend ──▶ website/api_v2 ◀── HTTPS ── fleet_host_agent ─┐ controls
                                  ▲    ▲                                      ▼
                                  │    └── game-runtime, ingest ─────── dedicated server + mod
                               Postgres

ticketboard ──▶ ticket-engine (tools_v2/) ──▶ .ai/tickets/
```

The six Rust crates (`website-api`, `website-frontend`, `website-map-engine`,
`website-graphics-engine`, `fleet-host-agent` and `ticketboard`) are members of the root Cargo
workspace; the mod is Enfusion script and data, built and checked by the xtask `mod` commands.

## Getting started

Run these from the repository root. The website, in this order (the full procedure is in
`apps/website/README.md`):

```bash
cargo xtask db up        # Postgres on host port 5434
cargo xtask mk rust-api  # the API on port 8080; stays in the foreground
cargo xtask mk leptos    # the app on 127.0.0.1:3000; stays in the foreground, in a second terminal
```

The other products:

```bash
cargo xtask mod compile           # compile-checks the mod's scripts in a headless Enfusion
cargo test -p fleet-host-agent    # the host agent's tests
cargo run -p ticketboard          # opens the ticket registry viewer; stays in the foreground
```

## Boundaries

- Depends on: `contracts_v2/`, the schemas and rules shared across the API, the mod and the host
  agent; `assets_v2/`, the map data the API serves; `tools_v2/ticket-engine/`, which ticketboard
  reads the registry through; Postgres, Discord and the Arma Reforger dedicated server.
- Used by: the members' browsers and the game servers at run time; the xtask commands in
  `tools_v2/xtask/` that build, test, check and deploy the products; and the developer tools in
  `tools_v2/developer-tools/`, which link the map engine and drive the app in a headless browser.
- Rules: the products share data only over the API and through the schemas in `contracts_v2/`: no
  crate here depends on a crate of another product (the only cross-folder path dependencies are
  inside `apps/website/` and ticketboard's on `tools_v2/ticket-engine/`); the engine layer rules
  of `apps/website/` are held by `cargo xtask verify engine-layers`.

## Related documentation

- [Documentation](/documentation_v2/README.md) — the map of every deeper document.
- [Local development](/documentation_v2/runbooks/local_development.md) — the full local setup.
- [Mod documentation](/documentation_v2/mod/README.md) — the mod's design, screens and export
  evidence.
