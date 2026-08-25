//! T-173 P6 — per-user world-layer visibility prefs + basemap view (React `worldLayerPrefs.ts`
//! parity: originally localStorage `tbd-mc-world-layers` + `tbd-mc-basemap-view`). The 12
//! cartographic layer toggles the Editor Preferences dialog exposes; the map host reads these each
//! settle to drive the residency glyph toggles + engine vector-lane visibility.
//!
//! Split rationale (React N8): render prefs that belong to the mission (hillshade / grid / basemap
//! *style* opacity) live in `meta.environment` (see `dto::MissionEnv`); which vector layers the
//! operator wants *shown*, and which basemap view they prefer, are per-user viewing preferences and
//! live in localStorage.
//!
//! **T-691 (3den E6) — the editor-preferences store.** The two scattered per-user keys are now
//! folded into ONE versioned store ([`EditorPrefs`]) behind a single key
//! ([`EDITOR_PREFS_KEY`]): a serde round-trip with defaults-on-parse-failure and a one-way
//! migration that reads the old `tbd-mc-*` keys when the new key is absent (so no operator's
//! toggles or basemap choice is lost on upgrade), then writes the store forward. This is the
//! storage seam later editor-preference tickets (T-688 aggregated settings, T-692 help) add keys
//! to without re-deriving localStorage handling — add a field to [`EditorPrefs`], bump
//! [`EDITOR_PREFS_VERSION`] if the shape needs a migration, done. The four historical free
//! functions ([`load_prefs`] / [`save_prefs`] / [`load_basemap_view`] / [`save_basemap_view`])
//! keep their signatures and are now thin accessors over the store, so the `world_assets` callers
//! (`world_host.rs`, `mod.rs`, `labels.rs`) are untouched.

// The localStorage helpers are only reached from the wasm32 editor host/dialog; on the native test
// build (where those callers are cfg'd out) they read as dead code, so allow it module-wide.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// T-691 — the one versioned key the editor-preferences store persists under. Supersedes the two
/// unversioned keys below, which are now read only during migration.
const EDITOR_PREFS_KEY: &str = "tbd-mc-editor-prefs";
/// T-691 — the store schema version. Bump when a field's shape changes in a way a raw serde load of
/// an older blob can't absorb (adding an `Option`/`#[serde(default)]` field does NOT need a bump);
/// [`migrate_store`] then owns the upgrade. Defaults-on-parse-failure is the floor either way.
const EDITOR_PREFS_VERSION: u32 = 1;

/// Legacy (pre-T-691) key: the bare `WorldLayerPrefs` JSON. Read once during migration when
/// [`EDITOR_PREFS_KEY`] is absent, so an operator's toggles survive the upgrade.
const LEGACY_LAYERS_KEY: &str = "tbd-mc-world-layers";
/// Legacy (pre-T-691) key: the bare basemap-view string. Read once during migration.
const LEGACY_BASEMAP_KEY: &str = "tbd-mc-basemap-view";

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
    /// The 12 `(key, value, label)` rows in Editor Preferences display order.
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

/// Basemap view: `"satellite"` (default) or `"map"` (cartographic pyramid). The store persists the
/// normalized string; [`normalize_basemap`] is the single validator both the store and the legacy
/// migration path go through, so a garbage value can never reach the map host.
const BASEMAP_SATELLITE: &str = "satellite";
const BASEMAP_MAP: &str = "map";

/// Return the input if it is a known basemap view, else the default (`"satellite"`).
#[must_use]
fn normalize_basemap(view: &str) -> String {
    if view == BASEMAP_MAP || view == BASEMAP_SATELLITE {
        view.to_string()
    } else {
        BASEMAP_SATELLITE.to_string()
    }
}

/// T-691 (3den E6) — the editor-local preferences store. One versioned blob under
/// [`EDITOR_PREFS_KEY`] holding every per-user editor preference; later tickets extend it by adding
/// fields (with `#[serde(default)]` so old blobs still load). `layers` and `basemap` are the two
/// T-173 prefs that seeded it.
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
    /// Parse a persisted blob, falling back to [`EditorPrefs::default`] on any serde failure (the
    /// defaults-on-garbage floor). Pure — no localStorage — so it is directly testable.
    #[must_use]
    fn from_json(raw: &str) -> Self {
        serde_json::from_str::<EditorPrefs>(raw).unwrap_or_default()
    }

    /// Serialize for persistence (empty string only if serde itself fails, which round-trip tests
    /// preclude for this shape).
    #[must_use]
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// T-691 — bring a freshly-loaded store up to the current version. Adding `#[serde(default)]`
/// fields needs no work here (a raw load already fills them); this exists so a *shape* change in a
/// future ticket has one obvious home, and so a blob that predates the version field (`version: 0`)
/// is stamped forward. Idempotent: a current-version store passes through unchanged.
fn migrate_store(mut prefs: EditorPrefs) -> EditorPrefs {
    if prefs.version < EDITOR_PREFS_VERSION {
        // No field-shape migrations exist yet (v0 → v1 is field-compatible via serde defaults);
        // future versions add their transforms here, gated on the incoming `version`.
        prefs.version = EDITOR_PREFS_VERSION;
    }
    // A basemap string that somehow slipped in unnormalized (older hand-edited blob) is corrected
    // on load so the store and the map host never disagree.
    prefs.basemap = normalize_basemap(&prefs.basemap);
    prefs
}

/// T-691 — build the store from the two legacy keys (one-way migration). Each key is read
/// independently and defaulted on absence/garbage, so a half-present legacy state (only the layers
/// key set, say) still preserves what it can. Pure over its `(layers_raw, basemap_raw)` inputs so
/// the migration rule is unit-testable without a DOM.
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

