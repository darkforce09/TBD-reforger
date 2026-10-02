**Status:** live

# Engine documentation

The documentation of the two engine crates parked in `legacy/`: `map_engine`, which holds the
[mission](/documentation/glossary/g_to_m.md#mission) domain, the world, streaming, spatial queries
and the map rendering of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator),
and `graphics_engine`, the `wgpu` renderer that knows no map concept. Developers and AI agents read
it below the crates' READMEs.

## Contents

```text
documentation/legacy/
├── graphics_engine/  the renderer: its modules, one frame, design and open work
└── map_engine/       the map engine: overview, map streaming, the editing layer, draft persistence
```

## How it works

Each folder sits at its crate's path with `legacy/` replaced by `documentation/legacy/` and `src/`
left out. When the [workspace restructure](/documentation/restructure/README.md) moves an engine
module into a crate under `crates/`, its documents move with it to that crate's documentation
path.

| Crate | Code | Documentation |
|---|---|---|
| `map_engine` | [`legacy/map_engine/`](/legacy/map_engine/README.md): the world, streaming, spatial queries, the map and the mission domain, with no UI dependency | [map engine documentation](/documentation/legacy/map_engine/README.md) |
| `graphics_engine` | [`legacy/graphics_engine/`](/legacy/graphics_engine/README.md): the `wgpu` renderer on WebGPU, with a WebGL backend a caller can force | [graphics engine documentation](/documentation/legacy/graphics_engine/README.md) |

## Code

- [Parked engine crates](/legacy/README.md) — the folder, its callers and its rules.

## Boundaries

- Depends on: the code under `legacy/`, which every document is checked against; the
  [README standard](/documentation/standards/readme_standard.md) and the templates in
  `documentation/standards/templates/`.
- Used by: the READMEs of the two crates and the
  [application documentation](/documentation/apps/README.md) of the app and the API that link them.
- Rules: a folder here mirrors a code folder and keeps its spelling; a disagreement between a
  document and the code is resolved in the code's favour.

## Related documentation

- [Application documentation](/documentation/apps/README.md) — the app and the API that use the
  engines.
- [Glossary](/documentation/glossary/README.md) — the platform's terms.
