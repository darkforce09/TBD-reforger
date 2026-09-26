# Ticket sync

`cargo xtask ticket sync`: regenerates the outputs derived from the
[ticket](/documentation_v2/glossary.md#ticket) files, the dispatch queue
`.ai/tickets/queue.json` and the recommended-next-work block of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) roadmap, and runs the ticket
column writer over the Eden gap analysis, which finds no table to rewrite there; it writes no
other file.

## Contents

```text
tools_v2/ticket-engine/src/sync/
├── gap_analysis.rs  the gap-analysis table parser, its round trip and the ticket column writer
├── markers.rs       writes the next-work block between the roadmap's `ticket-sync:next` markers
├── mod.rs           the module tree; re-exports `cmd_sync`, `generate_queue_json` and `refuse_empty_write`
├── queue_json.rs    `generate_queue_json`: the dispatch queue built from the registry projection
├── runner.rs        `cmd_sync`, which writes the three outputs in order, and `refuse_empty_write`
└── tests/           unit tests for the queue order, the checkmark lookup and the empty-write refusals
```

## How it works

`cmd_sync` takes the parents-only registry projection (`crate::registry`) and writes, in this
order:

1. `.ai/tickets/queue.json`: an `_comment` header, the defaults `batch_size` 10, `concurrency` 3,
   `worktree_base` `.ai/artifacts/worktrees` and `git_base` `main`, and every `ready`, `running`
   or `review` ticket with a spec, by order then id, as id, title, status, spec (the active
   slice's spec when it has one) and branch (`ticket/<id>` unless the ticket names one). The JSON
   is indented by two spaces with every non-ASCII character escaped.
2. The roadmap's block between `<!-- ticket-sync:next:start -->` and
   `<!-- ticket-sync:next:end -->`: a `### Recommended next work (auto-generated)` heading and
   the first ten `ready`, `queued`, `running` or `review` tickets that carry an order, by order
   then id.
3. The gap-analysis ticket column. The parser takes only a table whose header row contains both
   `| eden_id |` and `priority |` (`parse_gap_analysis` in `gap_analysis.rs`); it renames that
   header cell to `ticket` and fills the fourth cell of every row with at least five: the parent
   id after a `✅` in the row's notes cell, else a ticket whose `implements` lists the row's Eden
   or platform id, else the ticket `.ai/tickets/corpus-pins.toml` pins to that row, else `—`. The
   gap analysis's tables already head that column `ticket`, so no table matches, the file is
   written back unchanged, and its ticket column is kept by hand.

An absent roadmap or gap-analysis file, or a roadmap without its start marker, is skipped and
never created. `refuse_empty_write` stops any write that would leave the marker block empty or a
bare heading, and the gap-analysis column is rewritten only after the file's tables read and write
back byte for byte. `ticket check` runs the same round trip and requires both roadmap markers.

## Boundaries

- Depends on: `crate::registry` (the projection and its field helpers, `write_json_ascii`);
  `crate::corpus_pins` for the pinned gap rows; `crate::validation::constants` for the markers and
  the queue header; `crate::repository` (`QUEUE_JSON`, `WORKTREES_DIR`, and `documentation`'s
  `ROADMAP` and `GAP_ANALYSIS`).
- Used by: `cargo xtask ticket sync` and `ticket gap-round-trip`
  (`tools_v2/xtask/src/commands/ticket/mod.rs`); the ticket mutations in
  `tools_v2/ticket-engine/src/cli/`, which re-sync after they write, `set-status` through
  `generate_queue_json` alone; `ticket check`, which runs the gap-analysis round trip;
  `tools_v2/xtask/src/commands/schema/mission_flattening.rs`, which guards its own writes with
  `refuse_empty_write`.
- Rules:
  - a sync never collapses the marker block (`inject_next_refuses_empty_tickets_bare_heading`,
    `marker_inner_vacuous_detects_bare_heading`);
  - equal orders break ties by numeric id (`queue_breaks_order_ties_by_numeric_id`);
  - a `✅` captures the whole parent id, three digits or more, and never a dotted child
    (`a_checkmark_captures_the_parent_of_a_dotted_child`).

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — `queue.json` and the other files in the ticket
  folder.
- [Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md)
  — the document that carries the next-work block.
- [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)
  — the tables the ticket column writer reads; their ticket column is kept by hand.
