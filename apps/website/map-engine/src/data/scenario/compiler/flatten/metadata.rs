//! Role: metadata.
//! Position: `mission/compiler/flatten` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Mission metadata shared by the API and browser compiler; serde uses the canonical camelCase keys.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MissionMeta {
    /// Id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Author.
    pub author: String,
    /// Terrain.
    pub terrain: String,
    /// Custom terrain name.
    pub custom_terrain_name: String,
    /// Max players.
    pub max_players: i64,
    /// Time of day.
    pub time_of_day: String,
    /// Weather preset.
    pub weather_preset: String,
}

/// Canonical compile date anchor value.
pub(super) const COMPILE_DATE_ANCHOR: &str = "1989-06-14";

/// Canonical spawn zone radius m value.
pub(super) const SPAWN_ZONE_RADIUS_M: f64 = 150.0;

/// `mission.schema.json#/$defs/meta/name` — `maxLength: 120`.
pub(super) const META_NAME_MAX_CHARS: usize = 120;

/// Canonical role fallback value.
pub(super) const ROLE_FALLBACK: &str = "unassigned";

/// Stand-in for a squad with neither `callsign` nor `name`. Only reached when the squad also has no id, because the id is preferred — two unnamed squads must not collapse onto one callsign, or their derived slot ids collide and the mod's duplicate-id check (a hard error there) rejects the whole document.
pub(super) const CALLSIGN_FALLBACK: &str = "squad";

/// `TBD_RadioPlan.MAX_NETS` (`apps/mod/tbd-framework/.../Radio/TBD_RadioPlan.c:91`). **The schema states no `maxItems` on `radioPlan.nets`** — this limit exists only in the mod, which accepts the first 32 nets in DOCUMENT ORDER and drops the rest. It is mirrored here so the cut is made by the side that can make it fairly: see [`derive_radio_plan`] for why document order is load-bearing.
pub(super) const MOD_MAX_NETS: usize = 32;

/// `TBD_RadioPlan.MAX_LABEL_CHARS` (`TBD_RadioPlan.c:94`). Again mod-only — the schema puts no `maxLength` on `net.label`. `TBD_RadioPlan.CapLabel` truncates past it without a word to anyone, so the truncation is done here instead, where the compiled document a human can read already shows the string the player will see.
pub(super) const MOD_MAX_LABEL_CHARS: usize = 48;

/// `TBD_MarkerService.MAX_LABEL_CHARS` (`Markers/TBD_MarkerData.c:63`). A DIFFERENT consumer with a different budget from [`MOD_MAX_LABEL_CHARS`] — the radio plan's 48 is `TBD_RadioPlan`'s, this 64 is the marker wire's — so the two are deliberately separate constants rather than one shared number that would silently retune whichever mod class changed second.
pub(super) const MOD_MAX_MARKER_LABEL_CHARS: usize = 64;

/// Bottom of `mission.schema.json#/$defs/net/freqMHz` (`minimum: 30`) and the base of the net frequency allocation. Deliberately the schema's own floor and not a number lifted from a golden mission — see [`derive_radio_plan`].
pub(super) const NET_FREQ_BASE_MHZ: f64 = 30.0;

/// Canonical net freq step mhz value.
pub(super) const NET_FREQ_STEP_MHZ: f64 = 0.5;

/// The schema's `minLength: 1` string fields cannot take the empty string, and the editor does not guarantee these are set. Substitute rather than emit a document that fails our own contract.
pub(super) fn or_fallback<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

/// Lowercase into the schema's `^[a-z][a-z0-9_]*$` pattern.
pub(super) fn slug_key(raw: &str, fallback: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_repl = false;
    for c in raw.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
            out.push(c);
            prev_repl = false;
        } else if !prev_repl {
            out.push('_');
            prev_repl = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    match trimmed.chars().next() {
        Some(c) if c.is_ascii_lowercase() => trimmed.to_string(),
        _ => format!("f_{trimmed}"),
    }
}

/// Mission terrain key using the supplied domain data.
pub fn mission_terrain_key(terrain: &str, custom_terrain_name: &str) -> String {
    let raw = if terrain == "custom" && !custom_terrain_name.is_empty() {
        custom_terrain_name
    } else {
        terrain
    };
    slug_key(raw, "everon")
}

/// Reduce the mission UUID to the schema's `^msn_[a-z0-9]+$` id space.
pub(super) fn mission_doc_id(id: &str) -> String {
    let hex: String = id
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect();
    format!("msn_{}", if hex.is_empty() { "editor" } else { &hex })
}
