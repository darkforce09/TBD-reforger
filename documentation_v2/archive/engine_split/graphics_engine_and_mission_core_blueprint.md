# Master Modular Architectural Blueprint: Graphics Engine, Mission Core & Frontend Editor

## 1. Modular Architectural Philosophy

To satisfy our hard gating requirements (<500 lines per production file, strict separation of concerns, zero code bloat), this architecture rejects large, monolithic files (such as the legacy `residency.rs` at 3,175 lines, `engine.rs` at 7,071 lines, or `store.rs` at 15,915 lines). 

Instead, the codebase is decomposed into **fine-grained, deeply modular subfolders**:
- Every major subsystem is housed in its own directory with an authoritative `README.md`.
- Large subsystems (such as `streaming/`, `renderers/`, `spatial/`, and `doc/`) are further partitioned into cohesive, single-responsibility sub-directories.
- Every individual file is kept strictly under the **500-line production threshold**.
- Test files stay under the **1,000-line limit** and use the out-of-line pattern (`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;` or sibling `tests/` directories).

```text
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    PILLAR 3: apps/website/frontend/                             │
│                      THE CONDUCTOR (UI & Interaction)                           │
│  • Pure Leptos 0.8 CSR UI: Dock Panels, Modals, Menus, Outliner Tree, Toolbelt  │
│  • User Input: Keyboard Hotkeys (Ctrl+Z, Space), Pointer & Gesture Listeners    │
│  • Reactive Signals: Current Selection, Active Tool, Panel States               │
│  • Canvas Mount: Hosts the HTML <canvas> element and routes interaction events  │
└───────────────────────────┬─────────────────────────┬───────────────────────────┘
                            │                         │
               1. Mutate / Query Domain     2. Command / Viewport
                            │                         │
                            ▼                         ▼
┌───────────────────────────────────────┐ ┌───────────────────────────────────────┐
│     PILLAR 2: mission-core/           │ │   PILLAR 1: graphics-engine/          │
│     THE BRAIN (Headless Domain)       │ │   THE RENDERER (WebGPU & Spatial)     │
│  • Mission AST, Schemas & Extensions  │ │  • 10 WebGPU Pipelines & Shaders      │
│  • Game Export Compiler (Enfusion)    │ │  • Ortho (2D Map) & Orbit (3D Doll)   │
│  • Validation Rules & Wire Safety     │ │  • Terrain DEM, Hillshading, Contours │
│  • Yrs CRDT Document Store (doc/)     │ │  • 3D BVH Raycaster, LOS & Viewshed   │
│  • Headless Entity State Operations   │ │  • World 512m Chunk Streaming & LRU   │
│  (Pure Rust: Zero DOM, Zero WebGPU)   │ │  • NATO Symbology & Slot Atlas GPU    │
│                                       │ │  • 3D Arsenal Mannequin Preview Engine│
│                                       │ │  (Pure Engine: Zero DOM, Zero UI)     │
└───────────────────────────────────────┘ └───────────────────────────────────────┘
                    ▲
                    │ 3. Server-side Export & Validation
┌───────────────────┴───────────────────┐
│       apps/website/api_v2/               │
│       (Axum Backend Server)           │
└───────────────────────────────────────┘
```

---

## 2. Granular Directory Structure & File Mapping

### Pillar 1: `apps/website/graphics-engine/` (The Unified Graphics Engine)

Consolidates all rendering, WebGPU pipelines, camera, terrain math, shaders, spatial raycasting, and chunk streaming into one crate under `apps/website/`.

