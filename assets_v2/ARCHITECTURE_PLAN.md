# Asset Storage & Streaming Architecture (`assets_v2`)

How terrain data is partitioned, delivered, held in client memory, and tracked in version control.

---

## 1. Two Storage Tiers

```text
                          ┌──────────────────────────┐
                          │  Axum  ·  /map-assets/*  │
                          └────────────┬─────────────┘
                 ┌─────────────────────┴─────────────────────┐
                 ▼                                           ▼
      Tier 1: built-in datasets                   Tier 2: production volume
      assets_v2/terrains/                         /var/data/tbd/terrains/
      assets_v2/glyphs/                           (community uploads)
```

**Tier 1** is committed to the repository. It exists so a developer, a headless editor gate, or a CI job can boot the full Scenario Creator with no external storage and no network beyond the checkout.

**Tier 2** is a Docker named volume on the self-hosted servers, written by the terrain upload endpoints and read by the same static mount. Its layout and its ingest gates are specified in [`storage_spec/README.md`](./storage_spec/README.md).

The API resolves a directory per tier and serves both under one URL prefix. Neither tier is aware of the other, and a terrain moving from one to the other changes no consumer.

---

## 2. Spatial Partition and Residency

Each island is partitioned into a uniform 512 m grid (`DEFAULT_CHUNK_SIZE_M`). A chunk file holds the object instances whose origin falls in that cell, sorted by density tier so a partial read still yields the visually dominant objects. Everon's grid is 25 × 25 cells, of which 315 carry objects; the remainder are open water or empty terrain and have no file.

Residency is driven by the viewport, not by a fixed kernel:

| Control | Value | Role |
|:---|:---|:---|
| Memory budget | 1,536 MB default (`DEFAULT_BUDGET_MB`) | Ceiling across DEM, satellite mips, chunks and archives |
| LRU floor | 64 chunks (`LRU_MIN_CHUNKS`) | Chunks kept regardless of budget pressure, so panning does not thrash |
| Fetch concurrency | 12 in flight (`FETCH_CONCURRENCY`) | Bounds the request burst a pan produces |
| Failure cap | 3 attempts (`FETCH_FAILURE_CAP`) | A chunk that fails repeatedly is abandoned, not retried forever |
| Apply budget | 4 ms per frame (`APPLY_BUDGET_MS`) | Caps per-frame ingest so streaming cannot stall the render loop |

Eviction is least-recently-used above the floor. The budget is accounted in bytes actually resident, which is why the DEM and satellite mips participate in it rather than sitting outside as fixed overhead.

---

## 3. The Rate-Limit Seam

A cold Scenario Creator load requests up to 951 distinct files: the DEM, satellite mips, every chunk covering the initial viewport, the prefab and road archives, and the glyph atlas. Measured on the live stack while the mount sat above the limiter, 145,858 of the 145,861 `429`s the limiter had ever issued were map assets and none were authentication or ingest.

The mount is therefore registered **below** the rate-limit layer in the API router. `Router::layer` wraps only what is registered above it, so the limiter is never consulted for a map asset rather than being consulted and told to allow it. The exemption is structural: there is no path test anywhere in the limiter, because a `starts_with("/map-assets")` check would also exempt `/map-assets-admin` and would be defeated by a traversal. Byte ceilings and caching for this traffic belong to the reverse proxy, which already handles the prefix.

---

## 4. Version Control Policy

Bulk binaries are tracked in Git LFS, scoped to `assets_v2/terrains/**` by extension: `.png`, `.r16`, `.dem`, `.bin`, `.rkyv`, `.tbd-sat`, `.tbd-bath`, and the BLAS `.bvh` set.

Two deliberate exceptions:

1. **Forest density tiles.** `terrains/*/objects/density/*.bin` are excluded from the filter by the final rule in `.gitattributes`. They were committed as ordinary blobs, and letting the filter claim them would show all 625 as modified in every clone without a byte changing, because the clean filter would produce a pointer that differs from the stored blob. Because git takes the last matching line, that rule must stay last.
2. **Generated pyramids and scratch.** `terrains/*/tiles/` and `assets_v2/scratch/` are gitignored. Tile pyramids are rebuilt on demand from the satellite bundle, and export scratch is intermediate output. Neither is an input to anything downstream.

CI checks out pointers only and pulls the specific objects a job needs:

```bash
cargo xtask ci lfs-dem   # 16-bit DEM elevation grid (~69 MB for Everon)
cargo xtask ci lfs-sat   # unified satellite bundle (~146 MB for Everon)
```
