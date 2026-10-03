**Status:** live

# Mission editing documentation

The feature documentation of the mission editing crates, the headless editing layer of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): the editing host over the live
[mission](/documentation/glossary/g_to_m.md#mission) document, the hosted commands, the undo drive,
the map tools and the decisions behind local drafts. Developers and AI agents read it below the
crates' code READMEs.

## Contents

```text
documentation/crates/mission_editing/
├── mission_persistence/  the draft decisions of the mission_persistence crate
└── editing_layer.md      the editing host, hosted commands, undo drive and map tools
```

## How it works

The folder mirrors `crates/mission_editing/`. A document that spans several of its crates, as the
[editing layer](/documentation/crates/mission_editing/editing_layer.md) spans the session, the
commands and the tools, sits at this level; a document about one crate sits in a folder named after
that crate, as [draft persistence](/documentation/crates/mission_editing/mission_persistence/draft_persistence.md)
does. Read the editing layer first. Each document follows the
[feature doc template](/documentation/standards/templates/feature_doc.md).

## Code

- [Mission editing crates](/crates/mission_editing/README.md) — the session, commands, persistence
  and map tools crates the documents describe; their READMEs hold the exact rules and the tests
  that pin them.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md); the
  code of the mission editing crates and of the Mission Creator that calls them; the ticket
  registry in `.ai/tickets/` for open work; the glossary for its terms.
- Used by: the READMEs of the mission editing crates, the
  [map engine documentation](/documentation/legacy/map_engine/README.md) and its overview, and the
  [engine boundary rules](/documentation/standards/engine_boundary_rules.md).
- Rules: a document describes the committed code and stays within 500 lines; a document of one
  crate sits in that crate's folder here, with a README at each level.

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the headless
  editing layer and the browser ban the crate-tier law enforces on it.
- [Map engine documentation](/documentation/legacy/map_engine/README.md) — the crate that draws
  what the layer edits.
