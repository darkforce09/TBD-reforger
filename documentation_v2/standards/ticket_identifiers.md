**Status:** live

# Ticket identifiers

How a [ticket](/documentation_v2/glossary/n_to_z.md#ticket) id is formed, where tickets and their
documents live, and how the tooling reads ids out of commit subjects and documents. Every planned
piece of work, shipped or open, carries one `T-` id; the ticket files are the source of truth, and
the commands that read and write them are in the
[ticket command group README](/tools_v2/xtask/src/commands/ticket/README.md).

## The id

| Form | Meaning | Example |
|---|---|---|
| `T-` and three or more digits | a parent ticket, a work item or a program | `T-068`, `T-1150` |
| a parent id, then `.` and a number, repeated | a child slice, at any depth | `T-068.10`, `T-068.10.1` |

The pattern is `^T-[0-9]{3,}(\.[0-9]+)*$` (`ticketId` in `.ai/tickets/schema.json`).
`cargo xtask ticket add` mints the next parent id from the registry's `next_id`, and
`cargo xtask ticket add-child <parent>` the next child; nobody picks an id by hand. An id is never
reused: a shipped or cancelled ticket keeps its id, and new scope gets a new ticket. The ids that
`.ai/tickets/corpus-pins.toml` lists are never minted.

## The ticket file

One TOML file per ticket, parent and child alike: `.ai/tickets/T-<id>.toml`, beside the
`.ai/tickets/ROOT` marker. The fields that classify a ticket:

- `status`: `idea`, `queued`, `ready`, `running`, `review`, `shipped`, `deferred` or `cancelled`;
- `kind`: `program` (a ticket with child slices) or `work`;
- `class`: `bug`, `feature`, `chore`, `audit` or `docs`;
- `executor`: `claude-code`, `cursor-docs`, `workbench`, `human` or `ci`;
- `scope.domain`: `website`, `mod`, `schema`, `engine` or `repo`.

`.ai/tickets/schema.json` defines every field, and `cargo xtask ticket check` validates each file
against it. After a change, `cargo xtask ticket sync` writes `.ai/tickets/queue.json` and the
next-work block between the `<!-- ticket-sync:next:start -->` and `<!-- ticket-sync:next:end -->`
markers of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) roadmap;
those two are never edited by hand. The ticket column of the Eden gap analysis is kept by hand, as
[In documents](#in-documents) explains.

## Spec and plan files

| Document | Path |
|---|---|
| spec | `documentation_v2/tickets/specs/t<id>_<subject>.md`, the id without its `T-` and with dots as underscores (`t062_1_1_batch_save.md`); the ticket's `spec` field names it |
| plan | `documentation_v2/tickets/plans/t-<id>_plan.md`, the id lowercased with dots as underscores (`t-067_1_plan.md`) |

The plan path is `plan_path` in `tools_v2/ticket-engine/src/repository.rs`.
`cargo xtask ticket mark-ready <id> [SPEC] [PLAN]` defaults an unset `plan` field to that path and
refuses while the file is absent. The ticket templates are `.ai/tickets/spec_template.md`,
`.ai/tickets/plan_template.md` and `.ai/tickets/handoff_template.md`. A spec is live while its
ticket is `idea`, `queued` or `ready` and frozen once the ticket ships or is cancelled; lasting
knowledge then moves to the feature doc.

## In commit subjects

`cargo xtask ticket stamp-sha` and the token estimator find the commits that belong to a ticket by
reading ids out of commit subjects (`subject_ids` in
`tools_v2/ticket-engine/src/cli/shipping/commit_subjects.rs`):

- an id is `T-[0-9]+(\.[0-9]+)*`, taken to its last dotted number, so `T-068.10.1` counts as
  itself and not as `T-068`;
- it counts only when no ASCII letter or digit stands right before it, so a letter-prefixed token
  is no claim on a ticket;
- a subject that names an id twice counts it once.

A commit that lands a ticket names its full id in the subject, for example
`fix(gate): the T-180 place-path ban reads the tree that exists`.

## In documents

- **The Eden gap analysis.** Each row's ticket column is written by hand: a parent id, with `✅`
  when the ticket shipped, or `—`. `cargo xtask ticket sync` has a column writer that fills the
  cell from, in order, a checkmark followed by a parent id in the row's notes (`✅` then `T-` and
  three or more digits; a dotted suffix is not captured), a ticket whose `implements` lists the
  row's id, the gap implementations in `.ai/tickets/corpus-pins.toml`, else `—` (`CHECKMARK_TICKET`
  and `lookup_ticket_for_gap` in `tools_v2/ticket-engine/src/sync/gap_analysis.rs`); it rewrites
  only a table whose header holds `priority |`, and the gap analysis heads that column `ticket`,
  so the writer changes nothing there.
- **Retired planning codes.** Planning codes from before the `T-` registry (priority tiers,
  separate frontend and backend backlog numbers, lettered tracks and lettered requirement codes)
  are retired. `cargo xtask ticket check --strict` fails on any of them in `documentation_v2/`,
  `.ai/tickets/queue.json`, `CLAUDE.md` and the root `README.md`, outside the frozen records and
  the design exports; the patterns are `STRICT_LEGACY` in
  `tools_v2/ticket-engine/src/validation/constants.rs`. Use the ticket id instead.
- **Where no id goes.** READMEs never cite tickets; a feature doc links its open tickets under
  `## Open work`. Code comments carry no ticket ids: tests hold this for every tracked file under
  `tools_v2/` (`tools_v2/xtask/src/tests/tooling_prose_rules.rs`), for the API's sources
  (`no_ticket_references_in_source` in `apps/website/api_v2/src/tests/architecture_rules.rs`) and
  for the app's `src/v2/` tree (`apps/website/frontend/src/v2/tests/doc_audit/mod.rs`).

The repository also holds git tags named after ticket ids. No command creates them and no gate
reads them; the ticket file's `shipped_at` sha, written by `ticket stamp-sha`, is the record of
where a ticket landed.

## Adding or changing a ticket

1. Create it with `cargo xtask ticket add` or `add-child`, or change it with `set-status`,
   `reorder`, `mark-ready` or `advance-slice`; each write refuses while `ticket check` is red and
   refreshes the derived files after (`set-status` refreshes `queue.json` and repacks the wave
   lock).
2. Check the registry: `cargo xtask ticket check --strict`.
3. Commit the ticket files with the derived files `ticket sync` wrote, and update the narrative
   docs as the [commit checklist](/documentation_v2/standards/commit_checklist.md) lists.