```text
apps/website/graphics-engine/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    │
    ├── core/                                   <-- WebGPU Context & Frame State
    │   ├── README.md
    │   ├── context/
    │   │   ├── mod.rs
    │   │   ├── device.rs                       [← engine.rs:1247-1658 adapter, device, surface config]
    │   │   └── state.rs                        [← engine.rs canvas resize and device limits]
    │   ├── buffers/
    │   │   ├── mod.rs
    │   │   ├── pool.rs                         [← buffer_pool.rs: persistent LanePool, grow_capacity]
    │   │   └── readback.rs                     [← readback.rs: ReadbackLane, map_async guards]
    │   ├── culling/
    │   │   ├── mod.rs
    │   │   ├── compute.rs                      [← icon_cull_gpu.rs: WebGPU compute pipeline]
    │   │   └── oracle.rs                       [← compute_cull.rs: CPU reference frustum test]
    │   └── pipeline/
    │       ├── mod.rs
    │       ├── draw_order.rs                   [← draw_order.rs: LaneRole (48 lanes), lane_order()]
    │       ├── roles.rs                        [← draw_order.rs: role_id (24 IDs), tex_role_id]
    │       └── damage.rs                       [← damage.rs: RenderDamage, frame-skip dirty rects]
    │
    ├── camera/                                 <-- Camera Models & Projections
    │   ├── README.md
    │   ├── ortho/
    │   │   ├── mod.rs
    │   │   ├── camera.rs                       [← map-engine-core/camera/ortho.rs: OrthoCamera struct]
    │   │   ├── projection.rs                   [← camera/ortho.rs: view, projection, clip matrices]
    │   │   ├── unproject.rs                    [← camera/ortho.rs: unproject_xy, visible_world_rect]
    │   │   └── controllers.rs                  [← camera/ortho.rs: pan, zoom_at with fixed cursor]
    │   ├── orbit/
    │   │   ├── mod.rs
    │   │   ├── camera.rs                       [← doll/mod.rs: orbit camera model]
    │   │   ├── projection.rs                   [← doll/mod.rs: view_proj_wgpu, view_proj_gl]
    │   │   └── controllers.rs                  [← doll/mod.rs: yaw rotation controller]
    │   └── math/
    │       ├── mod.rs
    │       ├── glmat4.rs                       [← map-engine-core/camera/glmat4.rs: f64 4x4 ops]
    │       └── shaping.rs                      [← camera/ortho.rs: dimension coercion, JS rounding]
    │
    ├── renderers/                              <-- Render Pipelines & Primitive Drawers
    │   ├── README.md
    │   ├── engine/
    │   │   ├── mod.rs
    │   │   ├── engine.rs                       [← engine.rs:1122-1246: RenderEngine struct]
    │   │   ├── lifecycle.rs                    [← engine.rs: create(), atlas initialization]
    │   │   └── loop.rs                         [← engine.rs: render(), surface acquire, submit]
    │   ├── pipelines/
    │   │   ├── mod.rs
    │   │   ├── quad.rs                         [← engine.rs:437: create_quad_pipeline]
    │   │   ├── textured.rs                     [← engine.rs:494: create_textured_pipeline]
    │   │   ├── vector.rs                       [← engine.rs:579,843: line and polygon pipelines]
    │   │   ├── building.rs                     [← engine.rs:887: create_building_pipeline]
    │   │   └── icon.rs                         [← engine.rs:694,768: icon vertex and storage32 pipelines]
    │   ├── batching/
    │   │   ├── mod.rs
    │   │   ├── encoder.rs                      [← engine.rs:938: draw_batches() single-pass encoder]
    │   │   └── batch.rs                        [← engine.rs: BatchPayload, Batch]
    │   ├── text/
    │   │   ├── mod.rs
    │   │   ├── atlas.rs                        [← text_layout.rs: bake_ascii_atlas_rgba, font tables]
    │   │   ├── layout.rs                       [← text_layout.rs: TextGlyphInstance, cell UVs]
    │   │   ├── packing.rs                      [← text_layout.rs: pack_label_glyphs, pack_town_labels]
    │   │   └── lanes.rs                        [← engine.rs:3616: upload_text_labels, upload_town_labels]
    │   └── primitives/
    │       ├── mod.rs
    │       ├── vector_lines.rs                 [← engine.rs:5093: upload_polygon_mesh, upload_strip_tris]
    │       ├── hairlines.rs                    [← engine.rs: upload_hairline_segments, connections_bind]
    │       ├── selection.rs                    [← engine.rs:5305: upload_marquee, selection overlays]
    │       └── compose.rs                      [← map-engine-core/geometry/vector_compose.rs]
    │
    ├── spatial/                                <-- BVH, Point Index & Raycasting
    │   ├── README.md
    │   ├── indexing/
    │   │   ├── mod.rs
    │   │   ├── point_index.rs                  [← map-engine-core/spatial/point_index.rs: 2D CSR grid]
    │   │   └── cluster.rs                      [← map-engine-core/spatial/cluster.rs: marker clustering]
    │   ├── bvh/
    │   │   ├── mod.rs
    │   │   ├── tree.rs                         [← map-engine-core/bvh.rs: Bvh struct, any_hit, all_hits]
    │   │   ├── node.rs                         [← bvh.rs: triangle intersection kernel]
    │   │   ├── surface.rs                      [← bvh.rs: SurfaceKind (Opaque, Glass, Foliage)]
    │   │   └── sidecar.rs                      [← bvh_sidecar.rs: TBVH v2 parser & emitter]
    │   ├── terrain_los/
    │   │   ├── mod.rs
    │   │   ├── sampler.rs                      [← map-engine-core/dem/sample.rs: sample_segment]
    │   │   ├── viewshed.rs                     [← dem/sample.rs: compute_viewshed radial march]
    │   │   └── scheduler.rs                    [← dem/sample.rs: ViewshedJob incremental worker]
    │   └── world_los/
    │       ├── mod.rs
    │       ├── trace.rs                        [← world/occluder/trace.rs: WorldOccluder, evaluate_los]
    │       ├── tlas.rs                         [← occluder/tlas.rs: per-chunk AABB TLAS]
    │       ├── dda.rs                          [← occluder/dda.rs: Amanatides-Woo 2D chunk grid march]
    │       └── descriptor.rs                   [← occluder/descriptor.rs: PrefabDescriptor, BlasManifest]
    │
    ├── terrain/                                <-- DEM, Hillshading, Contours & Satellite
    │   ├── README.md
    │   ├── dem/
    │   │   ├── mod.rs
    │   │   ├── manifest.rs                     [← map-engine-core/dem/sample.rs: DemManifest]
    │   │   ├── sample.rs                       [← dem/sample.rs: bilinear_sample, elevation_meters]
    │   │   ├── raw.rs                          [← dem/raw.rs: TBDE binary parser & sink]
    │   │   ├── grid.rs                         [← dem/downsample.rs: DemVectorGrid pyramid]
    │   │   ├── png.rs                          [← dem/png_decode.rs: 16-bit PNG decoder]
    │   │   └── loader.rs                       [← world_assets/dem_load.rs: fetch & decode coordinator]
    │   ├── relief/
    │   │   ├── mod.rs
    │   │   ├── hillshade.rs                    [← dem/hillshade.rs: slope relief normal map generator]
    │   │   ├── contours.rs                     [← geometry/contours.rs: marching squares, summit rings]
    │   │   └── sea_band.rs                     [← geometry/sea_band.rs: hypsometric depth bands]
    │   ├── satellite/
    │   │   ├── mod.rs
    │   │   ├── quadtree.rs                     [← world_assets/satellite.rs: tile LOD coordinator]
    │   │   └── streamer.rs                     [← world_assets/tbd_sat.rs: TBDS v2 tile streaming]
    │   ├── roads/
    │   │   ├── mod.rs
    │   │   ├── styling.rs                      [← geometry/polyline_strip.rs: road styles & casings]
    │   │   └── mesh.rs                         [← geometry/vector_compose.rs: compose_roads_mesh]
    │   └── water/
    │       ├── mod.rs
    │       ├── loader.rs                       [← world_assets/water.rs: TBDB bathymetry loader]
    │       └── mesh.rs                         [← geometry/vector_compose.rs: compose_sea_mesh]
    │
    ├── architecture/                           <-- CAD Section Cutter & Multi-Floor LOS
    │   ├── README.md
    │   ├── blueprint/
    │   │   ├── mod.rs
    │   │   ├── model.rs                        [← building_blueprint.rs: BuildingBlueprint, BuildingLevel]
    │   │   ├── footprint.rs                    [← building_blueprint.rs: OverallFootprint, RoofGrid]
    │   │   └── attribution.rs                  [← building_blueprint.rs: LosHitKind, structural hits]
    │   ├── compound/
    │   │   ├── mod.rs
    │   │   ├── scene.rs                        [← building_compound.rs: CompoundBuilding assembly]
    │   │   ├── instances.rs                    [← building_compound.rs: Instance, LocalTransform]
    │   │   └── doors.rs                        [← building_compound.rs: DoorState hinge kinematics]
    │   ├── los/
    │   │   ├── mod.rs
    │   │   ├── walker.rs                       [← building_compound_los.rs: material ray walk]
    │   │   ├── materials.rs                    [← building_compound_los.rs: glass/foliage attenuation]
    │   │   └── wash.rs                         [← building_viewshed.rs: LevelWash multi-floor rasters]
    │   └── section/
    │       ├── mod.rs
    │       ├── cutter.rs                       [← building_section.rs: CAD 2D section cuts Seg2]
    │       ├── heightfield.rs                  [← building_section.rs: sparse HeightField tiles]
    │       └── index.rs                        [← building_section_index.rs: 1D Y-BVH acceleration]
    │
    ├── streaming/                              <-- 512m Chunk Residency & Asset Ingestion
    │   ├── README.md
    │   ├── scheduler/
    │   │   ├── mod.rs
    │   │   ├── residency.rs                    [← world/residency.rs: WorldResidency scheduler]
    │   │   └── budget.rs                       [← world/residency.rs: 4.0ms frame budget enforcer]
    │   ├── loaders/
    │   │   ├── mod.rs
    │   │   ├── world_loader.rs                 [← world_assets/world_host.rs: 512m chunk downloader]
    │   │   └── occluder_loader.rs              [← world_assets/occluder_host.rs: BLAS BVH downloader]
    │   ├── buffers/
    │   │   ├── mod.rs
    │   │   ├── packer.rs                       [← world/residency.rs: building & glyph buffer pack]
    │   │   └── revision.rs                     [← world/residency.rs: buffer revision tracking]
    │   ├── memory/
    │   │   ├── mod.rs
    │   │   ├── budget.rs                       [← world_assets/memory_budget.rs: wasm memory ledger]
    │   │   └── stats.rs                        [← memory_budget.rs: HUD suffix formatting]
    │   └── bridge/
    │       ├── mod.rs
    │       └── bridge.rs                       [← world_assets/bridge.rs: viewport sync bridge]
    │
    ├── formats/                                <-- Container Headers & rkyv Tier-2 Archives
    │   ├── README.md
    │   ├── containers/
    │   │   ├── mod.rs
    │   │   ├── header.rs                       [← world/binary/chunk_container.rs: ContainerHeader]
    │   │   ├── tbdc.rs                         [← chunk_container.rs: TbdcHeader placed objects]
    │   │   ├── tbde.rs                         [← chunk_container.rs: TbdeHeader DEM heightfield]
    │   │   ├── tbdb.rs                         [← chunk_container.rs: TbdbHeader bathymetry]
    │   │   └── tbds.rs                         [← chunk_container.rs: TbdsHeader satellite]
    │   ├── pod/
    │   │   ├── mod.rs
    │   │   └── instance.rs                     [← world/binary/pod.rs: ObjectInstancePod (32-byte)]
    │   ├── archives/
    │   │   ├── mod.rs
    │   │   ├── roads.rs                        [← world/binary/archives.rs: RoadNetworkArchive]
    │   │   ├── labels.rs                       [← archives.rs: MapLabelsArchive, TownLabel]
    │   │   ├── water.rs                        [← archives.rs: WaterVectorsArchive]
    │   │   ├── prefabs.rs                      [← archives.rs: PrefabCatalogArchive]
    │   │   ├── forest.rs                       [← archives.rs: ForestRegionsArchive]
    │   │   └── blueprints.rs                   [← archives.rs: BuildingBlueprintArchive, BlasEntry]
    │   └── density/
    │       ├── mod.rs
    │       └── tbdd.rs                         [← geometry/tbdd.rs: tree density grid codec]
    │
    ├── environment/                            <-- Placed World Features & Labels
    │   ├── README.md
    │   ├── buildings/
    │   │   ├── mod.rs
    │   │   ├── footprint.rs                    [← world/residency.rs: building footprint polygons]
    │   │   └── buffers.rs                      [← engine.rs: upload_world_buildings, outlines]
    │   ├── vegetation/
    │   │   ├── mod.rs
    │   │   ├── canopy.rs                       [← world/density_ladder.rs: tree density ladder]
    │   │   ├── mass.rs                         [← geometry/forest_mass.rs: canopy mass polygons]
    │   │   ├── density.rs                      [← geometry/density_island.rs: island stitching]
    │   │   └── loader.rs                       [← world_assets/forest_mass.rs: density uploader]
    │   └── locations/
    │       ├── mod.rs
    │       ├── towns.rs                        [← world/locations.rs: settlement label placement]
    │       ├── toponymy.rs                     [← world/locations.rs: spot elevation labels]
    │       ├── routes.rs                       [← world/road_labels.rs: road polyline labels]
    │       └── loader.rs                       [← world_assets/labels.rs: label asset downloader]
    │
    ├── symbology/                              <-- Tactical Symbology & Slot Atlas
    │   ├── README.md
    │   ├── nato/
    │   │   ├── mod.rs
    │   │   ├── roles.rs                        [← slots_gpu.rs: UnitRoleClass (5 roles)]
    │   │   ├── vehicles.rs                     [← slots_gpu.rs: VehicleKind (3 kinds)]
    │   │   ├── glyphs.rs                       [← slots_gpu.rs: procedural chevron/cross painters]
    │   │   └── palette.rs                      [← slots_gpu.rs: BLUFOR/OPFOR/INDFOR RGBA tints]
    │   ├── atlas/
    │   │   ├── mod.rs
    │   │   ├── slot_atlas.rs                   [← slots_gpu.rs: build_slot_atlas (128x64)]
    │   │   └── marker_atlas.rs                 [← scene.rs: build_marker_slot_atlas]
    │   ├── instances/
    │   │   ├── mod.rs
    │   │   ├── slots.rs                        [← slots_gpu.rs: pack_slot_instances (20-byte)]
    │   │   ├── patches.rs                      [← slots_gpu.rs: selected_row_patch (12-byte)]
    │   │   └── drag.rs                         [← slots_gpu.rs: pack_drag_overlay, vehicle drag]
    │   ├── links/
    │   │   ├── mod.rs
    │   │   ├── squad_links.rs                  [← squad_links.rs: leader-to-member line links]
    │   │   └── drag_preview.rs                 [← squad_links.rs: pack_squad_link_drag_preview]
    │   └── labels/
    │       ├── mod.rs
    │       ├── declutter.rs                    [← label.rs: importance-distance decluttering]
    │       └── glyph_math.rs                   [← world/glyph_math.rs: size-in-meters min-pixel]
    │
    ├── doll/                                   <-- 3D Character Mannequin Preview Engine
    │   ├── README.md
    │   ├── scene/
    │   │   ├── mod.rs
    │   │   ├── instances.rs                    [← doll/mod.rs: 14 equipment regions schematic]
    │   │   ├── regions.rs                      [← doll/mod.rs: region keys and state constants]
    │   │   └── geometry.rs                     [← doll/mod.rs: mesh_cube, mesh_cylinder]
    │   ├── renderer/
    │   │   ├── mod.rs
    │   │   ├── pipeline.rs                     [← doll3d.rs: create_doll_pipeline, doll.wgsl]
    │   │   ├── engine.rs                       [← doll3d.rs: DollEngine struct, render(), resize()]
    │   │   └── pack.rs                         [← doll_pack.rs: 80-byte instance pack]
    │   └── interaction/
    │       ├── mod.rs
    │       ├── camera.rs                       [← doll3d.rs: orbit camera yaw rotation]
    │       ├── pick.rs                         [← doll/mod.rs: ray_unit_box hit picking]
    │       └── callouts.rs                     [← doll/mod.rs: anchor_px screen projection]
    │
    ├── shaders/                                <-- WebGPU WGSL Shaders
    │   ├── shader.wgsl                         [← map-engine-render/shader.wgsl: 15 entry points]
    │   └── doll.wgsl                           [← map-engine-render/doll.wgsl: 2 entry points]
    │
    └── diagnostics/                            <-- Offscreen Benches & Hardware Probes
        ├── README.md
        ├── probes/
        │   ├── mod.rs
        │   ├── runner.rs                       [← probe.rs: run_self_check readback harness]
        │   ├── calibration.rs                  [← probe.rs: self_check() calibration probe]
        │   └── texture.rs                      [← engine.rs:5556: texture_self_check()]
        └── bench/
            ├── mod.rs
            └── frame_bench.rs                  [← engine.rs:1986: render_bench() frame-cost bench]
```

