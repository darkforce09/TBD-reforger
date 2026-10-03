# Applications

Every product the TBD Reforger platform ships: the website's API, single-page app and service
worker, the
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) [mod](/documentation/glossary/g_to_m.md#mod) that
game servers run, the agent that controls those servers, and the desktop viewer for the
[ticket](/documentation/glossary/n_to_z.md#ticket) registry. Developer tooling lives in `tools/`,
not here.

## Contents

```text
apps/
├── api/                     the website's REST API and realtime hub, crate `api`
├── fleet_host_agent/        the game-host agent for fleet commands, crate `fleet_host_agent`
├── frontend/                the website's single-page app and Mission Creator, crate `frontend`
├── mod/                     the Enfusion mod suite: game mod, Workbench export addon, MCP bridge
├── offline_service_worker/  the website's service worker for offline packs, crate `offline_service_worker`
└── ticketboard/             the native desktop viewer of the ticket registry, crate `ticketboard`
```

## How it works

The website's [API](/documentation/glossary/a_to_f.md#api) is the hub. Members use the single-page
app in a browser. On each game host, the dedicated server runs the mod, which reads its
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) and
[event](/documentation/glossary/a_to_f.md#event) roster from the API's
`/api/v1/game-runtime/` routes, reports server status and match results to `/api/v1/ingest/`,
and carries out the in-game fleet commands, such as loading a
[mission](/documentation/glossary/g_to_m.md#mission), through `/api/v1/fleet-executor/`. Beside the
server, the [fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) polls the API's
`/api/v1/fleet-executor/` routes over outbound HTTPS, carries out each
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) (process control,
[RCON](/documentation/glossary/n_to_z.md#rcon) commands, a switch of the server's
[mission header](/documentation/glossary/g_to_m.md#mission-header)) and reports every step to the
command ledger. [Ticketboard](/documentation/glossary/n_to_z.md#ticketboard) stands apart: it
reads `.ai/tickets/` through the `ticket_engine` crate in `tools/` and talks to none of the
others.

```text
browser ── frontend ──▶ api ◀── HTTPS ── fleet_host_agent ─┐ controls
   │                     ▲  ▲                               ▼
   └ offline_service_worker  └── game-runtime, ingest ── dedicated server + mod
                         │
                      Postgres

ticketboard ──▶ ticket_engine (tools/) ──▶ .ai/tickets/
```

The five Rust crates (`api`, `frontend`, `offline_service_worker`, `fleet_host_agent` and
`ticketboard`) are members of the root Cargo workspace; the frontend and the API link the map and
graphics engines parked in `legacy/`. The mod is Enfusion script and data, built and checked by
the xtask `mod` commands.

## Getting started

Run these from the repository root. The website, in this order (the full procedure is in
[local development](/documentation/runbooks/local_development.md)):

```bash
cargo xtask db up        # Postgres on host port 5434
cargo xtask mk rust-api  # the API on port 8080; stays in the foreground
cargo xtask mk leptos    # the app on 127.0.0.1:3000; stays in the foreground, in a second terminal
```

The other products:

```bash
cargo xtask mod compile           # compile-checks the mod's scripts in a headless Enfusion
cargo test -p fleet_host_agent    # the host agent's tests
cargo run -p ticketboard          # opens the ticket registry viewer; stays in the foreground
```

## Boundaries

- Depends on: `contracts/`, the schemas and rules shared across the API, the mod and the host
  agent; `assets/`, the map data the API serves; `tools/ticket_engine/`, which ticketboard
  reads the registry through; Postgres, Discord and the Arma Reforger dedicated server.
- Used by: the members' browsers and the game servers at run time; the xtask commands in
  `tools/xtask/` that build, test, check and deploy the products; and the developer tools in
  `tools/developer_tools/`, which link the map engine and drive the app in a headless browser.
- Rules: the products share data only over the API and through the schemas in `contracts/`: no
  crate here depends on a crate of another product, apart from the website's three (the frontend
  links `offline_service_worker`); the path dependencies leaving `apps/` go to `legacy/` (the
  engines), `crates/` and `tools/` (`repository_laws` and `verification_core` for the API, `ticket_engine` for
  ticketboard); the engine layer rules are held by `cargo xtask verify engine-layers`.

## Related documentation

- [Documentation](/documentation/README.md) — the map of every deeper document.
- [Local development](/documentation/runbooks/local_development.md) — the full local setup.
- [Mod documentation](/documentation/mod/README.md) — the mod's design, screens and export
  evidence.
