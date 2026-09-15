//! Role: world layer prefs.
//! Position: `editor` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
/// Re-export `website_graphics_engine::streaming::bridge::preferences::WorldLayerPrefs`.
pub use website_graphics_engine::streaming::bridge::preferences::WorldLayerPrefs;

const EDITOR_PREFS_KEY: &str = "tbd-mc-editor-prefs";

const EDITOR_PREFS_VERSION: u32 = 1;

const LEGACY_LAYERS_KEY: &str = "tbd-mc-world-layers";

const LEGACY_BASEMAP_KEY: &str = "tbd-mc-basemap-view";

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

    #[test]
    fn store_round_trips_through_json() {
        let mut store = EditorPrefs::default();
        store.layers.set("props", true);
        store.layers.set("roads", false);
        store.basemap = "map".to_string();
        let json = store.to_json();
        assert!(json.contains("townLabels") && json.contains("roadNames"));
        assert!(json.contains("\"basemap\":\"map\""));
        let back = EditorPrefs::from_json(&json);
        assert_eq!(store, back);
        assert!(back.layers.props && !back.layers.roads);
        assert_eq!(back.basemap, "map");
        assert_eq!(back.version, EDITOR_PREFS_VERSION);
    }

    #[test]
    fn store_defaults_on_garbage() {
        for junk in [
            "",
            "not json at all",
            "{",
            "null",
            "[1,2,3]",
            r#"{"basemap":42}"#,
            r#"{"layers":"nope"}"#,
        ] {
            let got = EditorPrefs::from_json(junk);
            assert_eq!(got, EditorPrefs::default(), "junk {junk:?} should default");
        }
    }

    #[test]
    fn store_partial_blob_fills_defaults() {
        let got = EditorPrefs::from_json("{}");
        assert_eq!(got.layers, WorldLayerPrefs::default());
        assert_eq!(got.basemap, "satellite");
        assert_eq!(got.version, 0);
    }

    #[test]
    fn migration_preserves_old_values() {
        let mut old = WorldLayerPrefs::default();
        old.set("props", true);
        old.set("roads", false);
        let old_layers_json = serde_json::to_string(&old).unwrap();

        let migrated = migrate_from_legacy(Some(&old_layers_json), Some("map"));
        assert!(migrated.layers.props, "props ON must survive migration");
        assert!(!migrated.layers.roads, "roads OFF must survive migration");
        assert_eq!(migrated.basemap, "map", "basemap choice must survive");
        assert_eq!(migrated.version, EDITOR_PREFS_VERSION);

        assert_eq!(
            migrated.layers,
            WorldLayerPrefs {
                props: true,
                roads: false,
                ..Default::default()
            }
        );
    }

    #[test]
    fn migration_defaults_on_absent_or_garbage_legacy() {
        let none = migrate_from_legacy(None, None);
        assert_eq!(none.layers, WorldLayerPrefs::default());
        assert_eq!(none.basemap, "satellite");

        let junk = migrate_from_legacy(Some("{bad"), Some("teal"));
        assert_eq!(junk.layers, WorldLayerPrefs::default());
        assert_eq!(junk.basemap, "satellite");

        let partial = migrate_from_legacy(None, Some("map"));
        assert_eq!(partial.layers, WorldLayerPrefs::default());
        assert_eq!(partial.basemap, "map");
    }

    #[test]
    fn basemap_is_always_normalized() {
        assert_eq!(normalize_basemap("map"), "map");
        assert_eq!(normalize_basemap("satellite"), "satellite");
        assert_eq!(normalize_basemap("garbage"), "satellite");
        assert_eq!(normalize_basemap(""), "satellite");

        let fixed = migrate_store(EditorPrefs {
            basemap: "nonsense".to_string(),
            ..Default::default()
        });
        assert_eq!(fixed.basemap, "satellite");
    }

    #[test]
    fn migrate_store_stamps_version_and_is_idempotent() {
        let stamped = migrate_store(EditorPrefs {
            version: 0,
            ..Default::default()
        });
        assert_eq!(stamped.version, EDITOR_PREFS_VERSION);

        assert_eq!(migrate_store(stamped.clone()), stamped);
    }
}
