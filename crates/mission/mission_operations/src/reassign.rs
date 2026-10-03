//! Role: reassign.
//! Position: the `reassign` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::ids::{FactionId, SquadId};

use super::projections::squad_rows;
use mission_document::MissionDocCore;

/// Where a batch reassign sends the selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReassignTarget {
    /// The faction picked in the modal (a `factionsById` row id).
    pub faction_id: FactionId,

    /// The squad picked in the modal, or empty for "the faction's first squad".
    pub squad_id: SquadId,
}

/// Faction label using the supplied domain data.
#[must_use]
pub fn faction_label(f: &super::rows::FactionRow) -> String {
    match (f.name.trim(), f.key.trim()) {
        ("", "") => f.id.to_string(),
        ("", key) => key.to_string(),
        (name, "") => name.to_string(),
        (name, key) if name == key => name.to_string(),
        (name, key) => format!("{name} ({key})"),
    }
}

/// Pure, and deliberately outside the `wasm32` block: the `axis_chip_class` / `nudge_step` precedent. The refusal strings are the user-visible half of requirement 4, and a message that only a source pin ever reads is a message nobody has proved the modal can produce — here `cargo test` calls the real function and reads the real sentence.
pub fn plan_reassign(
    factions: &[super::rows::FactionRow],
    squads: &[super::rows::SquadRow],
    faction_id: impl Into<FactionId>,
    squad_id: impl Into<SquadId>,
) -> Result<String, String> {
    let squad_id: SquadId = squad_id.into();
    let squad_id = squad_id.as_str();
    let faction_id: FactionId = faction_id.into();
    let faction_id = faction_id.as_str();
    let Some(faction) = factions.iter().find(|f| f.id == faction_id) else {
        return Err(format!(
            "That faction ({faction_id}) is no longer in this mission — reopen Attributes."
        ));
    };
    let picked = faction_label(faction);
    if squad_id.is_empty() {
        return faction
            .squad_ids
            .iter()
            .find(|sid| squads.iter().any(|s| s.id == ***sid))
            .cloned()
            .ok_or_else(|| {
                format!("{picked} has no squads yet — add one in the ORBAT dock, then reassign.")
            });
    }
    let Some(squad) = squads.iter().find(|s| s.id == squad_id) else {
        return Err(format!(
            "That squad ({squad_id}) is no longer in this mission — reopen Attributes."
        ));
    };
    if squad.faction_id != faction_id {
        let owner = factions
            .iter()
            .find(|f| f.id == squad.faction_id)
            .map_or_else(|| squad.faction_id.to_string(), faction_label);
        let name = if squad.name.trim().is_empty() {
            squad.id.to_string()
        } else {
            squad.name.clone()
        };
        return Err(format!(
            "Squad {name} belongs to {owner}, not {picked} — pick a squad under {picked}, or switch the faction first."
        ));
    }
    Ok(squad.id.to_string())
}

/// Apply reassign_slots to explicit document state.
pub fn reassign_slots(core: &MissionDocCore, ids: Vec<String>, dest: String) -> usize {
    let mut moved = 0usize;
    for id in &ids {
        match core.slot_squad_id(id.as_str()) {
            None => continue,
            Some(current) if current == dest => continue,
            Some(_) => {}
        }
        core.move_slot_to_squad_keep_source(id.as_str(), dest.as_str());
        moved += 1;
    }
    moved
}

/// Document operation over explicit authored state.
pub fn restore_moves(
    core: &MissionDocCore,
    snapshot: &[super::attrs::SlotAttrs],
) -> Option<Vec<(String, String)>> {
    let squads = squad_rows(core);
    Some(
        snapshot
            .iter()
            .filter(|snap| {
                core.slot_squad_id(snap.id.as_str())
                    .is_some_and(|s| s != snap.squad)
                    && squads.iter().any(|s| s.id == *snap.squad)
            })
            .map(|snap| (snap.id.to_string(), snap.squad.clone()))
            .collect::<Vec<_>>(),
    )
}

/// Document operation over explicit authored state.
pub fn restore_slot_squads(core: &MissionDocCore, moves: Vec<(String, String)>) -> usize {
    for (id, squad) in &moves {
        core.move_slot_to_squad_keep_source(id.as_str(), squad.as_str());
    }
    moves.len()
}
