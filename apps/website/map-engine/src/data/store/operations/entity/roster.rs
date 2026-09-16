//! Role: roster.
//! Position: `doc/operations/entity` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::APPLY_ANCHOR_X;
use super::APPLY_ANCHOR_Y;
use super::MissionDocCore;
use super::faction_rows;
use super::slot_rows;
use super::squad_rows;

/// Slot fields the ORBAT Manager inspector / `format_slot_line` need (from `slots_json`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrbatSlotDetail {
    /// Id.
    pub id: String,

    /// Role.
    pub role: String,

    /// Tag.
    pub tag: String,

    /// Callsign.
    pub callsign: String,

    /// Rank.
    pub rank: String,

    /// Index.
    pub index: u32,

    /// Squad id.
    pub squad_id: String,

    /// Summary.
    pub summary: String,

    /// Primary.
    pub primary: String,

    /// Launcher.
    pub launcher: String,
}

/// Slot details using the supplied domain data.
pub fn slot_details(core: &MissionDocCore) -> Vec<OrbatSlotDetail> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| {
            let o = v.as_object()?;
            let lo = o.get("loadout").and_then(|l| l.as_object());
            Some(OrbatSlotDetail {
                id: o.get("id")?.as_str()?.to_string(),
                role: o
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                tag: o
                    .get("tag")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default()
                    .to_string(),
                callsign: o
                    .get("callsign")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default()
                    .to_string(),
                rank: o
                    .get("rank")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                index: o
                    .get("index")
                    .and_then(|i| i.as_u64().or_else(|| i.as_i64().map(|n| n as u64)))
                    .unwrap_or(0) as u32,
                squad_id: o
                    .get("squadId")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                summary: lo
                    .and_then(|m| m.get("summary"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                primary: lo
                    .and_then(|m| m.get("primary"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                launcher: lo
                    .and_then(|m| m.get("launcher"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

/// Canonical `faction-{SIDE}` id for an Eden side chip. Does **not** mint a faction row.
pub fn side_faction_id(side: &str) -> String {
    format!("faction-{side}")
}

/// Ensure `faction-{SIDE}` exists in the doc (mint if missing).
pub fn ensure_side_faction(core: &MissionDocCore, side: &str) -> String {
    let faction_id = side_faction_id(side);
    let factions = faction_rows(core);
    if !factions.iter().any(|f| f.id == faction_id) {
        core.add_faction(&faction_id, side, side);
    }
    faction_id
}

/// Mint squad id for side using the supplied domain data.
pub fn mint_squad_id_for_side(core: &MissionDocCore, side: &str) -> String {
    let existing: std::collections::HashSet<String> =
        squad_rows(core).into_iter().map(|s| s.id).collect();
    let mut n: u32 = 1;
    loop {
        let id = format!("squad-{side}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// Canonical orbat slot spacing x value.
pub const ORBAT_SLOT_SPACING_X: f64 = 15.0;

/// One slot's map position out of a parsed `slots_json`. `None` when the id is stale (the slot was removed) or the stored position is malformed.
pub fn slot_xy(root: &serde_json::Value, id: &str) -> Option<(f64, f64)> {
    let pos = root.get(id)?.get("position")?;
    Some((pos.get("x")?.as_f64()?, pos.get("y")?.as_f64()?))
}

/// The squad's anchor out of an already-parsed `slots_json`: the leader if it still exists, else the first slot that does. `None` only for a squad with nothing live to anchor against, which leaves the fallback to the caller.
pub fn squad_anchor_in(
    root: &serde_json::Value,
    sq: &crate::data::store::operations::rows::SquadRow,
) -> Option<(f64, f64)> {
    std::iter::once(sq.leader_slot_id.as_str())
        .chain(sq.slot_ids.iter().map(String::as_str))
        .filter(|id| !id.is_empty())
        .find_map(|id| slot_xy(root, id))
}

/// Squad anchor xy using the supplied domain data.
pub fn squad_anchor_xy(
    core: &MissionDocCore,
    sq: &crate::data::store::operations::rows::SquadRow,
) -> Option<(f64, f64)> {
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    squad_anchor_in(&root, sq)
}

/// Next slot xy using the supplied domain data.
pub fn next_slot_xy(
    core: &MissionDocCore,
    sq: &crate::data::store::operations::rows::SquadRow,
) -> (f64, f64) {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return (APPLY_ANCHOR_X, APPLY_ANCHOR_Y);
    };
    let (ax, ay) = squad_anchor_in(&root, sq).unwrap_or((APPLY_ANCHOR_X, APPLY_ANCHOR_Y));
    let x = sq
        .slot_ids
        .iter()
        .filter_map(|id| slot_xy(&root, id).map(|(x, _)| x))
        .reduce(f64::max)
        .map_or(ax, |max_x| max_x + ORBAT_SLOT_SPACING_X);
    (x, ay)
}

/// Snapshot of squads/factions/slot details for the ORBAT Manager (one doc read).
#[derive(Clone, Debug, Default)]
pub struct OrbatManagerSnapshot {
    /// Factions.
    pub factions: Vec<crate::data::store::operations::rows::FactionRow>,

    /// Squads.
    pub squads: Vec<crate::data::store::operations::rows::SquadRow>,

    /// Slots.
    pub slots: Vec<OrbatSlotDetail>,
}

/// Domain representation of placed slot choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedSlotChoice {
    /// Id.
    pub id: String,

    /// Label.
    pub label: String,
}

/// Apply orbat_manager_snapshot to explicit document state.
pub fn orbat_manager_snapshot(core: &MissionDocCore) -> OrbatManagerSnapshot {
    OrbatManagerSnapshot {
        factions: faction_rows(core),
        squads: squad_rows(core),
        slots: slot_details(core),
    }
}

/// Apply orbat_add_squad to explicit document state.
pub fn orbat_add_squad(core: &MissionDocCore, side: String) -> Option<String> {
    if !matches!(side.as_str(), "BLUFOR" | "OPFOR" | "INDFOR") {
        return None;
    }
    let faction_id = ensure_side_faction(core, &side);
    let squad_id = mint_squad_id_for_side(core, &side);
    let ordinal = faction_rows(core)
        .iter()
        .find(|f| f.id == faction_id)
        .map(|f| f.squad_ids.len())
        .unwrap_or(0);
    let name = format!("Squad {}", ordinal + 1);
    core.add_squad(&squad_id, &faction_id, &name, None);
    Some(squad_id)
}

/// Apply orbat_remove_slot to explicit document state.
pub fn orbat_remove_slot(core: &MissionDocCore, slot_id: String) -> bool {
    let detail = slot_details(core)
        .into_iter()
        .find(|s| s.id == slot_id)
        .unwrap_or_default();
    let squad_id = detail.squad_id.clone();
    if squad_id.is_empty() {
        return false;
    }
    let sq = squad_rows(core).into_iter().find(|s| s.id == squad_id);
    let was_leader = sq.as_ref().is_some_and(|s| s.leader_slot_id == slot_id);
    let remaining: Vec<String> = sq
        .map(|s| s.slot_ids.into_iter().filter(|id| id != &slot_id).collect())
        .unwrap_or_default();
    core.remove_slots(vec![slot_id]);
    if remaining.is_empty() {
        core.remove_squad(&squad_id);
    } else if was_leader && let Some(next) = remaining.first() {
        core.set_leader(&squad_id, next);
    }
    true
}

/// Apply placed_slot_choices to explicit document state.
pub fn placed_slot_choices(core: &MissionDocCore) -> Vec<PlacedSlotChoice> {
    let mut rows: Vec<PlacedSlotChoice> = slot_rows(core)
        .into_iter()
        .map(|s| {
            let label = if s.role.is_empty() {
                s.id.clone()
            } else {
                format!("{} ({})", s.role, s.id)
            };
            PlacedSlotChoice { id: s.id, label }
        })
        .collect();
    rows.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    rows
}

/// Apply orbat_update_slot_fields to explicit document state.
pub fn orbat_update_slot_fields(
    core: &MissionDocCore,
    slot_id: String,
    role: Option<String>,
    tag: Option<String>,
    callsign: Option<String>,
    rank: Option<String>,
) -> bool {
    if role.is_some() || tag.is_some() {
        core.update_slot(&slot_id, role, tag, None);
    }
    if callsign.is_some() || rank.is_some() {
        core.update_slot_identity(&slot_id, callsign, rank);
    }
    true
}

/// Deliberately role-keyed, not "any squad-mate": borrowing the squad leader's prefab for a Rifleman would swap the character out for the wrong one, which is worse than leaving it unset.
pub fn asset_id_for_role(
    core: &MissionDocCore,
    sq: &crate::data::store::operations::rows::SquadRow,
    role: &str,
) -> Option<String> {
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    let obj = root.as_object()?;

    let squads = squad_rows(core);
    let siblings = faction_rows(core)
        .into_iter()
        .find(|f| f.id == sq.faction_id)
        .map(|f| f.squad_ids)
        .unwrap_or_default();

    let mut candidates: Vec<&String> = sq.slot_ids.iter().collect();
    for sid in &siblings {
        if let Some(s) = squads.iter().find(|s| s.id == *sid) {
            candidates.extend(s.slot_ids.iter());
        }
    }

    candidates.into_iter().find_map(|id| {
        let slot = obj.get(id)?;
        if slot.get("role").and_then(serde_json::Value::as_str) != Some(role) {
            return None;
        }
        slot.get("assetId")
            .and_then(serde_json::Value::as_str)
            .filter(|a| !a.is_empty())
            .map(ToString::to_string)
    })
}
