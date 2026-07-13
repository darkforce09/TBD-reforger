//! wasm-bindgen shim over `map-engine-core`. Thin: every export forwards to a pure core function
//! and returns typed-array-friendly types (`Vec<f32>` → `Float32Array`, `&[u16]` ← `Uint16Array`).
//! Grids/geometry cross as an opaque `DemGrid` handle + result structs whose getters clone the
//! backing `Vec`s into JS typed arrays.

use map_engine_core::camera::OrthoCamera;
use map_engine_core::dem::{DemVectorGrid, downsample, hillshade, png_decode, sample};
use map_engine_core::doc::{MissionDocCore, SlotSoa};
use map_engine_core::geometry::{
    contours, forest_mass, sea_band, tbdd,
    vector_compose::{
        self, CONTOUR_RGBA, LandcoverInput, RoadInput, compose_contour_hairlines,
        compose_forest_mesh, compose_landcover_mesh, compose_roads_mesh, compose_sea_mesh,
    },
};
use map_engine_core::spatial::cluster;
use map_engine_core::spatial::point_index::PointIndex;
use map_engine_core::world::build_airfield_apron_mesh;
use map_engine_core::world::{
    TerrainSizeM, WorldError, WorldResidency as CoreWorldResidency,
    WorldSpatialIndex as CoreWorldSpatialIndex, WorldStore as CoreWorldStore,
    chunk_ids_for_viewport as core_chunk_ids_for_viewport, class_visible as core_class_visible,
    contour_interval_for_zoom as core_contour_interval_for_zoom,
};
use wasm_bindgen::prelude::*;

/// LOD gate: `class_visible(class, deckZoom)` — Class R vs `lodGates.ts` (T-151.5 L6).
#[wasm_bindgen]
#[must_use]
pub fn class_visible(cls: &str, deck_zoom: f64) -> bool {
    core_class_visible(cls, deck_zoom)
}

/// Exhaustive LOD scan helper: returns JSON array of `{cls,z,v}` for vitest parity.
#[wasm_bindgen]
#[must_use]
pub fn class_visible_scan_json() -> String {
    use map_engine_core::world::WORLD_RENDER_CLASSES;
    let mut out = String::from("[");
    let mut first = true;
    for cls in WORLD_RENDER_CLASSES {
        for i in 0..=120 {
            let z = -6.0 + f64::from(i) * 0.1;
            let z = (z * 10.0).round() / 10.0;
            let v = core_class_visible(cls, z);
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&format!(
                "{{\"cls\":\"{cls}\",\"z\":{z},\"v\":{}}}",
                if v { "true" } else { "false" }
            ));
        }
    }
    out.push(']');
    out
}

// The wgpu render engine (T-151.0 L1). Re-exporting the `#[wasm_bindgen]` `RenderEngine` from this
// crate makes wasm-bindgen emit its bindings into the single bundler pkg, so `RenderEngine` and
// `MissionDoc` share one wasm linear memory (the zero-copy doc→GPU precondition, program D1).
// wasm32-only: `map-engine-render`'s GPU/web stack does not compile natively, so a plain
// `cargo build --workspace` links only the crate's native-safe parts.
#[cfg(target_arch = "wasm32")]
pub use map_engine_render::RenderEngine;

// ---------------------------------------------------------------------------------------------
// T-151.7.3 — pure slot GPU helpers (FE smoke / parity; SoT is map-engine-core::slots_gpu)
// ---------------------------------------------------------------------------------------------

/// Cluster mode gate: `slot_len > 500 && zoom ≤ −4` (pure; engine also exposes `cluster_mode`).
#[wasm_bindgen(js_name = slot_cluster_mode)]
#[must_use]
pub fn slot_cluster_mode(slot_len: u32, deck_zoom: f64) -> bool {
    map_engine_core::slots_gpu::cluster_mode(slot_len, deck_zoom)
}

/// Meters per CSS pixel at deck zoom (`2^(-zoom)`).
#[wasm_bindgen]
#[must_use]
pub fn px_to_m_at_zoom(deck_zoom: f64) -> f32 {
    map_engine_core::slots_gpu::px_to_m_at_zoom(deck_zoom)
}

/// Pack slot rings from interleaved xy + selected flags (test / parity only).
#[wasm_bindgen]
#[must_use]
pub fn pack_slot_instances(xy: &[f32], selected: Vec<u8>) -> Vec<u8> {
    let sel: Vec<bool> = selected.iter().map(|&b| b != 0).collect();
    map_engine_core::slots_gpu::pack_slot_instances(xy, &sel)
}

/// Drag phase: 0=idle 1=start 2=delta 3=restart 4=end.
#[wasm_bindgen]
#[must_use]
pub fn classify_drag_transition(
    had: bool,
    has: bool,
    ids_changed: bool,
    delta_changed: bool,
) -> u8 {
    use map_engine_core::slots_gpu::DragGpuPhase;
    match map_engine_core::slots_gpu::classify_drag_transition(had, has, ids_changed, delta_changed)
    {
        DragGpuPhase::Idle => 0,
        DragGpuPhase::Start => 1,
        DragGpuPhase::Delta => 2,
        DragGpuPhase::Restart => 3,
        DragGpuPhase::End => 4,
    }
}

// ---------------------------------------------------------------------------------------------
// dem::sample
// ---------------------------------------------------------------------------------------------

/// `Uint16Array` DEM raster → `Float32Array` meters. Byte-identical to `buildMetersCache`.
#[wasm_bindgen]
#[must_use]
pub fn meters_cache(raster: &[u16], min_m: f64, max_m: f64) -> Vec<f32> {
    sample::meters_cache(raster, min_m, max_m)
}

/// `uint16ToMeters` (`sampleElevation.ts:9`).
#[wasm_bindgen]
#[must_use]
pub fn uint16_to_meters(u16v: f64, min_m: f64, max_m: f64) -> f64 {
    sample::uint16_to_meters(u16v, min_m, max_m)
}

/// `bilinearSample` on an f32 (meters) raster (`sampleElevation.ts:39`).
#[wasm_bindgen]
#[must_use]
pub fn bilinear_sample_f32(raster: &[f32], width: u32, height: u32, px: f64, py: f64) -> f64 {
    sample::bilinear_sample(raster, width as usize, height as usize, px, py)
}

/// `bilinearSample` on a uint16 raster (the anchor path).
#[wasm_bindgen]
#[must_use]
pub fn bilinear_sample_u16(raster: &[u16], width: u32, height: u32, px: f64, py: f64) -> f64 {
    sample::bilinear_sample(raster, width as usize, height as usize, px, py)
}

/// `worldToPixel` (`sampleElevation.ts:17`) → `[u, v, px, py]`.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn world_to_pixel(
    x: f64,
    z: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    width_px: u32,
    height_px: u32,
    flip_x: bool,
    flip_z: bool,
) -> Vec<f64> {
    let m = manifest(
        min_x, min_y, max_x, max_y, width_px, height_px, flip_x, flip_z, 0.0, 0.0,
    );
    let pc = sample::world_to_pixel(x, z, &m);
    vec![pc.u, pc.v, pc.px, pc.py]
}

/// `sampleElevationMeters` on the uint16 grid (`sampleElevation.ts:67`). Returns `NaN` on
/// out-of-bounds (the TS throws; `DemController.sampleElevation` clamps first).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn sample_elevation_meters_u16(
    x: f64,
    z: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    width_px: u32,
    height_px: u32,
    flip_x: bool,
    flip_z: bool,
    height_min_m: f64,
    height_max_m: f64,
    raster: &[u16],
) -> f64 {
    let m = manifest(
        min_x,
        min_y,
        max_x,
        max_y,
        width_px,
        height_px,
        flip_x,
        flip_z,
        height_min_m,
        height_max_m,
    );
    sample::sample_elevation_meters(x, z, &m, raster, width_px as usize, height_px as usize)
        .unwrap_or(f64::NAN)
}

#[allow(clippy::too_many_arguments)]
fn manifest(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    width_px: u32,
    height_px: u32,
    flip_x: bool,
    flip_z: bool,
    height_min_m: f64,
    height_max_m: f64,
) -> sample::DemManifest {
    sample::DemManifest {
        min_x,
        min_y,
        max_x,
        max_y,
        width_px: width_px as usize,
        height_px: height_px as usize,
        flip_x,
        flip_z,
        height_min_m,
        height_max_m,
    }
}

// ---------------------------------------------------------------------------------------------
// dem::downsample + geometry over the grid (opaque DemGrid handle)
// ---------------------------------------------------------------------------------------------

/// An opaque DEM vector grid living in wasm memory. Built by `downsample`; queried by `sea_band`
/// / `contours`; halved by `reduce` (the coarse-interval contour pyramid).
#[wasm_bindgen]
pub struct DemGrid {
    inner: DemVectorGrid,
}

#[wasm_bindgen]
impl DemGrid {
    /// Box-average a meters raster into the ~1600² vector grid. `downsampleDemGrid`.
    #[must_use]
    pub fn downsample(
        meters: &[f32],
        width: u32,
        height: u32,
        factor: u32,
        world_width_m: f64,
        world_height_m: f64,
    ) -> DemGrid {
        DemGrid {
            inner: downsample::downsample_dem_grid(
                meters,
                width as usize,
                height as usize,
                factor as usize,
                world_width_m,
                world_height_m,
            ),
        }
    }

