**Status:** live

# README template: area root

**When to use:** the top of a code tree, or a folder that groups several products without being one
(`crates/`, `apps/`, `apps/mod/`, `tools/`, `contracts/`, `assets/`). The
[README standard](/documentation/standards/readme_standard.md) defines every rule this template
follows; the area root kind adds Getting started.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. An area root that
also holds files of a later kind, such as deploy files, adds that kind's sections after Getting
started, in kind-table order.

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

- [<document title>](/documentation/<path to the document>) — <what it covers>
````

## Worked sample

Written from `crates/`. The sample sits in a fenced block, so no gate reads it as a README; the
folder's own README.md is written from the same code and may differ.

````markdown
# Crates

The workspace's product crates, one folder per category. The applications are crates here like
any other: the API server, the single-page app and its offline service worker, and the game
server host agent; the library crates under them hold the domains, the map engine and the shared
foundations.

## Contents

```text
crates/
├── api/       the API's crates and the server `api_server` that assembles them
├── fleet/     the game server host agent beside each game server, crate `game_server_host_agent`
├── frontend/  the single-page app's crates by layer; the app `frontend_application` and the offline service worker in `shell/`
└── mission/   the mission domain's shared crates, such as the wire-safety scans
```

## How it works

The API is the hub. Members use the single-page app in a browser. On each game host, the dedicated
server runs the mod in `apps/mod/`, which reads its mission deployment from the API's
`/api/v1/game-runtime/` routes and reports results to `/api/v1/ingest/`. Beside the server, the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) polls
`/api/v1/fleet-executor/` over outbound HTTPS and carries out each fleet command.

```text
browser ── frontend_application ──▶ api_server ◀── HTTPS ── game_server_host_agent ─┐ controls
                                     ▲  ▲                                          ▼
                                     │  └── game-runtime, ingest ── dedicated server + mod
                                  Postgres
```

## Getting started

Run these from the repository root, in this order (the full procedure is in the local development
runbook):

```bash
cargo xtask db up        # Postgres on host port 5434
cargo xtask mk rust-api  # the API server on port 8080; stays in the foreground
cargo xtask mk leptos    # the app on 127.0.0.1:3000; stays in the foreground, in a second terminal
```

## Boundaries

- Depends on: `contracts/`, the schemas shared across the API, the mod and the host agent;
  `assets/`, the map data the API serves; Postgres, Discord and the Arma Reforger dedicated server.
- Used by: the members' browsers and the game servers at run time; the xtask commands that build,
  test, check and deploy the products.
- Rules: the products share data only over the API and through the schemas in `contracts/`; no
  member depends on an application, and the crate tiers law and its firewalls hold every edge
  (`cargo xtask verify crate-tiers`).

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — the full local setup.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the laws every crate
  is held to.
````
