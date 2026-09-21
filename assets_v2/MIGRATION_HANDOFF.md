# Map Assets Relocation Handoff

`packages/map-assets` is gone. Its contents live in `assets_v2/`, split into what is served, what is shared, and what is local scratch.

---

## 1. Where everything went

| Legacy path | New path |
|:---|:---|
| `everon/` | `terrains/everon/` |
| `arland/` | `terrains/arland/` |
| `terrain-registry.json` | `terrains/terrain-registry.json` |
| `glyphs/{atlas,svg,manifest.json}` | `glyphs/` |
| `everon/staging/` | `scratch/everon/` (gitignored) |
| `everon/tiles/` | `terrains/everon/tiles/` (gitignored) |
| `README.md` | retired; replaced by the new tree's documents |

Every file kept its name, its bytes and its URL. `/map-assets/**` is byte-identical from a client's point of view.

## 2. The two structural changes

**Scratch left the served tree.** Export intermediates were at `everon/staging/` — inside the directory the API serves wholesale at `/map-assets`. They are now at `assets_v2/scratch/`, outside it. That is 1.5 GB of uncommitted local output that was one `ServeDir` away from being public. `repository_layout`'s sibling test pins the separation.

**Glyphs left the terrain tree.** The glyph atlas is shared by every terrain, so it sits beside `terrains/` rather than inside one island's directory. The URL is unchanged, which means the two directories are now joined at the router: a `/map-assets/glyphs` mount registered below the same rate-limit seam, on a more specific path that axum resolves ahead of the terrain catch-all. The gate harness does the same, and both directories travel as one `MapAssetMounts` value so a caller cannot wire one and forget the other.

## 3. Git LFS

The attribute rules were rewritten **before** the move, because a binary re-hashed while its filter is unspecified enters git as raw bytes and no later rule reclaims it. The move itself was directory-level `git mv`, one `rename(2)` each, so no blob was re-read.

| Proof | Result |
|:---|:---|
| Tracked LFS files | 2,013, unchanged |
| LFS files outside `assets_v2/` | 0 |
| `git lfs push --dry-run origin main` | 0 objects — renames create no new OIDs |
| Staged entries that were not exact renames | only `.gitattributes`, `.gitignore`, and the retired READMEs |
| `git check-attr filter` on `terrains/everon/dem/*.png`, `objects/chunks/0_0.bin` | `lfs` |
| `git check-attr filter` on `objects/density/0_0.bin` | `unset` |

The 625 forest-density tiles keep their exemption, and its rule must stay the **last** line of `.gitattributes`: git takes the final match, and letting the `.bin` rule claim them would show all 625 as modified in every clone without a byte changing.

## 4. Live verification

Served through the gate harness with `--map-assets assets_v2/terrains`:

| Request | Result |
|:---|:---|
| `/map-assets/terrain-registry.json` | 200, 804 B |
| `/map-assets/everon/manifest.json` | 200, 3,601 B |
| `/map-assets/everon/objects/chunks/10_10.bin` | 200, 89,280 B |
| `/map-assets/everon/dem/everon-dem-16bit.png` | 200, 71,911,548 B (real LFS content, not a pointer) |
| `/map-assets/glyphs/manifest.json` | 200, 3,788 B |
| `/map-assets/glyphs/atlas/world-glyphs.webp` | 200, 42,514 B |
| `/map-assets/../../../etc/passwd` and the glyph equivalent | 404 |

`cargo test -p website-api --test map_assets_rate_limit_exemption` (9 tests) additionally proves through the real router that a 200-request burst — five times the global limit — to one file from each directory returns 200 throughout, while `/uploads` and the API routes still refuse.

## 5. The deployment host

Resolved in the deploy path; one manual step remains on the server.

The answer turned out to be *both* halves, not either. The API's defaults resolve against its
working directory, and neither production runtime satisfies them — the container has no WORKDIR and
no asset copy, and the host unit runs from `apps/website`, three levels below which is outside the
repo. `MAP_ASSETS_DIR` and `GLYPH_ASSETS_DIR` are therefore pinned to absolute paths on the unit
(`scripts/deploy/tbd-website-api.service`, now version-controlled) and on the compose `api` service,
which also gains read-only bind mounts because its image ships only the binary.

The terrain tree itself still has to move on the server, once. `cargo xtask deploy website` now
probes for it before the rsync and refuses the deploy if the host is still on the old layout,
printing the exact `mv`. It does not move the data itself: that is ~590 MB of production content,
and a refused deploy is a better outcome than a half-finished automatic move.

A host with no asset tree at all warns and proceeds — a library-only site never requests
`/map-assets`.

Separately, the rsync's `--exclude=packages/map-assets/` had been rewritten to
`--exclude=assets_v2/terrains/` as a literal substitution. Because the exclude list of a `--delete`
rsync is also a delete guard, that rewrite left the server's own asset copy unprotected while also
pushing 1.5 GB of local scratch. Both are fixed.

## 6. Documentation

The scaffold documents that stood here before the move described files that do not exist (`chunk_*.tbdc`, `elevation.dem`, `satellite.tbds`, MSDF glyph atlases), a residency budget of 512 MB against a real 1,536 MB, a 25-chunk kernel against a real 64-chunk LRU floor, 625 object chunks against a real 315, and a table of surface anchors with invented names. They are rewritten from the live tree and from the T-935 storage spec. The production volume specification is now marked as the unbuilt design it is: no upload endpoint, no volume and no ingest gate exists yet.
