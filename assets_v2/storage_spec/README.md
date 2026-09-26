# Uploaded terrain volume specification

The folder that anchors the design of a second terrain storage tier: a persistent volume on the
self-hosted server that holds terrains uploaded at runtime. No code implements it; every terrain
the platform serves is a built-in dataset under [assets_v2/terrains](/assets_v2/terrains/README.md).
The design itself lives in
[Uploaded terrain volume](/documentation_v2/assets_v2/uploaded_terrain_volume.md).

## Contents

```text
assets_v2/storage_spec/
```

## Format

- Encoding: none in the repository. The designed volume, `tbd-terrain-storage`, mounts at
  `/var/data/tbd/terrains` and holds one folder per terrain in the built-in layout; the
  [design](/documentation_v2/assets_v2/uploaded_terrain_volume.md) gives the tree, the six ingest
  gates and the operating rules.
- Schema: `contracts_v2/definitions/terrain-manifest.schema.json`, `terrain-anchors.schema.json`
  and `terrain-registry.schema.json`.
- Adding a file: this folder holds its README only; a change to the design is a change to the
  design document.

## Producers and consumers

- Producers: none; no upload endpoint writes the volume.
- Consumers: none; no code reads `TBD_TERRAIN_STORAGE_DIR`, and the API serves `/map-assets` from
  `MAP_ASSETS_DIR` alone (`apps/website/api_v2/src/core/http_router.rs`).

## Boundaries

- Depends on: the terrain manifest, anchor and registry schemas in `contracts_v2/definitions/`, and
  the layout of the built-in terrains in `assets_v2/terrains/`.
- Used by: nothing.
- Rules: the uploaded tier keeps the built-in tier's manifest contract, so one loader serves both.

## Related documentation

- [Uploaded terrain volume](/documentation_v2/assets_v2/uploaded_terrain_volume.md) — the design:
  the volume layout, the ingest gates, the operating rules, decisions and open work.
- [Terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md) —
  the built-in tier whose manifest contract the uploaded tier keeps.
