**Status:** live

# Ticket identifiers

How a [ticket](/documentation/glossary/n_to_z.md#ticket) is named, where tickets and their
documents live, and how tickets are cited in commit subjects and documents. Every planned piece of
work, shipped or open, is a ticket in the central ticket manager under the project `reforger`;
its database is the source of truth, and its `ttm` command line, written
`ttm --project reforger <verb>`, reads and writes it. The workspace reaches it only through the
[ticket manager client](/tools/foundation/ticket_manager_client/README.md).

## The reference

| Form | Meaning | Example |
|---|---|---|
| a slug: lowercase words joined by `-` | a parent ticket, a work item or a program | `slot-identity` |
| a parent slug, then `.` and a child segment, repeated | a child slice, at any depth | `slot-identity.flatten-emit` |
| `T-` and three or more digits, then `.` and a number, repeated | the legacy number of an imported ticket | `T-068`, `T-674.1` |

Tickets are keyed by their slugs. A ticket imported from the legacy files keeps its old number as
its legacy id, and every command accepts either form, so `T-674.1` still resolves
(`ttm --project reforger resolve <ref>` names the slug behind a reference). The ticket manager
derives a new ticket's slug from its title (`ttm --project reforger add`,
`ttm --project reforger add-child <parent>`), and a new ticket gets no `T-` number. A shipped or
cancelled ticket keeps its slug, and new scope gets a new ticket. The tools accept a reference of
lowercase letters, digits, `-` and `.` with no empty or dash-edged dot segment, or a legacy number
(`is_ticket_reference` in the client).

## Where tickets live

Each ticket is one record in the ticket manager, parent and child alike. The fields that classify
a ticket:

- `status`: from `idea` through `queued`, `ready`, `running` and `review` to `shipped`, or
  `deferred` or `cancelled`;
- `kind`: `program` (a ticket with child slices) or `work`;
- `executor`: who runs it; a slice run takes only `claude-code`;
- its scope: the domain and the files it owns, which the wave packer reads.

`ttm --project reforger show <ref>` prints one ticket, `ttm --project reforger check` validates
the project, and `ttm --project reforger next` lists the next work. `.ai/tickets/` still holds
the legacy TOML files as data awaiting `ttm import`; nothing in the workspace reads them, and
they are not the live registry.

## Spec and plan files

The ticket manager holds each ticket's spec and plan; a slice run refuses a ticket whose spec it
does not hold, and `ttm --project reforger brief <ref>` hands the agent both. The records already
written sit under `documentation/tickets/`, frozen once their ticket ships or is cancelled:

| Document | Path |
|---|---|
| spec | `documentation/tickets/specs/t<id>_<subject>.md`, the legacy id without its `T-` and with dots as underscores (`t062_1_1_batch_save.md`) |
| plan | `documentation/tickets/plans/t-<id>_plan.md`, the legacy id lowercased with dots as underscores (`t-067_1_plan.md`) |

`ttm --project reforger mark-ready <ref>` marks a ticket ready once its readiness gate holds. A
spec is live while its ticket is `idea`, `queued` or `ready`; once the ticket ships or is
cancelled, lasting knowledge moves to the feature doc.

## In commit subjects

A commit that lands a ticket names it in the subject, by slug or, for an imported ticket, by its
legacy number, for example `fix(gate): the T-180 place-path ban reads the tree that exists`. The
ticket manager links a ticket to the commits whose subjects name it, and every command also
accepts a commit sha prefix as a reference.

## In documents

- **The Eden gap analysis.** Each row's ticket column is written by hand: a parent ticket, with
  `✅` when the ticket shipped, or `—`.
- **Retired planning codes.** Planning codes from before the `T-` registry (priority tiers,
  separate frontend and backend backlog numbers, lettered tracks and lettered requirement codes)
  are retired outside the frozen records and the design exports; review holds this. Cite the
  ticket instead.
- **Where no reference goes.** READMEs never cite tickets; a feature doc links its open tickets
  under `## Open work`. Code comments carry no ticket references; review holds this.

The repository also holds git tags named after ticket ids. No command creates them and no gate
reads them; the landing sha the ticket manager records (`ttm --project reforger land <ref> --sha
<sha>`, which the wave driver runs) is the record of where a ticket landed.

## Adding or changing a ticket

1. Create it with `ttm --project reforger add` or `add-child`, or change it with `set-status`,
   `reorder`, `mark-ready` or `advance-slice`; the ticket manager records every change in its
   history (`ttm --project reforger history <ref>`).
2. Check the project: `ttm --project reforger check --strict`, which also fails on warnings.
3. Update the narrative docs in the commit that changes the code, as the
   [commit checklist](/documentation/standards/commit_checklist.md) lists; a ticket change is no
   file in the repository and needs no commit.