---

### Pillar 2: `apps/website/mission-core/` (Headless Domain Brain)

Houses the pure mission compiler and the interactive CRDT document store under `apps/website/`.

```text
apps/website/mission-core/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    │
    ├── slot_line.rs                            [← map-engine-core/slot_line.rs: ORBAT slot text formatter]
    │
    ├── mission/                                <-- Mission Compiler & AST (Zero wgpu, Zero yrs)
    │   ├── README.md
    │   ├── mod.rs
    │   ├── ast/
    │   │   ├── mod.rs
    │   │   ├── scenario.rs                     [← mission/compile.rs: scenario root metadata]
    │   │   ├── factions.rs                     [← mission/orbat.rs: factions, sides, squads]
    │   │   ├── slots.rs                        [← mission/compile.rs: unit slots and loadouts]
    │   │   └── vehicles.rs                     [← mission/compile.rs: placed vehicles and seats]
    │   ├── extensions/
    │   │   ├── mod.rs
    │   │   ├── authored.rs                     [← mission/extensions.rs: AuthoredBlocks parser]
    │   │   ├── radio.rs                        [← mission/radio_plan.rs: radio frequencies]
    │   │   ├── objectives.rs                   [← mission/tasks.rs + win_conditions.rs]
    │   │   ├── environment.rs                  [← mission/weather.rs + audio.rs]
    │   │   ├── modules.rs                      [← mission/spawn_modules.rs]
    │   │   └── tactical_graphics.rs            [← mission/tactical_graphics.rs: phase lines]
    │   ├── compiler/
    │   │   ├── mod.rs
    │   │   ├── compiler.rs                     [← mission/compile.rs: compile_payload exporter]
    │   │   ├── flatten.rs                      [← mission/flatten.rs: flatten_to_mod_document]
    │   │   └── kit.rs                          [← mission/kit.rs: KitAliases table]
    │   └── validation/
    │       ├── mod.rs
    │       ├── validator.rs                    [← mission/validate.rs: validate_editor_payload]
    │       ├── rules.rs                        [← mission/validate.rs: default_registry rules]
    │       └── wire_safety.rs                  [← mission/wire_safety.rs: scan_editor_payload]
    │
    └── doc/                                    <-- Yrs CRDT Document Store (feature = "doc")
        ├── README.md
        ├── mod.rs
        ├── store/
        │   ├── mod.rs
        │   ├── doc_core.rs                     [← doc/store.rs: MissionDocCore struct & Yrs map]
        │   ├── hydrate.rs                      [← doc/store.rs: JSON payload hydration into CRDT]
        │   ├── export.rs                       [← doc/store.rs: CRDT serialization to JSON payload]
        │   └── queries.rs                      [← doc/store.rs: entity lookups, slot counting]
        ├── crdt/
        │   ├── mod.rs
        │   ├── id_arrays.rs                    [← doc/id_arrays.rs: ordered ID lists in CRDT]
        │   ├── soa.rs                          [← doc/soa.rs: SlotSoa snapshot generator]
        │   └── undo.rs                         [← doc/undo_groups.rs: transaction batching]
        ├── picking/
        │   ├── mod.rs
        │   ├── slot_picker.rs                  [← doc/store.rs: pick_slot at world (wx, wy)]
        │   ├── vehicle_picker.rs               [← doc/store.rs: pick_vehicle at world (wx, wy)]
        │   └── marquee.rs                      [← doc/store.rs: marquee_slot_ids in world bounds]
        └── operations/                         <-- Headless Entity State Mutations
            ├── README.md
            ├── mod.rs                          [← editor/state/operations.rs]
            ├── entity_ops.rs                   [← editor/state/operations/entity.rs: add/delete unit]
            ├── squad_ops.rs                    [← editor/state/operations/reassign.rs: move squad]
            ├── faction_ops.rs                  [← doc/apply_faction.rs: faction hydration]
            ├── orbat_ops.rs                    [← doc/place_orbat.rs: stamp squad into world]
            ├── cargo_ops.rs                    [← editor/state/operations/cargo.rs: cargo items]
            ├── compositions.rs                 [← editor/state/operations/compositions.rs: prefabs]
            ├── graphics_ops.rs                 [← editor/state/operations/tactical_graphics.rs]
            ├── attrs_ops.rs                    [← editor/state/operations/attrs.rs: stance, rank]
            └── transform_ops.rs                [← editor/state/operations/transform.rs: translation]
```

