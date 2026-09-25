**Status:** live

# Website documentation

The documentation of the community's web platform in `apps/website/`: the REST
[API](/documentation_v2/glossary.md#api) and its [SSE](/documentation_v2/glossary.md#sse)
streams, and the single-page app with every page and the
[Mission Creator](/documentation_v2/glossary.md#mission-creator). Developers and AI agents read it
below the code READMEs, for behaviour, design, open work and decisions.

## Contents

```text
documentation_v2/website/
├── api_v2/    the API: overview, environment variables, decisions and verification evidence
├── frontend/  the single-page app: page and app feature docs, design references, the editor corpus
├── graphics-engine/  the pure renderer: its modules, one frame, design and open work
└── map-engine/  the map engine: overview, map streaming, the editing layer, draft persistence
```

## How it works

The folders mirror `apps/website/` and keep its code folder spellings, so the documentation of a
crate sits at the same path as its code with `apps/` replaced by `documentation_v2/`. Each folder
opens with a README index; the documents inside follow the templates in
`documentation_v2/standards/templates/`: feature docs for a page, an app or a cross-cutting
subject, and `decisions.md` logs for the decisions behind them.

The four crates of the platform, and where their documentation starts:

| Crate | Code | Documentation |
|---|---|---|
| `website-api` | [`apps/website/api_v2/`](/apps/website/api_v2/README.md): Axum and sqlx on Postgres, serving `/api/v1` on port 8080 | [API documentation](/documentation_v2/website/api_v2/README.md) |
| `website-frontend` | [`apps/website/frontend/`](/apps/website/frontend/README.md): Leptos 0.8 compiled to WebAssembly, served by Trunk on port 3000 in development | [frontend documentation](/documentation_v2/website/frontend/README.md) |
| `website-map-engine` | [`apps/website/map-engine/`](/apps/website/map-engine/README.md): the world, streaming, spatial queries, the map and the [mission](/documentation_v2/glossary.md#mission) domain, with no UI dependency | its code READMEs |
| `website-graphics-engine` | [`apps/website/graphics-engine/`](/apps/website/graphics-engine/README.md): the `wgpu` renderer on WebGPU, with a WebGL backend a caller can force, which knows no map concept | its code READMEs |

The browser runs the app, which calls the API over `/api/v1` and SSE and streams terrain from
`/map-assets`; both link the map engine, and only the map engine uses the graphics engine. The
[website README](/apps/website/README.md) draws the whole picture and gives the commands that run
it.

## Code

- [Website platform](/apps/website/) — the four crates, the API's release image and the staging
  compose file.
- [API](/apps/website/api_v2/) — described under `api_v2/`.
- [Single-page app](/apps/website/frontend/) — described under `frontend/`.

## Boundaries

- Depends on: the code under `apps/website/`, which every document is checked against; the
  [README standard](/documentation_v2/standards/readme_standard.md) and the templates in
  `documentation_v2/standards/templates/`; the glossary for its terms.
- Used by: the [website README](/apps/website/README.md) and the READMEs of its crates, which
  link these documents under Related documentation; the runbooks, which link the API and page
  docs.
- Rules: a folder here mirrors a code folder and keeps its spelling; a document describes the
  committed code, and a disagreement between a document and the code is recorded with both
  places and resolved in the code's favour.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — running the API, the
  database and the app locally.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — deploying the API and
  the app to the deploy host.
- [Glossary](/documentation_v2/glossary.md) — the platform's terms.
