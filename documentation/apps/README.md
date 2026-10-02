**Status:** live

# Application documentation

The documentation of the products in `apps/` that are Rust crates: the REST
[API](/documentation/glossary/a_to_f.md#api) and its [SSE](/documentation/glossary/n_to_z.md#sse)
streams, the single-page app with every page and the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the
[fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) and
[ticketboard](/documentation/glossary/n_to_z.md#ticketboard). Developers and AI agents read it below
the code READMEs, for behaviour, design, open work and decisions. The mod's documents are in
[mod/](/documentation/mod/README.md), and the map and graphics engines' in
[legacy/](/documentation/legacy/README.md).

## Contents

```text
documentation/apps/
├── api/               the API: overview, environment variables, decisions and verification evidence
├── fleet_host_agent/  the host agent: how it carries out fleet commands
├── frontend/          the single-page app: page and app feature docs, design references, the editor corpus
└── ticketboard/       the ticket registry viewer: its design
```

## How it works

Each folder sits at its crate's path with the leading `apps/` replaced by `documentation/apps/`
and `src/` left out; the single-page app's documents also leave out `src/v2/` until the
[restructure](/documentation/restructure/README.md) reshapes that tree. Each folder opens with a
README index; the documents inside follow the templates in `documentation/standards/templates/`:
feature docs for a page, an app or a cross-cutting subject, and `decisions.md` logs for the
decisions behind them.

| Crate | Code | Documentation |
|---|---|---|
| `api` | [`apps/api/`](/apps/api/README.md): Axum and sqlx on Postgres, serving `/api/v1` on port 8080 | [API documentation](/documentation/apps/api/README.md) |
| `frontend` | [`apps/frontend/`](/apps/frontend/README.md): Leptos 0.8 compiled to WebAssembly, served by Trunk on port 3000 in development | [frontend documentation](/documentation/apps/frontend/README.md) |
| `fleet_host_agent` | [`apps/fleet_host_agent/`](/apps/fleet_host_agent/README.md): the agent beside each game-server instance | [host agent documentation](/documentation/apps/fleet_host_agent/README.md) |
| `ticketboard` | [`apps/ticketboard/`](/apps/ticketboard/README.md): the native egui viewer of `.ai/tickets/` | [ticketboard documentation](/documentation/apps/ticketboard/README.md) |

The browser runs the app, which calls the API over `/api/v1` and SSE and streams terrain from
`/map-assets`; both link the map engine in `legacy/`. The service worker in
`apps/offline_service_worker/` is documented with the app's offline support in
[frontend/](/documentation/apps/frontend/README.md). The
[applications README](/apps/README.md) draws the whole picture and gives the commands that run
each product.

## Code

- [Applications](/apps/README.md) — every product and how they talk to each other.
- [API](/apps/api/README.md) — described under `api/`.
- [Single-page app](/apps/frontend/README.md) — described under `frontend/`.
- [Fleet host agent](/apps/fleet_host_agent/README.md) — described under `fleet_host_agent/`.
- [Ticketboard](/apps/ticketboard/README.md) — described under `ticketboard/`.

## Boundaries

- Depends on: the code under `apps/`, which every document is checked against; the
  [README standard](/documentation/standards/readme_standard.md) and the templates in
  `documentation/standards/templates/`; the glossary for its terms.
- Used by: the [applications README](/apps/README.md) and the READMEs of its crates, which link
  these documents under Related documentation; the runbooks, which link the API, host agent and
  page docs.
- Rules: a folder here mirrors a code folder and keeps its spelling; a document describes the
  committed code, and a disagreement between a document and the code is recorded with both
  places and resolved in the code's favour.

## Related documentation

- [Local development](/documentation/runbooks/local_development.md) — running the API, the
  database and the app locally.
- [Website deployment](/documentation/runbooks/website_deployment.md) — deploying the API and
  the app to the deploy host.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — deploying the
  game servers and their host agents.
- [Glossary](/documentation/glossary/README.md) — the platform's terms.
