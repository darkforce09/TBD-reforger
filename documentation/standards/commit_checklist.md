**Status:** live

# Commit checklist

What every commit that changes code carries, for people and AI agents alike. The pre-commit
checks are formatting, clippy and the tests of the crates you touched. Keep the documentation
truthful when you change a folder's surface; that is advice, not a gate.

## Before you start

**Authority.** Running code wins over every document, then `CLAUDE.md` (its laws), then the
feature docs and roadmaps under `documentation/`, then the archive, which is history and never
working context.

| Work | Read first |
|---|---|
| any work | the [ticket](/documentation/glossary/n_to_z.md#ticket), its spec and its plan (`cargo xtask ticket brief <id>`) |
| the app's pages | [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md): every route, its page folder and its feature doc |
| the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) | its [roadmap](/documentation/crates/frontend/workspaces/mission_creator_workspace/mission_creator_roadmap.md) and [decisions](/documentation/crates/frontend/workspaces/mission_creator_workspace/decisions.md) |
| the [API](/documentation/glossary/a_to_f.md#api) | the [API overview](/documentation/crates/api/api_server/api_overview.md) and the code in `crates/api/api_server/` |
| where a new file goes | [Where does X go?](/documentation/standards/where_does_x_go.md) |
| comments and cross-boundary tags | [Documentation standards](/documentation/standards/documentation_standards.md) |
| code rules | [Coding standards](/documentation/standards/coding_standards/README.md) |
| ticket ids in commits and files | [Ticket identifiers](/documentation/standards/ticket_identifiers.md) |

## Related updates

| What changed | Also update |
|---|---|
| a ticket shipped | the ticket's status and the feature doc's Open work; `cargo xtask ticket ship <id>` and `cargo xtask ticket stamp-sha <id> <sha>` are optional bookkeeping |
| a program's active slice | `cargo xtask ticket advance-slice <id>` |
| a route added or removed | `crates/frontend/shell/frontend_application/src/app_routes.rs` and `crates/frontend/foundation/frontend_route_table/src/routes.rs`; the route table of the [frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md); the page's feature doc and README |
| a page's visible surface | the page's feature doc and its code folder's README |
| the navigation or sidebar | `crates/frontend/shell/frontend_application/src/shell/` and [App layout and navigation](/documentation/crates/frontend/shell/frontend_application/shell/app_layout_and_navigation.md) |
| an API model | the model in `crates/api/api_<domain>/src/models/`, the DTO in `crates/frontend/foundation/frontend_api_dtos/src/` and its R-api golden (CLAUDE.md law 9) |
| a cross-boundary type or handler | its `@contract`, `@route` or `@authority` tag, per the documentation standards |
| a schema | the definition in `contracts/definitions/`, its fixture, and the regenerated types (`cargo xtask ci schema-codegen`) |
| the Mission Creator | [decisions](/documentation/crates/frontend/workspaces/mission_creator_workspace/decisions.md), the [feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md) or the [Eden gap analysis](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/eden_gap_analysis.md), as the change touches them |
| a code folder's surface | its README, when the README describes what changed |
| work put off | the ticket's status set to `deferred` (`cargo xtask ticket set-status <id> deferred`); never `shipped` before it is verified |
| documentation only | a commit of its own |

## Never edit by hand

- What `cargo xtask ticket sync` writes: `.ai/tickets/queue.json` and the next-work block of the
  Mission Creator roadmap, between its `<!-- ticket-sync:next:start -->` and
  `<!-- ticket-sync:next:end -->` markers. Change the ticket, then sync. The ticket column of the
  Eden gap analysis is not among them: sync rewrites only gap tables whose header still has a
  `priority` column, and the analysis's tables have a `ticket` column, so that column is edited by
  hand.
- The generated contract types under `crates/contracts/contract_schema_types/src/generated/`;
  regenerate them.
- The frozen records: `documentation/tickets/` once a ticket ships or is cancelled, and
  `documentation/archive/`. Only their links change.
- The design exports in a `visual_references/` folder, which are references, not the source of
  the UI; the live UI is the Leptos code under `crates/frontend/shell/frontend_application/src/`.

Markdown never goes under a `docs` folder in `crates/`, `mod/`, `tools/`, `contracts/` or
`assets/`; it goes in `documentation/`, beside the feature it describes.

## Verify before committing

From the repository root:

```bash
cargo xtask mk rust-fmt
cargo xtask mk rust-clippy
cargo test -p <crate>
```

Expected: formatting and clippy pass, and the tests of every crate the change touches pass. For
the API's integration tests run `cargo xtask db test-it` (needs `cargo xtask db up`).

Everything else is on demand: `cargo xtask mk ci-local-leptos` for a frontend release build,
`cargo xtask mk leptos-gates` for a risky Mission Creator runtime change,
`cargo xtask ticket check` after editing tickets, and `cargo xtask verify link-check --path <folder>`
after moving documentation. The
[Testing and CI](/documentation/runbooks/testing_and_ci.md) runbook lists the gates and where
they run.

What to test follows the pre-alpha test policy (CLAUDE.md law 11): core logic (math, file and
wire formats, CRDT merge and undo, auth and permissions, mission compile and validation, data
integrity, destructive-operation guards); never source text, prose, CSS classes, constants,
`Debug`/`Display` output or file layout.

## Commit conventions

- Commit directly to `main`; create no branch (CLAUDE.md law 2). The one exception is the
  `slice/<id>` branches that `cargo xtask platform slice-worktree` and the
  [wave](/documentation/glossary/n_to_z.md#wave) tooling create, merge and delete themselves.
- Subject: `type(scope): summary`, with the type one of `feat`, `fix`, `refactor`, `test`, `docs`
  or `chore`. A commit that lands a ticket names its id in the subject: the optional
  `ticket stamp-sha` and the token estimator read ticket ids from commit subjects, as
  [Ticket identifiers](/documentation/standards/ticket_identifiers.md#in-commit-subjects)
  describes.
- A commit written with an AI agent ends with a `Co-Authored-By:` trailer.
- An agent commits only when asked. When the tree holds someone else's uncommitted work, stage
  only your own hunks.

## Related documentation

- [Taking a ticket from idea to shipped](/documentation/runbooks/ticket_run_pipeline.md) — the
  ticket lifecycle around a landing commit: ready, run, ship and stamp.
