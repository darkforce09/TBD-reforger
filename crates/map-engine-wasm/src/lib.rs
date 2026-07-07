//! wasm-bindgen shim over `map-engine-core`. Thin: every export forwards to a pure core function
//! and returns typed-array-friendly types (`Vec<f32>` → `Float32Array`, `&[u16]` ← `Uint16Array`).
//! Grids/geometry cross as an opaque `DemGrid` handle + result structs whose getters clone the
//! backing `Vec`s into JS typed arrays.

use map_engine_core::dem::{DemVectorGrid, downsample, hillshade, png_decode, sample};
use map_engine_core::doc::{MissionDocCore, SlotSoa};
use map_engine_core::geometry::{contours, forest_mass, sea_band, tbdd};
use map_engine_core::spatial::cluster;
use map_engine_core::spatial::point_index::PointIndex;
use wasm_bindgen::prelude::*;

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
}

/// Per-cell marching squares over a TBDD corner grid. `forestMassFromCorners`.
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
        anchor_x: Option<f64>,
        anchor_y: Option<f64>,
        width: f64,
        height: f64,
    ) {
        self.inner.paste_slots(
            ids, squad_ids, layer_ids, src_x, src_y, src_rot, zs, roles, tags, asset_ids, stances,
            anchor_x, anchor_y, width, height,
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
