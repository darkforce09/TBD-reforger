# T-935 — Map binary storage: hybrid rkyv + raw POD

Owner: command center. Source: operator spec 2026-09-04 (hybrid rkyv for complex metadata, raw
`#[repr(C)]` POD for bulk GPU data; flatbuffers considered and rejected) and audit.md Finding 1.4
(apps/website/audit.md:117-124: main-thread gzip-JSON chunk ingest in residency.rs:716-737 and
chunk.rs:46-97). All multi-byte fields on disk are **little-endian**. All headers are 32 bytes,
`#[repr(C)]`, `bytemuck::Pod`, with field orders chosen so the struct has no padding.

## 1. Tiers

| Tier | Encoding | Assets | Load |
|---|---|---|---|
| 1 | raw `#[repr(C)]` POD + bytemuck | chunk instances, DEM grid, forest density (TBDD) | `cast_slice` → `queue.write_buffer` |
| 2 | rkyv 0.8 archives (bytecheck on) | roads, labels, water vectors, prefab catalog + type inventory, forest regions, building blueprints, satellite index | `access_checked::<T>` |
| 3 | mipmapped containers | satellite `.tbd-sat` v2, bathymetry `.tbd-bath` | HTTP Range / header-computed offsets |

## 2. `ObjectInstancePod` (32 B) — correction to the operator sketch

The operator's 24 B sketch `{x,y,z,rotation_deg,prefab_id:u32,class_code,_pad[3]}` dropped
pitch, roll and scale. T-090.12.1 put them on the wire (everon manifest `objects.schemaVersion`
1.1.0, `transforms: "yaw+pitch+roll+scale"`), and `WorldChunk` (chunk.rs:20) carries them as SoA
columns. The POD therefore is:

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjectInstancePod {
    pub x: f32, pub y: f32, pub z: f32,          // metres, world space
    pub yaw: f32, pub pitch: f32, pub roll: f32, // degrees
    pub scale: f32,                              // uniform; 1.0 when absent
    pub prefab_id: u16,                          // index into the prefab catalog (≤ 1623 today)
    pub class_code: u8,                          // same byte as cls_codes
    pub _pad: u8,                                // 0
}
const _: () = assert!(core::mem::size_of::<ObjectInstancePod>() == 32);
```

JSON rows with 5 numbers decode as pitch = roll = 0, scale = 1 — the parity test in T-935.2
proves `.bin` decode == `.json.gz` decode on all 315 everon chunks.

## 3. Containers

### 3.1 `TBDC` — chunk container (`objects/chunks/{cx}_{cy}.bin`)
```
TbdcHeader (32 B): magic [u8;4] = b"TBDC" | version u16 = 1 | flags u16 = 0 | count u32
                   | cx i16 | cy i16 | reserved [u8;16]
payload: count × ObjectInstancePod (32 B each)      file length = 32 + 32 × count
```

### 3.2 `TBDE` — raw DEM (`dem/elevation.dem`)
```
TbdeHeader (32 B): magic b"TBDE" | version u16 = 1 | flags u16 = 0 | width u32 | height u32
                   | scale_m f32 | offset_m f32 | reserved [u8;8]
payload: width × height u16, row-major, row 0 = north edge (same orientation as the PNG)
metres(x, y) = offset_m + sample * scale_m     (everon: offset −204.78, scale (375.53+204.78)/65535)
```

### 3.3 `TBDB` — bathymetry (`water/bathymetry.tbd-bath`)
```
TbdbHeader (32 B): magic b"TBDB" | version u16 = 1 | mip_count u16 | width u32 | height u32
                   | depth_scale f32 | reserved [u8;12]
per level L = 0..mip_count (w_L = max(1, width >> L), h_L likewise):
    depth: w_L × h_L u16 (metres = v × depth_scale), then mask: w_L × h_L u8 (1 = water),
    each level padded to a multiple of 4 bytes. Offsets are computed from the header.
