//! Stores and migrates the editor world-layer visibility preferences.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
/// Re-export `website_map_engine::streaming::bridge::preferences::WorldLayerPrefs`.
pub use website_map_engine::streaming::bridge::preferences::WorldLayerPrefs;

const EDITOR_PREFS_KEY: &str = "tbd-mc-editor-prefs";

const EDITOR_PREFS_VERSION: u32 = 1;

const LEGACY_LAYERS_KEY: &str = "tbd-mc-world-layers";

const LEGACY_BASEMAP_KEY: &str = "tbd-mc-basemap-view";

/// What the editor's layer menus need of a [`WorldLayerPrefs`]: the visibility rows to draw, in
/// display order, and a keyed setter to write one of them back.
///
/// An extension trait rather than inherent methods, because the preference struct itself belongs
/// to the map engine's streaming bridge and carries no menu vocabulary.
pub trait WorldLayerPrefsView {
    /// Visibility controls in display order, with their storage keys and captions.
    #[must_use]
    fn rows(&self) -> [(&'static str, bool, &'static str); 12];
    /// Update a known visibility key; ignore unknown keys.
    fn set(&mut self, key: &str, on: bool);
}

impl WorldLayerPrefsView for WorldLayerPrefs {
    fn rows(&self) -> [(&'static str, bool, &'static str); 12] {
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

    fn set(&mut self, key: &str, on: bool) {
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

const BASEMAP_SATELLITE: &str = "satellite";
const BASEMAP_MAP: &str = "map";

#[must_use]
fn normalize_basemap(view: &str) -> String {
    if view == BASEMAP_MAP || view == BASEMAP_SATELLITE {
        view.to_string()
    } else {
        BASEMAP_SATELLITE.to_string()
    }
}

/// Editor prefs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditorPrefs {
    /// Schema version of the persisted blob (see [`EDITOR_PREFS_VERSION`]).
    #[serde(default)]
    pub version: u32,

    /// The 12 world-layer visibility toggles.
    #[serde(default)]
    pub layers: WorldLayerPrefs,

    /// The basemap view (`"satellite"` / `"map"`).
    #[serde(default = "default_basemap")]
    pub basemap: String,
}

fn default_basemap() -> String {
    BASEMAP_SATELLITE.to_string()
}

impl Default for EditorPrefs {
    fn default() -> Self {
        Self {
            version: EDITOR_PREFS_VERSION,
            layers: WorldLayerPrefs::default(),
            basemap: default_basemap(),
        }
    }
}

impl EditorPrefs {
    #[must_use]
    fn from_json(raw: &str) -> Self {
        serde_json::from_str::<EditorPrefs>(raw).unwrap_or_default()
    }

    #[must_use]
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

fn migrate_store(mut prefs: EditorPrefs) -> EditorPrefs {
    if prefs.version < EDITOR_PREFS_VERSION {
        prefs.version = EDITOR_PREFS_VERSION;
    }

    prefs.basemap = normalize_basemap(&prefs.basemap);
    prefs
}

#[must_use]
fn migrate_from_legacy(layers_raw: Option<&str>, basemap_raw: Option<&str>) -> EditorPrefs {
    let layers = layers_raw
        .and_then(|r| serde_json::from_str::<WorldLayerPrefs>(r).ok())
        .unwrap_or_default();
    let basemap = basemap_raw.map_or_else(default_basemap, normalize_basemap);
    EditorPrefs {
        version: EDITOR_PREFS_VERSION,
        layers,
        basemap,
    }
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Load store.
#[must_use]
pub fn load_store() -> EditorPrefs {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = storage() {
            if let Ok(Some(raw)) = s.get_item(EDITOR_PREFS_KEY) {
                return migrate_store(EditorPrefs::from_json(&raw));
            }

            let layers_raw = s.get_item(LEGACY_LAYERS_KEY).ok().flatten();
            let basemap_raw = s.get_item(LEGACY_BASEMAP_KEY).ok().flatten();
            if layers_raw.is_some() || basemap_raw.is_some() {
                let migrated = migrate_from_legacy(layers_raw.as_deref(), basemap_raw.as_deref());
                let _ = s.set_item(EDITOR_PREFS_KEY, &migrated.to_json());
                return migrated;
            }
        }
    }
    EditorPrefs::default()
}

/// Save store.
pub fn save_store(prefs: &EditorPrefs) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = storage() {
            let mut out = prefs.clone();
            out.version = EDITOR_PREFS_VERSION;
            out.basemap = normalize_basemap(&out.basemap);
            let _ = s.set_item(EDITOR_PREFS_KEY, &out.to_json());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = prefs;
}

/// Load prefs.
#[must_use]
pub fn load_prefs() -> WorldLayerPrefs {
    load_store().layers
}

/// Save prefs.
pub fn save_prefs(p: &WorldLayerPrefs) {
    let mut store = load_store();
    store.layers = *p;
    save_store(&store);
}

/// Load basemap view.
#[must_use]
pub fn load_basemap_view() -> String {
    load_store().basemap
}

/// Save basemap view.
pub fn save_basemap_view(view: &str) {
    let mut store = load_store();
    store.basemap = normalize_basemap(view);
    save_store(&store);
}

#[cfg(test)]
#[path = "tests/world_layer_prefs/serialization_and_migration.rs"]
mod tests;
