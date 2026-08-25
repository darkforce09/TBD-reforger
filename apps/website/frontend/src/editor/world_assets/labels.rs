//! T-173 H5 — cartographic text-label host (town names / road names / DEM height labels). The
//! engine text lanes (`upload_town_labels` / `upload_road_labels` / `upload_text_labels`) and the
//! core placement/pack logic shipped in the React era (T-152.7–.9/.16) but were never bridged on
//! the Leptos host. This module fetches the label sources once, then packs + uploads per zoom band
//! (memoized) so the town/road/height Mission Settings toggles are live rather than inert.

use map_engine_core::dem::peaks::{find_peaks, HeightLabel};
use map_engine_core::dem::sample::DemManifest;
use map_engine_core::world::{
    build_road_label_draw_set, parse_locations_json, parse_road_names_json, RoadNamesFile,
    RoadSegment,
};
use map_engine_render::text_layout::{
    pack_height_label_glyphs, pack_road_label_bytes, pack_text_icon_bytes, pack_town_label_bytes,
    text_char_meters,
};

use crate::editor::mission_editor::boot_progress::{BootEvent, BootSeg};
use crate::editor::tools::select_tool::EngineHandle;
use crate::editor::world_layer_prefs::WorldLayerPrefs;

use super::fetch::fetch_text;

pub struct LabelHost {
    towns: Vec<map_engine_core::world::LocationLabel>,
    road_names: Option<RoadNamesFile>,
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
            road_names: None,
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
        // T-628 — two of the world segment's declared files. Both report on completion regardless of
        // outcome: `road-names.json` is not shipped for everon and 404s, and a request that came
        // back 404 is still a request that finished.
        if let Some(txt) = fetch_text(&format!("{base}/locations.json")).await {
            if let Ok(t) = parse_locations_json(&txt) {
                self.towns = t;
            }
        }
        report(BootEvent::Done(BootSeg::World, 1));
        if let Some(txt) = fetch_text(&format!("{base}/road-names.json")).await {
            if let Ok(r) = parse_road_names_json(&txt) {
                self.road_names = Some(r);
            }
        }
        report(BootEvent::Done(BootSeg::World, 1));
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
        let road_bytes = match (&self.road_names, prefs.road_names) {
            (Some(names), true) => {
                let placements = build_road_label_draw_set(names, &self.road_segments, zoom);
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
