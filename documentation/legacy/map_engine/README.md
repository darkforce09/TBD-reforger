**Status:** live

# Map engine documentation

The documentation of `map_engine`, the crate that holds the
[mission](/documentation/glossary/g_to_m.md#mission) domain, the static world, streaming, spatial
queries, the map overlay and the render engine of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator). Developers and AI agents read it
below the crate's code READMEs, for the flows across modules, the reasons, the open work and the
decisions.

## Contents

```text
documentation/legacy/map_engine/
├── map_engine_overview.md  the crate's layers, feature tiers and consumers, from canvas to frame
└── map_streaming.md        boot, chunk residency, the memory budget and the loaders
```

## How it works

Start with the [overview](/documentation/legacy/map_engine/map_engine_overview.md): it lays
out the modules by side (authored, static, draw, support), the feature tier each compiles under,
and the path from a mounted canvas to a drawn frame, and it leads to the streaming feature doc here
and to the two feature docs of the mission editing crates, which live at those crates' mirror under
[`documentation/crates/mission_editing/`](/documentation/crates/mission_editing/README.md). Each
follows the [feature doc template](/documentation/standards/templates/feature_doc.md).

| Doc | Covers | Code |
|---|---|---|
| [Map engine overview](/documentation/legacy/map_engine/map_engine_overview.md) | layers, tiers, consumers, crate-wide open work | [`legacy/map_engine/`](/legacy/map_engine/README.md) |
| [Map streaming](/documentation/legacy/map_engine/map_streaming.md) | boot sequence, viewport passes, residency, memory budget, loaders | [`src/streaming/`](/legacy/map_engine/src/streaming/README.md) |
| [Editing layer](/documentation/crates/mission_editing/editing_layer.md) | editing host, hosted commands, undo, tools and picks | [`crates/mission_editing/`](/crates/mission_editing/README.md) |
| [Draft persistence](/documentation/crates/mission_editing/mission_persistence/draft_persistence.md) | draft keys, merge, classify, adopt, snapshot pair | [`mission_persistence`](/crates/mission_editing/mission_persistence/src/README.md) |

The code READMEs state what each folder declares (files, constants, public surface, the tests
that hold its rules); the documents here link them rather than repeat them. The layer rules the
crate lives under are a standard, [engine boundary rules](/documentation/standards/engine_boundary_rules.md),
not a document of this folder. A module whose behaviour outgrows its README gets a feature doc
here and a Contents line; a feature doc of a module deeper than the crate's top-level modules sits
at the module's path mirror under this folder, with a README at each level.

## Code

- [Map engine](/legacy/map_engine/) — the crate the overview describes.
- [Streaming](/legacy/map_engine/src/streaming/) — described in `map_streaming.md`.
- [Mission editing crates](/crates/mission_editing/) — described by the
  [mission editing documentation](/documentation/crates/mission_editing/README.md).
- [Render engine](/legacy/map_engine/src/frame/) — the frame path the overview follows.

## Boundaries

- Depends on: the code of `legacy/map_engine/` and the Mission Creator code that calls it,
  which every claim is checked against; the feature doc template; the ticket registry in
  `.ai/tickets/` for open work; the glossary for its terms.
- Used by: the READMEs of `legacy/map_engine/` and its `streaming/` and `frame/` folders, the
  mission editing crates' READMEs, which link these documents under Related documentation; the mortar calculator and
  ballistics catalogs page docs; the
  [website documentation](/documentation/apps/README.md) index; the engine boundary rules
  and the graphics engine documentation.
- Rules: a document describes the committed code, and a disagreement with the code or another
  document goes under Known discrepancies with both places; open work lists only open tickets,
  each checked in `.ai/tickets/`.

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the layering
  and the walls `cargo xtask verify engine-layers` enforces.
- [Graphics engine documentation](/documentation/legacy/graphics_engine/README.md) — the
  renderer this crate draws with.
- [Mission Creator documentation](/documentation/apps/frontend/workspaces/editor/README.md) — the
  app this crate backs.
