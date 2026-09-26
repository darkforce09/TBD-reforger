**Status:** live

# Documentation archive

The documents that no longer describe how the repository works but are kept as a record: program
plans and handoffs, audits, product plans, retired designs and the stubs of moved paths, one folder
per topic. Every file under it is archived — frozen records: read it for why something is the way
it is, and read the live document it points at for how it is now.

## Contents

```text
documentation_v2/archive/
├── api_v2_refactor/            inventory, plan and phase handoffs of the API's domain restructuring
├── assets_v2_relocation/       census, storage plan and handoff of the move of assets into assets_v2
├── audits/                     codebase and architecture audits and their finding-to-ticket maps
├── contracts_v2_relocation/    census, pipeline policy and handoff of the move into contracts_v2
├── documentation_v2_refactor/  inventory and plan of the move from docs/ into documentation_v2
├── engine_split/               program, blueprint and baseline of the graphics and map engine split
├── factory_runs/               dated factory procedures, briefs, kickoffs and run ledgers
├── frontend_v2_migration/      log of the frontend's move into its src/v2 domain tree
├── go_and_react_era_design/    platform and Mission Creator designs for a Go and React stack
├── handoffs_and_kickoffs/      agent handoffs and the Mission Creator agent execution contract
├── monorepo_migration/         runbook, manifests and old indexes of the monorepo merge
├── product_plans/              the platform build plan, the mod milestones and a milestone post
├── redirect_stubs/             stubs of retired docs/ paths, each pointing at its document
├── shipped_history/            the shipped-work log kept out of the agent instruction file
└── tools_v2_refactor/          inventory, plan and phase records of the tooling restructuring
```

## How it works

A document moves here when it stops describing the live system: a finished program's plans and
handoffs, a dated run record, a superseded design, a redirect stub. It moves into the topic folder
of the program or subject it belongs to, and a new topic gets its own snake_case folder and README.
Before a live document is archived, every fact it holds that is still true is carried to a live
document.

Every archived file starts with `**Status:** archived`, followed by `— see [its replacement](…)`
when a live document replaces it, and is never reworded after it lands: only its links change,
and a link to code that no longer exists becomes a GitHub permalink with the full commit id.
Archived files keep the text, dates and [ticket](/documentation_v2/glossary/n_to_z.md#ticket) ids of
their time, so a path, command or name inside one may no longer exist; the live documents are the
authority.

| Topic | Live replacement |
|---|---|
| API restructuring | [API documentation](/documentation_v2/website/api_v2/README.md) |
| Assets and contracts moves | [assets](/documentation_v2/assets_v2/README.md), [contracts](/documentation_v2/contracts_v2/README.md) |
| Documentation move | [documentation entry](/documentation_v2/README.md), [documentation standards](/documentation_v2/standards/documentation_standards.md) |
| Engine split | [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) |
| Factory runs | [factory waves](/documentation_v2/runbooks/factory_waves/README.md) |
| Frontend move and Go and React era designs | [frontend documentation](/documentation_v2/website/frontend/README.md), [Mission Creator](/documentation_v2/website/frontend/apps/editor/README.md), [design system](/documentation_v2/design_system/README.md) |
| Tooling restructuring | [tooling documentation](/documentation_v2/tools_v2/README.md) |

Audits, handoffs, the monorepo merge, the product plans and the shipped history have no single
replacement; each topic README says where their subject lives now.

## Code

None: the archive describes no live code. Each topic README links the code folders its records
concern.

## Boundaries

- Depends on: the live documents the archived files point at; the documentation standards, which
  set the status line and the frozen-record rule.
- Used by: `ARCHIVE_DIR` in `tools_v2/xtask/src/core/repository_layout.rs`, through which
  `cargo xtask verify link-check` and `cargo xtask verify markdown-placement` judge the tree as
  frozen records; `SCAN_EXEMPT_PREFIXES` and `ARCHIVED_WAVE_PLAN_READERS` in
  `tools_v2/ticket-engine/src/repository.rs`, which let archived text quote retired identifiers;
  the ticket files in `.ai/tickets/` that cite archived sources; live documents, the root README
  and a few code comments that link an archived record.
- Rules: an archived file is never reworded, only its links change; it carries
  `**Status:** archived`; it is exempt from the 500-line limit and judged only on its links
  (`cargo xtask verify link-check`); each topic folder has a README listing its files.

## Related documentation

- [Documentation standards](/documentation_v2/standards/documentation_standards.md) — the document
  lifecycle, status lines and the frozen-record rule.
- [Ticket specs and plans](/documentation_v2/tickets/README.md) — the other frozen tree.
