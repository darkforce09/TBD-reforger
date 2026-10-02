//! ORBAT tree construction for the editor outliner.

use super::*;

/// Builds the ORBAT browse tree from factions, squads, and slots in document order.
pub fn build_orbat(
    factions: &[FactionRow],
    squads: &[SquadRow],
    slots: &[SlotRow],
) -> Vec<OutlinerNode> {
    let squad_by_id = |id: &str| squads.iter().find(|s| s.id == id);
    let slot_by_id = |id: &str| slots.iter().find(|s| s.id == id);

    let mut out: Vec<OutlinerNode> = Vec::new();
    // Deterministic faction order (doc map iteration is arbitrary).
    let mut ordered: Vec<&FactionRow> = factions.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    for f in ordered {
        let squad_nodes: Vec<OutlinerNode> = f
            .squad_ids
            .iter()
            .filter_map(|sid| squad_by_id(sid))
            .map(|sq| {
                let slot_children: Vec<OutlinerNode> = sq
                    .slot_ids
                    .iter()
                    .filter_map(|id| slot_by_id(id))
                    .map(|s| {
                        let is_leader = !sq.leader_slot_id.is_empty() && s.id == sq.leader_slot_id;
                        slot_node_leader(s, is_leader)
                    })
                    .collect();
                OutlinerNode {
                    id: sq.id.clone(),
                    label: format!("{} ({})", sq.name, slot_children.len()),
                    kind: NodeKind::Squad,
                    children: slot_children,
                    is_leader: false,
                    // ORBAT tree is squad-scoped, not layer-scoped — flags never apply here.
                    hidden: false,
                    locked: false,
                    hidden_effective: false,
                    locked_effective: false,
                    // ORBAT rows are never comments (comments live in the layer tree only).
                    tooltip: String::new(),
                }
            })
            .collect();
        out.push(OutlinerNode {
            id: f.id.clone(),
            label: f.name.clone(),
            kind: NodeKind::Faction,
            children: squad_nodes,
            is_leader: false,
            hidden: false,
            locked: false,
            hidden_effective: false,
            locked_effective: false,
            tooltip: String::new(),
        });
    }
    out
}

/// filter ORBAT tree to squads under factions whose [`FactionRow::key`] equals `side_key`.
/// Uses **key**, never name substring (G8). Returns squad nodes only (side tabs replace faction headers).
#[must_use]
pub fn filter_orbat_squads_by_side_key(
    factions: &[FactionRow],
    squads: &[SquadRow],
    slots: &[SlotRow],
    side_key: &str,
) -> Vec<OutlinerNode> {
    let tree = build_orbat(factions, squads, slots);
    let matching_faction_ids: std::collections::HashSet<&str> = factions
        .iter()
        .filter(|f| f.key == side_key)
        .map(|f| f.id.as_str())
        .collect();
    tree.into_iter()
        .filter(|n| matching_faction_ids.contains(n.id.as_str()))
        .flat_map(|f| f.children)
        .collect()
}

/// G1 — near-fullscreen dialog class list (Faction Manager pattern; not `ui::Dialog` / `max-w-xl`).
pub const ORBAT_MANAGER_DIALOG_CLASS: &str = "glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex h-[min(800px,90vh)] w-[min(1100px,95vw)] max-w-6xl -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none";

/// G7 — empty-state when the filtered side has no squads (never Stitch sample strings).
pub const ORBAT_MANAGER_EMPTY: &str = "No squads on this side yet — place a unit or add a squad.";
