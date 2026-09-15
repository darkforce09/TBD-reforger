//! Role: Module boundary for doc/crdt/undo_groups/tests.
//! Position: `doc/crdt/undo_groups/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::doc::MissionDocCore;

fn slot_doc(clock: Arc<dyn Clock>) -> MissionDocCore {
    let doc = MissionDocCore::with_undo_clock(clock);
    doc.set_origin_init(true);
    doc.add_slot(
        "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.set_origin_init(false);
    doc
}

fn x_of(doc: &MissionDocCore) -> f32 {
    let soa = doc.materialize();
    let i = soa.ids.iter().position(|id| id == "s0").expect("s0");
    soa.xs[i]
}

mod cases_1;
