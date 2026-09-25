**Status:** live

# Debug benches documentation

The feature documentation of the URL-only debug benches: routes that drive one part of the map
engine in isolation, with none of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document, persistence or chrome
around it. Developers and AI agents read it before changing a bench or the line-of-sight code a
bench probes.

## Contents

```text
documentation_v2/website/frontend/apps/debug/
├── building_viewer_page.md  the building viewer bench: one blueprint, its line of sight and viewshed
└── world_los_page.md        the world line-of-sight bench: the object catalogue around a map point
```

## How it works

Each feature doc follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md)
and covers one bench. Both benches mount their own canvas and render engine, read only committed
assets under `/map-assets`, need no sign-in, have no navigation entry and take every run parameter
from the URL, so a reading reproduces from its address.

| Bench | Route and component | Feature doc |
|---|---|---|
| Building viewer | `/debug/building-viewer`, `BuildingViewerPage` | [building_viewer_page.md](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md) |
| World line of sight | `/debug/world-los`, `WorldLosPage` | [world_los_page.md](/documentation_v2/website/frontend/apps/debug/world_los_page.md) |

A new bench gets its own feature doc here, named after its route component, a line in Contents and
a row in the table.

## Code

- [Debug benches](/apps/website/frontend/src/v2/apps/debug/) — both route components, the shared
  interior lanes and each bench's pure and browser halves.
- [Map engine line of sight](/apps/website/map-engine/src/spatial/los/) — the building and world
  occluders the benches probe.

## Boundaries

- Depends on: the feature doc template; the bench code, the map engine it drives and the ticket
  registry the feature docs are written from.
- Used by: the in-code READMEs of the debug benches and their subfolders, which link the feature
  docs; the frontend apps README.
- Rules: a feature doc keeps its name, which those links use; a bench has no design set, and its
  feature doc stays within 500 lines.

## Related documentation

- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  editor whose line-of-sight tool reads the same occluders.
