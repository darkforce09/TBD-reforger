//! Snapshot for the ORBAT manager.

use super::*;

#[derive(Clone, Debug, Default)]
/// A slot projected from the mission document for the inspector.
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
pub(super) struct Snap {
    pub(super) factions: Vec<crate::v2::apps::editor::ui::outliner::outliner::FactionRow>,
    pub(super) squads: Vec<crate::v2::apps::editor::ui::outliner::outliner::SquadRow>,
    pub(super) slots: Vec<SlotDetail>,
}

/// Reads the active mission ORBAT snapshot.
pub(super) fn read_snapshot() -> Snap {
    #[cfg(target_arch = "wasm32")]
    {
        let s = engine_ops::orbat_manager_snapshot();
        Snap {
            factions: s.factions,
            squads: s.squads,
            slots: s
                .slots
                .into_iter()
                .map(|d| SlotDetail {
                    id: d.id,
                    role: d.role,
                    tag: d.tag,
                    callsign: d.callsign,
                    rank: d.rank,
                    index: d.index,
                    squad_id: d.squad_id,
                    summary: d.summary,
                    primary: d.primary,
                    launcher: d.launcher,
                })
                .collect(),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Snap::default()
    }
}

/// Projects snapshot slots into outliner rows.
pub(super) fn slot_rows_from(
    snap: &Snap,
) -> Vec<crate::v2::apps::editor::ui::outliner::outliner::SlotRow> {
    snap.slots
        .iter()
        .map(
            |s| crate::v2::apps::editor::ui::outliner::outliner::SlotRow {
                id: s.id.clone(),
                role: s.role.clone(),
            },
        )
        .collect()
}

/// Filters outliner nodes by the active search query.
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
                if !squad_hit {
                    sq.children = kids;
                } else {
                    sq.children = kids;
                }
                Some(sq)
            } else {
                None
            }
        })
        .collect()
}