---

### Pillar 3: `apps/website/frontend/src/v2/apps/editor/` (The UI Conductor)

Refactored from the legacy `apps/website/frontend/src/editor/` into clean, modular UI components under 500 LOC each:

```text
apps/website/frontend/src/v2/apps/editor/
├── README.md
├── mod.rs                                      [Component export: <ScenarioEditor />]
├── layout.rs                                   [Master UI dock frame: left, center, right, bottom]
│
├── canvas/                                     <-- Viewport Mount & Canvas Gestures
│   ├── README.md
│   ├── mount.rs                                [Mounts HTML <canvas>, initializes graphics_engine]
│   ├── gestures.rs                             [Pointer events: middle-click pan, mouse-wheel zoom]
│   ├── hotkeys.rs                              [Keyboard shortcuts: Ctrl+Z, Delete, Space, Tab]
│   └── overlays.rs                             [Selection marquee box & measurement HUDs]
│
├── panels/                                     <-- Docked Panels & Inspectors
│   ├── README.md
│   ├── top_strip/
│   │   ├── mod.rs
│   │   ├── menu_bar.rs                         [File, Edit, View, Help dropdown menus]
│   │   ├── mission_actions.rs                  [Save, Export, Validate buttons, save status]
│   │   └── history_actions.rs                  [Undo, Redo buttons with disabled state signals]
│   ├── toolbelt/
│   │   ├── mod.rs
│   │   ├── tools.rs                            [Select, Move, Rotate, Place, Measure, LOS buttons]
│   │   └── grid_snap.rs                        [Snap mode selector: 1m, 5m, free, angle snap]
│   ├── dock_left/
│   │   ├── mod.rs
│   │   ├── outliner_tree.rs                    [Squad/unit hierarchy tree with drag-and-drop]
│   │   ├── asset_drawer.rs                     [Prefab asset search drawer]
│   │   └── compositions.rs                     [Saved composition template stamps]
│   ├── dock_right/
│   │   ├── mod.rs
│   │   ├── unit_inspector.rs                   [Loadout gear, role dropdown, skill, rank]
│   │   ├── vehicle_inspector.rs                [Vehicle crew seat matrix, cargo slots]
│   │   ├── faction_inspector.rs                [Faction briefing text, radio frequency editor]
│   │   └── marker_inspector.rs                 [Tactical graphic type, line color, label text]
│   └── bottom_bar/
│   │   ├── mod.rs
│   │   ├── readout.rs                          [Cursor world coords, elevation ASL, MGRS grid]
│   │   └── telemetry.rs                        [Render frame time HUD rf <ms>, memory ledger]
│
├── modals/                                     <-- Dialog Windows
│   ├── README.md
│   ├── validation.rs                           [Interactive findings list with "Jump to Entity"]
│   ├── settings.rs                             [Scenario settings: title, time of day, weather]
│   ├── attributes.rs                           [Custom YAML attributes editor]
│   └── arsenal.rs                              [Embeds <DollCanvas /> for 3D loadout customization]
│
├── tools/                                      <-- Interactive Tool State Machines
│   ├── README.md
│   ├── select_tool.rs                          [Entity selection, box drag, multi-select]
│   ├── place_tool.rs                           [Prefab stamping & coordinate unprojection]
│   ├── ruler_tool.rs                           [Two-point distance, bearing, slope calculation]
│   └── los_tool.rs                             [Observer pin placement & elevation profile card]
│
└── state/                                      <-- Client Application State
    ├── README.md
    ├── selection.rs                            [RwSignal for selected entity IDs, active faction]
    ├── tools.rs                                [RwSignal for current active tool mode]
    ├── session.rs                              [Session persistence & auto-save timer]
    └── tab_lock.rs                             [Multi-tab collision detection via localStorage]
```

