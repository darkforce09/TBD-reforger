//! Role: loader.
//! Position: `environment/locations` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::locations::peaks::HeightLabel;
use crate::environment::locations::peaks::find_peaks;
use crate::environment::locations::route_labels::RoadNamesFile;
use crate::environment::locations::route_labels::build_road_label_draw_set_from_archive;
use crate::environment::locations::route_labels::parse_road_names_json;
use crate::environment::locations::route_placement::RoadLabelPlacement;
use crate::environment::locations::route_placement::build_road_label_draw_set;
use crate::environment::locations::towns::map_labels_from_bytes;
use crate::environment::locations::towns::parse_locations_json;
use crate::renderers::text::metrics::text_char_meters;
use crate::renderers::text::packing::pack_height_label_glyphs;
use crate::renderers::text::packing::pack_road_label_bytes;
use crate::renderers::text::packing::pack_text_icon_bytes;
use crate::renderers::text::packing::pack_town_label_bytes;
use crate::streaming::loaders::manifest::LabelsBlock;
use crate::streaming::loaders::manifest::parse_manifest_binary;
use crate::terrain::dem::manifest::DemManifest;
use crate::terrain::roads::network::RoadSegment;

use crate::core::context::handles::EngineHandle;
use crate::streaming::bridge::preferences::WorldLayerPrefs;
use crate::streaming::bridge::progress::BootEvent;
use crate::streaming::bridge::progress::BootSeg;

use crate::streaming::host::WORLD_LABEL_FILES;
use crate::streaming::loaders::fetch::fetch_bytes;
use crate::streaming::loaders::fetch::fetch_text;

const MAP_LABELS_ENCODING: &str = "rkyv-map-labels-v1";

enum RoadLabelSource {
    /// No curated names for this terrain (`road-names.json` 404s on everon today).
    None,

    /// `road-names.json`, joined to live segment geometry per zoom.
    Json(RoadNamesFile),

    /// `map_labels.rkyv`'s baked `road_names` lane, already in declutter order.
    Archive(Vec<RoadLabelPlacement>),
}

/// Label host.
pub struct LabelHost {
    towns: Vec<crate::symbology::labels::importance::LocationLabel>,
    road_names: RoadLabelSource,
    road_segments: Vec<RoadSegment>,
    peaks: Vec<HeightLabel>,
    ready: bool,

    last: Option<(i64, bool, bool, bool)>,
}

impl LabelHost {
    /// New.
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

    /// Towns.
    #[must_use]
    pub(crate) fn towns(&self) -> &[crate::symbology::labels::importance::LocationLabel] {
        &self.towns
    }

    /// Fetch + parse the label sources and compute DEM peaks. `road_segments` come from the world store (already loaded); `dem_meters` is the decoded 16-bit DEM raster.
    pub async fn init(
        &mut self,
        base: &str,
        dem_meters: &[f32],
        dem_w: u32,
        dem_h: u32,
        road_segments: Vec<RoadSegment>,
        report: &dyn Fn(BootEvent),
    ) {
        if !self.init_from_archive(base, report).await && Self::labels_block(base).await.is_none() {
            self.init_from_json(base, report).await;
        }
        self.road_segments = road_segments;

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

    async fn labels_block(base: &str) -> Option<LabelsBlock> {
        let raw = fetch_bytes(&format!("{base}/manifest.json")).await?;
        let json: serde_json::Value = serde_json::from_slice(&raw).ok()?;
        let block = parse_manifest_binary(&json).labels?;
        (!block.path.is_empty() && block.encoding == MAP_LABELS_ENCODING).then_some(block)
    }

    async fn init_from_json(&mut self, base: &str, report: &dyn Fn(BootEvent)) {
        if let Some(txt) = fetch_text(&format!("{base}/locations.json")).await
            && let Ok(t) = parse_locations_json(&txt)
        {
            self.towns = t;
        }
        report(BootEvent::Done(BootSeg::World, 1));
        if let Some(txt) = fetch_text(&format!("{base}/road-names.json")).await
            && let Ok(r) = parse_road_names_json(&txt)
        {
            self.road_names = RoadLabelSource::Json(r);
        }
        report(BootEvent::Done(BootSeg::World, 1));
    }

    /// Pack + upload the three label lanes for the current zoom + toggles (memoized per band). The engine ensures its ASCII text atlas on first upload; visibility follows the per-lane toggle.
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
