# TBD Reforger — Map Data Storage & Binary Architecture (user-supplied spec, 2026-09-04)
Strategy: Hybrid rkyv (complex metadata) + raw #[repr(C)] POD (bulk GPU data)

## Tiers
- Tier 1 raw #[repr(C)] POD + bytemuck: chunk instances, DEM grid, forest density, vertex arrays. Direct byte->VRAM via queue.write_buffer.
- Tier 2 rkyv zero-copy archives: building blueprints, road network, water vectors, locations/labels.
- Tier 3 mipmapped binary container (.tbd-sat / .tbd-bath): satellite ortho, bathymetry.

## Domains
1. Object placements: src objects/chunks/{cx}_{cy}.json | objects/all_buildings.jsonl -> .bin per chunk.
   ```rust
   #[repr(C, align(4))]
   pub struct ObjectInstancePod { x: f32, y: f32, z: f32, rotation_deg: f32, prefab_id: u32, class_code: u8, _pad: [u8;3] } // 24 bytes
   ```
   Load: bytemuck::cast_slice(&bytes) -> wgpu::Queue::write_buffer.
2. Building blueprints: src prefabs/buildings/<slug>.json + prefabs/prefabs_catalog.json -> prefabs/building_blueprints.rkyv.
   BuildingBlueprintArchive { prefab_id:u32, slug:String, vertical_profile, levels: Vec<BuildingLevelArchive> }
   BuildingLevelArchive { level_index:u8, elevation_range:[f32;2], walls, doors, windows, stairs, furniture }
3. Roads: src roads/all_roads.json -> roads/road_network.rkyv (aka roads.rkyv).
   RoadNetworkArchive { segments: Vec<RoadSegmentArchive { id:String, road_class:u8, width_m:f32, centerline_points: Vec<[f32;2]> }> }
4. Water: bathymetry_depth.png (16-bit) -> .tbd-bath or raw 16-bit grid; lakes.json + rivers.json -> water/water_vectors.rkyv.
5. DEM: dem/dem-16bit.png (12800x12800) -> dem/elevation.dem raw u16 POD; elev = data[z*w+x] * 0.1 m.
6. Satellite: satellite/tiles/*.png | ortho.png -> satellite/everon-sat.tbd-sat single mipmapped container with indexed header.
7. Locations: towns_settlements.json, height_labels.json, road_names.json -> locations/map_labels.rkyv (aka locations.rkyv).
(8. Forest density -> raw .bin POD, listed in diagram only.)

## Web loading flow
- Cold boot: fetch building_blueprints.rkyv, roads.rkyv, locations.rkyv; rkyv::access::<Archived..>(&bytes); parse <1ms.
- Pan/zoom: visible 512m chunks -> fetch_bytes {cx}_{cy}.bin -> bytemuck::cast_slice::<u8, ObjectInstancePod> -> queue.write_buffer; <0.05ms/chunk.
- 2.5D viewshed/LOS: raycast against ArchivedBuildingBlueprint walls/sills/stairs, zero indirection.

## Context
- audit.md Finding 1.4 (apps/website/audit.md:117-124): main-thread gzip-JSON chunk ingest in crates/map-engine-core/src/world/residency.rs:716-737, chunk.rs:46-97 — fix = flat binary ingest.
- User earlier said "Rkyv" then "flatbuffers" — spec doc chooses rkyv; treat flatbuffers as considered/rejected unless user says otherwise.
