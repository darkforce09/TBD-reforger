//! T-173 P6 — per-user world-layer visibility prefs + basemap view (React `worldLayerPrefs.ts`
//! parity: localStorage `tbd-mc-world-layers` + `tbd-mc-basemap-view`). The 12 cartographic layer
//! toggles the Mission Settings dialog exposes; the map host reads these each settle to drive the
//! residency glyph toggles + engine vector-lane visibility.
//!
//! Split rationale (React N8): render prefs that belong to the mission (hillshade / grid / basemap
//! *style* opacity) live in `meta.environment` (see `dto::MissionEnv`); which vector layers the
//! operator wants *shown* is a per-user viewing preference and lives in localStorage.

// The localStorage helpers are only reached from the wasm32 editor host/dialog; on the native test
// build (where those callers are cfg'd out) they read as dead code, so allow it module-wide.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

const LAYERS_KEY: &str = "tbd-mc-world-layers";
const BASEMAP_KEY: &str = "tbd-mc-basemap-view";

/// The 12 world-layer toggles (superset of `WorldClassToggles`). `props` defaults **off** (T-152.20
/// L2); everything else defaults on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLayerPrefs {
    pub roads: bool,
    pub buildings: bool,
    pub forest: bool,
    pub trees: bool,
    pub props: bool,
    pub contours: bool,
    pub sea: bool,
    pub fences: bool,
    pub airfield: bool,
    pub heights: bool,
    #[serde(rename = "townLabels")]
    pub town_labels: bool,
    #[serde(rename = "roadNames")]
    pub road_names: bool,
}

impl Default for WorldLayerPrefs {
    fn default() -> Self {
        Self {
            roads: true,
            buildings: true,
            forest: true,
            trees: true,
            props: false,
            contours: true,
            sea: true,
            fences: true,
            airfield: true,
            heights: true,
            town_labels: true,
            road_names: true,
        }
    }
}

impl WorldLayerPrefs {
    /// The 12 `(key, value, label)` rows in Mission Settings display order.
    #[must_use]
    pub fn rows(&self) -> [(&'static str, bool, &'static str); 12] {
        [
            ("roads", self.roads, "Roads"),
            ("buildings", self.buildings, "Buildings"),
            ("forest", self.forest, "Forest mass"),
            ("trees", self.trees, "Trees"),
            ("props", self.props, "Props"),
            ("contours", self.contours, "Contours"),
            ("sea", self.sea, "Sea"),
            ("fences", self.fences, "Fences"),
            ("airfield", self.airfield, "Airfield"),
            ("heights", self.heights, "Height labels"),
            ("townLabels", self.town_labels, "Town labels"),
            ("roadNames", self.road_names, "Road names"),
        ]
    }

    /// Flip one toggle by key. Unknown keys are ignored.
    pub fn set(&mut self, key: &str, on: bool) {
        match key {
            "roads" => self.roads = on,
            "buildings" => self.buildings = on,
            "forest" => self.forest = on,
            "trees" => self.trees = on,
            "props" => self.props = on,
            "contours" => self.contours = on,
            "sea" => self.sea = on,
            "fences" => self.fences = on,
            "airfield" => self.airfield = on,
            "heights" => self.heights = on,
            "townLabels" => self.town_labels = on,
            "roadNames" => self.road_names = on,
            _ => {}
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Load the persisted prefs (defaults when unset / on a non-wasm host).
#[must_use]
pub fn load_prefs() -> WorldLayerPrefs {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = storage() {
            if let Ok(Some(raw)) = s.get_item(LAYERS_KEY) {
                if let Ok(p) = serde_json::from_str::<WorldLayerPrefs>(&raw) {
                    return p;
                }
            }
        }
    }
    WorldLayerPrefs::default()
}

/// Persist the prefs (no-op off wasm).
pub fn save_prefs(p: &WorldLayerPrefs) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = storage() {
            if let Ok(json) = serde_json::to_string(p) {
                let _ = s.set_item(LAYERS_KEY, &json);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = p;
}

/// Basemap view: `"satellite"` (default) or `"map"` (cartographic pyramid).
#[must_use]
pub fn load_basemap_view() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = storage() {
            if let Ok(Some(v)) = s.get_item(BASEMAP_KEY) {
                if v == "map" || v == "satellite" {
                    return v;
                }
            }
        }
    }
    "satellite".to_string()
}

/// Persist the basemap view (no-op off wasm).
pub fn save_basemap_view(view: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = storage() {
            let _ = s.set_item(BASEMAP_KEY, view);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = view;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_props_off_rest_on() {
        let p = WorldLayerPrefs::default();
        assert!(!p.props);
        assert!(p.roads && p.buildings && p.forest && p.trees && p.contours && p.sea);
        assert!(p.fences && p.airfield && p.heights && p.town_labels && p.road_names);
    }

    #[test]
    fn rows_cover_all_twelve_keys() {
        let p = WorldLayerPrefs::default();
        assert_eq!(p.rows().len(), 12);
    }

    #[test]
    fn set_flips_by_key_and_ignores_unknown() {
        let mut p = WorldLayerPrefs::default();
        p.set("props", true);
        assert!(p.props);
        p.set("nonsense", false);
        assert_eq!(
            p,
            WorldLayerPrefs {
                props: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn round_trips_through_json_with_react_keys() {
        let p = WorldLayerPrefs::default();
        let j = serde_json::to_string(&p).unwrap();
        assert!(j.contains("townLabels") && j.contains("roadNames"));
        let back: WorldLayerPrefs = serde_json::from_str(&j).unwrap();
        assert_eq!(p, back);
    }
}
