# Uploaded terrain volume specification

The design of a second terrain storage tier: a persistent volume on the self-hosted server that
holds terrains uploaded at runtime. No code implements it. Every terrain the platform serves is a
built-in dataset committed under [assets_v2/terrains](/assets_v2/terrains/README.md) and served
from the repository checkout; this folder fixes the design of the upload seam.

## Contents

```text
assets_v2/storage_spec/
```

## How it works

The specification has three parts: the layout of the volume, the ingest gates an upload passes
before it becomes servable, and the operating rules of the volume.

### Ingest gates

An upload is a multipart archive. It is expanded into a staging directory on the volume, checked,
and only then moved into place, in this order:

```text
multipart archive
  └─▶ 1 uncompressed-size quota
        └─▶ 2 path traversal audit
              └─▶ 3 manifest schema validation
                    └─▶ 4 bounds and partition alignment
                          └─▶ 5 surface anchor agreement
                                └─▶ 6 atomic placement, then registry reload
```

1. **Quota.** The uncompressed size is computed from the archive index before extraction and
   rejected past a per-terrain ceiling, with a second ceiling on total volume use. Deciding after
   extraction is how a decompression bomb fills a disk.
2. **Path traversal.** An entry path is rejected if it is absolute, escapes the staging root once
   normalised, or is a symlink. A rejection aborts the whole upload rather than skipping the entry.
3. **Manifest validation.** `manifest.json` validates against
   `contracts_v2/definitions/terrain-manifest.schema.json`, and the terrain id matches the
   directory name and is absent from the terrain registry.
4. **Bounds and partition.** The world bounds are positive, a whole multiple of the 512 m chunk
   size, and consistent with the height map's pixel dimensions and declared metres per pixel.
   Every chunk file name falls inside the grid those bounds imply.
5. **Anchors.** When the upload declares surface anchors, resampling the height map at each anchor
   agrees within the manifest's threshold. A terrain whose elevation disagrees with its own probe
   log is misaligned, and placed objects float or sink on it.
6. **Atomic placement.** The staged directory is renamed into the volume in one operation, then
   the registry is reloaded, so a partially visible terrain is never servable.

### Operating rules

- The mount is read-only in the [API](/documentation_v2/glossary.md#api) container. Writes happen in
  the upload handler's staging area on the same volume, so the final rename stays within one
  filesystem.
- Removing a terrain is a registry edit, then a directory removal, in that order, so nothing
  resolves a manifest that is about to disappear.
- The volume stays outside the code deployment: `cargo xtask deploy website` transfers no terrain
  data (its rsync excludes `assets_v2/terrains/`), and the server keeps its own copy across
  releases.

## Format

- Encoding: in the design, an uploaded terrain directory is structurally identical to a built-in
  one under `assets_v2/terrains/`, because both are addressed through the same manifest contract;
  the API resolves one directory per tier and serves both under `/map-assets`, so no consumer
  tells them apart. The designed volume is a Docker named volume, `tbd-terrain-storage`, mounted
  read-only into the API container at `/var/data/tbd/terrains`, with `TBD_TERRAIN_STORAGE_DIR`
  naming the mount:

  ```text
  /var/data/tbd/terrains/
  └── <terrain-id>/
      ├── manifest.json                    validated against terrain-manifest.schema.json
      ├── dem/<terrain-id>-dem-16bit.png   16-bit elevation grid
      ├── satellite/<terrain-id>-sat.tbd-sat
      ├── objects/chunks/{cx}_{cy}.bin     512 m object chunks
      ├── objects/density/*.bin
      ├── prefabs/, roads/, locations/
      └── anchors/verification.json
  ```

- Schema: `contracts_v2/definitions/terrain-manifest.schema.json` for each manifest,
  `contracts_v2/definitions/terrain-anchors.schema.json` for the anchor log and
  `contracts_v2/definitions/terrain-registry.schema.json` for the registry the upload joins.
- Adding a file: this folder holds its README only; a change to the design is a change to this
  README.

## Producers and consumers

- Producers: none; no upload endpoint writes the volume.
- Consumers: none; no code reads `TBD_TERRAIN_STORAGE_DIR`, and the API serves `/map-assets` from
  `MAP_ASSETS_DIR` alone (`apps/website/api_v2/src/core/http_router.rs`).

## Boundaries

- Depends on: the terrain manifest, anchor and registry schemas in `contracts_v2/definitions/`, and
  the layout of the built-in terrains in `assets_v2/terrains/`.
- Used by: nothing.
- Rules: the uploaded tier keeps the built-in tier's manifest contract, so one loader serves both;
  a terrain becomes visible only after every gate passes and the atomic rename completes.

## Related documentation

- [Uploaded terrain volume](/documentation_v2/assets_v2/uploaded_terrain_volume.md) — the design
  of this tier with its known discrepancies, decisions and open work.
- [Terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md) —
  the built-in tier whose manifest contract the uploaded tier keeps.
