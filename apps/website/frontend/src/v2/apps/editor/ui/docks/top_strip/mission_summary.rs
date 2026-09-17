//! Mission summary for the top command strip.

use super::*;

/// The three Eden sides, in header order, paired with the schema faction `key` each derives from.
///
/// The `key` half is the value `factionsById[..].key` holds (`asset_catalog` `EDEN_SIDES`, and the
/// `orbat_add_squad` guard on `editor_ops.rs:4249`); the `label` half is the milsim-facing word the
/// header shows. WOG's 94%-consistent community naming convention grew out of exactly this label
/// vocabulary, so the labels are part of the stable format the summary line pins below.
pub(super) const CENSUS_SIDES: [(&str, &str); 3] =
    [("BLUFOR", "WEST"), ("OPFOR", "EAST"), ("INDFOR", "IND")];

/// A per-side slot tally plus the unassigned remainder — the census the header badge renders.
///
/// `west` / `east` / `ind` are the BLUFOR / OPFOR / INDFOR slot counts; `unassigned` is every slot
/// whose `squadId` does not resolve through a squad to a faction carrying one of the three side keys
/// (a dangling `squadId`, or a faction with an empty/unknown `key`). `total` counts EVERY slot, so
/// `west + east + ind + unassigned == total` always — the invariant that makes the malformed
/// "counts don't add up" state unrepresentable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotCensus {
    pub west: usize,
    pub east: usize,
    pub ind: usize,
    /// Slots that resolve to no known side. Shown in the badge ONLY when nonzero (spec).
    pub unassigned: usize,
    pub total: usize,
}

impl SlotCensus {
    /// The per-side count for a schema faction `key`, or 0 for a key that is not one of the three
    /// Eden sides (which is what makes such a slot land in `unassigned`, not in a side bucket).
    pub(super) fn count_for_key(&self, key: &str) -> usize {
        match key {
            "BLUFOR" => self.west,
            "OPFOR" => self.east,
            "INDFOR" => self.ind,
            _ => 0,
        }
    }
}

/// Derive the per-side census PURELY from the ORBAT rows — the header's single source of truth.
///
/// Reuses the snapshot's own rows (fed by the engine's `census_input`, which reads them once via
/// `orbat_manager_snapshot`); it never re-parses the document. Each `(slot, squadId)` walks
/// squad → faction → `key`; an id that dangles at any hop (deleted squad, faction with no side key)
/// falls through to `unassigned`. `slot_squad_ids` is one entry per slot — its length IS `total`, so
/// the buckets can never disagree with the slot set.
///
/// Pure + total (no panics, no I/O): the whole reason it lives here and not behind the wasm gate.
#[must_use]
pub fn census_from_rows(
    factions: &[crate::v2::apps::editor::ui::outliner::outliner::FactionRow],
    squads: &[crate::v2::apps::editor::ui::outliner::outliner::SquadRow],
    slot_squad_ids: &[String],
) -> SlotCensus {
    let mut side_of_squad: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for sq in squads {
        if let Some(f) = factions.iter().find(|f| f.id == sq.faction_id) {
            side_of_squad.insert(sq.id.as_str(), f.key.as_str());
        }
    }
    let mut c = SlotCensus::default();
    for squad_id in slot_squad_ids {
        c.total += 1;
        match side_of_squad.get(squad_id.as_str()).copied() {
            Some("BLUFOR") => c.west += 1,
            Some("OPFOR") => c.east += 1,
            Some("INDFOR") => c.ind += 1,
            _ => c.unassigned += 1,
        }
    }
    c
}

/// Human terrain name for the summary (`everon` → `Everon`). Mirrors the `terrain_label` idiom used
/// across the mission pages (`event_hub.rs:50`, `create_mission_dialog.rs:18`) — capitalize the
/// first char — kept local so this owned file carries no cross-module dependency for a one-liner.
pub(super) fn terrain_label(t: &str) -> String {
    let mut ch = t.chars();
    match ch.next() {
        Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
        None => String::new(),
    }
}

/// **The community naming format — KEEP STABLE. Other tools parse this string.**
///
/// Format the stable mission summary used by downstream readers. WEST and
/// EAST counts are always present; IND and unassigned counts append when nonzero.
/// Existing separators and field order form the parser contract.
#[must_use]
pub fn summary_line(census: &SlotCensus, terrain: &str, mode: Option<&str>) -> String {
    let mut out = String::new();
    if let Some(m) = mode {
        let m = m.trim();
        if !m.is_empty() {
            out.push_str(m);
            out.push(' ');
        }
    }
    let terrain = terrain_label(terrain);
    let terrain = if terrain.is_empty() {
        "Unknown".to_string()
    } else {
        terrain
    };
    out.push_str(&format!(
        "{} on {} — WEST {} v EAST {}",
        census.total, terrain, census.west, census.east
    ));
    if census.ind > 0 {
        out.push_str(&format!(" (+{} IND)", census.ind));
    }
    if census.unassigned > 0 {
        out.push_str(&format!(" ({} unassigned)", census.unassigned));
    }
    out
}
