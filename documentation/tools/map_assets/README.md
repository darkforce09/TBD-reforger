**Status:** live

# Map asset crates documentation

The documents on the map asset crates in `tools/map_assets/` whose flows go deeper than their code
READMEs. Developers and AI agents read them below the crates' code READMEs, for the flows that
cross lanes, the reasons and the open work.

## Contents

```text
documentation/tools/map_assets/
└── map_raster_pipeline.md  `map`: satellite, Map view, labels, water archives and the glyph atlas
```

## How it works

The document follows the [feature doc template](/documentation/standards/templates/feature_doc.md)
and covers the raster pipeline end to end. The code READMEs are exact about each crate and are
linked, not repeated:

| Executable | What it does | Code |
|---|---|---|
| `map` | builds and verifies a terrain's satellite container, tile pyramids, cartographic render, labels and water archives, and the world-glyph atlas | [`map_raster_pipeline`](/tools/map_assets/map_raster_pipeline/README.md) |

The executable is a one-line `developer_tools` binary
([executables README](/tools/developer_tools/src/bin/README.md)); the world export it reads from is
covered by the [terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md)
document.

## Code

- [Map asset crates](/tools/map_assets/) — the crates; `map_raster_pipeline` is the one the
  document covers.

## Boundaries

- Depends on: the crate's code, the xtask tasks that run it and the committed assets it writes,
  which every claim is checked against; the feature doc template.
- Used by: the READMEs of `tools/map_assets/` and the raster pipeline crate, which link the
  document under Related documentation; the [tooling documentation](/documentation/tools/README.md)
  index.
- Rules: the document describes the committed code, and a disagreement goes under Known
  discrepancies with both places.
