//! Role: markers.
//! Position: the `entity::markers` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::ids::FactionId;
use mission_model::ids::MarkerId;

use super::MissionDocCore;

/// One authored marker, as the dock lists it. The `(faction_id, id)` pair is the address both store mutators take, carried on every row so a listed marker can be moved, re-captioned, re-iconed or deleted without a second lookup.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerRow {
    /// `factionsById` key — `faction-BLUFOR` / `-OPFOR` / `-INDFOR`.
    pub faction_id: FactionId,

    /// Doc-internal id. Addressing only; it never reaches the wire (`$defs/marker` is `additionalProperties: false`, and the serde boundary drops the key for free).
    pub id: MarkerId,

    /// ATTR-FIELD-MRK-POSITION — world metres. `$defs/marker` is `{x, z}`: a marker is a MAP glyph, so it carries no height, unlike a slot's `{x, y, z}` position.
    pub x: f64,

    /// Z.
    pub z: f64,

    /// ATTR-FIELD-MRK-TYPE — one of the 64 closed `$defs/marker.icon` aliases.
    pub icon: String,

    /// ATTR-FIELD-MRK-TEXT — the caption, stored VERBATIM (the mod caps it at render time; capping here would destroy the authored value in the one place the author could still fix it).
    pub label: String,
}

impl MarkerRow {
    /// The side chip this marker belongs to (`BLUFOR` / `OPFOR` / `INDFOR`), derived from the `faction-{SIDE}` id [`side_faction_id`](crate::entity::side_faction_id) names (minted later by [`ensure_side_faction`](crate::entity::ensure_side_faction) on first slot/squad). Falls back to the whole id for a faction that came from a library import under some other naming.
    #[must_use]
    pub fn side(&self) -> &str {
        self.faction_id
            .as_str()
            .strip_prefix("faction-")
            .unwrap_or(self.faction_id.as_str())
    }

    /// The palette row's right-hand readout — ATTR-FIELD-MRK-POSITION at a glance.
    #[must_use]
    pub fn position_summary(&self) -> String {
        format!("{:.0}, {:.0}", self.x, self.z)
    }
}

/// The parse, taking the core directly. Split out because the Mission Creator's `place_at_impl` (in the `frontend` app's armed placement) already holds the doc borrow when it needs the list (to mint an unused id), and re-entering through `marker_rows` (in the `map_engine` crate's hosted commands) there would re-open a borrow the place path is in the middle of.
pub fn marker_rows_of(core: &MissionDocCore) -> Vec<MarkerRow> {
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.briefing_marker_rows_json())
    else {
        return Vec::new();
    };
    let Some(arr) = rows.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|r| {
            Some(MarkerRow {
                faction_id: r.get("factionId")?.as_str()?.into(),
                id: r.get("id")?.as_str()?.into(),
                x: r.get("x")?.as_f64()?,
                z: r.get("z")?.as_f64()?,
                icon: r
                    .get("icon")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                label: r
                    .get("label")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

/// Mint a marker id unused anywhere in the document.
pub fn mint_marker_id(rows: &[MarkerRow]) -> String {
    let taken: std::collections::HashSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let mut n: u32 = 1;
    loop {
        let id = format!("mk-{n}");
        if !taken.contains(id.as_str()) {
            return id;
        }
        n = n.saturating_add(1);
    }
}
