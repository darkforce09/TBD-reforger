# Map & Spatial Fixtures (`contracts_v2/fixtures/map/`)

Twenty fixtures covering the object pipeline: chunks, catalogs, regions, roads, manifests, and the phased import.

---

## 1. Contents

| Fixture | Role |
|:---|:---|
| `map-object-chunk-sample.bin` + `.json` | A 512 m chunk in both encodings. The parity pair. |
| `density/density-fixture.bin` + `.json` | A forest-density tile in both encodings |
| `map-object-catalog-everon-sample.json` | Prefab catalog with classification metadata |
| `map-object-prefabs-sample.json` | Prefab metadata rows |
| `map-object-instances-sample.json` | Placed instance rows |
| `map-object-resolved-sample.json` | Resolved references with transforms and tags |
| `map-object-regions-everon-sample.json` | Chunk region boundaries |
| `regions-derivation-fixture.json` | Input for the forest-region derivation |
| `map-object-roads-sample.json` | Road splines and intersection nodes |
| `type-inventory-pending-everon.json` | Type inventory in its pending state |
| `terrain-registry.sample.json` | Registry shape sample |
| `terrain-manifest-everon-unified-satellite.json` | Manifest with a unified satellite bundle |
| `terrain-manifest-everon-dual-tiles.json` | Manifest with both tile pyramids |
| `terrain-manifest-everon-tile-only-satellite.json` | Manifest whose satellite is tiles only |
| `phased/P1-buildings.json`, `phased/P2-trees.json`, `phased/P1-anchor-fixture.json` | Per-phase import fixtures |
| `locations-everon-sample.json` | Settlements, hills, and landmarks |

---

## 2. Invariants

1. **Dual-encoding parity.** Decoding `map-object-chunk-sample.bin` must produce exactly what decoding `map-object-chunk-sample.json` produces, field for field. The same holds for the density pair. This is the only check that catches an alignment, padding, or endianness mistake in the zero-copy reader, because a wrong-but-well-formed decode is otherwise indistinguishable from a right one.
2. **Instances stay inside their chunk.** Every instance in a chunk sample lies within that chunk's 512 m bounds.
3. **The three terrain manifests are all legal.** They pin that the loader still accepts the unified bundle, the dual pyramid, and the legacy tile-only arrangement — the manifest contract is what lets a dataset ship a subset.
