//! Role: environment.
//! Position: `doc/operations` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::doc::MissionDocCore;

/// The environment block of a mission row: time of day and weather as authored.
#[derive(Clone, Debug, PartialEq)]
pub struct MissionEnv {
    /// Terrain.
    pub terrain: String,

    /// Time.
    pub time: String,

    /// Weather.
    pub weather: String,

    /// Show hillshade.
    pub show_hillshade: bool,

    /// Hillshade opacity.
    pub hillshade_opacity: f64,

    /// Show grid.
    pub show_grid: bool,
}

impl Default for MissionEnv {
    fn default() -> Self {
        Self {
            terrain: String::new(),
            time: String::new(),
            weather: String::new(),
            show_hillshade: true,
            hillshade_opacity: 0.4,
            show_grid: true,
        }
    }
}

/// Apply read_env to explicit document state.
pub fn read_env(core: &MissionDocCore) -> Option<MissionEnv> {
    let root: serde_json::Value = serde_json::from_str(&core.small_maps_json()).ok()?;
    let meta = root.get("meta")?;
    let env = meta.get("environment");
    let s = |v: Option<&serde_json::Value>, k: &str, def: &str| {
        v.and_then(|e| e.get(k))
            .and_then(|x| x.as_str())
            .unwrap_or(def)
            .to_string()
    };
    Some(MissionEnv {
        terrain: meta
            .get("terrain")
            .and_then(|t| t.as_str())
            .unwrap_or("everon")
            .to_string(),
        time: s(env, "time", "06:00"),
        weather: s(env, "weather", "clear"),
        show_hillshade: env
            .and_then(|e| e.get("showHillshade"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        hillshade_opacity: env
            .and_then(|e| e.get("hillshadeOpacity"))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.4),
        show_grid: env
            .and_then(|e| e.get("showGrid"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
    })
}
