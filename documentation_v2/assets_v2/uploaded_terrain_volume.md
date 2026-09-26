**Status:** live

# Uploaded terrain volume

The design of a second terrain storage tier: a persistent volume on the self-hosted server that
holds terrains uploaded at runtime, beside the built-in terrains committed under
`assets_v2/terrains/`. Nothing implements it yet. Developers read it before building terrain
uploads, so the upload seam keeps the built-in tier's contract.

## Where it lives

- Code: none. The specification's folder is
  [`assets_v2/storage_spec/`](/assets_v2/storage_spec/README.md); the built-in tier it mirrors is
  [`assets_v2/terrains/`](/assets_v2/terrains/README.md).
- Entry: none yet. The design's entry is an upload endpoint on the
  [API](/documentation_v2/glossary/a_to_f.md#api) that no route serves.
- Related features: [terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md),
  the built-in tier's flow and gates.

## Behaviour

Every terrain the platform serves today is a built-in dataset, committed under
`assets_v2/terrains/` and served from the repository checkout at `/map-assets`. The design below
adds a second source without changing what the browser sees.

### Ingest gates

An upload is a multipart archive. It is expanded into a staging folder on the volume, checked,
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
2. **Path traversal.** An entry is rejected if its path is absolute, escapes the staging root once
   normalised, or is a symlink. A rejection aborts the whole upload rather than skipping the
   entry.
3. **Manifest validation.** `manifest.json` validates against
   `contracts_v2/definitions/terrain-manifest.schema.json`, and the terrain id matches the folder
   name and is absent from the terrain registry.
4. **Bounds and partition.** The world bounds are positive, a whole multiple of the 512 m chunk
   size, and consistent with the height map's pixel dimensions and declared metres per pixel.
   Every chunk file name falls inside the grid those bounds imply.
5. **Anchors.** When the upload declares surface anchors, resampling the height map at each anchor
   agrees within the manifest's threshold. A terrain whose elevation disagrees with its own probe
   log is misaligned, and objects placed on it float or sink.
6. **Atomic placement.** The staged folder is renamed into the volume in one operation, then the
   registry is reloaded, so a partly visible terrain is never servable.

### Operating rules

- The mount is read-only in the API container. Writes happen in the upload handler's staging area
  on the same volume, so the final rename stays within one filesystem.
- Removing a terrain is a registry edit, then a folder removal, in that order, so nothing resolves
  a manifest that is about to disappear.
- The volume stays outside the code deployment: `cargo xtask deploy website` transfers no terrain
  data (its rsync excludes `assets_v2/terrains/`), and the server keeps its own copy across
  releases.

### Known discrepancies

- The design has the API resolve one folder per tier and serve both under `/map-assets` (Data,
  below) — the API serves `/map-assets` from
  `MAP_ASSETS_DIR` alone (`apps/website/api_v2/src/core/http_router.rs`), and no code reads
  `TBD_TERRAIN_STORAGE_DIR`.

## Data

- The designed volume: a Docker named volume `tbd-terrain-storage`, mounted read-only into the API
  container at `/var/data/tbd/terrains`, with `TBD_TERRAIN_STORAGE_DIR` naming the mount. The API
  resolves one folder per tier and serves both under `/map-assets`. An uploaded terrain folder has
  the same shape as a built-in one, because both are addressed through the same manifest
  contract:

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

- The schemas it keeps: `terrain-manifest.schema.json` for each manifest,
  `terrain-anchors.schema.json` for the anchor log and `terrain-registry.schema.json` for the
  registry an upload joins, all in `contracts_v2/definitions/`.

## Design

The uploaded tier keeps the built-in tier's manifest contract, so one loader serves both and no
consumer tells them apart. A terrain becomes visible only after every gate passes and the atomic
rename completes. The gates run in order of cost and risk: the checks that protect the server (size,
paths) come before the checks that protect the map (schema, bounds, anchors).

## Open work

None. No ticket in `.ai/tickets/` covers terrain uploads or this volume.

## Decisions

- An uploaded terrain has the built-in layout: one manifest contract, one loader, one mount path
  for the browser.
- Size is decided from the archive index before extraction: a decompression bomb must never reach
  the disk.
- Placement is one rename on one filesystem: a terrain is either wholly servable or absent.