/// T-691 — load the editor-preferences store. Order: the versioned key (serde round-trip →
/// migrate → defaults on garbage); else a one-way migration from the legacy `tbd-mc-*` keys, then
/// persist the migrated store forward under the new key so the legacy read happens at most once;
/// else defaults. Off wasm (native test build) this is always [`EditorPrefs::default`].
#[must_use]
pub fn load_store() -> EditorPrefs {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = storage() {
            if let Ok(Some(raw)) = s.get_item(EDITOR_PREFS_KEY) {
                return migrate_store(EditorPrefs::from_json(&raw));
            }
            // New key absent → migrate the legacy keys once and write the result forward.
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

/// T-691 — persist the editor-preferences store (no-op off wasm). The version is stamped current on
/// write so a load never sees a stale version it wrote itself.
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

/// Load the persisted world-layer prefs (defaults when unset / on a non-wasm host). Thin accessor
/// over [`load_store`] (T-691) — signature preserved for the `world_assets` callers.
#[must_use]
pub fn load_prefs() -> WorldLayerPrefs {
    load_store().layers
}

/// Persist the world-layer prefs (no-op off wasm). Reads the current store, swaps in `p`, and
/// writes it back so the basemap half is not clobbered (T-691).
pub fn save_prefs(p: &WorldLayerPrefs) {
    let mut store = load_store();
    store.layers = *p;
    save_store(&store);
}

/// Basemap view: `"satellite"` (default) or `"map"` (cartographic pyramid). Thin accessor over
/// [`load_store`] (T-691).
#[must_use]
pub fn load_basemap_view() -> String {
    load_store().basemap
}

/// Persist the basemap view (no-op off wasm). Reads the current store, swaps in the normalized
/// `view`, and writes it back so the layer toggles are not clobbered (T-691).
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

    // ── T-691 (3den E6) store tests ────────────────────────────────────────────────────────────

    /// Store round-trip: a non-default store survives serialize → parse byte-for-byte, and keeps
    /// the React layer key names in the persisted form (the `world_assets` reader depends on them).
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

    /// Defaults-on-garbage: a blob that is not a valid store parses to the default store, never a
    /// panic — the floor that keeps a corrupt localStorage from bricking the editor.
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

    /// Absent fields fall back to per-field defaults (the forward-compat path a future ticket's new
    /// key rides): a blob missing `version`/`basemap` still loads, with the layer defaults intact.
    #[test]
    fn store_partial_blob_fills_defaults() {
        let got = EditorPrefs::from_json("{}");
        assert_eq!(got.layers, WorldLayerPrefs::default());
        assert_eq!(got.basemap, "satellite");
        assert_eq!(got.version, 0); // absent version → serde default 0, stamped forward on migrate
    }

    /// Migration preserves old values: fire the legacy migration rule with real operator values
    /// (props ON, roads OFF, basemap "map") and prove they carry into the new store verbatim.
    #[test]
    fn migration_preserves_old_values() {
        let mut old = WorldLayerPrefs::default();
        old.set("props", true); // operator turned props ON
        old.set("roads", false); // operator turned roads OFF
        let old_layers_json = serde_json::to_string(&old).unwrap();

        let migrated = migrate_from_legacy(Some(&old_layers_json), Some("map"));
        assert!(migrated.layers.props, "props ON must survive migration");
        assert!(!migrated.layers.roads, "roads OFF must survive migration");
        assert_eq!(migrated.basemap, "map", "basemap choice must survive");
        assert_eq!(migrated.version, EDITOR_PREFS_VERSION);
        // The rest of the toggles are unchanged from the old blob (i.e. defaults here).
        assert_eq!(
            migrated.layers,
            WorldLayerPrefs {
                props: true,
                roads: false,
                ..Default::default()
            }
        );
    }

    /// Migration is defensive on a half-present / garbage legacy state: a missing layers key and a
    /// garbage basemap both fall back to defaults rather than losing the whole store.
    #[test]
    fn migration_defaults_on_absent_or_garbage_legacy() {
        // Nothing present at all → full defaults.
        let none = migrate_from_legacy(None, None);
        assert_eq!(none.layers, WorldLayerPrefs::default());
        assert_eq!(none.basemap, "satellite");

        // Garbage layers + bogus basemap → defaults for both, still a valid store.
        let junk = migrate_from_legacy(Some("{bad"), Some("teal"));
        assert_eq!(junk.layers, WorldLayerPrefs::default());
        assert_eq!(junk.basemap, "satellite");

        // Only basemap present, and valid → layers default, basemap carried.
        let partial = migrate_from_legacy(None, Some("map"));
        assert_eq!(partial.layers, WorldLayerPrefs::default());
        assert_eq!(partial.basemap, "map");
    }

    /// The migration/normalize path only ever emits a known basemap view.
    #[test]
    fn basemap_is_always_normalized() {
        assert_eq!(normalize_basemap("map"), "map");
        assert_eq!(normalize_basemap("satellite"), "satellite");
        assert_eq!(normalize_basemap("garbage"), "satellite");
        assert_eq!(normalize_basemap(""), "satellite");
        // migrate_store corrects an unnormalized stored basemap on load.
        let fixed = migrate_store(EditorPrefs {
            basemap: "nonsense".to_string(),
            ..Default::default()
        });
        assert_eq!(fixed.basemap, "satellite");
    }

    /// migrate_store stamps a pre-version blob forward and is idempotent at the current version.
    #[test]
    fn migrate_store_stamps_version_and_is_idempotent() {
        let stamped = migrate_store(EditorPrefs {
            version: 0,
            ..Default::default()
        });
        assert_eq!(stamped.version, EDITOR_PREFS_VERSION);
        // Idempotent: running it again changes nothing.
        assert_eq!(migrate_store(stamped.clone()), stamped);
    }
}
