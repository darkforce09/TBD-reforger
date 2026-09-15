//! Role: compositions.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use crate::editor::state::history as mission_history;
use website_mission_core::doc::MissionDocCore;

/// Expose website mission core :: doc :: operations :: compositions ::  composition row at this domain boundary.
pub use website_mission_core::doc::operations::compositions::CompositionRow;

#[allow(unused_imports)]
use super::{attrs::*, cargo::*, context::*, entity::*, transform::*};

/// Save composition using the supplied domain data.
#[must_use]
pub fn save_composition(title: String, category: String, author: String) -> Option<String> {
    let new_id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let sel: Vec<String> = ctx.selection.borrow().clone();
        if sel.is_empty() {
            return None;
        }
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_mission_core::doc::operations::compositions::save_composition(
            core,
            title,
            category,
            author,
            sel,
            &ctx.next_id,
        )
    });
    if new_id.is_some() {
        mission_history::after_local_edit();
    }
    new_id
}

/// Composition rows using the supplied domain data.
#[must_use]
pub fn composition_rows() -> Vec<CompositionRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        website_mission_core::doc::operations::compositions::composition_rows(core)
    })
}

/// Composition count using the supplied domain data.
#[must_use]
pub fn composition_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| {
                ctx.doc
                    .borrow()
                    .as_ref()
                    .map(MissionDocCore::composition_count)
            })
            .unwrap_or(0)
    })
}

/// Rename composition using the supplied domain data.
pub fn rename_composition(id: String, title: String) -> bool {
    edit_composition(|core| core.set_composition_title(&id, &title))
}

/// Recategorize composition using the supplied domain data.
pub fn recategorize_composition(id: String, category: String) -> bool {
    edit_composition(|core| core.set_composition_category(&id, &category))
}

/// Set composition author using the supplied domain data.
pub fn set_composition_author(id: String, author: String) -> bool {
    edit_composition(|core| core.set_composition_author(&id, &author))
}

/// Delete composition using the supplied domain data.
pub fn delete_composition(id: String) -> bool {
    let did = edit_composition(|core| core.remove_composition(&id));
    if did {
        OPS_CTX.with(|c| {
            if let Some(ctx) = c.borrow().as_ref() {
                let clear =
                    matches!(&*ctx.pending.borrow(), Some(Pending::Composition(p)) if *p == id);
                if clear {
                    *ctx.pending.borrow_mut() = None;
                }
            }
        });
    }
    did
}

/// Shared edit tail for the composition mutators: run `f` against the core, then the dirty tail (one undo step). Returns `false` when there is no doc.
pub(super) fn edit_composition(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}