    /// Reconstruct a grid from parts (tests / a grid shipped from JS). `origin` defaults to 0.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn from_parts(
        data: Vec<f32>,
        cols: u32,
        rows: u32,
        cell_x: f64,
        cell_y: f64,
        origin_x: f64,
        origin_y: f64,
        max_elev_m: f64,
    ) -> DemGrid {
        DemGrid {
            inner: DemVectorGrid {
                data,
                cols: cols as usize,
                rows: rows as usize,
                cell_x,
                cell_y,
                origin_x,
                origin_y,
                max_elev_m,
            },
        }
    }

    /// 2× reduction for the coarse contour pyramid. `reduceGrid2x`.
    #[must_use]
    pub fn reduce(&self) -> DemGrid {
        DemGrid {
            inner: downsample::reduce_grid_2x(&self.inner),
        }
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn cols(&self) -> u32 {
        self.inner.cols as u32
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn rows(&self) -> u32 {
        self.inner.rows as u32
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn cell_x(&self) -> f64 {
        self.inner.cell_x
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn cell_y(&self) -> f64 {
        self.inner.cell_y
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn max_elev_m(&self) -> f64 {
        self.inner.max_elev_m
    }
    /// A copy of the grid's meters data.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn data(&self) -> Vec<f32> {
        self.inner.data.clone()
    }

    /// Sea-band fill geometry. `buildSeaBandGeometry`.
    #[must_use]
    pub fn sea_band(&self) -> SeaBandResult {
        SeaBandResult {
            inner: sea_band::build_sea_band_geometry(&self.inner),
        }
    }

    /// Contour isoline segments `[x0,y0,x1,y1]…`. `contourSegments`.
    #[must_use]
    pub fn contours(&self, levels: &[f64]) -> Vec<f32> {
        contours::contour_segments(&self.inner, levels)
    }

    /// T-152.5 DEM-flat airfield apron mesh for `bbox` `[minX,minY,maxX,maxY]`.
    #[must_use]
    pub fn compose_airfield_apron(&self, bbox: &[f64]) -> PolyMeshResult {
        let b = if bbox.len() == 4 {
            [bbox[0], bbox[1], bbox[2], bbox[3]]
        } else {
            [0.0, 0.0, 0.0, 0.0]
        };
        PolyMeshResult {
            inner: build_airfield_apron_mesh(&self.inner, b),
        }
    }

    /// Apron qualifying cell area m² (G2 gate oracle).
    #[must_use]
    pub fn apron_qualifying_area_m2(&self, bbox: &[f64]) -> f64 {
        use map_engine_core::world::apron_qualifying_area_m2;
        let b = if bbox.len() == 4 {
            [bbox[0], bbox[1], bbox[2], bbox[3]]
        } else {
            return 0.0;
        };
        apron_qualifying_area_m2(&self.inner, b)
    }
}

/// Positive contour levels for an interval up to a max elevation. `contourLevels`.
#[wasm_bindgen]
#[must_use]
pub fn contour_levels(interval_m: f64, max_elev_m: f64) -> Vec<f64> {
    contours::contour_levels(interval_m, max_elev_m)
}

/// Grid 2×-reduction count for a contour interval. `contourGridReductions`.
#[wasm_bindgen]
#[must_use]
pub fn contour_grid_reductions(interval_m: f64) -> u32 {
    contours::contour_grid_reductions(interval_m) as u32
}

/// Sea-band geometry result (deck `SolidPolygonLayer` binary form).
#[wasm_bindgen]
pub struct SeaBandResult {
    inner: sea_band::SeaBandGeometry,
}

#[wasm_bindgen]
impl SeaBandResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_positions(&self) -> Vec<f32> {
        self.inner.fill_positions.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_start_indices(&self) -> Vec<u32> {
        self.inner.fill_start_indices.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_colors(&self) -> Vec<u8> {
        self.inner.fill_colors.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn polygon_count(&self) -> u32 {
        self.inner.polygon_count
    }

    /// Triangulate + pack for wgpu polygon upload (T-151.4). `layer_alpha` = seaFillAlpha.
    #[must_use]
    pub fn compose_mesh(&self, layer_alpha: f64) -> PolyMeshResult {
        let m = compose_sea_mesh(&self.inner, layer_alpha);
        PolyMeshResult { inner: m }
    }
}

/// Packed polygon mesh for `RenderEngine.upload_polygon_mesh` (T-151.4).
#[wasm_bindgen]
pub struct PolyMeshResult {
    inner: vector_compose::PolyMeshGpu,
}

#[wasm_bindgen]
impl PolyMeshResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn positions(&self) -> Vec<f32> {
        self.inner.positions.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn colors(&self) -> Vec<f32> {
        self.inner.colors.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn indices(&self) -> Vec<u32> {
        self.inner.indices.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn polygon_count(&self) -> u32 {
        self.inner.polygon_count
    }
}

/// Packed hairline verts for `RenderEngine.upload_hairline_segments`.
#[wasm_bindgen]
pub struct HairlineResult {
    inner: vector_compose::HairlineGpu,
}

#[wasm_bindgen]
impl HairlineResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn verts(&self) -> Vec<f32> {
        self.inner.verts.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn segment_count(&self) -> u32 {
        self.inner.segment_count
    }
}

/// Compose contour segments → hairline verts (T-151.4).
#[wasm_bindgen]
#[must_use]
pub fn compose_contours_hairline(segments: &[f32]) -> HairlineResult {
    HairlineResult {
        inner: compose_contour_hairlines(segments, CONTOUR_RGBA),
    }
}

// ---------------------------------------------------------------------------------------------
// geometry::tbdd
// ---------------------------------------------------------------------------------------------

/// Decoded TBDD density grid.
#[wasm_bindgen]
pub struct TbddResult {
    inner: tbdd::TbddGrid,
}

#[wasm_bindgen]
impl TbddResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn version(&self) -> u32 {
        u32::from(self.inner.version)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn cell_m(&self) -> u32 {
        u32::from(self.inner.cell_m)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn cols(&self) -> u32 {
        u32::from(self.inner.cols)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn rows(&self) -> u32 {
        u32::from(self.inner.rows)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn channel_count(&self) -> u32 {
        self.inner.channels.len() as u32
    }
    /// A copy of channel `idx` corner counts (empty if out of range).
    #[must_use]
    pub fn channel(&self, idx: u32) -> Vec<u16> {
        self.inner
            .channels
            .get(idx as usize)
            .cloned()
            .unwrap_or_default()
    }
}

/// Decode one TBDD buffer. Throws (JS) on bad magic / truncation, mirroring `decodeTBDD`.
///
/// # Errors
/// Returns a JS error string on a short buffer, bad magic, or truncated channel block.
#[wasm_bindgen]
pub fn decode_tbdd(bytes: &[u8]) -> Result<TbddResult, JsError> {
    tbdd::decode_tbdd(bytes)
        .map(|inner| TbddResult { inner })
        .map_err(|e| JsError::new(&e.to_string()))
}

// ---------------------------------------------------------------------------------------------
// geometry::forest_mass
// ---------------------------------------------------------------------------------------------

/// Forest mass geometry result.
#[wasm_bindgen]
pub struct ForestMassResult {
    inner: forest_mass::ForestMassGeometry,
}

#[wasm_bindgen]
impl ForestMassResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_positions(&self) -> Vec<f32> {
        self.inner.fill_positions.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_start_indices(&self) -> Vec<u32> {
        self.inner.fill_start_indices.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn outline_segments(&self) -> Vec<f32> {
        self.inner.outline_segments.clone()
    }

    /// Triangulate fill + pack outline for wgpu (T-151.4). `fill_alpha` = forestFillAlpha.
    #[must_use]
    pub fn compose(&self, fill_alpha: f64) -> ForestComposeResult {
        let (fill, outline) = compose_forest_mesh(&self.inner, fill_alpha);
        ForestComposeResult { fill, outline }
    }
}

/// Forest mass compose: fill mesh + outline hairlines.
#[wasm_bindgen]
pub struct ForestComposeResult {
    fill: vector_compose::PolyMeshGpu,
    outline: vector_compose::HairlineGpu,
}

#[wasm_bindgen]
impl ForestComposeResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_positions(&self) -> Vec<f32> {
        self.fill.positions.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_colors(&self) -> Vec<f32> {
        self.fill.colors.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_indices(&self) -> Vec<u32> {
        self.fill.indices.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn fill_polygon_count(&self) -> u32 {
        self.fill.polygon_count
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn outline_verts(&self) -> Vec<f32> {
        self.outline.verts.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn outline_segment_count(&self) -> u32 {
        self.outline.segment_count
    }
}

/// Production iso threshold (`forest_mass::DENSITY_ISO` — Rust is source of truth).
/// wgpu must call this (or pass it into `forest_mass`) instead of a TS constant (T-151.5.1).
#[wasm_bindgen]
#[must_use]
pub fn density_iso() -> f64 {
    forest_mass::DENSITY_ISO
}

/// Per-cell marching squares over a TBDD corner grid. `forestMassFromCorners`.
/// Pass `density_iso()` for production; explicit `iso` remains for Class R / tuning tests.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn forest_mass(
    corners: &[u16],
    cols: u32,
    rows: u32,
    origin_x: f64,
    origin_y: f64,
    cell_m: f64,
    iso: f64,
) -> ForestMassResult {
    ForestMassResult {
        inner: forest_mass::forest_mass_from_corners(
            corners,
            cols as usize,
            rows as usize,
            origin_x,
            origin_y,
            cell_m,
            iso,
        ),
    }
}

// ---------------------------------------------------------------------------------------------
// dem::hillshade
// ---------------------------------------------------------------------------------------------

/// Row-flipped RGBA hillshade image.
#[wasm_bindgen]
pub struct HillshadeResult {
    inner: hillshade::Hillshade,
}

#[wasm_bindgen]
impl HillshadeResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn width(&self) -> u32 {
        self.inner.w as u32
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn height(&self) -> u32 {
        self.inner.h as u32
    }
    /// A copy of the RGBA bytes (the JS wraps these in `ImageData(width, height)`).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn data(&self) -> Vec<u8> {
        self.inner.data.clone()
    }
}

/// Horn hillshade over a meters raster. `buildHillshadeImage`.
#[wasm_bindgen]
#[must_use]
pub fn hillshade(meters: &[f32], src_w: u32, src_h: u32) -> HillshadeResult {
    HillshadeResult {
        inner: hillshade::build_hillshade_image(meters, src_w as usize, src_h as usize),
    }
}

// ---------------------------------------------------------------------------------------------
// dem::png_decode
// ---------------------------------------------------------------------------------------------

/// Decoded DEM: the meters cache + raster dims.
#[wasm_bindgen]
pub struct DecodedDem {
    inner: png_decode::DecodedDem,
}

#[wasm_bindgen]
impl DecodedDem {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn width(&self) -> u32 {
        self.inner.width
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn height(&self) -> u32 {
        self.inner.height
    }
    /// A copy of the f32 meters cache (the JS stores it as its `Float32Array` DEM cache).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn meters(&self) -> Vec<f32> {
        self.inner.meters.clone()
    }
}

/// Decode a 16-bit grayscale DEM PNG straight to the meters cache. Replaces the pngjs decode
/// (`DemTexture.ts`). Throws (JS) on a malformed / non-16-bit-grayscale PNG.
///
/// # Errors
/// Returns a JS error on decode failure or an unexpected pixel format.
#[wasm_bindgen]
pub fn dem_decode_png_to_meters(
    bytes: &[u8],
    min_m: f64,
    max_m: f64,
) -> Result<DecodedDem, JsError> {
    png_decode::decode_png_to_meters(bytes, min_m, max_m)
        .map(|inner| DecodedDem { inner })
        .map_err(|e| JsError::new(&e.to_string()))
}

// ---------------------------------------------------------------------------------------------
// mission compiler (shared with the Axum backend)
// ---------------------------------------------------------------------------------------------

/// Flatten the editor payload → canonical mod mission document JSON (mirrors the server's
/// `GET /missions/:id/compiled`). `meta_json` = camelCase `MissionMeta`; `payload_json` = the
/// stored `MissionPayload` the client already built. Same Rust code the backend runs.
///
/// # Errors
/// Returns a JS error on parse failure or a compile error (e.g. no placed slots).
#[wasm_bindgen]
pub fn flatten_mod_document(meta_json: &[u8], payload_json: &[u8]) -> Result<Vec<u8>, JsError> {
    map_engine_core::mission::flatten::flatten_mod_document_json(meta_json, payload_json)
        .map_err(|e| JsError::new(&e))
}

// ---------------------------------------------------------------------------------------------
// spatial index (Phase 3 spike — the Rust replacement for the JS rbush)
// ---------------------------------------------------------------------------------------------

/// A grid point index over a slot SoA (parallel `x`/`y` columns; row index = handle). Queries
/// return the same result set as the JS rbush (`slotSpatialIndex`/`worldSpatialIndex`).
#[wasm_bindgen]
pub struct SlotIndex {
    inner: PointIndex,
}

#[wasm_bindgen]
impl SlotIndex {
    /// Build the index over parallel `x`/`y` columns. `cell` = grid cell size (world units).
    #[must_use]
    pub fn build(xs: &[f32], ys: &[f32], cell: f64) -> SlotIndex {
        SlotIndex {
            inner: PointIndex::build(xs.to_vec(), ys.to_vec(), cell),
        }
    }

    /// Handles inside the inclusive bbox (rbush `pickRect`).
    #[must_use]
    pub fn pick_rect(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<u32> {
        self.inner.pick_rect(min_x, min_y, max_x, max_y)
    }

    /// Nearest handle within a circular radius, or `-1` (rbush `pickNearest`).
    #[must_use]
    pub fn pick_nearest(&self, x: f64, y: f64, radius: f64) -> i32 {
        self.inner
            .pick_nearest(x, y, radius)
            .map_or(-1, |i| i as i32)
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn size(&self) -> u32 {
        self.inner.len() as u32
    }
}

// ---------------------------------------------------------------------------------------------
// document core (Phase 3.0 spike — the yrs replacement for the JS Yjs Y.Doc)
// ---------------------------------------------------------------------------------------------

/// A `yrs`-backed mission document with a cached slot SoA. `apply_update` absorbs Yjs-wire (v1)
/// byte-streams; `encode_state` emits the persistence stream; `refresh` re-materializes the SoA
/// cache that the column getters + the zero-copy `*_ptr`/`slot_len` view read. Class S: parity with
/// the JS `Y.Doc` is result-set equality, not CRDT-byte identity.
#[wasm_bindgen]
pub struct MissionDoc {
    inner: MissionDocCore,
    /// Materialized on `refresh`; the column getters + pointer views read this (so a `Float32Array`
    /// view onto `slot_xs_ptr()` stays valid until the next `refresh`/mutation grows memory).
    soa: SlotSoa,
}

/// T-151.7.3 — borrow-only MissionDoc → engine SoA bind (engine never owns the doc).
///
/// TS owns `WasmMissionDoc` lifetime; this only `refresh`es and copies ids/xy into the engine
/// bridge cache for the current selection/drag/cluster policy.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn bind_mission_doc(engine: &mut RenderEngine, doc: &mut MissionDoc) {
    doc.refresh();
    engine.slots_bind_soa(doc.soa.ids.clone(), &doc.soa.xy);
}

#[wasm_bindgen]
impl MissionDoc {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> MissionDoc {
        MissionDoc {
            inner: MissionDocCore::new(),
            soa: SlotSoa::default(),
        }
    }

    /// Apply a Yjs-wire (v1) update — the bytes `Y.encodeStateAsUpdate(doc)` emits.
    ///
    /// # Errors
    /// Returns a JS error on a malformed update or integration failure.
    pub fn apply_update(&self, bytes: &[u8]) -> Result<(), JsError> {
        self.inner.apply_update(bytes).map_err(|e| JsError::new(&e))
    }

    /// Encode the document as a Yjs-wire (v1) update stream (the persistence blob).
    #[must_use]
    pub fn encode_state(&self) -> Vec<u8> {
        self.inner.encode_state()
    }

    /// The 8 small root maps + `meta` as one JSON object (MapSnapshot minus slotsById) — lets the
    /// non-render readers (compile/Outliner/Attributes) source the full model from the shadow (3.2.2).
    #[must_use]
    pub fn small_maps_json(&self) -> String {
        self.inner.small_maps_json()
    }

    /// The slots map as JSON (`slotsById`) — full, exact-f64 `Slot`s for non-render readers (3.2.3).
    #[must_use]
    pub fn slots_json(&self) -> String {
        self.inner.slots_json()
    }

    /// Re-materialize the cached slot SoA. Call after `apply_update` / a mutation before reading the
    /// column getters or building a zero-copy view — this is the point where memory may grow and any
    /// prior `Float32Array` view onto `slot_xs_ptr()` is invalidated.
    pub fn refresh(&mut self) {
        self.soa = self.inner.materialize();
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn slot_len(&self) -> u32 {
        self.soa.len() as u32
    }

    // Column getters (copy the cache into JS typed arrays — the parity path).
    #[must_use]
    pub fn slot_ids(&self) -> Vec<String> {
        self.soa.ids.clone()
    }
    #[must_use]
    pub fn slot_xs(&self) -> Vec<f32> {
        self.soa.xs.clone()
    }
    #[must_use]
    pub fn slot_ys(&self) -> Vec<f32> {
        self.soa.ys.clone()
    }
    #[must_use]
    pub fn slot_zs(&self) -> Vec<f32> {
        self.soa.zs.clone()
    }
    #[must_use]
    pub fn slot_rotations(&self) -> Vec<f32> {
        self.soa.rotations.clone()
    }
    #[must_use]
    pub fn slot_stance(&self) -> Vec<u8> {
        self.soa.stance.clone()
    }
    #[must_use]
    pub fn slot_role_idx(&self) -> Vec<u32> {
        self.soa.role_idx.clone()
    }
    #[must_use]
    pub fn slot_tag_idx(&self) -> Vec<u32> {
        self.soa.tag_idx.clone()
    }
    #[must_use]
    pub fn slot_squad_idx(&self) -> Vec<u32> {
        self.soa.squad_idx.clone()
    }
    #[must_use]
    pub fn slot_layer_idx(&self) -> Vec<u32> {
        self.soa.layer_idx.clone()
    }
    #[must_use]
    pub fn roles(&self) -> Vec<String> {
        self.soa.roles.clone()
    }
    #[must_use]
    pub fn tags(&self) -> Vec<String> {
        self.soa.tags.clone()
    }
    #[must_use]
    pub fn squads(&self) -> Vec<String> {
        self.soa.squads.clone()
    }
    #[must_use]
    pub fn layers(&self) -> Vec<String> {
        self.soa.layers.clone()
    }

    // Zero-copy view (criterion 6): raw offsets into wasm linear memory. JS builds
    // `new Float32Array(wasm.memory.buffer, ptr, slot_len)` — no copy. Valid until the next
    // `refresh`/mutation (memory growth detaches the view; rebuild it after).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn slot_xs_ptr(&self) -> u32 {
        self.soa.xs.as_ptr() as usize as u32
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn slot_ys_ptr(&self) -> u32 {
        self.soa.ys.as_ptr() as usize as u32
    }
    /// Offset of the interleaved `[x0,y0,…]` column — the deck.gl `getPosition` binary attribute,
    /// read as `new Float32Array(memory.buffer, slot_xy_ptr, 2 * slot_len)`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn slot_xy_ptr(&self) -> u32 {
        self.soa.xy.as_ptr() as usize as u32
    }

    // Full-fidelity entity creation (batch 3a; ids minted JS-side). `add_slot` is also the
    // undo-script mutator (criterion 4): one yrs transaction = one undo step.
    #[allow(clippy::too_many_arguments)]
    pub fn add_slot(
        &self,
        id: &str,
        squad_id: &str,
        layer_id: &str,
        index: u32,
        role: &str,
        tag: Option<String>,
        asset_id: Option<String>,
        x: f64,
        y: f64,
        z: f64,
        rotation: f64,
    ) {
        self.inner.add_slot(
            id, squad_id, layer_id, index, role, tag, asset_id, x, y, z, rotation,
        );
    }
    pub fn add_faction(&self, id: &str, key: &str, name: &str) {
        self.inner.add_faction(id, key, name);
    }
    pub fn add_squad(&self, id: &str, faction_id: &str, name: &str, callsign: Option<String>) {
        self.inner.add_squad(id, faction_id, name, callsign);
    }

    // Bulk paste (batch 3b): JS mints the k ids + resolves each slot's target squad/layer; the
    // parallel arrays are index-aligned per slot. `""` tag/asset = omit.
    #[allow(clippy::too_many_arguments)]
    pub fn paste_slots(
        &self,
        ids: Vec<String>,
        squad_ids: Vec<String>,
        layer_ids: Vec<String>,
        src_x: Vec<f64>,
        src_y: Vec<f64>,
        src_rot: Vec<f64>,
        zs: Vec<f64>,
        roles: Vec<String>,
        tags: Vec<String>,
        asset_ids: Vec<String>,
        stances: Vec<String>,
        loadouts: Vec<String>,
        anchor_x: Option<f64>,
        anchor_y: Option<f64>,
        width: f64,
        height: f64,
    ) {
        self.inner.paste_slots(
            ids, squad_ids, layer_ids, src_x, src_y, src_rot, zs, roles, tags, asset_ids, stances,
            loadouts, anchor_x, anchor_y, width, height,
        );
    }

    // Layer removal + meta (batch 3c). JS mints reseed_id (used only when the subtree = every layer).
    pub fn remove_editor_layer(&self, id: &str, reseed_id: &str) {
        self.inner.remove_editor_layer(id, reseed_id);
    }
    pub fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }
    pub fn update_environment(&self, patch_json: &str) {
        self.inner.update_environment(patch_json);
    }
    pub fn apply_row_meta(
        &self,
        title: &str,
        terrain: &str,
        time_of_day: Option<String>,
        weather: Option<String>,
    ) {
        self.inner
            .apply_row_meta(title, terrain, time_of_day, weather);
    }
    pub fn seed_meta(&self, id: &str, title: &str) {
        self.inner.seed_meta(id, title);
    }

    // Lossless hydrate (batch 3d): load a compiled json_payload verbatim. JS transforms the lossy
    // orbat[] path → an editor-shaped payload (minting ids) before calling this; JS mints the
    // default_layer_id used only when the payload carries no layers.
    pub fn hydrate(&self, payload_json: &str, default_layer_id: &str) {
        self.inner.hydrate(payload_json, default_layer_id);
    }
    pub fn set_slot_position(&self, id: &str, x: f64, y: f64, z: f64, rotation: f64) {
        self.inner.set_slot_position(id, x, y, z, rotation);
    }
    pub fn remove_slot(&self, id: &str) {
        self.inner.remove_slot(id);
    }
    /// Bulk-seed `n` random slots in one transaction — the browser-harness generator (criterion 6).
    pub fn seed_random(&self, n: u32, w: f64, h: f64, seed: f64) {
        self.inner.seed_random(n, w, h, seed as u64);
    }

    /// Set or clear a slot's embedded Smart Forge `loadout` (T-068.10); JSON string or undefined.
    pub fn update_slot_loadout(&self, id: &str, loadout_json: Option<String>) {
        self.inner.update_slot_loadout(id, loadout_json);
    }

    // Batch-1 slot-lifecycle mutators (full-fidelity ports of ydoc.ts; ids passed in from JS).
    pub fn update_slot(
        &self,
        id: &str,
        role: Option<String>,
        tag: Option<String>,
        stance: Option<String>,
    ) {
        self.inner.update_slot(id, role, tag, stance);
    }
    #[allow(clippy::too_many_arguments)]
    pub fn update_slot_position(
        &self,
        id: &str,
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        rotation: Option<f64>,
        width: f64,
        height: f64,
    ) {
        self.inner
            .update_slot_position(id, x, y, z, rotation, width, height);
    }
    pub fn move_entities(&self, ids: Vec<String>, dx: f64, dy: f64, zs: Vec<f64>) {
        self.inner.move_entities(ids, dx, dy, zs);
    }
    pub fn remove_slots(&self, ids: Vec<String>) {
        self.inner.remove_slots(ids);
    }

    // Batch-2 editor-layer mutators.
    pub fn add_editor_layer(&self, id: &str, name: &str, parent_id: Option<String>) {
        self.inner.add_editor_layer(id, name, parent_id);
    }
    pub fn rename_editor_layer(&self, id: &str, name: &str) {
        self.inner.rename_editor_layer(id, name);
    }
    pub fn reparent_editor_layer(&self, id: &str, new_parent_id: Option<String>) {
        self.inner.reparent_editor_layer(id, new_parent_id);
    }
    pub fn move_slot_to_layer(&self, slot_id: &str, target_layer_id: &str) {
        self.inner.move_slot_to_layer(slot_id, target_layer_id);
    }

    pub fn undo(&mut self) -> bool {
        self.inner.undo()
    }
    pub fn redo(&mut self) -> bool {
        self.inner.redo()
    }
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.inner.can_undo()
    }
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.inner.can_redo()
    }

    /// Bracket boot / hydrate / default-seeding: while init-mode is on, mutations are `INIT`
    /// (untracked) so a load is not an undo step.
    pub fn set_origin_init(&self, on: bool) {
        self.inner.set_origin_init(on);
    }

    /// True if the doc holds authored content (any faction/slot/objective/vehicle/marker).
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.inner.has_content()
    }
}

impl Default for MissionDoc {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------------------------
// cluster index (Phase 3.0 spike — the Rust replacement for the JS supercluster)
// ---------------------------------------------------------------------------------------------

/// Result of a `ClusterIndex::get_clusters` query — parallel columns in world meters. `leaves[i] < 0`
/// marks a cluster bubble (`counts[i] > 1`); otherwise it is the leaf's row handle (`counts[i] == 1`).
#[wasm_bindgen]
pub struct ClusterResult {
    xs: Vec<f64>,
    ys: Vec<f64>,
    counts: Vec<u32>,
    leaves: Vec<i32>,
}

#[wasm_bindgen]
impl ClusterResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn xs(&self) -> Vec<f64> {
        self.xs.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn ys(&self) -> Vec<f64> {
        self.ys.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn counts(&self) -> Vec<u32> {
        self.counts.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn leaves(&self) -> Vec<i32> {
        self.leaves.clone()
    }
}

/// A supercluster-compatible cluster hierarchy over a slot SoA (parallel `x`/`y` world columns).
/// `get_clusters` returns the same cluster bubbles + lone leaves as the JS `supercluster`
/// (`slotClusterIndex`).
#[wasm_bindgen]
pub struct ClusterIndex {
    inner: cluster::ClusterIndex,
}

#[wasm_bindgen]
impl ClusterIndex {
    /// Build over parallel `x`/`y` world columns, given the active terrain bounds (for the
    /// linear world→lng/lat normalization supercluster is fed).
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(xs: &[f32], ys: &[f32], terrain_w: f64, terrain_h: f64) -> ClusterIndex {
        let world: Vec<(f64, f64)> = xs
            .iter()
            .zip(ys.iter())
            .map(|(&x, &y)| (f64::from(x), f64::from(y)))
            .collect();
        ClusterIndex {
            inner: cluster::ClusterIndex::build(&world, terrain_w, terrain_h),
        }
    }

    /// Clusters/leaves inside a world-meter bbox at a deck zoom (mirrors `getClusters`).
    #[must_use]
    pub fn get_clusters(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        deck_zoom: f64,
    ) -> ClusterResult {
        let markers = self
            .inner
            .get_clusters(min_x, min_y, max_x, max_y, deck_zoom);
        let mut xs = Vec::with_capacity(markers.len());
        let mut ys = Vec::with_capacity(markers.len());
        let mut counts = Vec::with_capacity(markers.len());
        let mut leaves = Vec::with_capacity(markers.len());
        for m in markers {
            xs.push(m.x);
            ys.push(m.y);
            counts.push(m.count);
            leaves.push(m.leaf as i32);
        }
        ClusterResult {
            xs,
            ys,
            counts,
            leaves,
        }
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn leaf_count(&self) -> u32 {
        self.inner.leaf_count() as u32
    }
}

// ---------------------------------------------------------------------------------------------
// camera::OrthoCamera (T-151 — parity exports for orthoCamera.parity.test.ts)
// ---------------------------------------------------------------------------------------------

/// deck.gl-parity orthographic camera (T-151). Thin shim over `camera::OrthoCamera` so the
/// vitest live-oracle suite (`features/_wasm/orthoCamera.parity.test.ts`) can compare the
/// wasm build against the in-process deck.gl viewport; the render engine consumes the same
/// core type directly in `map-engine-render`.
#[wasm_bindgen]
pub struct OrthoCameraJs {
    inner: OrthoCamera,
}

#[wasm_bindgen]
impl OrthoCameraJs {
    /// Construct from CSS-pixel dimensions + view state (unclamped, like `new OrthoCamera`).
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(width_px: f64, height_px: f64, target_x: f64, target_y: f64, zoom: f64) -> Self {
        Self {
            inner: OrthoCamera::new(width_px, height_px, target_x, target_y, zoom),
        }
    }

    /// `2^zoom` (pixels per meter) — the T2 scale-drift probe.
    #[must_use]
    pub fn scale(&self) -> f64 {
        self.inner.scale()
    }

    /// `viewport.viewMatrix` (column-major 16).
    #[must_use]
    pub fn view_matrix(&self) -> Vec<f64> {
        self.inner.view_matrix().to_vec()
    }

    /// `viewport.projectionMatrix`.
    #[must_use]
    pub fn projection_matrix(&self) -> Vec<f64> {
        self.inner.projection_matrix().to_vec()
    }

    /// `viewport.viewProjectionMatrix`.
    #[must_use]
    pub fn view_projection(&self) -> Vec<f64> {
        self.inner.view_projection().to_vec()
    }

    /// `viewport.pixelProjectionMatrix`.
    #[must_use]
    pub fn pixel_projection(&self) -> Vec<f64> {
        self.inner.pixel_projection().to_vec()
    }

    /// `viewport.pixelUnprojectionMatrix` (empty vec if singular — deck warns instead).
    #[must_use]
    pub fn pixel_unprojection(&self) -> Vec<f64> {
        self.inner
            .pixel_unprojection()
            .map(|m| m.to_vec())
            .unwrap_or_default()
    }

    /// `viewport.project([x, y, z])` → `[px, py, pz]` (top-left).
    #[must_use]
    pub fn project(&self, x: f64, y: f64, z: f64) -> Vec<f64> {
        self.inner.project([x, y, z]).to_vec()
    }

    /// `viewport.unproject([px, py])` → `[wx, wy]` (world z=0 plane).
    #[must_use]
    pub fn unproject_xy(&self, px: f64, py: f64) -> Vec<f64> {
        self.inner.unproject_xy(px, py).to_vec()
    }

    /// `viewport.getBounds()` → `[minX, minY, maxX, maxY]`.
    #[must_use]
    pub fn visible_world_rect(&self) -> Vec<f64> {
        self.inner.visible_world_rect().to_vec()
    }

    /// Drag-pan by a CSS-pixel delta (content follows cursor).
    pub fn pan(&mut self, dx_px: f64, dy_px: f64) {
        self.inner.pan(dx_px, dy_px);
    }

    /// Cursor-anchored zoom (clamped to the view-state band).
    pub fn zoom_at(&mut self, dz: f64, cursor_x_px: f64, cursor_y_px: f64) {
        self.inner.zoom_at(dz, cursor_x_px, cursor_y_px);
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn target_x(&self) -> f64 {
        self.inner.target_x()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn target_y(&self) -> f64 {
        self.inner.target_y()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.inner.zoom()
    }
}

// ---------------------------------------------------------------------------------------------
// world::WorldStore (T-151.2 W2) — object-export parser handle
// ---------------------------------------------------------------------------------------------

fn world_err(e: WorldError) -> JsError {
    JsError::new(&e.to_string())
}

/// `obbCorners` (`buildingLayer.ts:47`) — Class T. Returns the 4 corners flattened
/// `[x0,y0,x1,y1,x2,y2,x3,y3]` for the parity harness's ≤1-ULP-vs-TS check.
#[wasm_bindgen]
#[must_use]
pub fn obb_corners(x: f64, y: f64, half_x: f64, half_y: f64, rotation_deg: f64) -> Vec<f64> {
    let c = map_engine_core::world::obb_corners(x, y, half_x, half_y, rotation_deg);
    vec![
        c[0][0], c[0][1], c[1][0], c[1][1], c[2][0], c[2][1], c[3][0], c[3][1],
    ]
}

/// `extractRoadCenterline` (`roadLayer.ts:72`) — Class T. `points` is `[x0,y0,x1,y1,…]`; returns
/// `[widthM, x0,y0,x1,y1,…]`, or an empty vec when the centerline is degenerate (< 2 vertices).
#[wasm_bindgen]
#[must_use]
pub fn road_centerline(points: &[f64]) -> Vec<f64> {
    let pts: Vec<[f64; 2]> = points.chunks_exact(2).map(|c| [c[0], c[1]]).collect();
    match map_engine_core::world::extract_road_centerline(&pts) {
        Some((path, width)) => {
            let mut out = Vec::with_capacity(1 + path.len() * 2);
            out.push(width);
            for p in path {
                out.push(p[0]);
                out.push(p[1]);
            }
            out
        }
        None => Vec::new(),
    }
}

/// Wasm handle over the Rust world-object parser (T-151.2). A separate handle from `MissionDoc`
/// / `RenderEngine` but sharing the one linear memory: `parse_chunk_gz` parses one chunk into the
/// store's `last_chunk`, whose SoA columns JS reads via the copy getters (the Class R parity
/// path) or the `*_ptr`/`*_len` zero-copy views (`new Float32Array(memory.buffer, ptr, len)` — the
/// W3 render feed). `stats()` is additive and independent of `RenderEngine::stats()`.
#[wasm_bindgen]
pub struct WorldStore {
    inner: CoreWorldStore,
}

#[wasm_bindgen]
impl WorldStore {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> WorldStore {
        WorldStore {
            inner: CoreWorldStore::new(),
        }
    }

    /// Parse the terrain manifest's `objects` block (declared counts + paths).
    ///
    /// # Errors
    /// A `JsError` when the JSON is invalid or the object-export paths are absent.
    pub fn load_manifest_json(&mut self, json: &str) -> Result<(), JsError> {
        self.inner.load_manifest_json(json).map_err(world_err)
    }

    /// Load `prefabs.json.gz` → the prefab lookup + `has_oversized`. Returns the prefab count.
    ///
    /// # Errors
    /// A `JsError` on a bad gzip/JSON payload.
    pub fn load_prefabs_gz(&mut self, bytes: &[u8]) -> Result<u32, JsError> {
        self.inner
            .load_prefabs_gz(bytes)
            .map(|n| n as u32)
            .map_err(world_err)
    }

    /// Parse one `objects/chunks/{id}.json.gz` into `last_chunk`. Returns its instance count.
    ///
    /// # Errors
    /// A `JsError` on a bad gzip/JSON payload.
    pub fn parse_chunk_gz(&mut self, id: &str, bytes: &[u8]) -> Result<u32, JsError> {
        self.inner.parse_chunk_gz(id, bytes).map_err(world_err)
    }

    /// Load `roads.json.gz` (centerlined). Returns the kept segment count.
    ///
    /// # Errors
    /// A `JsError` on a bad gzip/JSON payload.
    pub fn load_roads_gz(&mut self, bytes: &[u8]) -> Result<u32, JsError> {
        self.inner
            .load_roads_gz(bytes)
            .map(|n| n as u32)
            .map_err(world_err)
    }

    /// Load `forest-regions.json.gz`. Returns the kept region count.
    ///
    /// # Errors
    /// A `JsError` on a bad gzip/JSON payload.
    pub fn load_forest_regions_gz(&mut self, bytes: &[u8]) -> Result<u32, JsError> {
        self.inner
            .load_forest_regions_gz(bytes)
            .map(|n| n as u32)
            .map_err(world_err)
    }

    // Last-chunk copy getters (clone the column into a JS typed array — the parity path).

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_count(&self) -> u32 {
        self.inner.last_chunk.as_ref().map_or(0, |c| c.count)
    }
    #[must_use]
    pub fn chunk_positions(&self) -> Vec<f32> {
        self.inner
            .last_chunk
            .as_ref()
            .map(|c| c.positions.clone())
            .unwrap_or_default()
    }
    #[must_use]
    pub fn chunk_prefab_idx(&self) -> Vec<u16> {
        self.inner
            .last_chunk
            .as_ref()
            .map(|c| c.prefab_idx.clone())
            .unwrap_or_default()
    }
    #[must_use]
    pub fn chunk_rotations(&self) -> Vec<f32> {
        self.inner
            .last_chunk
            .as_ref()
            .map(|c| c.rotations.clone())
            .unwrap_or_default()
    }
    #[must_use]
    pub fn chunk_z(&self) -> Vec<f32> {
        self.inner
            .last_chunk
            .as_ref()
            .map(|c| c.z.clone())
            .unwrap_or_default()
    }
    #[must_use]
    pub fn chunk_cls_codes(&self) -> Vec<u8> {
        self.inner
            .last_chunk
            .as_ref()
            .map(|c| c.cls_codes.clone())
            .unwrap_or_default()
    }
    /// Row indices gathered for one render-class code (empty when the class is absent). Copy
    /// getter for the Class S `rowsByClass` parity assert; a per-class ptr view is deferred to W3.
    #[must_use]
    pub fn chunk_rows_for_class(&self, code: u8) -> Vec<u32> {
        self.inner
            .last_chunk
            .as_ref()
            .and_then(|c| c.rows_by_class.get(&code).cloned())
            .unwrap_or_default()
    }

    // Zero-copy views (criterion 6): raw offsets into wasm linear memory for the last chunk's
    // columns. JS builds `new Float32Array(memory.buffer, ptr, len)` — no copy. Valid until the
    // next `parse_chunk_gz` (memory growth detaches the view; rebuild it after).

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_positions_ptr(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.positions.as_ptr() as usize as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_positions_len(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.positions.len() as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_prefab_idx_ptr(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.prefab_idx.as_ptr() as usize as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_prefab_idx_len(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.prefab_idx.len() as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_rotations_ptr(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.rotations.as_ptr() as usize as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_rotations_len(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.rotations.len() as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_z_ptr(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.z.as_ptr() as usize as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_z_len(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.z.len() as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_cls_codes_ptr(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.cls_codes.as_ptr() as usize as u32)
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunk_cls_codes_len(&self) -> u32 {
        self.inner
            .last_chunk
            .as_ref()
            .map_or(0, |c| c.cls_codes.len() as u32)
    }

    /// Aggregate world counters as a JSON string (additive — NOT `RenderEngine::stats()`).
    #[must_use]
    pub fn stats(&self) -> String {
        let prefab_count = self.inner.prefab_by_id.len();
        let instance_count_total = self.inner.instance_count_total() as u64;
        let chunk_count_loaded = self.inner.chunks_loaded;
        let road_segment_count = self.inner.roads.len();
        let forest_region_count = self.inner.regions.len();
        let has_oversized = self.inner.has_oversized;
        format!(
            "{{\"prefab_count\":{prefab_count},\"instance_count_total\":{instance_count_total},\
             \"chunk_count_loaded\":{chunk_count_loaded},\"road_segment_count\":{road_segment_count},\
             \"forest_region_count\":{forest_region_count},\"has_oversized\":{has_oversized}}}"
        )
    }

    /// Road segment count after `load_roads_gz` (T-151.4 census pin: 888).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn road_segment_count(&self) -> u32 {
        self.inner.roads.len() as u32
    }

    /// Land-cover region count after `load_forest_regions_gz` (T-151.4 census pin: 36).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn landcover_region_count(&self) -> u32 {
        self.inner.regions.len() as u32
    }

    /// Compose LOD-filtered road casing + centerline strips at `deck_zoom` (T-151.4 L5).
    /// `airfield_polish`: T-152.5 cartographic runway styling when true.
    #[must_use]
    pub fn compose_roads(&self, deck_zoom: f64, airfield_polish: bool) -> RoadComposeResult {
        let inputs: Vec<RoadInput<'_>> = self
            .inner
            .roads
            .iter()
            .map(|r| RoadInput {
                road_class: r.road_class.as_str(),
                points: r.points.as_slice(),
                width_m: r.width_m,
            })
            .collect();
        let m = compose_roads_mesh(&inputs, deck_zoom, airfield_polish);
        RoadComposeResult { inner: m }
    }

    /// Runway segment count (G1 census).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn runway_segment_count(&self) -> u32 {
        self.inner
            .roads
            .iter()
            .filter(|r| r.road_class == "runway")
            .count() as u32
    }

    /// Airfield bbox `[minX, minY, maxX, maxY]` or empty when unknown.
    #[must_use]
    pub fn airfield_bbox(&self) -> Vec<f64> {
        self.inner
            .airfield_bbox()
            .map(|b| b.to_vec())
            .unwrap_or_default()
    }

    /// Compose all land-cover regions into one polygon mesh (T-151.4 L6).
    #[must_use]
    pub fn compose_landcover(&self) -> PolyMeshResult {
        let inputs: Vec<LandcoverInput<'_>> = self
            .inner
            .regions
            .iter()
            .map(|r| LandcoverInput {
                kind: r.kind.as_str(),
                rings: r.polygon.as_slice(),
            })
            .collect();
        PolyMeshResult {
            inner: compose_landcover_mesh(&inputs),
        }
    }
}

/// Road strip compose for wgpu upload.
#[wasm_bindgen]
pub struct RoadComposeResult {
    inner: vector_compose::RoadMeshGpu,
}

#[wasm_bindgen]
impl RoadComposeResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn casing(&self) -> Vec<f32> {
        self.inner.casing.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn centerline(&self) -> Vec<f32> {
        self.inner.centerline.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn segment_count(&self) -> u32 {
        self.inner.segment_count
    }
}

impl Default for WorldStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------------------------
// world residency (T-151.3 W3 — multi-chunk LRU + building GPU-buffer composer, wgpu path only).
// Separate handle from `WorldStore`: the W2 single-chunk parity path is untouched.
// ---------------------------------------------------------------------------------------------

/// Viewport-driven chunk residency for the wgpu mount (port of `chunkStore.ts`). Coordinates are
/// emitted in WORLD meters; the render engine subtracts `scene::ANCHOR` when packing instances.
#[wasm_bindgen]
pub struct WorldResidency {
    inner: CoreWorldResidency,
}

#[wasm_bindgen]
impl WorldResidency {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> WorldResidency {
        WorldResidency {
            inner: CoreWorldResidency::new(),
        }
    }

    /// Parse the terrain manifest (`objects` block + top-level `worldBounds`).
    ///
    /// # Errors
    /// A `JsError` when the JSON is invalid or the object-export paths are absent.
    pub fn load_manifest_json(&mut self, json: &str) -> Result<(), JsError> {
        self.inner.load_manifest_json(json).map_err(world_err)
    }

    /// Load `prefabs.json.gz` → class table + u16 building lookup. Returns the prefab count.
    ///
    /// # Errors
    /// A `JsError` on a bad gzip/JSON payload.
    pub fn load_prefabs_gz(&mut self, bytes: &[u8]) -> Result<u32, JsError> {
        self.inner
            .load_prefabs_gz(bytes)
            .map(|n| n as u32)
            .map_err(world_err)
    }

    /// Load the chunk-index (`objects/chunks/manifest.json`) cell set. Returns the cell count.
    ///
    /// # Errors
    /// A `JsError` when the JSON is invalid.
    pub fn load_chunk_index_json(&mut self, json: &str) -> Result<u32, JsError> {
        self.inner
            .load_chunk_index_json(json)
            .map(|n| n as u32)
            .map_err(world_err)
    }

    /// Pin the viewport chunk set; returns the missing ids to fetch (marked in-flight).
    #[must_use]
    pub fn set_viewport(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        deck_zoom: f64,
    ) -> Vec<String> {
        self.inner
            .set_viewport(min_x, min_y, max_x, max_y, deck_zoom)
    }

    /// Parse + insert one delivered `objects/chunks/{id}.json.gz`. Returns its instance count.
    ///
    /// # Errors
    /// A `JsError` on a bad gzip/JSON payload.
    pub fn ingest_chunk_gz(&mut self, id: &str, bytes: &[u8]) -> Result<u32, JsError> {
        self.inner.ingest_chunk_gz(id, bytes).map_err(world_err)
    }

    /// Cache a requested-but-undelivered chunk as hydrated-empty (never re-requested).
    pub fn note_undelivered(&mut self, id: &str) {
        self.inner.note_undelivered(id);
    }

    /// Record the frame's apply stats, then evict + rebuild the GPU buffers.
    pub fn end_apply_frame(&mut self, elapsed_ms: f64) {
        self.inner.end_apply_frame(elapsed_ms);
    }

    /// Building fill instances (WORLD coords): 10 f32 each `[x, y, hx, hy, cos, sin, r, g, b, a]`.
    #[must_use]
    pub fn world_building_fill(&self) -> Vec<f32> {
        self.inner.world_building_fill()
    }

    /// Building outline vertices (WORLD coords): 6 f32 each `[x, y, r, g, b, a]` (`LineList`).
    #[must_use]
    pub fn world_building_outline(&self) -> Vec<f32> {
        self.inner.world_building_outline()
    }

    /// T-152.4 fence + pier strip triangle-list verts (WORLD coords): 6 f32/vert.
    #[must_use]
    pub fn world_fence_strips(&self) -> Vec<f32> {
        self.inner.world_fence_strips()
    }

    /// Register atlas icon keys in UV-table order (must match `upload_glyph_atlas` UV order).
    pub fn set_glyph_key_map(&mut self, keys: Vec<String>) {
        self.inner.set_glyph_key_map(&keys);
    }

    /// User layer toggles (trees / props / buildings-for-badges).
    pub fn set_glyph_toggles(&mut self, trees: bool, props: bool, buildings: bool) {
        self.inner.set_glyph_toggles(trees, props, buildings);
    }

    /// T-152.4 cartographic fence/railing strip toggle.
    pub fn set_fences_toggle(&mut self, fences: bool) {
        self.inner.set_fences_toggle(fences);
    }

    /// T-152.5 airfield apron / runway polish / hangar-tower icon toggle.
    pub fn set_airfield_toggle(&mut self, on: bool) {
        self.inner.set_airfield_toggle(on);
    }

    /// Set airfield bbox from loaded WorldStore roads (after `load_roads_gz`).
    pub fn set_airfield_bbox_from_store(&mut self, store: &WorldStore) {
        self.inner
            .set_airfield_bbox_from_runways(store.inner.roads.as_slice());
    }

    /// Packed tree+vegetation icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_tree_glyphs(&self) -> Vec<u8> {
        self.inner.world_tree_glyphs()
    }

    /// Packed prop+rockLarge icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_prop_glyphs(&self) -> Vec<u8> {
        self.inner.world_prop_glyphs()
    }

    /// Packed building-badge icon instances (WORLD coords, 20 B each).
    #[must_use]
    pub fn world_badge_glyphs(&self) -> Vec<u8> {
        self.inner.world_badge_glyphs()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn tree_glyph_count(&self) -> u32 {
        self.inner.tree_glyph_count()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn prop_glyph_count(&self) -> u32 {
        self.inner.prop_glyph_count()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn badge_glyph_count(&self) -> u32 {
        self.inner.badge_glyph_count()
    }

    /// T-151.8 — strict-visible draw-set size.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunks_draw(&self) -> u32 {
        self.inner.chunks_draw()
    }

    /// T-151.8 — exact-count tree heatmap rung active.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn heatmap_trees(&self) -> bool {
        self.inner.heatmap_trees_active()
    }

    /// T-151.8 — exact tree+veg count over draw_ids.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn exact_tree_count(&self) -> u32 {
        self.inner.exact_tree_count_draw()
    }

    /// T-151.8 — R32Uint density grid (LE bytes) for `upload_density_grid`.
    #[must_use]
    pub fn density_grid_r32_bytes(&self) -> Vec<u8> {
        self.inner.density_grid_r32_bytes()
    }

    /// T-151.8 — `[width, height]` of the density grid.
    #[must_use]
    pub fn density_grid_size(&self) -> Vec<u32> {
        let (w, h) = self.inner.density_grid_dims();
        vec![w, h]
    }

    /// Nearest world instance id `"{chunkId}:{row}"` within `radius_m`; `mask` = optional class
    /// bitmask over the 5 render-class codes (bit `c` set ⇒ class `c` allowed).
    #[must_use]
    pub fn pick_nearest(
        &mut self,
        x: f64,
        y: f64,
        radius_m: f64,
        mask: Option<u32>,
    ) -> Option<String> {
        self.inner.pick_nearest(x, y, radius_m, mask)
    }

    /// World instance ids inside a world-meter bbox; `mask` as in [`Self::pick_nearest`].
    #[must_use]
    pub fn pick_rect(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        mask: Option<u32>,
    ) -> Vec<String> {
        self.inner.pick_rect(min_x, min_y, max_x, max_y, mask)
    }

    /// Resident chunk ids (sorted) — parity/debug.
    #[must_use]
    pub fn resident_chunk_ids(&self) -> Vec<String> {
        self.inner.resident_chunk_ids()
    }

    /// Ordered eviction victims since construction — Class S eviction-order log.
    #[must_use]
    pub fn eviction_log(&self) -> Vec<String> {
        self.inner.eviction_log()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn pinned_building_count(&self) -> u32 {
        self.inner.pinned_building_count()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn chunks_resident(&self) -> u32 {
        self.inner.chunks_resident() as u32
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn frames_over_budget(&self) -> u32 {
        self.inner.frames_over_budget() as u32
    }

    /// In-flight (requested, not yet delivered) chunk count — T-151.4.1.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn inflight_count(&self) -> u32 {
        self.inner.inflight_count() as u32
    }

    /// Drop all in-flight marks (call on fetch abort so ids can be re-requested) — T-151.4.1.
    pub fn clear_inflight(&mut self) {
        self.inner.clear_inflight();
    }

    /// Mark chunk ids as in-flight after `clear_inflight` (active fetch) — T-151.4.1.
    pub fn mark_inflight(&mut self, ids: Vec<String>) {
        self.inner.mark_inflight(&ids);
    }

    /// Every pinned id is resident (or pin set empty) — T-151.4.1.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn pin_settled(&self) -> bool {
        self.inner.pin_settled()
    }

    /// Additive residency stats JSON (NOT `RenderEngine::stats()`).
    #[must_use]
    pub fn stats(&self) -> String {
        self.inner.stats_json()
    }
}

impl Default for WorldResidency {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------------------------
// world spatial index (T-151.3 W3 — chunk-keyed, class-filtered; the rbush `worldSpatialIndex`
// replacement for the wgpu path). Exposed standalone so the pick parity test drives it directly.
// ---------------------------------------------------------------------------------------------

/// Chunk-keyed, class-filterable world point index. `pick_*` return the same id set as the JS
/// rbush `worldSpatialIndex` (Class S). ids are `"{chunkId}:{row}"`; radii are world meters.
#[wasm_bindgen]
pub struct WorldSpatialIndex {
    inner: CoreWorldSpatialIndex,
}

#[wasm_bindgen]
impl WorldSpatialIndex {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> WorldSpatialIndex {
        WorldSpatialIndex {
            inner: CoreWorldSpatialIndex::new(),
        }
    }

    /// Bulk-insert one chunk's compacted SoA columns (idempotent). Rows with `cls == 255`
    /// (`NO_CLASS`) are skipped; each kept row's id is `"{chunk_id}:{i}"`.
    pub fn insert_chunk(&mut self, chunk_id: &str, xs: &[f32], ys: &[f32], cls: &[u8]) {
        self.inner.insert_chunk(chunk_id, xs, ys, cls);
    }

    /// Remove a chunk's instances (LRU eviction). Unknown chunk = no-op.
    pub fn remove_chunk(&mut self, chunk_id: &str) {
        self.inner.remove_chunk(chunk_id);
    }

    /// Nearest instance id within a circular `radius_m`, optional class `mask`, else `None`.
    #[must_use]
    pub fn pick_nearest(
        &mut self,
        x: f64,
        y: f64,
        radius_m: f64,
        mask: Option<u32>,
    ) -> Option<String> {
        self.inner.pick_nearest(x, y, radius_m, mask)
    }

    /// Instance ids inside a world-meter bbox, optional class `mask`.
    #[must_use]
    pub fn pick_rect(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        mask: Option<u32>,
    ) -> Vec<String> {
        self.inner.pick_rect(min_x, min_y, max_x, max_y, mask)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn size(&self) -> u32 {
        self.inner.size() as u32
    }
}

impl Default for WorldSpatialIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// The viewport → chunk-id set (T-151.3 W3, Class R) — a direct binding of
/// `map_engine_core::world::chunk_ids_for_viewport` for the `chunkMathRust.parity.test.ts`
/// cross-check against the JS `chunkIdsForViewport`. `extra_ring` = 0 skips the oversized ring.
#[wasm_bindgen]
#[must_use]
#[allow(clippy::too_many_arguments)] // a flat bbox+terrain+opts binding for the JS parity harness
pub fn world_chunk_ids_for_viewport(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    width: f64,
    height: f64,
    chunk_size_m: f64,
    extra_ring: i32,
) -> Vec<String> {
    map_engine_core::world::chunk_ids_for_viewport(
        [min_x, min_y, max_x, max_y],
        map_engine_core::world::TerrainSizeM { width, height },
        chunk_size_m,
        i64::from(extra_ring),
    )
}

// ── T-151.11.3 (audits B-01…B-05) — policy exports so no live TS twin remains ────────────────

/// Sea fill fade ladder (`sea_band.rs` SoT; replaces the live `worldmap/seaBand.ts` twin).
#[wasm_bindgen]
#[must_use]
pub fn sea_fill_alpha(deck_zoom: f64) -> f64 {
    sea_band::sea_fill_alpha(deck_zoom)
}

/// Forest fill fade ladder (`forest_mass.rs` SoT; replaces the live `worldmap/forestMass.ts` twin).
#[wasm_bindgen]
#[must_use]
pub fn forest_fill_alpha(deck_zoom: f64) -> f64 {
    forest_mass::forest_fill_alpha(deck_zoom)
}

/// Contour interval ladder (`lod_gates.rs` SoT; replaces the live `worldmap/lodGates.ts` twin).
#[wasm_bindgen]
#[must_use]
pub fn contour_interval_for_zoom(deck_zoom: f64) -> f64 {
    core_contour_interval_for_zoom(deck_zoom)
}

/// Deck-zoom → supercluster-zoom bucket (`spatial/cluster.rs` SoT; replaces the
/// `slotClusterIndex.ts` local copy used for cache invalidation).
#[wasm_bindgen]
#[must_use]
pub fn deck_zoom_to_super_zoom(deck_zoom: f64) -> i32 {
    cluster::deck_zoom_to_super_zoom(deck_zoom)
}

/// DEM vector-grid downsample factor (`dem/downsample.rs` SoT; replaces the
/// `worldmap/demGrid.ts` constant on the live path).
#[wasm_bindgen]
#[must_use]
pub fn dem_vector_grid_factor() -> u32 {
    downsample::DEM_VECTOR_GRID_FACTOR as u32
}

/// Airfield apron DEM downsample factor (T-152.5 G2 — coarser σ gate grid).
#[wasm_bindgen]
#[must_use]
pub fn dem_apron_grid_factor() -> u32 {
    downsample::APRON_DEM_DOWNSAMPLE_FACTOR as u32
}

/// Viewport → chunk-id set (512 m grid + preload margin + optional oversized ring) —
/// `chunk_math.rs` SoT; replaces the live `worldmap/chunkMath.ts` call in the forest lane.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn chunk_ids_for_viewport(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    terrain_w: f64,
    terrain_h: f64,
    chunk_size_m: f64,
    extra_ring: i32,
) -> Vec<String> {
    core_chunk_ids_for_viewport(
        [min_x, min_y, max_x, max_y],
        TerrainSizeM {
            width: terrain_w,
            height: terrain_h,
        },
        chunk_size_m,
        i64::from(extra_ring),
    )
}

#[wasm_bindgen]
impl WorldResidency {
    /// T-151.11.3 (B-04): open an ingest frame — the ≤ 4 ms/frame budget policy lives in core
    /// (`APPLY_BUDGET_MS`); this wrapper only supplies the clock.
    pub fn begin_ingest_frame(&mut self) {
        self.inner.begin_ingest_frame_at(js_sys::Date::now());
    }

    /// True once the open ingest frame has consumed the core apply budget.
    #[must_use]
    pub fn frame_budget_exhausted(&self) -> bool {
        self.inner.ingest_budget_exhausted_at(js_sys::Date::now())
    }

    /// Close the ingest frame (records stats + evicts + rebuilds via `end_apply_frame`).
    pub fn end_ingest_frame(&mut self) {
        self.inner.end_ingest_frame_at(js_sys::Date::now());
    }

    /// Building fill/outline lanes should draw (user toggle ∧ zoom gate) — T-151.11.3 / P-04.
    #[must_use]
    pub fn buildings_visible(&self) -> bool {
        self.inner.buildings_visible()
    }

    /// T-152.4 fence/pier strip lane visibility (fences toggle ∧ prop LOD gate).
    #[must_use]
    pub fn fences_visible(&self) -> bool {
        self.inner.fences_visible()
    }
}

// ---------------------------------------------------------------------------------------------
// T-152.1 — cartographic text labels (declutter in core; GPU upload later via RenderEngine)
// ---------------------------------------------------------------------------------------------

use map_engine_core::label::{LabelSpec, declutter};

/// Handle holding the current label set after declutter (T-152.1).
#[wasm_bindgen]
pub struct TextLabelStore {
    drawn: Vec<LabelSpec>,
    deck_zoom: f64,
}

#[wasm_bindgen]
impl TextLabelStore {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> TextLabelStore {
        TextLabelStore {
            drawn: Vec::new(),
            deck_zoom: 0.0,
        }
    }

    /// Replace labels. `flat` is `[id, x, y, importance, …]` interleaved with UTF-16 text
    /// lengths — simpler API: pass JSON array of `{id,x,y,importance,text}`.
    pub fn set_labels_json(&mut self, json: &str, deck_zoom: f64) {
        self.deck_zoom = deck_zoom;
        let parsed: Vec<LabelJson> = serde_json::from_str(json).unwrap_or_default();
        let specs: Vec<LabelSpec> = parsed
            .into_iter()
            .map(|j| LabelSpec {
                id: j.id,
                x: j.x,
                y: j.y,
                importance: j.importance,
                text: j.text,
            })
            .collect();
        self.drawn = declutter(&specs, deck_zoom);
    }

    /// Number of labels that survive declutter (G5: empty → 0).
    #[wasm_bindgen(js_name = text_label_count)]
    #[must_use]
    pub fn text_label_count(&self) -> u32 {
        self.drawn.len() as u32
    }

    /// Drawn set as JSON (debug / M1 inject).
    #[must_use]
    pub fn drawn_json(&self) -> String {
        let v: Vec<LabelJson> = self
            .drawn
            .iter()
            .map(|l| LabelJson {
                id: l.id,
                x: l.x,
                y: l.y,
                importance: l.importance,
                text: l.text.clone(),
            })
            .collect();
        serde_json::to_string(&v).unwrap_or_else(|_| "[]".into())
    }
}

impl Default for TextLabelStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct LabelJson {
    id: u32,
    x: i32,
    y: i32,
    importance: u16,
    text: String,
}

// ---------------------------------------------------------------------------------------------
// T-152.7 — height markers (DEM peaks + ASL labels)
// ---------------------------------------------------------------------------------------------

use map_engine_core::dem::peaks::{
    declutter_height_labels, find_peaks, height_label_min_sep_m as core_height_label_min_sep_m,
    HeightLabel, HeightLabelKind,
};

#[derive(serde::Deserialize, serde::Serialize)]
struct HeightLabelJson {
    x: f64,
    y: f64,
    value_m: i32,
    kind: String,
}

fn label_from_json(j: HeightLabelJson) -> HeightLabel {
    let kind = if j.kind == "contour" {
        HeightLabelKind::Contour
    } else {
        HeightLabelKind::Peak
    };
    HeightLabel {
        x: j.x,
        y: j.y,
        value_m: j.value_m,
        kind,
    }
}

fn label_to_json(l: &HeightLabel) -> HeightLabelJson {
    HeightLabelJson {
        x: l.x,
        y: l.y,
        value_m: l.value_m,
        kind: match l.kind {
            HeightLabelKind::Peak => "peak".into(),
            HeightLabelKind::Contour => "contour".into(),
        },
    }
}

/// Find DEM peaks on a meters cache raster (world manifest bounds).
#[wasm_bindgen]
pub fn find_peaks_from_meters(
    meters: &[f32],
    width: u32,
    height: u32,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    flip_x: bool,
    flip_z: bool,
) -> String {
    let m = sample::DemManifest {
        min_x,
        min_y,
        max_x,
        max_y,
        width_px: width as usize,
        height_px: height as usize,
        flip_x,
        flip_z,
        height_min_m: 0.0,
        height_max_m: 0.0,
    };
    let labels = find_peaks(meters, width as usize, height as usize, &m);
    serde_json::to_string(&labels.iter().map(label_to_json).collect::<Vec<_>>())
        .unwrap_or_else(|_| "[]".into())
}

/// Declutter height labels at `deck_zoom`; returns JSON array.
#[wasm_bindgen]
pub fn declutter_height_labels_json(json: &str, deck_zoom: f64) -> String {
    let parsed: Vec<HeightLabelJson> = serde_json::from_str(json).unwrap_or_default();
    let labels: Vec<HeightLabel> = parsed.into_iter().map(label_from_json).collect();
    let out = declutter_height_labels(&labels, deck_zoom);
    serde_json::to_string(&out.iter().map(label_to_json).collect::<Vec<_>>())
        .unwrap_or_else(|_| "[]".into())
}

/// Pack height labels into 20 B icon instances for `upload_text_labels`.
#[wasm_bindgen]
pub fn pack_height_label_bytes(json: &str, deck_zoom: f64) -> Vec<u8> {
    let parsed: Vec<HeightLabelJson> = serde_json::from_str(json).unwrap_or_default();
    let labels: Vec<HeightLabel> = parsed.into_iter().map(label_from_json).collect();
    let char_m = map_engine_render::text_layout::text_char_meters(deck_zoom);
    let glyphs =
        map_engine_render::text_layout::pack_height_label_glyphs(&labels, deck_zoom, char_m);
    map_engine_render::text_layout::pack_text_icon_bytes(&glyphs, deck_zoom)
}

/// G4 sep oracle (meters).
#[wasm_bindgen]
#[must_use]
pub fn height_label_min_sep_m(deck_zoom: f64) -> f64 {
    core_height_label_min_sep_m(deck_zoom)
}

/// Verify G2/G3 gates on a label JSON array; returns error JSON or `"[]"`.
#[wasm_bindgen]
pub fn verify_height_labels_json(
    json: &str,
    meters: &[f32],
    width: u32,
    height: u32,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    flip_x: bool,
    flip_z: bool,
    height_min_m: f64,
    height_max_m: f64,
) -> String {
    let parsed: Vec<HeightLabelJson> = serde_json::from_str(json).unwrap_or_default();
    let m = sample::DemManifest {
        min_x,
        min_y,
        max_x,
        max_y,
        width_px: width as usize,
        height_px: height as usize,
        flip_x,
        flip_z,
        height_min_m,
        height_max_m,
    };
    let mut errors: Vec<String> = Vec::new();
    for l in &parsed {
        let label = label_from_json(HeightLabelJson {
            x: l.x,
            y: l.y,
            value_m: l.value_m,
            kind: l.kind.clone(),
        });
        if let Some(elev) = sample::sample_elevation_from_meters_cache(
            label.x,
            label.y,
            &m,
            meters,
            width as usize,
            height as usize,
        ) {
            if elev <= 0.0 {
                errors.push(format!("G3 sea: ({},{}) elev={elev}", label.x, label.y));
            }
            if (f64::from(label.value_m) - elev).abs() > 0.5 {
                errors.push(format!(
                    "G2 ASL: ({},{}) label={} sample={elev}",
                    label.x, label.y, label.value_m
                ));
            }
        }
    }
    serde_json::to_string(&errors).unwrap_or_else(|_| "[]".into())
}

/// G-contour optional: operator waived — contour index labels not shipped in T-152.7.
#[wasm_bindgen]
#[must_use]
pub fn height_contour_labels_waived() -> bool {
    true
}

// ---------------------------------------------------------------------------------------------
// T-152.8 — town name labels (locations.json + A3 importance declutter)
// ---------------------------------------------------------------------------------------------

use map_engine_core::world::{
    declutter_town_labels, parse_locations_json, town_declutter_invariant_holds, LocationLabel,
};

/// Parse `locations.json` array; returns JSON or `"[]"` on failure.
#[wasm_bindgen]
pub fn parse_locations_json_wasm(json: &str) -> String {
    match parse_locations_json(json) {
        Ok(rows) => serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into()),
        Err(_) => "[]".into(),
    }
}

/// Declutter town labels at `deck_zoom`; returns JSON array of drawn rows.
#[wasm_bindgen]
pub fn declutter_town_labels_json(json: &str, deck_zoom: f64) -> String {
    let parsed: Vec<LocationLabel> = serde_json::from_str(json).unwrap_or_default();
    let out = declutter_town_labels(&parsed, deck_zoom);
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
}

/// Pack town labels into 20 B icon instances for `upload_town_labels`.
#[wasm_bindgen]
pub fn pack_town_label_bytes(json: &str, deck_zoom: f64) -> Vec<u8> {
    let parsed: Vec<LocationLabel> = serde_json::from_str(json).unwrap_or_default();
    map_engine_render::text_layout::pack_town_label_bytes(&parsed, deck_zoom)
}

/// G3 oracle: every drawn row satisfies the A3 predicate at `deck_zoom`.
#[wasm_bindgen]
#[must_use]
pub fn town_declutter_invariant_holds_json(json: &str, deck_zoom: f64) -> bool {
    let all: Vec<LocationLabel> = serde_json::from_str(json).unwrap_or_default();
    let drawn = declutter_town_labels(&all, deck_zoom);
    town_declutter_invariant_holds(&drawn, &all, deck_zoom)
}

/// Verify G2/G4 on locations + drawn set; returns error JSON or `"[]"`.
#[wasm_bindgen]
pub fn verify_town_labels_json(
    source_json: &str,
    drawn_json: &str,
    deck_zoom: f64,
    required_json: &str,
) -> String {
    let source: Vec<LocationLabel> = serde_json::from_str(source_json).unwrap_or_default();
    let drawn: Vec<LocationLabel> = serde_json::from_str(drawn_json).unwrap_or_default();
    let required: Vec<String> = serde_json::from_str(required_json).unwrap_or_default();
    let mut errors: Vec<String> = Vec::new();

    if !town_declutter_invariant_holds(&drawn, &source, deck_zoom) {
        errors.push("G3: declutter invariant failed".into());
    }

    let norm = |s: &str| s.to_lowercase().replace(' ', "");
    let drawn_names: Vec<String> = drawn.iter().map(|l| norm(&l.name)).collect();
    for town in &required {
        let k = norm(town);
        let ok = drawn_names.iter().any(|n| n == &k || n.contains(&k[..k.len().min(6)]));
        if !ok {
            errors.push(format!("G2: missing required town \"{town}\""));
        }
    }

    let by_id: std::collections::HashMap<_, _> =
        source.iter().map(|l| (l.id.as_str(), l.name.trim())).collect();
    for d in &drawn {
        if let Some(src_name) = by_id.get(d.id.as_str()) {
            if d.name.trim() != *src_name {
                errors.push(format!(
                    "G4: name mismatch id={} drawn=\"{}\" source=\"{}\"",
                    d.id, d.name, src_name
                ));
            }
        } else {
            errors.push(format!("G4: unknown id {}", d.id));
        }
    }

    serde_json::to_string(&errors).unwrap_or_else(|_| "[]".into())
}
