//! T-173 H5 — cartographic text-label host (town names / road names / DEM height labels). The
//! engine text lanes (`upload_town_labels` / `upload_road_labels` / `upload_text_labels`) and the
//! core placement/pack logic shipped in the React era (T-152.7–.9/.16) but were never bridged on
//! the Leptos host. This module fetches the label sources once, then packs + uploads per zoom band
//! (memoized) so the town/road/height Mission Settings toggles are live rather than inert.
//!
//! # T-935.7 — one archive instead of the JSON pair
//!
//! When the terrain manifest carries a `labels` block (spec §5), a single `map_labels.rkyv` fetch
//! replaces both text fetches below and both lanes come out of it validated and zero-copy. With no
//! block is absent the JSON path runs unchanged. Everon names `locations/map_labels.rkyv`.
//!
//! Three things about that split are worth stating where they can be checked:
//!
//! * **The archive is read through [`map_labels_from_bytes`], in the core.** `world_assets` is
//!   `#[cfg(target_arch = "wasm32")]` and this repo has no wasm-bindgen-test harness, so a decode
//!   written here would be gated by compilation alone. Everything that can be tested natively is.
//! * **The height lane does not change hands.** The archive carries `height_labels`, but the SPA's
//!   height labels are `find_peaks` over the DEM raster and always have been — `height-labels.json`
//!   is fetched by nothing in this crate. Swapping the lane would change what renders, and the
//!   acceptance for this slice is that it does not.
//! * **The manifest read is the price of the gate.** `bootstrap` already has the manifest parsed,
//!   but its `LabelHost::init` call site lives in `world_assets/mod.rs`, which another slice owns
//!   this wave; a parameter would be the better shape and is a one-line change once that file is
//!   free. Until then the block is read here, and only the archive branch is charged for it.

use map_engine_core::dem::peaks::{find_peaks, HeightLabel};
use map_engine_core::dem::sample::DemManifest;
use map_engine_core::world::{
    build_road_label_draw_set, build_road_label_draw_set_from_archive, map_labels_from_bytes,
    parse_locations_json, parse_manifest_binary, parse_road_names_json, LabelsBlock,
    RoadLabelPlacement, RoadNamesFile, RoadSegment,
};
use map_engine_render::text_layout::{
    pack_height_label_glyphs, pack_road_label_bytes, pack_text_icon_bytes, pack_town_label_bytes,
    text_char_meters,
};

use crate::editor::mission_editor::boot_progress::{BootEvent, BootSeg};
use crate::editor::tools::select_tool::EngineHandle;
use crate::editor::world_layer_prefs::WorldLayerPrefs;

use super::fetch::{fetch_bytes, fetch_text};
use super::WORLD_LABEL_FILES;

/// The `labels.encoding` this build reads (spec §5). A block declaring anything else is a manifest
/// written for a different reader, and the fallback for a binary block is the JSON path that still
/// works — so it is treated as absent rather than guessed at.
const MAP_LABELS_ENCODING: &str = "rkyv-map-labels-v1";

/// Where the curated road names came from, which decides how they are decluttered.
///
/// The two lanes are not interchangeable at draw time: `build_road_label_draw_set` sorts candidates
/// by `priority`/`segment_id`, and the archive's wire type carries neither — its lane is written
/// pre-sorted instead, so it must go through the variant that does **not** re-sort.
enum RoadLabelSource {
    /// No curated names for this terrain (`road-names.json` 404s on everon today).
    None,
    /// `road-names.json`, joined to live segment geometry per zoom.
    Json(RoadNamesFile),
    /// `map_labels.rkyv`'s baked `road_names` lane, already in declutter order.
    Archive(Vec<RoadLabelPlacement>),
}

pub struct LabelHost {
    towns: Vec<map_engine_core::world::LocationLabel>,
    road_names: RoadLabelSource,
    road_segments: Vec<RoadSegment>,
    peaks: Vec<HeightLabel>,
    ready: bool,
    /// Memo: (zoom band ×2 rounded, town_on, road_on, height_on) of the last pack+upload. The zoom
    /// band is load-bearing for the height lane: since T-641 the height-label declutter is a
    /// **screen-space** grid cull (Eden ~1 per 150×150 px — `peaks::declutter_height_labels`), so its
    /// kept set changes with zoom; re-keying on the band re-packs the correct on-screen density as
    /// the user zooms (and also drives the town-label fade). Panning within a band reuses the pack.
    last: Option<(i64, bool, bool, bool)>,
}

impl LabelHost {
    pub fn new() -> Self {
        Self {
            towns: Vec::new(),
            road_names: RoadLabelSource::None,
            road_segments: Vec::new(),
            peaks: Vec::new(),
            ready: false,
            last: None,
        }
    }

    /// T-762 — Places-dock read of the already-parsed town index (`locations.json` from boot).
    #[must_use]
    pub(super) fn towns(&self) -> &[map_engine_core::world::LocationLabel] {
        &self.towns
    }