---

## 3. Authoritative Cargo.toml Specifications

### 3.1. `apps/website/mission-core/Cargo.toml`
```toml
[package]
name = "website-mission-core"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "UNLICENSED"
publish = false
description = "Pure Rust headless domain compute for TBD missions: AST, ORBAT compiler, validation, and Yrs CRDT document model."

[features]
default = ["compiler"]
# Headless compiler: AST, ORBAT hierarchy flattening, kit aliases, wire safety, game export
compiler = ["dep:serde", "dep:serde_json", "dep:thiserror"]
# Interactive CRDT document model (used exclusively by the frontend editor)
doc = ["compiler", "dep:yrs"]

[dependencies]
serde = { version = "1", features = ["derive"], optional = true }
serde_json = { version = "1", features = ["preserve_order"], optional = true }
thiserror = { version = "2", optional = true }
yrs = { version = "0.27", optional = true }

[dev-dependencies]
serde_json = { version = "1", features = ["float_roundtrip", "preserve_order"] }
```

### 3.2. `apps/website/graphics-engine/Cargo.toml`
```toml
[package]
name = "website-graphics-engine"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "UNLICENSED"
publish = false
description = "Unified WebGPU graphics and spatial computation engine: 2D map, 3D doll, terrain, BVH raycasting, and residency."

[lib]
crate-type = ["rlib"]

[features]
default = ["bvh", "terrain", "formats", "symbology"]
bvh = []                                    # 3D BVH raycaster & collision sidecars
terrain = ["bvh", "dep:earcutr"]             # DEM, hillshade, contours, sea band, roads
formats = ["dep:rkyv", "dep:bytemuck"]      # TBDC, TBDE, TBDB, TBDS binary container headers
streaming = ["formats", "dep:flate2"]       # WorldResidency 512m chunk scheduler & LRU cache
symbology = ["dep:bytemuck"]                # MIL-STD-2525 NATO icons, slot atlas

[dependencies]
bytemuck = { version = "1", features = ["derive", "min_const_generics"], optional = true }
earcutr = { version = "0.5", optional = true }
flate2 = { version = "1", optional = true }
rkyv = { version = "0.8.18", optional = true, default-features = false, features = ["std", "bytecheck", "little_endian"] }
png = { version = "0.17", optional = true }
serde = { version = "1", features = ["derive"], optional = true }
serde_json = { version = "1", features = ["float_roundtrip", "preserve_order"], optional = true }
thiserror = { version = "2", optional = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wgpu = { version = "29", default-features = false, features = ["webgpu", "webgl", "wgsl"] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
    "HtmlCanvasElement", "Window", "ImageBitmap", "Performance",
    "Headers", "Request", "RequestInit", "Response"
] }
console_error_panic_hook = "0.1"

[dev-dependencies]
serde_json = { version = "1", features = ["float_roundtrip", "preserve_order"] }
```

