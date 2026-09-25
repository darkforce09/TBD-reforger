**Status:** live

# Map engine documentation

The documentation of `website-map-engine`, the crate that holds the
[mission](/documentation_v2/glossary.md#mission) domain, the static world, streaming, spatial
queries, the map overlay and the render engine of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator). Developers and AI agents read it
below the crate's code READMEs, for the flows across modules, the reasons, the open work and the
decisions.

## Contents

```text
documentation_v2/website/map-engine/
├── draft_persistence.md    local drafts: keys, merge, classify, adopt and the snapshot pair
├── editing_layer.md        the editing host, hosted commands, undo drive and map tools
├── map_engine_overview.md  the crate's layers, feature tiers and consumers, from canvas to frame
└── map_streaming.md        boot, chunk residency, the memory budget and the loaders
```

## How it works

Start with the [overview](/documentation_v2/website/map-engine/map_engine_overview.md): it lays
out the modules by side (authored, static, draw, support), the feature tier each compiles under,
and the path from a mounted canvas to a drawn frame, and it leads to the three feature docs. Each
follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).

| Doc | Covers | Code |
|---|---|---|
| [Map engine overview](/documentation_v2/website/map-engine/map_engine_overview.md) | layers, tiers, consumers, crate-wide open work | [`apps/website/map-engine/`](/apps/website/map-engine/README.md) |
| [Map streaming](/documentation_v2/website/map-engine/map_streaming.md) | boot sequence, viewport passes, residency, memory budget, loaders | [`src/streaming/`](/apps/website/map-engine/src/streaming/README.md) |
| [Editing layer](/documentation_v2/website/map-engine/editing_layer.md) | editing host, hosted commands, undo, tools and picks | [`src/editing/`](/apps/website/map-engine/src/editing/README.md) |
| [Draft persistence](/documentation_v2/website/map-engine/draft_persistence.md) | draft keys, merge, classify, adopt, snapshot pair | [`src/editing/persist/`](/apps/website/map-engine/src/editing/persist/README.md) |

The code READMEs state what each folder declares (files, constants, public surface, the tests
that hold its rules); the documents here link them rather than repeat them. The layer rules the
crate lives under are a standard, [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md),
not a document of this folder. A module whose behaviour outgrows its README gets a feature doc
here and a Contents line.

## Code

- [Map engine](/apps/website/map-engine/) — the crate the overview describes.
- [Streaming](/apps/website/map-engine/src/streaming/) — described in `map_streaming.md`.
- [Editing](/apps/website/map-engine/src/editing/) — described in `editing_layer.md` and
  `draft_persistence.md`.
- [Render engine](/apps/website/map-engine/src/frame/) — the frame path the overview follows.

## Boundaries

- Depends on: the code of `apps/website/map-engine/` and the Mission Creator code that calls it,
  which every claim is checked against; the feature doc template; the ticket registry in
  `.ai/tickets/` for open work; the glossary for its terms.
- Used by: the READMEs of `apps/website/map-engine/` and its `streaming/`, `editing/` and `frame/`
  folders, which link these documents under Related documentation; the
  [website documentation](/documentation_v2/website/README.md) index; the engine boundary rules
  and the graphics engine documentation.
- Rules: a document describes the committed code, and a disagreement with the code or another
  document goes under Known discrepancies with both places; open work lists only open tickets,
  each checked in `.ai/tickets/`.

## Related documentation

- [Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) — the layering
  and the walls `cargo xtask verify engine-layers` enforces.
- [Graphics engine documentation](/documentation_v2/website/graphics-engine/README.md) — the
  renderer this crate draws with.
- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  app this crate backs.