    /// Fetch + parse the label sources and compute DEM peaks. `road_segments` come from the world
    /// store (already loaded); `dem_meters` is the decoded 16-bit DEM raster.
    pub async fn init(
        &mut self,
        base: &str,
        dem_meters: &[f32],
        dem_w: u32,
        dem_h: u32,
        road_segments: Vec<RoadSegment>,
        report: &dyn Fn(BootEvent),
    ) {
        if !self.init_from_archive(base, report).await {
            // JSON only when the manifest has no labels block. A named archive that 404s must
            // not silently fetch locations.json (T-935.13 runtime-fetch cutover).
            if Self::labels_block(base).await.is_none() {
                self.init_from_json(base, report).await;
            }
        }
        self.road_segments = road_segments;
        // Peaks over the full 12.8 km Everon extent (DEM raster is north-up, no axis flip).
        let manifest = DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 12_800.0,
            max_y: 12_800.0,
            width_px: dem_w as usize,
            height_px: dem_h as usize,
            flip_x: false,
            flip_z: false,
            height_min_m: -204.78,
            height_max_m: 375.53,
        };
        self.peaks = find_peaks(dem_meters, dem_w as usize, dem_h as usize, &manifest);
        self.ready = true;
    }

    /// T-935.7 — the binary path. `true` when the archive supplied both label lanes; `false` sends
    /// `init` to the JSON path, which is then reached exactly as it was before this existed.
    ///
    /// Every step falls back rather than fails: no `labels` block, an encoding this build does not
    /// read, a 404, a truncated file, an archive from a future schema. The JSON files are still on
    /// disk through wave 6, so the cost of a bad archive is a slower load, not a blank map.
    ///
    /// T-628 accounting: the archive **is** both of `WORLD_LABEL_FILES`, so it reports both units
    /// on success and nothing on any failure path. The manifest read is not one of the declared
    /// files and is not reported — the segment's `Finish` settles the difference, and a bar that
    /// under-counts by one cached request is the honest direction to be wrong in.
    async fn init_from_archive(&mut self, base: &str, report: &dyn Fn(BootEvent)) -> bool {
        let Some(block) = Self::labels_block(base).await else {
            return false;
        };
        let Some(raw) = fetch_bytes(&format!("{base}/{}", block.path)).await else {
            return false;
        };
        let Ok(labels) = map_labels_from_bytes(&raw) else {
            return false;
        };
        self.towns = labels.towns;
        self.road_names = RoadLabelSource::Archive(labels.road_names);
        report(BootEvent::Done(BootSeg::World, WORLD_LABEL_FILES));
        true
    }

    /// The terrain manifest's `labels` block, when it names an archive this build can read.
    async fn labels_block(base: &str) -> Option<LabelsBlock> {
        let raw = fetch_bytes(&format!("{base}/manifest.json")).await?;
        let json: serde_json::Value = serde_json::from_slice(&raw).ok()?;
        let block = parse_manifest_binary(&json).labels?;
        (!block.path.is_empty() && block.encoding == MAP_LABELS_ENCODING).then_some(block)
    }

    /// The pre-T-935.7 path, unchanged.
    ///
    /// T-628 — two of the world segment's declared files. Both report on completion regardless of
    /// outcome: `road-names.json` is not shipped for everon and 404s, and a request that came
    /// back 404 is still a request that finished.
    async fn init_from_json(&mut self, base: &str, report: &dyn Fn(BootEvent)) {
        if let Some(txt) = fetch_text(&format!("{base}/locations.json")).await {
            if let Ok(t) = parse_locations_json(&txt) {
                self.towns = t;
            }
        }
        report(BootEvent::Done(BootSeg::World, 1));
        if let Some(txt) = fetch_text(&format!("{base}/road-names.json")).await {
            if let Ok(r) = parse_road_names_json(&txt) {
                self.road_names = RoadLabelSource::Json(r);
            }
        }
        report(BootEvent::Done(BootSeg::World, 1));
    }

    /// Pack + upload the three label lanes for the current zoom + toggles (memoized per band). The
    /// engine ensures its ASCII text atlas on first upload; visibility follows the per-lane toggle.
    pub fn push(&mut self, engine: &EngineHandle, zoom: f64, prefs: &WorldLayerPrefs) {
        if !self.ready {
            return;
        }
        let band = (zoom * 2.0).round() as i64;
        let key = (band, prefs.town_labels, prefs.road_names, prefs.heights);
        if self.last == Some(key) {
            return;
        }
        self.last = Some(key);
        let char_m = text_char_meters(zoom);

        let town_bytes = if prefs.town_labels {
            pack_town_label_bytes(&self.towns, zoom)
        } else {
            Vec::new()
        };
        // T-935.7 — the two sources take different declutter entry points and must not be
        // swapped: the archive's lane is written pre-sorted precisely because its wire type has
        // nowhere to put the `priority` and `segment_id` the JSON sort keys off, so re-sorting it
        // would order the road names by a priority derived from the visibility class instead of
        // the one the emitter measured.
        let road_bytes = match (&self.road_names, prefs.road_names) {
            (RoadLabelSource::Json(names), true) => {
                let placements = build_road_label_draw_set(names, &self.road_segments, zoom);
                pack_road_label_bytes(&placements, zoom)
            }
            (RoadLabelSource::Archive(candidates), true) => {
                let placements = build_road_label_draw_set_from_archive(candidates, zoom);
                pack_road_label_bytes(&placements, zoom)
            }
            _ => Vec::new(),
        };
        // Height labels: the `Height labels` world toggle gates the lane; the density inside it is
        // the T-641 screen-space cull (`pack_height_label_glyphs` → `declutter_height_labels` holds
        // ~1 label per 150×150 px at this `zoom`). Dot/triangle + horizontal integer form unchanged.
        let height_bytes = if prefs.heights {
            let glyphs = pack_height_label_glyphs(&self.peaks, zoom, char_m);
            pack_text_icon_bytes(&glyphs, zoom)
        } else {
            Vec::new()
        };

        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_town_labels(&town_bytes, prefs.town_labels);
            e.upload_road_labels(&road_bytes, prefs.road_names);
            e.upload_text_labels(&height_bytes, prefs.heights);
        }
    }
}

impl Default for LabelHost {
    fn default() -> Self {
        Self::new()
    }
}