### 3.3. Modifications to Existing Workspace Consumers

#### `apps/website/api_v2/Cargo.toml`
```toml
# Replace:
# map-engine-core = { path = "../../../crates/map-engine-core", features = ["mission"] }
# With:
website-mission-core = { path = "../mission-core" }
```

#### `apps/website/frontend/Cargo.toml`
```toml
# In [dependencies]:
website-mission-core = { path = "../mission-core", features = ["compiler", "doc"] }

# In [target.'cfg(target_arch = "wasm32")'.dependencies]:
# Replace map-engine-render and map-engine-core with:
website-graphics-engine = { path = "../graphics-engine", features = ["streaming", "terrain", "formats", "symbology"] }

# In [dev-dependencies]:
website-graphics-engine = { path = "../graphics-engine", features = ["streaming", "terrain", "formats", "symbology"] }
```

#### `xtask/Cargo.toml` & `tools/tbd-tools/Cargo.toml`
```toml
# Replace map-engine-core with:
website-graphics-engine = { path = "../apps/website/graphics-engine", features = ["bvh", "terrain", "formats"] }
```

#### Root `Cargo.toml`
```toml
members = [
    "apps/ticketboard",
    "apps/website/api_v2",
    "apps/website/frontend",
    "apps/website/graphics-engine",  # NEW
    "apps/website/mission-core",     # NEW
    # "crates/map-engine-core",      # REMOVED
    # "crates/map-engine-render",    # REMOVED
    "crates/tbd-gate",
    "crates/tbd-tickets",
    "xtask",
    "tools/tbd-tools",
]
```

