# Asset Census (`assets_v2`)

Every committed asset, its size, its encoding, and what reads it.

---

## 1. Everon

| Path (under `terrains/everon/`) | Format | Size | Role |
|:---|:---|:---:|:---|
| `manifest.json` | JSON | 3.6 KB | Bounds, DEM scaling, asset paths, LOD gates. The only entry point. |
| `dem/everon-dem-16bit.png` | PNG, LFS | 69 MB | 6400² `uint16` elevation grid, 2 m per pixel, −204.78 m to 375.53 m |
| `satellite/everon-sat.tbd-sat` | Container, LFS | 146 MB | 12,800² satellite pyramid, 14 mips, embedded offset index |
| `objects/chunks/*.bin` | POD, LFS | 60 MB | 315 populated 512 m chunks of `ObjectInstancePod` |
| `objects/chunks/*.json.gz` | GZ JSON | (in the above) | Parity twin of each chunk |
| `objects/density/*.bin` | TBDD, raw blobs | 13 MB | 625 forest-density tiles, 8 m cells |
| `objects/prefabs.rkyv` + `.json.gz` | rkyv / GZ JSON | — | Prefab catalog, 1,623 entries |
| `objects/forest-regions.rkyv` + `.json.gz` | rkyv / GZ JSON | — | Derived forest outlines |
| `objects/type-inventory.rkyv` + `.json` | rkyv / JSON | — | Corpus-wide prefab type counts |
| `objects/roads.json.gz` | GZ JSON | — | Road network, JSON form |
| `roads/road_network.rkyv` | rkyv, LFS | 432 KB | Road splines and intersection nodes |
| `locations/map_labels.rkyv` | rkyv | 8 KB | Placed map labels |
| `locations.json`, `road-names.json`, `height-labels.json` | JSON | ~13 KB | Place names, road names, spot elevations |
| `anchors/verification.json` + `.example.json` + `surface-y-log.txt` | JSON / text | 16 KB | Engine-probed surface heights and the raw probe log |
| `prefabs/blas/*.bvh` | Binary, LFS | 33 MB | 1,690 bottom-level acceleration structures |
| `prefabs/blas-manifest.json` | JSON | — | BLAS geometry index |
| `prefabs/building_blueprints.rkyv` | rkyv | — | Voxelised building interiors |
| `prefabs/descriptors/*.json` | JSON | 19 MB | 1,623 per-prefab property and bounds records |
| `prefabs/buildings/`, `prefabs/scenes/` | JSON | 1.6 MB | Authored blueprints, instances, and viewer scene specs |

Totals: 1,216,066 object instances across 1,623 prefab types.

## 2. Arland

| Path | Format | Size | Role |
|:---|:---|:---:|:---|
| `terrains/arland/manifest.json` | JSON | 756 B | Bounds and DEM scaling for an island whose export has not run |

## 3. Shared

| Path | Format | Size | Role |
|:---|:---|:---:|:---|
| `terrains/terrain-registry.json` | JSON | ~2 KB | Catalog of registered terrains |
| `glyphs/manifest.json` | JSON | 3.8 KB | 29 world-object glyphs: source, size, anchor, tint, colour |
| `glyphs/atlas/world-glyphs.webp` + `.json` | WebP / JSON | 56 KB | Packed atlas and its rectangle index |
| `glyphs/svg/*.svg` | SVG | 120 KB | 29 authored glyph sources |

---

## 4. Excluded From Version Control

| Path | Size | Reason |
|:---|:---:|:---|
| `terrains/everon/tiles/` | 247 MB | WebP pyramids rebuilt on demand from the satellite bundle |
| `scratch/everon/` | 1.5 GB | Export intermediates: stitched orthophotos, landcover masks, water spikes, raw Workbench dumps |

Neither is an input to anything downstream of the export that produced it, so a fresh clone is complete without them.
