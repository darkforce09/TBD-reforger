//! Role: reassign.
//! Position: `doc/operations` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::projections::squad_rows;
use crate::data::store::MissionDocCore;

/// Where a batch reassign sends the selection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReassignTarget {
    /// The faction picked in the modal (a `factionsById` row id).
    pub faction_id: String,

    /// The squad picked in the modal, or empty for "the faction's first squad".
    pub squad_id: String,
}

/// Faction label using the supplied domain data.
#[must_use]
pub fn faction_label(f: &super::rows::FactionRow) -> String {
    match (f.name.trim(), f.key.trim()) {
        ("", "") => f.id.clone(),
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
    faction_id: &str,
    squad_id: &str,
) -> Result<String, String> {
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
            .find(|sid| squads.iter().any(|s| &&s.id == sid))
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
            .map_or_else(|| squad.faction_id.clone(), faction_label);
        let name = if squad.name.trim().is_empty() {
            squad.id.clone()
        } else {
            squad.name.clone()
        };
        return Err(format!(
            "Squad {name} belongs to {owner}, not {picked} — pick a squad under {picked}, or switch the faction first."
        ));
    }
    Ok(squad.id.clone())
}

/// Apply reassign_slots to explicit document state.
pub fn reassign_slots(core: &MissionDocCore, ids: Vec<String>, dest: String) -> usize {
    let mut moved = 0usize;
    for id in &ids {
        match core.slot_squad_id(id) {
            None => continue,
            Some(current) if current == dest => continue,
            Some(_) => {}
        }
        core.move_slot_to_squad_keep_source(id, &dest);
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
                core.slot_squad_id(&snap.id)
                    .is_some_and(|s| s != snap.squad)
                    && squads.iter().any(|s| s.id == snap.squad)
            })
            .map(|snap| (snap.id.clone(), snap.squad.clone()))
            .collect::<Vec<_>>(),
    )
}

/// Document operation over explicit authored state.
pub fn restore_slot_squads(core: &MissionDocCore, moves: Vec<(String, String)>) -> usize {
    for (id, squad) in &moves {
        core.move_slot_to_squad_keep_source(id, squad);
    }
    moves.len()
}