---

## 4. Critical Traps & String Pin Invariants for Fable 5.1

> [!CAUTION]
> **Compile-Time `include_str!` Traps:**
> 1. `draw_order_tests.rs:420`:
>    ```rust
>    // Current:
>    const HIST: &str = include_str!("../../../apps/website/frontend/src/editor/state/history.rs");
>    // Must be updated to point to the relocated history or doc rebind file:
>    const HIST: &str = include_str!("../../../apps/website/mission-core/src/doc/store/doc_core.rs");
>    ```
> 2. `pages/debug/building_interior_tests.rs:176`:
>    ```rust
>    // Current:
>    const SRC: &str = include_str!("../../../../../../crates/map-engine-render/src/draw_order.rs");
>    // Must be updated to point to the new graphics-engine draw order file:
>    const SRC: &str = include_str!("../../../../../graphics-engine/src/core/pipeline/draw_order.rs");
>    ```
> 3. In `graphics-engine/src/core/culling/oracle.rs`:
>    Ensure relative paths for `include_str!("../../shaders/shader.wgsl")` and `include_str!("compute.rs")` are updated cleanly.

> [!IMPORTANT]
> **Hard Gating Rules:**
> - **Zero Test Breakage:** Native tests must report **at least 1,450 passed; 0 failed; 0 ignored** at every git commit checkpoint (`MIGRATION.md`).
> - **Size Limits:** Maximum **500 lines for production files**, maximum **1,000 lines for test files**. No allowlist additions permitted.
> - **No Inline Tests:** Test blocks must be in out-of-line test files (`tests/` directory or `#[path = "tests/<file>.rs"]`).
> - **Decoupled Picking Coordinates:** When migrating `doc/store.rs`, refactor `pick_slot` and `pick_vehicle` to take pure world coordinates `(wx: f64, wy: f64)`. Never import `OrthoCamera` inside `mission-core`.

