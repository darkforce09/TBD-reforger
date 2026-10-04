//! Snapshot for the ORBAT manager.

#[cfg(target_arch = "wasm32")]
use super::*;

#[derive(Clone, Debug, Default)]
/// A slot projected from the mission document for the inspector.
#[cfg(target_arch = "wasm32")]
pub(super) struct SlotDetail {
    pub(super) id: String,
    pub(super) role: String,
    pub(super) tag: String,
    pub(super) callsign: String,
    pub(super) rank: String,
    pub(super) index: u32,
    pub(super) squad_id: String,
    pub(super) summary: String,
    pub(super) primary: String,
    pub(super) launcher: String,
}

#[derive(Clone, Debug, Default)]
/// The mission ORBAT snapshot used to render the dialog.
#[cfg(target_arch = "wasm32")]
pub(super) struct Snap {
    pub(super) factions: Vec<mission_operations::rows::FactionRow>,
    pub(super) squads: Vec<mission_operations::rows::SquadRow>,
    pub(super) slots: Vec<SlotDetail>,
}

/// Reads the active mission ORBAT snapshot.
#[cfg(target_arch = "wasm32")]
pub(super) fn read_snapshot() -> Snap {
    {
        let s = engine_ops::orbat_manager_snapshot();
        Snap {
            factions: s.factions,
            squads: s.squads,
            slots: s
                .slots
                .into_iter()
                .map(|d| SlotDetail {
                    id: d.id.into_inner(),
                    role: d.role,
                    tag: d.tag,
                    callsign: d.callsign,
                    rank: d.rank,
                    index: d.index,
                    squad_id: d.squad_id.into_inner(),
                    summary: d.summary,
                    primary: d.primary,
                    launcher: d.launcher,
                })
                .collect(),
        }
    }
}

/// Projects snapshot slots into outliner rows.
#[cfg(target_arch = "wasm32")]
pub(super) fn slot_rows_from(snap: &Snap) -> Vec<mission_operations::rows::SlotRow> {
    snap.slots
        .iter()
        .map(|s| mission_operations::rows::SlotRow {
            id: s.id.clone().into(),
            role: s.role.clone(),
        })
        .collect()
}

/// Filters outliner nodes by the active search query.
#[cfg(target_arch = "wasm32")]
pub(super) fn filter_search(nodes: Vec<OutlinerNode>, q: &str) -> Vec<OutlinerNode> {
    nodes
        .into_iter()
        .filter_map(|mut sq| {
            let squad_hit = sq.label.to_lowercase().contains(q);
            let kids: Vec<_> = sq
                .children
                .into_iter()
                .filter(|c| c.label.to_lowercase().contains(q) || squad_hit)
                .collect();
            if squad_hit || !kids.is_empty() {
                sq.children = kids;
                Some(sq)
            } else {
                None
            }
        })
        .collect()
}
