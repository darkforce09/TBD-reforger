# Map assets

The map data the web platform serves and draws: the built-in terrain datasets, the world-object
glyph set, and the specification of a production volume for uploaded terrains. The
[API](/documentation_v2/glossary/a_to_f.md#api) serves the first two under `/map-assets`, the map engine
streams and decodes them, and the developer tools write them.

## Contents

```text
assets_v2/
├── glyphs/        the world-object glyph set: SVG sources, the packed atlas and its manifest
├── storage_spec/  the specification of a production volume for uploaded terrains, not yet built
└── terrains/      the built-in terrain datasets and the terrain registry, served at `/map-assets`
```

## How it works

The API mounts `terrains/` at `/map-assets` and `glyphs/` at `/map-assets/glyphs`
(`apps/website/api_v2/src/core/http_router.rs`). Their directories come from `MAP_ASSETS_DIR` and
`GLYPH_ASSETS_DIR`, and default to `../../../assets_v2/terrains` and `../../../assets_v2/glyphs`,
which resolve here from the API's working directory `apps/website/api_v2/`. Both mounts sit below
the API's rate limiter, so streaming a terrain spends no request tokens. In development the app's
Trunk server proxies `/map-assets` to the API.

```text
developer tools ──write──▶ terrains/   glyphs/
                                │         │
             API  /map-assets ◀─┘         └─▶ /map-assets/glyphs
                         │
browser: website-frontend ─▶ website-map-engine (fetch, decode, stream) ─▶ website-graphics-engine
```

A terrain is reached only through the manifest its registry entry names, so a dataset may ship a
subset of files and the map engine degrades on what the manifest lists. The glyph manifest names
one glyph per render key that `contracts_v2/rules/prefab-classify.json` assigns to a prefab.
`storage_spec/` describes a second, uploaded-terrain tier with the same manifest contract; no code
reads it yet.

## Format

- Encoding:
  - `terrains/`: JSON (the registry, manifests, labels, locations, prefab descriptors, some of it
    gzipped) and binary payloads: a 16-bit PNG height map, `TBDC` object chunks and `TBDD`
    forest-density tiles (`.bin`), the `.tbd-sat` satellite bundle, `rkyv` archives and `.bvh`
    building hierarchies. `apps/website/map-engine/src/io/` defines the binary formats. The
    binaries are stored in Git LFS (`.gitattributes`), except the forest-density tiles, which are
    plain git blobs.
  - `glyphs/`: SVG sources, a WebP atlas with a JSON rectangle index, and a JSON manifest.
  - `storage_spec/`: its README only.
- Schema: `contracts_v2/definitions/terrain-registry.schema.json` for the registry,
  `contracts_v2/definitions/terrain-manifest.schema.json` for each terrain manifest,
  `contracts_v2/definitions/terrain-anchors.schema.json`,
  `contracts_v2/definitions/prefab-descriptor.schema.json` and
  `contracts_v2/definitions/blas-manifest.schema.json` for the terrain's anchors and prefab data.
- Adding a file: a new terrain is a folder under `terrains/` with a `manifest.json` and a registry
  entry, written by the export tooling; `cargo xtask schema terrain-manifest --terrain <id>` checks
  the manifest against its schema and the terrains contract, and `cargo xtask schema map-glyphs`
  checks glyph coverage. The child READMEs give each tree's rules.

## Producers and consumers

- Producers: the developer tools in `tools_v2/developer-tools/`: the world export pipeline
  (`tools_v2/developer-tools/src/world_export_pipeline/`), the map raster pipeline, which also
  builds the glyph atlas (`tools_v2/developer-tools/src/map_raster_pipeline/`), and the blueprint
  compiler (`tools_v2/developer-tools/src/blueprint/`), which resolve these paths through
  `tools_v2/developer-tools/src/repository_layout.rs`. The glyph SVG sources are hand-authored.
- Consumers:
  - the API's `/map-assets` and `/map-assets/glyphs` mounts, in
    `apps/website/api_v2/src/core/http_router.rs`;
  - the map engine in `apps/website/map-engine/`, which fetches and decodes the datasets in the
    browser and reads them from disk in its native tests;
  - the developer tools' map verifications (`tools_v2/developer-tools/src/map_verification/`) and
    the xtask schema gates;
  - `cargo xtask ci lfs-dem` and `cargo xtask ci lfs-sat`, which pull the Everon height map and
    satellite bundle from LFS;
  - `cargo xtask deploy website`, whose rsync leaves `terrains/` out (each host keeps its own copy,
    checked by its asset preflight) and ships `glyphs/`;
  - the staging compose file `apps/website/docker-compose.staging.yml`, which mounts both trees
    read-only into the API container.

## Boundaries

- Depends on: the schemas in `contracts_v2/definitions/` and the prefab classification rules in
  `contracts_v2/rules/prefab-classify.json`.
- Used by: the API, the map engine, the developer tools, the xtask schema, `ci` and deploy
  commands, the staging compose file and the CI workflows in `.github/workflows/`, as listed
  above.
- Rules: the tree holds data only, no code; a terrain is addressed only through its manifest; an
  LFS pattern in `.gitattributes` must exist before the first file of its kind is committed, and
  the forest-density exception stays the file's last rule; a clone without LFS content holds
  the JSON files and the forest-density tiles, and only pointer files for every other binary.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — pulling the LFS map
  assets and serving them to the app.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — how the deployed host
  gets its terrain tree.
- [Terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md) —
  the flow from a Workbench world to the datasets here, and their gates.
- [Uploaded terrain volume](/documentation_v2/assets_v2/uploaded_terrain_volume.md) — the design of
  the uploaded tier `storage_spec/` specifies.
