//! Role: merge report.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// `offset` shifts every merged entity's authored position by `(dx, dy)` world meters. `None` (and the `(0.0, 0.0)` it collapses to) keeps the source mission's coordinates verbatim — the default, because a mission is a coherent spatial document. The template-into-a-corner case supplies a delta.
#[derive(Debug, Clone, Copy, Default)]
pub struct MergeOpts {
    /// World-space `(dx, dy)` applied to every placed entity; `None` = keep authored positions.
    pub offset: Option<(f64, f64)>,
}

/// Domain representation of merge report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeReport {
    /// Slots added to the document.
    pub slots_added: u32,

    /// Incoming squads folded into a resident squad (same name+side).
    pub squads_merged: u32,

    /// Incoming squads created fresh (no resident match).
    pub squads_created: u32,

    /// Incoming factions folded into a resident faction (same name+side key).
    pub factions_merged: u32,

    /// Incoming factions created fresh.
    pub factions_created: u32,

    /// Vehicles added.
    pub vehicles_added: u32,

    /// Mission-placed entities (world objects) added.
    pub entities_added: u32,

    /// Zones added.
    pub zones_added: u32,

    /// Triggers added.
    pub triggers_added: u32,

    /// Compositions (self-contained templates) added.
    pub compositions_added: u32,

    /// Markers added.
    pub markers_added: u32,

    /// Malformed rows the merge tolerated: `(kind, id, reason)`. Never a panic.
    pub skipped: Vec<(String, String, String)>,
}

impl MergeReport {
    /// Serialize the report to a compact JSON object for the wasm command seam. `skipped` becomes an array of `{kind,id,reason}` objects. Uses `serde_json::Value` (no derive) so this stays inside the `doc` feature without a `serde::Serialize` dependency on these types.
    #[must_use]
    pub fn to_json_string(&self) -> String {
        let skipped: Vec<serde_json::Value> = self
            .skipped
            .iter()
            .map(|(kind, id, reason)| {
                serde_json::json!({ "kind": kind, "id": id, "reason": reason })
            })
            .collect();
        serde_json::json!({
            "slots_added": self.slots_added,
            "squads_merged": self.squads_merged,
            "squads_created": self.squads_created,
            "factions_merged": self.factions_merged,
            "factions_created": self.factions_created,
            "vehicles_added": self.vehicles_added,
            "entities_added": self.entities_added,
            "zones_added": self.zones_added,
            "triggers_added": self.triggers_added,
            "compositions_added": self.compositions_added,
            "markers_added": self.markers_added,
            "skipped": skipped,
        })
        .to_string()
    }
}
