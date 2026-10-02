//! Role: orbat slot template.
//! Position: `mission/ast/factions` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Deserialize, HashMap, Serialize, Value};

/// One ordered, distinct slot in a squad: a role + loadout + optional tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbatSlotTemplate {
    /// Role.
    #[serde(default)]
    pub role: String,
    /// Loadout.
    #[serde(default)]
    pub loadout: String,
    /// Tag.
    #[serde(default)]
    pub tag: String,
}

/// A squad + its ordered slot list (list position = its 1-based ORBAT number).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbatSquadTemplate {
    /// Faction.
    pub faction: String,
    /// Callsign.
    #[serde(default)]
    pub callsign: String,
    /// Squad.
    #[serde(default)]
    pub squad: String,
    /// Slots.
    #[serde(default)]
    pub slots: Vec<OrbatSlotTemplate>,
}

/// Do NOT trim one side only — a unilateral trim breaks the case where ORBAT `"  USA  "` and armory `"  USA  "` agree today. Normalising both sides needs a migration for live rows.
pub fn validate_faction_join_key(faction: &str) -> Result<(), &'static str> {
    if faction.trim().is_empty() {
        return Err("faction is required");
    }
    if faction != faction.trim() {
        return Err("faction must not have leading or trailing whitespace");
    }
    Ok(())
}

/// Extract the ORBAT squad list: an explicit top-level `"orbat"` array wins; otherwise it is derived from the editor graph (Save Version omits `orbat`).
pub fn parse_orbat_template(payload: &[u8]) -> Vec<OrbatSquadTemplate> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Top {
        orbat: Vec<OrbatSquadTemplate>,
    }
    let top: Top = serde_json::from_slice(payload).unwrap_or_default();
    if !top.orbat.is_empty() {
        return top.orbat;
    }
    derive_orbat_from_editor(payload)
}

/// Human-readable kit string for Event/Export `orbat[].slots[].loadout`. Prefers embedded `summary`; else joins display names of `primary` + `launcher` with `" + "`.
pub(super) fn loadout_summary_from_value(lo: Option<&Value>) -> String {
    let Some(lo) = lo.filter(|v| !v.is_null()) else {
        return String::new();
    };
    if let Some(s) = lo.get("summary").and_then(Value::as_str) {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let primary = lo
        .get("primary")
        .and_then(Value::as_str)
        .map(display_resource)
        .filter(|s| !s.is_empty());
    let launcher = lo
        .get("launcher")
        .and_then(Value::as_str)
        .map(display_resource)
        .filter(|s| !s.is_empty());
    [primary, launcher]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Basename after last `/` or `}`, then strip trailing `.et`.
pub(super) fn display_resource(resource: &str) -> String {
    let base = resource
        .rsplit(['/', '}'])
        .next()
        .unwrap_or(resource)
        .trim();
    base.strip_suffix(".et").unwrap_or(base).to_string()
}

/// Reconstruct the ORBAT from the editor graph, mirroring `compile.ts` order EXACTLY: factions in array order → each `squadIds` → resolve squad → each `slotIds` → resolve slots → sort by `index` ascending. `loadout` is the slot summary (or primary+launcher rebuild) when present; `""` when absent.
pub fn derive_orbat_from_editor(payload: &[u8]) -> Vec<OrbatSquadTemplate> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Ep {
        editor: Eg,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Eg {
        factions: Vec<F>,
        squads: Vec<S>,
        slots: Vec<Sl>,
    }
    #[derive(Deserialize, Default)]
    #[serde(rename_all = "camelCase", default)]
    struct F {
        key: String,
        squad_ids: Vec<String>,
    }
    #[derive(Deserialize, Default)]
    #[serde(rename_all = "camelCase", default)]
    struct S {
        id: String,
        callsign: String,
        name: String,
        slot_ids: Vec<String>,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Sl {
        id: String,
        index: i64,
        role: String,
        tag: String,
        loadout: Option<Value>,
    }

    let Ok(e) = serde_json::from_slice::<Ep>(payload) else {
        return Vec::new();
    };
    let ed = e.editor;
    if ed.factions.is_empty() {
        return Vec::new();
    }

    let squads_by_id: HashMap<&str, &S> = ed.squads.iter().map(|s| (s.id.as_str(), s)).collect();
    let slots_by_id: HashMap<&str, &Sl> = ed.slots.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut out: Vec<OrbatSquadTemplate> = Vec::new();
    for f in &ed.factions {
        for squad_id in &f.squad_ids {
            let Some(sq) = squads_by_id.get(squad_id.as_str()) else {
                continue;
            };
            let mut rows: Vec<&Sl> = sq
                .slot_ids
                .iter()
                .filter_map(|id| slots_by_id.get(id.as_str()).copied())
                .collect();
            rows.sort_by_key(|s| s.index);
            let slots = rows
                .iter()
                .map(|r| OrbatSlotTemplate {
                    role: r.role.clone(),
                    loadout: loadout_summary_from_value(r.loadout.as_ref()),
                    tag: r.tag.clone(),
                })
                .collect();
            out.push(OrbatSquadTemplate {
                faction: f.key.clone(),
                callsign: sq.callsign.clone(),
                squad: sq.name.clone(),
                slots,
            });
        }
    }
    out
}