Downsample rule: depth = max of the 2×2 block, mask = any-water.
```

### 3.4 `TBDD` — forest density (`objects/density/*.bin`, unchanged layout)
Existing format from tools/tbd-tools/src/density.rs:24. T-935.5 only replaces the byte loop in
geometry/tbdd.rs:51 with a Pod header parse + `cast_slice`; a Class-R test keeps the 625
committed tiles bit-exact.

### 3.5 `TBDS` v2 — satellite (`satellite/everon-sat.tbd-sat`)
```
fixed header: magic b"TBDS" | version u16 = 2 | flags u16 | index_len u32 | reserved to 32 B
index: rkyv TbdSatIndexV2 { base_w u32, base_h u32, tile_px u16,
        levels: Vec<Level { w_tiles u32, h_tiles u32, tiles: Vec<Tile { offset u64, len u32, format u8 }> }> }
payload: tile bytes at the recorded offsets (offset 0 = start of payload)
```
The reader (`tbd_sat.rs`) Range-reads the first 64 KB, dispatches on `version` (1 = existing
hand-packed table, 2 = rkyv index) and shares the tile Range math.

## 4. rkyv archives (Tier 2) — `crates/map-engine-core/src/world/binary/archives.rs`

```rust
RoadNetworkArchive { segments: Vec<RoadSegmentArchive { id: String, road_class: u8, width_m: f32, centerline: Vec<[f32; 2]> }> }
MapLabelsArchive   { towns: Vec<TownLabel>, height_labels: Vec<HeightLabel>, road_names: Vec<RoadNameLabel> }
WaterVectorsArchive{ lakes: Vec<WaterBody>, rivers: Vec<WaterLine>, ponds: Vec<WaterBody> }   // WaterBody { id, surface_y: f32, ring: Vec<[f32;2]> }
PrefabCatalogArchive { prefabs: Vec<PrefabEntry>, type_inventory: TypeInventory }             // mirrors prefab.rs:15/33 + type-inventory.json
ForestRegionsArchive { regions: Vec<ForestRegion> }                                            // mirrors regions.rs:10
BuildingBlueprintArchive { descriptors: Vec<OccluderDescriptor>, blas_index: Vec<BlasEntry>,
                           blueprints: Vec<BuildingBlueprint { prefab_id: u32, slug: String, vertical_profile: VerticalProfile,
                                                              levels: Vec<BuildingLevel { level_index: u8, elevation_range: [f32; 2],
                                                                                          walls, doors, windows, stairs, furniture }> }> }
TbdSatIndexV2 (section 3.5)
```
All derive `Archive, Serialize, Deserialize` with `#[rkyv(check_bytes)]`; the only entry point
for readers is `access_checked::<T>(&[u8]) -> Result<&Archived<T>>` (validation on, never
`access_unchecked`). Files: `roads/road_network.rkyv`, `locations/map_labels.rkyv`,
`water/water_vectors.rkyv`, `objects/prefabs.rkyv`, `objects/forest-regions.rkyv`,
`objects/type-inventory.rkyv` (optional, inventory also lives in the catalog),
`prefabs/building_blueprints.rkyv`.

## 5. Manifest blocks (all optional, `serde(default)`; schema in T-935.12)

```json
"objects": { "...": "existing fields",
  "binary": { "schemaVersion": "1.0.0", "container": "TBDC", "containerVersion": 1,
              "pod": "ObjectInstancePod", "podBytes": 32,
              "chunks": "objects/chunks/{cx}_{cy}.bin", "prefabs": "objects/prefabs.rkyv",
              "roads": "roads/road_network.rkyv", "regions": "objects/forest-regions.rkyv",
              "typeInventory": "objects/type-inventory.rkyv" } },
"dem":       { "...": "existing", "raw": { "path": "dem/elevation.dem", "encoding": "tbde-v1" } },
"labels":    { "path": "locations/map_labels.rkyv", "encoding": "rkyv-map-labels-v1" },
"water":     { "vectors": "water/water_vectors.rkyv", "bathymetry": "water/bathymetry.tbd-bath", "encoding": "tbdb-v1" },
"buildings": { "archive": "prefabs/building_blueprints.rkyv", "blas": "prefabs/blas" },
"tiles.satellite.unified.encoding": "tbd-sat-v2"
```
A block that is absent means "use the JSON/PNG path". T-935.13 is the only slice that writes
these blocks into `packages/map-assets/everon/manifest.json`.

## 6. Loader flow (frontend `world_assets`)

1. Boot: fetch `manifest.json` (world_host.rs:99). For each asset, pick binary when its block is
   present. Tier-2 files are fetched once (`fetch_bytes`) and kept as `Vec<u8>` owned by the
   store; `access_checked` gives `&Archived<T>` views with no deserialize.
2. Chunks (pan/zoom): `fetch_bytes({cx}_{cy}.bin)` → `WorldResidency::ingest_chunk_bin` →
   `chunk_bin::parse_chunk_bin` (header checks, aligned copy if needed, `cast_slice`) → SoA
   `WorldChunk`. No gzip, no serde.
3. DEM: `dem_load.rs` streams `elevation.dem` into one `Vec<u16>`; metres computed lazily.
4. Occluders: one `building_blueprints.rkyv` fetch replaces 1623 descriptor fetches; `.bvh`
   sidecars are fetched by index from the archive. LOS/viewshed raycast against archived walls.
5. Satellite: Range-read header + index, then tiles by offset. Water: `WaterMask::is_water`.

## 7. Migration order (waves) and dual emission

| Wave | Slices | Rule |
|---|---|---|
| 1 | .1 | formats + manifest structs; nothing else changes |
| 2 | .2 chunks, .4 DEM, .5 density, .7 labels, .8 buildings, .10 sat v2 | emitters dual-emit; loaders dormant |
| 3 | .3 chunk ingest, .6 roads | after .2 (build.rs / tools world/mod.rs) |
| 4 | .9 water, .11 catalog | after .3 (world/mod.rs, world_host.rs), .7 (map/mod.rs), .6 (store sniff) |
| 5 | .12 schema + golden + gates + LFS | before any binary data is committed |
| 6 | .13 cutover | regenerate everon, flip manifest, delete gz emit + flate2, record numbers |

Owns are file-disjoint within a wave; shared files (`world/mod.rs`, `build.rs`, `world_host.rs`,
`world_assets/mod.rs`, `map/mod.rs`) are edited by one slice per wave.

## 8. Claude Code prompts

## Claude Code prompt — T-935.1

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.1 && pwd && git branch --show-current   # slice/T-935.1
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
This spec §2-§5; docs/plans/t-935_1_plan.md; crates/map-engine-core/Cargo.toml;
crates/map-engine-core/src/world/{mod.rs, chunk.rs:1-120, manifest.rs}.
═══ PROBLEM ═══
No binary format definitions exist in map-engine-core; rkyv is used nowhere and bytemuck only in
map-engine-render. Every later slice needs one shared POD, three headers and seven archive types.
═══ SHIPPED ═══
T-090.12.1 (yaw+pitch+roll+scale on the wire, objects.schemaVersion 1.1.0) — the POD keeps all four.
═══ LANGUAGE GATE ═══
Rust only; edition per crate; no scripts.
═══ LOCKED ═══
- ObjectInstancePod is exactly 32 B, field order as §2, compile-time asserted.
- Headers are 32 B, padding-free, LE; magics TBDC/TBDE/TBDB as §3.
- Only `access_checked` is public for readers; no `access_unchecked` anywhere.
- `world` feature enables `binary`; `binary` pulls no flate2/serde_json.
- Existing everon manifest.json must parse unchanged (test it).
═══ DO ═══
1. Cargo.toml: feature `binary` (rkyv 0.8 + bytecheck, bytemuck derive), `world` enables it.
2. world/binary/{mod,pod,chunk_container,archives}.rs per §2-§4; register in world/mod.rs.
3. manifest.rs: optional blocks per §5 with serde(default).
4. Round-trip test per type; corruption test (flip a byte → Err); size asserts.
5. Perturbation: `_pad: [u8;2]` → paste the red const-assert, restore, touch, green.
6. cargo test -p map-engine-core --all-features; cargo xtask ci verify file-length.
═══ DO NOT ═══
No emitter/loader edits; no files outside owns; no --all-features omission; no git add -A/stash.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask platform wave gate --slice T-935.1
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.2

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.2 && pwd && git branch --show-current   # slice/T-935.2
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
Spec §2-§3.1; docs/plans/t-935_2_plan.md; tools/tbd-tools/src/world/build.rs:135-520 and mod.rs;
crates/map-engine-core/src/world/{binary/, chunk.rs:46-97}.
═══ PROBLEM ═══
build.rs:503 writes every chunk as gzip-9 JSON only. The loader must inflate and parse it on the
main thread (audit 1.4). A binary twin of each chunk is needed before any loader can change.
═══ SHIPPED ═══
T-935.1 (ObjectInstancePod, TbdcHeader). Do not redefine them.
═══ LANGUAGE GATE ═══
Rust, edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- Dual emission: the gz-JSON write stays byte-identical.
- File = TbdcHeader + count × POD; rows with 5 numbers → pitch=roll=0, scale=1.
- Parity test covers every committed everon chunk (315) — not a sample.
- Generated .bin files are not committed (LFS rules land in T-935.12).
═══ DO ═══
1. binary_emit.rs: write_chunk_bin(path, cx, cy, rows); register in world/mod.rs.
2. build.rs: call it beside the gz write at :503; Cargo.toml enables map-engine-core `binary`.
3. Parity test: emit to tempdir, cast_slice the .bin, decode JSON via chunk.rs, compare columns.
4. Perturbation: swap yaw/pitch in the emitter → red on real data; restore, touch, green.
5. Run build-objects once; confirm 315 .bin files of 32 + 32×N bytes; leave them uncommitted.
═══ DO NOT ═══
No loader edits; no manifest edits; no crates/ or apps/ changes.
═══ VERIFY ═══
cargo test -p tbd-tools binary_emit ; cargo test -p map-engine-core --all-features ;
cargo xtask platform wave gate --slice T-935.2
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.3

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.3 && pwd && git branch --show-current   # slice/T-935.3
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
cargo xtask map world-los --cell 18_0 --probe 9350,15,280 9380,15,290 > /tmp/los-before.txt
═══ READ ═══
Spec §3.1, §6; docs/plans/t-935_3_plan.md; residency.rs:700-760 (only that span), chunk.rs:46-97,
world/binary/, apps/website/frontend/src/editor/world_assets/world_host.rs:90-140 and :400-460.
═══ PROBLEM ═══
Chunk ingest is gzip + serde on the main thread (residency.rs:716-737). world_host.rs:426 knows
only the .json.gz URL. residency.rs is allowlisted SIZE-3, so the parser cannot live there.
═══ SHIPPED ═══
T-935.1 (formats, manifest.objects.binary), T-935.2 (emitter, parity proven).
═══ LANGUAGE GATE ═══
Rust only.
═══ LOCKED ═══
- residency.rs grows ≤ 40 lines: call sites only; parser in chunk_bin.rs.
- Length/magic/version checks before any cast; misaligned → aligned copy; never unwrap.
- gz path stays byte-identical; binary branch only when manifest.objects.binary is Some.
- world-los output on cell 18_0 identical before/after.
═══ DO ═══
1. chunk_bin.rs: parse_chunk_bin(&[u8]) -> Result<WorldChunk>; register in world/mod.rs.
2. residency.rs: ingest_chunk_bin(cx, cy, bytes) reusing the gz path's bookkeeping.
3. world_host.rs: branch at :426 on manifest.objects.binary.chunks; fill {cx}_{cy}.
4. Tests: two-row round-trip; truncated / bad magic / bad version → Err.
5. Perturbation: skip the length check → truncated test red; restore, touch, green.
6. Diff world-los after vs /tmp/los-before.txt — must be empty.
═══ DO NOT ═══
No manifest.json edits; no emitter edits; no residency refactors beyond the call sites.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask mk leptos-gates ;
cargo xtask platform wave gate --slice T-935.3
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.4

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.4 && pwd && git branch --show-current   # slice/T-935.4
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
Spec §3.2, §6; docs/plans/t-935_4_plan.md; tools/tbd-tools/src/world/aux.rs:1080-1140;
crates/map-engine-core/src/dem/{mod.rs, png_decode.rs:40-100}; world_assets/mod.rs:600-660.
═══ PROBLEM ═══
The DEM boots from a 71.9 MB 16-bit PNG decoded through several full-grid copies. A raw u16 grid
with a TBDE header needs no decoder and streams into its final buffer.
═══ SHIPPED ═══
T-935.1 (TbdeHeader, manifest.dem.raw).
═══ LANGUAGE GATE ═══
Rust; edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- u16 samples in .dem equal the PNG's; scale_m = (max−min)/65535, offset_m = min.
- One allocation in the loader; metres computed lazily; no f32 grid.
- PNG path untouched and used when manifest.dem.raw is None.
═══ DO ═══
1. aux.rs: write_elevation_dem beside :1109.
2. dem/raw.rs (+ dem/mod.rs): RawDem parse, sample_u16, metres.
3. world_assets/dem_load.rs (+ mod.rs): raw when manifest.dem.raw is Some, else PNG loader.
4. Test: 4×3 grid → PNG + .dem → identical samples, metres within 1e-6.
5. Perturbation: swap width/height on header read → red; restore, touch, green.
═══ DO NOT ═══
No manifest.json edits; no deletion of the PNG path; no residency edits.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features dem ; cargo test -p tbd-tools elevation_dem ;
cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-935.4
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.5

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.5 && pwd && git branch --show-current   # slice/T-935.5
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
Spec §3.4; docs/plans/t-935_5_plan.md; crates/map-engine-core/src/geometry/tbdd.rs;
tools/tbd-tools/src/density.rs:1-120.
═══ PROBLEM ═══
decode_tbdd (tbdd.rs:51) assembles every cell byte by byte. 625 everon tiles are committed and
must stay bit-exact, so only the decoder changes.
═══ SHIPPED ═══
T-935.1 (bytemuck available under `binary`).
═══ LANGUAGE GATE ═══
Rust.
═══ LOCKED ═══
- On-disk TBDD layout unchanged; density.rs emits identical bytes.
- Class-R test keeps the old loop verbatim under cfg(test) and scrubs its own source.
- Unaligned → aligned copy; short payload → Err.
═══ DO ═══
1. Copy the current loop into mod parity_reference (cfg(test)) before any edit.
2. Pod header struct + cast_slice decode.
3. Class-R test over all 625 objects/density/*.bin; density.rs round-trip test.
4. Perturbation: off-by-one header length → red; restore, touch, green.
═══ DO NOT ═══
No format changes; no forest_mass.rs edits.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features tbdd ; cargo test -p tbd-tools density ;
cargo xtask platform wave gate --slice T-935.5
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.6

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.6 && pwd && git branch --show-current   # slice/T-935.6
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
Spec §4; docs/plans/t-935_6_plan.md; tools/tbd-tools/src/world/build.rs:1100-1190, bin/world.rs;
crates/map-engine-core/src/world/{roads.rs, store.rs:40-130, binary/archives.rs}.
═══ PROBLEM ═══
build-roads writes objects/roads.json.gz only; roads.rs parses JSON; store.rs has no format
switch. The road network should load through a validated rkyv archive.
═══ SHIPPED ═══
T-935.1 (RoadNetworkArchive), T-935.2 (tools world/mod.rs shape).
═══ LANGUAGE GATE ═══
Rust; edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- Dual emission; JSON output byte-identical.
- store.rs sniffs gzip magic 1f 8b → JSON else rkyv; empty buffer → Err.
- Parity on everon: ids, classes, widths, points bit-equal.
═══ DO ═══
1. roads_emit.rs (+ world/mod.rs): write_road_network_rkyv; bin/world.rs calls it after JSON.
2. roads.rs: from_archive; store.rs: magic sniff.
3. Parity test; perturbation drops the last centerline point → red; restore, touch, green.
═══ DO NOT ═══
No build.rs edits (T-935.2/.11 own it in other waves); no world_host.rs edits.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features roads ; cargo test -p tbd-tools roads_emit ;
cargo xtask platform wave gate --slice T-935.6
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.7

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.7 && pwd && git branch --show-current   # slice/T-935.7
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
Spec §4; docs/plans/t-935_7_plan.md; tools/tbd-tools/src/{bin/map.rs, map/mod.rs};
world_assets/labels.rs:50-110; crates/map-engine-core/src/world/{locations.rs, road_labels.rs}.
═══ PROBLEM ═══
labels.rs fetches three JSON files and parses them with serde_json. One MapLabelsArchive replaces
the three fetches with a validated zero-copy view.
═══ SHIPPED ═══
T-935.1 (MapLabelsArchive, manifest.labels).
═══ LANGUAGE GATE ═══
Rust; edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- Three JSON files stay the source of truth; the archive is derived from them.
- labels.rs JSON path untouched when manifest.labels is None.
- Parity on everon for towns, height labels, road names.
═══ DO ═══
1. map/labels_emit.rs (+ map/mod.rs); bin/map.rs `labels-rkyv --terrain <dir>`.
2. locations.rs, road_labels.rs: from_archive; labels.rs: single fetch branch.
3. Parity test; perturbation drops road_names → red; restore, touch, green.
═══ DO NOT ═══
No manifest.json edits; no water code (T-935.9).
═══ VERIFY ═══
cargo test -p map-engine-core --all-features labels ; cargo test -p tbd-tools labels_emit ;
cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-935.7
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.8

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.8 && pwd && git branch --show-current   # slice/T-935.8
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
cargo xtask map world-los --cell 18_0 --probe 9350,15,280 9380,15,290 > /tmp/los-before.txt
═══ READ ═══
Spec §4, §6; docs/plans/t-935_8_plan.md; xtask/src/map_blueprint/{mod.rs, library_cli.rs,
library.rs:560-620}; world_assets/occluder_host.rs:40-150; crates/map-engine-core/src/{world/occluder/descriptor.rs, building_blueprint.rs}.
═══ PROBLEM ═══
The occluder host fetches blas-manifest.json plus 1623 descriptor JSON files (19 MB) at boot.
One BuildingBlueprintArchive (descriptors + BLAS index + blueprint levels) replaces the fan-out.
═══ SHIPPED ═══
T-090.12.2 (1,691 BLAS sidecars), T-935.1 (BuildingBlueprintArchive, manifest.buildings).
═══ LANGUAGE GATE ═══
Rust.
═══ LOCKED ═══
- Blueprint levels for prefabs without golden data come from a Workbench pass the operator runs:
  the archive command prints that exact command; this is not a deferral.
- JSON branch in occluder_host.rs stays until T-935.13.
- world-los on cell 18_0 identical before/after.
═══ DO ═══
1. archive_emit.rs (+ mod.rs, library_cli.rs `archive --terrain everon`).
2. descriptor.rs, building_blueprint.rs: from_archived.
3. occluder_host.rs: archive branch; .bvh indices from the archive.
4. Parity test on 1623 descriptors + 6 blueprints; perturbation skips the BLAS index → red.
5. Diff world-los after vs /tmp/los-before.txt — must be empty.
═══ DO NOT ═══
No manifest.json edits; no BLAS/bvh regeneration.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features building ; cargo xtask map-blueprint archive --terrain everon ;
cargo xtask platform wave gate --slice T-935.8
═══ MANUAL ═══
Operator runs the printed Workbench batch command to extract remaining blueprint levels.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.9

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.9 && pwd && git branch --show-current   # slice/T-935.9
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
ls -la packages/map-assets/everon/staging/water/   # three staging files must exist
═══ READ ═══
Spec §3.3, §4, §6; docs/plans/t-935_9_plan.md; tools/tbd-tools/src/map/mod.rs;
crates/map-engine-core/src/world/{mod.rs, binary/}; world_assets/mod.rs (module list only).
═══ PROBLEM ═══
No code reads the water exports (two 328 MB rasters + vectors JSON). Water must become a binary
asset with a mask query for placement guards.
═══ SHIPPED ═══
T-935.1 (TbdbHeader, WaterVectorsArchive, manifest.water), T-935.7 (map subcommand shape), T-935.3.
═══ LANGUAGE GATE ═══
Rust; edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- Rasters are streamed line by line; never fully in memory.
- TBDB level layout and downsample rule per §3.3; deterministic bytes.
- Subcommand registered through map/mod.rs only (bin/map.rs is not yours).
═══ DO ═══
1. map/water_emit.rs (+ map/mod.rs): vectors → rkyv; rasters → TBDB with mips.
2. core world/water.rs (+ world/mod.rs): parse, is_water, depth_m, mip selection.
3. frontend world_assets/water.rs (+ mod.rs): fetch when manifest.water is Some; WaterMask.
4. 4×4 / 3-mip tests; perturbation flips mask polarity → red; restore, touch, green.
5. Run `map water` on everon staging; print sizes; do not commit outputs.
═══ DO NOT ═══
No manifest.json edits; no bin/map.rs edits; no residency edits.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features water ; cargo test -p tbd-tools water_emit ;
cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-935.9
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.10

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.10 && pwd && git branch --show-current   # slice/T-935.10
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
Spec §3.5; docs/plans/t-935_10_plan.md; tools/tbd-tools/src/map/unified.rs;
world_assets/tbd_sat.rs; crates/map-engine-core/src/world/binary/archives.rs (TbdSatIndexV2).
═══ PROBLEM ═══
The TBDS v1 tile table is hand-packed and parsed byte by byte. v2 replaces it with a validated
rkyv index while the committed v1 file must keep loading.
═══ SHIPPED ═══
T-935.1 (TbdSatIndexV2).
═══ LANGUAGE GATE ═══
Rust; edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- v1 writer and reader retained; reader dispatches on header version.
- Payload/tile layout unchanged between v1 and v2.
- Index longer than 64 KB → second Range request, never a full download.
═══ DO ═══
1. unified.rs: `--container-version 2` default; header + index + tiles; v1 behind `1`.
2. tbd_sat.rs: version switch; v2 index via access_checked; shared Range math.
3. Round-trip test (3 mips, both versions); perturbation: index_len one byte short → red.
═══ DO NOT ═══
No regeneration of the committed everon-sat.tbd-sat (T-935.13); no manifest edits.
═══ VERIFY ═══
cargo test -p tbd-tools unified ; cargo test -p map-engine-core --all-features ;
cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-935.10
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.11

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.11 && pwd && git branch --show-current   # slice/T-935.11
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
Spec §4-§6; docs/plans/t-935_11_plan.md; tools/tbd-tools/src/world/{build.rs:470-760, mod.rs};
crates/map-engine-core/src/world/{prefab.rs, regions.rs, store.rs}; world_assets/world_host.rs:120-170.
═══ PROBLEM ═══
Prefab catalog, forest regions and type inventory ship as JSON; world_host.rs fetches JSON for
all objects.* assets. Archives plus a manifest-keyed fetch branch complete the objects tier.
═══ SHIPPED ═══
T-935.1, T-935.2 (build.rs), T-935.3 (world_host.rs binary branch), T-935.6 (store.rs sniff).
═══ LANGUAGE GATE ═══
Rust; edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- Dual emission; JSON outputs byte-identical.
- Type inventory lives inside PrefabCatalogArchive; separate file optional.
- JSON fetches remain default until T-935.13.
═══ DO ═══
1. world/catalog_emit.rs (+ mod.rs); build.rs calls both writers beside the JSON writes.
2. prefab.rs, regions.rs: from_archive.
3. world_host.rs: archive URL per asset (prefabs, regions, roads, type inventory) when named.
4. Parity test on everon; perturbation shuffles prefab order → red; restore, touch, green.
═══ DO NOT ═══
No manifest.json edits; no residency edits; no schema edits.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features catalog ; cargo test -p tbd-tools catalog_emit ;
cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-935.11
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.12

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.12 && pwd && git branch --show-current   # slice/T-935.12
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
Spec §2, §5; docs/plans/t-935_12_plan.md; packages/tbd-schema/schema/{terrain-manifest,
map-object-instance}.schema.json; xtask/src/{golden_gate.rs:440-520, schema_gates.rs:3320-3420}; .gitattributes.
═══ PROBLEM ═══
Schemas and gates know only the JSON formats, and no LFS rule covers the new binary extensions.
The cutover commit must be gated and LFS-routed before it lands.
═══ SHIPPED ═══
T-935.1-.11 (every format, emitter and loader).
═══ LANGUAGE GATE ═══
Rust + JSON Schema. No scripts.
═══ LOCKED ═══
- Blocks optional; everon manifest validates with and without them.
- Golden .bin is produced by the T-935.2 emitter from the golden JSON, never hand-written.
- LFS rules are scoped so the committed density *.bin files are not re-routed.
═══ DO ═══
1. Schemas per §5; binary row documented beside the JSON row.
2. .gitattributes rules; `git check-attr filter` on one path per extension.
3. Golden map-object-chunk-sample.bin; golden_gate.rs binary check; schema_gates.rs path existence.
4. Perturbation: flip one byte of the golden .bin → gate red; restore, touch, green.
═══ DO NOT ═══
No manifest.json edits; no emitter/loader edits.
═══ VERIFY ═══
cargo xtask ci schema-validate ; cargo xtask ci verify map-object-golden ;
cargo xtask ci verify-terrain-strict ; cargo xtask platform wave gate --slice T-935.12
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```

## Claude Code prompt — T-935.13

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-935.13 && pwd && git branch --show-current   # slice/T-935.13
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target ; git lfs version
cargo xtask map world-los --cell 18_0 --probe 9350,15,280 9380,15,290 > /tmp/los-before.txt
du -sh packages/map-assets/everon/{objects,dem,satellite,prefabs} > /tmp/sizes-before.txt
═══ READ ═══
Spec §5-§7; docs/plans/t-935_13_plan.md; packages/map-assets/everon/manifest.json;
crates/map-engine-core/Cargo.toml; world_assets/mod.rs; tools/tbd-tools/src/world/build.rs:480-520.
═══ PROBLEM ═══
Every loader has a dormant binary branch but everon still boots from JSON/PNG because the
manifest names those paths; gz-JSON emit and flate2 are still present.
═══ SHIPPED ═══
T-935.1-.12; T-943 (LFS-safe wave push).
═══ LANGUAGE GATE ═══
Rust + JSON.
═══ LOCKED ═══
- Data files are emitter outputs, never hand-edited; committed through LFS.
- DEM PNG and descriptor JSON stay (emitter inputs); gz-JSON chunk/catalog files are deleted.
- world-los on cell 18_0 identical before/after; before/after numbers in the commit body.
═══ DO ═══
1. Regenerate everon with every emitter; fill manifest blocks per §5; delete gz-JSON files.
2. build.rs: remove the gz chunk writer; Cargo.toml: drop flate2 from `world`; mod.rs: binary default.
3. Gates; diff world-los vs /tmp/los-before.txt; measure after; write the commit body.
═══ DO NOT ═══
No schema edits; no new formats; no git add -A; no push.
═══ VERIFY ═══
cargo xtask ci verify-terrain-strict ; cargo test -p map-engine-core --all-features ;
cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-935.13
═══ MANUAL ═══
Command center pushes with `cargo xtask platform wave push` on a git-lfs host and pastes the mode line.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
