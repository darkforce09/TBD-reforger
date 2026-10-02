**Status:** live

# Documentation program records

The records of the documentation program that built this documentation tree: its plan, writing
brief, style lock, move manifest and its summary, pin catalogue, writer slices, ticket rewrites,
follow-up tickets and progress checkpoint. Status: archived — frozen records; they quote the
paths, counts and writers of their time.

## Contents

```text
documentation/archive/refactor_v2/
├── refactor_followup_tickets.md      the follow-up tickets the program filed
├── refactor_move_manifest/           the move manifest's summary, live targets, archive sets and checkpoint answers
├── refactor_move_manifest.tsv        the row-by-row move plan: source, target, action, class, writer, note
├── refactor_orphan_spec_links.tsv    ticket spec links that named no file, with the fix of each
├── refactor_phase4_slices.tsv        the README writers' slices of the code trees
├── refactor_phase5_slices.tsv        the feature doc writers' slices of the documentation tree
├── refactor_pin_catalogue.md         every place outside the documentation that spelled a moved path
├── refactor_program_plan.md          the program plan: phases, checkpoints and agent roster
├── refactor_progress_checkpoint.md   the program's resume file
├── refactor_style_lock.md            the style rules every writer followed
├── refactor_ticket_rewrites.tsv      the ticket field rewrites the move required
└── refactor_writing_brief.md         the brief every writing agent read first
```

## How it works

The plan set the phases; the manifest and its summary fixed where every document went; the
slices split the writing between agents under the brief and the style lock; the pin catalogue and
the ticket rewrites kept references outside the tree in step; the checkpoint recorded each phase
as it closed. The program's result is the documentation tree and its standards. A record here is
never edited after it lands.

## Code

- [xtask documentation gates](/tools/xtask/src/verifications/documentation/) — the readme-coverage,
  link-check and markdown-placement gates the program built and ran.

## Boundaries

- Depends on: nothing live; the records quote the tree of their time.
- Used by: the [documentation move records](/documentation/archive/documentation_v2_refactor/README.md),
  which link the program plan; the restructure research reports and the mod UI documentation index,
  which cite a record.
- Rules: never reworded; only links change.

## Related documentation

- [Documentation entry](/documentation/README.md) — the tree the program built, as it is now.
- [Documentation standards](/documentation/standards/documentation_standards.md) — the layout,
  lifecycle and gates the program set.
