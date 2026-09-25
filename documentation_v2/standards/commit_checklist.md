**Status:** live

# Commit checklist

What every commit that changes code carries, for people and AI agents alike. Documentation ships
in the same commit as the code it describes, whichever agent or person writes that code; a commit
never leaves the documentation stale.

## Before you start

**Authority.** Running code wins over every document, then `CLAUDE.md` (its laws), then the
feature docs and roadmaps under `documentation_v2/`, then the archive, which is history and never
working context.

| Work | Read first |
|---|---|
| any work | the [ticket](/documentation_v2/glossary.md#ticket), its spec and its plan (`cargo xtask ticket brief <id>`) |
| the app's pages | [Frontend documentation](/documentation_v2/website/frontend/README.md): every route, its page folder and its feature doc |
| the [Mission Creator](/documentation_v2/glossary.md#mission-creator) | its [roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md) and [decisions](/documentation_v2/website/frontend/apps/editor/decisions.md) |
| the [API](/documentation_v2/glossary.md#api) | the [API overview](/documentation_v2/website/api_v2/api_overview.md) and the code in `apps/website/api_v2/` |
| where a new file goes | [Where does X go?](/documentation_v2/standards/where_does_x_go.md) |
| comments and cross-boundary tags | [Documentation standards](/documentation_v2/standards/documentation_standards.md) |
| code rules and their gates | [Coding standards](/documentation_v2/standards/coding_standards/README.md) |
| ticket ids in commits and files | [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) |

## Same-commit updates

| What changed | Update in the same commit |
|---|---|
| a ticket shipped | `cargo xtask ticket ship <id>` (it runs `ticket sync`), then after the commit `cargo xtask ticket stamp-sha <id> <sha>`; the feature doc's Open work |
| a program's active slice | `cargo xtask ticket advance-slice <id>` |
| a route added or removed | `apps/website/frontend/src/app_routes.rs` and `apps/website/frontend/src/router.rs`; the route table of the [frontend documentation](/documentation_v2/website/frontend/README.md); the page's feature doc and README |
| a page's visible surface | the page's feature doc and its code folder's README |
| the navigation or sidebar | `apps/website/frontend/src/v2/pages/navigation/` and [App layout and navigation](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md) |
| an API model | the model in `apps/website/api_v2/src/<domain>/models/`, the DTO in `apps/website/frontend/src/v2/core/api/dto/` and its R-api golden (CLAUDE.md law 9) |
| a cross-boundary type or handler | its `@contract`, `@route` or `@authority` tag, per the documentation standards |
| a schema | the definition in `contracts_v2/definitions/`, its fixture, and the regenerated types (`cargo xtask ci schema-codegen`) |
| the Mission Creator | [decisions](/documentation_v2/website/frontend/apps/editor/decisions.md), the [feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md) or the [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md), as the change touches them |
| a code folder's files | its README's Contents, which `cargo xtask verify readme-coverage` checks |
| work put off | the ticket's status set to `deferred` (`cargo xtask ticket set-status <id> deferred`); never `shipped` before it is verified |
| documentation only | a commit of its own |

## Never edit by hand

- What `cargo xtask ticket sync` writes: `.ai/tickets/queue.json`, the next-work block of the
  Mission Creator roadmap and the ticket column of the Eden gap analysis. Change the ticket, then
  sync.
- The generated contract types in the `generated/` folders under `apps/website/api_v2/src/`;
  regenerate them.
- The frozen records: `documentation_v2/tickets/` once a ticket ships or is cancelled, and
  `documentation_v2/archive/`. Only their links change.
- The design exports in a `visual_references/` folder, which are references, not the source of
  the UI; the live UI is the Leptos code under `apps/website/frontend/src/v2/`.

Markdown never goes under a `docs` folder in `apps/`, `contracts_v2/` or `assets_v2/`; it goes in
`documentation_v2/`, beside the feature it describes.

## Verify before committing

Run what the change touches, from the repository root:

```bash
cargo xtask mk ci-local-leptos
```

Expected: formatting, clippy for `wasm32`, the app's tests and a release build pass (for the app).

```bash
cargo xtask db test-it
```

Expected: the API's tests pass against a new database (for the API or the database; needs
`cargo xtask db up`).

```bash
cargo xtask ticket check --strict
```

Expected: `check OK` (for tickets, specs, plans or documents under `documentation_v2/`).

```bash
cargo xtask verify link-check --with-untracked --path <folder>
```

Expected: exit 0 (for documentation; `readme-coverage` and `markdown-placement` take the same
flags). The whole gate is `cargo xtask ci ci-local`; the
[Testing and CI](/documentation_v2/runbooks/testing_and_ci.md) runbook lists every gate and where
it runs.

## Commit conventions

- Commit directly to `main`; create no branch (CLAUDE.md law 2). The one exception is the
  `slice/<id>` branches that `cargo xtask platform slice-worktree` and the
  [wave](/documentation_v2/glossary.md#wave) tooling create, merge and delete themselves.
- Subject: `type(scope): summary`, with the type one of `feat`, `fix`, `refactor`, `test`, `docs`
  or `chore`. A commit that lands a ticket names its id in the subject: `ticket stamp-sha` and the
  token estimator read ticket ids from commit subjects, as
  [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md#in-commit-subjects)
  describes.
- A commit written with an AI agent ends with a `Co-Authored-By:` trailer.
- An agent commits only when asked. When the tree holds someone else's uncommitted work, stage
  only your own hunks.

## Related documentation

- [Taking a ticket from idea to shipped](/documentation_v2/runbooks/ticket_run_pipeline.md) — the
  ticket lifecycle around a landing commit: ready, run, ship and stamp.
