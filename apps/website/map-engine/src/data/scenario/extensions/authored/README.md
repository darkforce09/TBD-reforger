# Authored-block list and carrier

The one list of the optional blocks a [mission](/documentation_v2/glossary.md#mission) author
writes, each with its check, and the code that moves them: from the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document onto the saved payload,
and from the saved payload into the compiled document. Its items are reached as
`data::scenario::extensions`, which re-exports them.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/authored/
├── authored_block.rs  `AUTHORED_BLOCKS`, the copy onto the payload root, the two compile readers
├── mod.rs             the module tree; re-exports every item of `authored_block.rs`
└── tests/             unit tests for the list, the copy, both readers and the carrier's bytes
```

## How it works

`AUTHORED_BLOCKS` holds one `AuthoredBlock` per block, its top-level key as
`contracts_v2/definitions/mission.schema.json` spells it and its check, in this order: `radioPlan`,
`winConditions`, `tasks`, `weatherTimeline`, `audio`, `spawnModules`, `tacticalGraphics`. The
Mission Creator keeps each block in the document's environment bag, `meta.environment`, and clears
one by writing `null`. Three steps read the list:

```text
meta.environment ── copy_authored_blocks: each listed key, unless null, verbatim ──▶ payload root
payload root ── EditorPayload's named fields ──▶ authored_blocks_root
  ├─ radioPlan, winConditions ── AuthoredBlocks::parse ───────▶ typed fields of the document
  └─ the other five ───────────── ExtensionBlocks::from_payload ─▶ the document root, verbatim
```

- **Save.** `compile_payload` calls `copy_authored_blocks`, which copies each listed key that is
  present and not `null` onto the payload root without checking it, so a save never loses work in
  progress; the bag keeps its own copy, which a reload reads. An unlisted key stays in the bag, and
  `is_authored_block` lets the payload builder skip a `payloadExtras` key that names a block, so a
  stale parked copy cannot come back.
- **Document-owned blocks.** `AuthoredBlocks::parse` checks and parses the two blocks of
  `DOCUMENT_OWNED_BLOCKS`, which the compiled document models with typed fields; the compiler
  builds those fields from the parsed values, and derives its own when a block is absent or refused.
- **Carried blocks.** `ExtensionBlocks::from_payload` checks the other five and keeps each valid
  one verbatim. `ExtensionBlocks` is flattened into `ModMissionDocument` by serde, so each block
  lands at the document root in list order, a key appears once at most (`set` replaces), and an
  empty carrier adds no bytes.

Both readers answer a refused block as `(key, clause)`: the compiler drops the block and reports
a warning finding under `COMPILE-WIN-CONDITIONS`, whichever block it names, and a warning keeps
the [API](/documentation_v2/glossary.md#api) from making an
[artifact](/documentation_v2/glossary.md#artifact) of that version. A listed key reaches the
readers only through its named field and `authored_block_value` arm in `EditorPayload`
(`apps/website/map-engine/src/data/scenario/ast/authoring.rs`).

## Boundaries

- Depends on: the seven block modules' `validate`, and `parse` of `radio_plan` and
  `win_conditions`; `serde` (the carrier's `Serialize`) and `serde_json`.
- Used by: `crate::data::scenario::compile`, whose payload builder
  (`apps/website/map-engine/src/data/scenario/compiler/payload/serialization.rs`) calls
  `copy_authored_blocks` and `is_authored_block`; `crate::data::scenario::ast`, whose
  `authored_blocks_root` walks `AUTHORED_BLOCKS`; `crate::data::scenario::flatten`, whose
  `compile_graph.rs` runs both readers and whose `ModMissionDocument` holds the carrier. No code
  outside the crate imports these items.
- Rules: every document-owned block is listed and has a parse arm
  (`every_document_owned_block_is_registered`, `every_document_owned_block_has_a_parse_arm` in
  `tests/cases_1.rs`); the carrier withholds document-owned blocks, never emits a key twice and
  adds nothing when empty (`the_carrier_withholds_a_document_modelled_block`,
  `the_carrier_is_ordered_and_cannot_emit_a_key_twice`,
  `an_empty_carrier_adds_nothing_to_the_document`); a malformed block is saved, then refused at
  compile (`a_malformed_block_is_saved_but_refused_at_compile`); every listed key reaches the
  compiled document (`every_authored_block_key_reaches_the_wire` in
  `apps/website/map-engine/src/data/scenario/compiler/flatten/tests/cases_3.rs`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — the seven blocks' definitions
  and where each sits in the compiled document.
- [Mission editor payload schema](/contracts_v2/definitions/mission-editor-payload.schema.json) —
  the saved payload's open root and its account of the authored-block copy.