---

## 5. Turn-Key Execution Checklist for Fable 5.1

### Phase 1: Carve Out `website-mission-core`
- [ ] Initialize `apps/website/mission-core/Cargo.toml` and `src/lib.rs`.
- [ ] Move `crates/map-engine-core/src/mission/` -> `apps/website/mission-core/src/mission/`.
- [ ] Move `crates/map-engine-core/src/slot_line.rs` -> `apps/website/mission-core/src/slot_line.rs`.
- [ ] Add compatibility re-export shim in `crates/map-engine-core/src/lib.rs`:
  ```rust
  #[cfg(feature = "mission")]
  pub use website_mission_core::mission;
  pub use website_mission_core::slot_line;
  ```
- [ ] Update `apps/website/api_v2/Cargo.toml` to depend on `website-mission-core`.
- [ ] **Verification Checkpoint:** Run `cargo check --workspace` and `cargo test -p website-api`. Confirm all tests pass.

### Phase 2: Relocate CRDT Store (`doc/`)
- [ ] Decouple picking functions in `crates/map-engine-core/src/doc/store.rs` to take `(wx: f64, wy: f64)` instead of `&OrthoCamera`.
- [ ] Move `crates/map-engine-core/src/doc/` -> `apps/website/mission-core/src/doc/` behind `#[cfg(feature = "doc")]`.
- [ ] Move `apps/website/frontend/src/editor/state/operations/` -> `apps/website/mission-core/src/doc/operations/`.
- [ ] Add `doc` feature to `website-mission-core/Cargo.toml` enabling `yrs`.
- [ ] Add compatibility re-export in `map-engine-core/src/lib.rs`:
  ```rust
  #[cfg(feature = "doc")]
  pub use website_mission_core::doc;
  ```
- [ ] Update `apps/website/frontend/Cargo.toml` to link `website-mission-core` with `features = ["compiler", "doc"]`.
- [ ] **Verification Checkpoint:** Run `cargo test -p website-frontend --bin website-frontend`. Confirm **1,450 passed**.

### Phase 3: Unify Graphics Engine (`website-graphics-engine`)
- [ ] Initialize `apps/website/graphics-engine/Cargo.toml` and directory skeleton.
- [ ] Transfer render modules from `crates/map-engine-render/src/` into `graphics-engine/src/core/`, `renderers/`, `shaders/`, and `doll/`.
- [ ] Transfer spatial/terrain modules from `crates/map-engine-core/src/`:
  - `camera/` -> `graphics-engine/src/camera/`
  - `dem/` -> `graphics-engine/src/terrain/dem/`
  - `geometry/` -> `graphics-engine/src/terrain/relief/` & `roads/`
  - `spatial/` & `bvh` -> `graphics-engine/src/spatial/`
  - `building_*` -> `graphics-engine/src/architecture/`
  - `world/` -> `graphics-engine/src/streaming/`, `formats/`, `environment/`
  - `symbology/` -> `graphics-engine/src/symbology/`
- [ ] Absorb `apps/website/frontend/src/editor/world_assets/` into `graphics-engine/src/streaming/` and `src/terrain/`.
- [ ] Update compile-time string pins in `draw_order_tests.rs` and `building_interior_tests.rs`.
- [ ] Update Cargo dependencies in `apps/website/frontend`, `xtask`, and `tools/tbd-tools`.
- [ ] Remove `crates/map-engine-core` and `crates/map-engine-render` from repo root and update root `Cargo.toml`.
- [ ] **Verification Checkpoint:** Run `cargo check --workspace`, `cargo xtask db test-it`, and `cargo test --workspace --all-features`.

### Phase 4: Frontend V2 Integration & Cleanup
- [ ] Create high-level mounts `<MapCanvas />` and `<DollCanvas />` in `apps/website/frontend/src/v2/graphics_engine/`.
- [ ] Refactor `apps/website/frontend/src/v2/apps/editor/` to consume `website-mission-core` and `<MapCanvas />`.
- [ ] Delete legacy monolith directory `apps/website/frontend/src/editor/`.
- [ ] **Verification Checkpoint:** Run `cargo xtask db up && cargo xtask ci ci-local`, test dev-login, and verify `window.__selfChecks.calibration()` passes byte-exact in browser.
