//! The registered draft-persist hook the undo driver's edit tail runs after a committed edit.
//!
//! **Role:** holds the one function that arms the draft write for a committed edit, so the undo
//! driver (`history::after_doc_change`) can schedule it without naming the session layer that
//! owns the IndexedDB draft writer.
//! **Position:** part of the engine seam, under the session. The session registers its
//! `persist::schedule_edit_persist` here (`session::persist::register_edit_persist`), and the
//! Mission Creator page runs that registration first thing at mount, before the canvas mount
//! installs the history context; the undo driver calls `schedule_edit_persist`.
//! **Signals & state:** one thread-local cell, `EDIT_PERSIST_HOOK`, holding the registered hook;
//! `None` until the first page mount registers it.
//! **Invariants:** a registration replaces the previous one, so a remount never stacks two
//! writers; the registered hook is a capture-free function, so a registration that outlives its
//! page is harmless. With nothing registered, an edit arms no draft write: the edit stays in the
//! document and the dirty flag still marks it unsaved.

use std::cell::RefCell;
use std::rc::Rc;

use mission_document::MissionDocCore;

/// The shared document cell the hook receives, the same shape as the hosted document's
/// `DocHandle`.
pub type EditPersistDocument = Rc<RefCell<Option<MissionDocCore>>>;

/// The registered draft-persist hook: `(document, mission_id)`.
pub type EditPersistHook = Rc<dyn Fn(EditPersistDocument, &mission_model::ids::MissionId)>;

thread_local! {
    /// The draft-persist hook the session registers. Thread-local because the document cell it
    /// receives is a `!Send` `Rc`.
    static EDIT_PERSIST_HOOK: RefCell<Option<EditPersistHook>> = const { RefCell::new(None) };
}

/// Registers the draft-persist hook, replacing any earlier registration.
pub fn register_edit_persist_hook(hook: EditPersistHook) {
    EDIT_PERSIST_HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
}

/// Arms the draft write for one committed edit through the registered hook. Returns `true` when a
/// hook ran and `false` when none is registered, in which case no draft write is armed.
pub fn schedule_edit_persist(
    doc: EditPersistDocument,
    mission_id: &mission_model::ids::MissionId,
) -> bool {
    let hook = EDIT_PERSIST_HOOK.with(|slot| slot.borrow().clone());
    match hook {
        Some(hook) => {
            hook(doc, mission_id);
            true
        }
        None => false,
    }
}

#[cfg(test)]
#[path = "../tests/document_host/edit_persist_hook.rs"]
mod tests;
