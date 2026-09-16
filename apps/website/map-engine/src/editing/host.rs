//! Role: the live authored document, and the one borrow chain every editing command reaches it by.
//! Position: `editing` in the map engine.
//! Signals & state: one installed host per thread — the document handle, the selected-id set, and
//! the id minter.
//! Invariants: the handle is the SAME shared cell a restore or a hydrate swaps into, so a command
//! always sees the live document rather than a snapshot taken at mount. Every entry point below
//! opens exactly one borrow and drops it before returning, so a caller can never hold a read
//! across a write. Nothing here is a reactive signal: what a host chooses to mirror into its own
//! surface is the host's business and is not modelled on this side of the wall.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::data::store::MissionDocCore;

/// The hosted document. A shared cell rather than an owned value: a restore or a server hydrate
/// replaces the document wholesale, and every command must see the replacement without being
/// re-registered.
pub type DocHandle = Rc<RefCell<Option<MissionDocCore>>>;

/// The app-side selected-id set. Selection is not document state — a mission is the same mission
/// whatever is highlighted.
pub type SelectionHandle = Rc<RefCell<Vec<String>>>;

/// Everything a document command reaches that is not presentation.
pub struct EditingHost {
    /// The live document.
    pub doc: DocHandle,

    /// The selected ids.
    pub selection: SelectionHandle,

    /// Monotonic minter for placed ids; every mint still proves uniqueness against the document.
    pub next_id: Cell<u32>,
}

thread_local! {
    static HOST: RefCell<Option<EditingHost>> = const { RefCell::new(None) };
}

/// Install the host, once, after the document exists. A later install replaces the earlier one
/// wholesale — a second mount is a second document.
pub fn install(doc: DocHandle, selection: SelectionHandle) {
    HOST.with(|h| {
        *h.borrow_mut() = Some(EditingHost {
            doc,
            selection,
            next_id: Cell::new(0),
        });
    });
}

/// Run `f` against the installed host. `None` before any host is installed, which every caller
/// reports as "nothing happened" rather than as a failure.
pub fn with_host<R>(f: impl FnOnce(&EditingHost) -> R) -> Option<R> {
    HOST.with(|h| h.borrow().as_ref().map(f))
}

/// Run `f` against the live document. `None` before a host is installed or while the handle holds
/// no document.
pub fn with_doc<R>(f: impl FnOnce(&MissionDocCore) -> R) -> Option<R> {
    with_host(|host| host.doc.borrow().as_ref().map(f)).flatten()
}

/// Run `f` against the live document mutably. Separate from [`with_doc`] because the two borrows
/// cannot overlap: a caller that needs both must let one end first.
pub fn with_doc_mut<R>(f: impl FnOnce(&mut MissionDocCore) -> R) -> Option<R> {
    with_host(|host| host.doc.borrow_mut().as_mut().map(f)).flatten()
}

/// A clone of the live document handle — the same shared cell a restore swaps into. For callers
/// that need the document outside the install scope.
#[must_use]
pub fn doc_handle() -> Option<DocHandle> {
    with_host(|host| host.doc.clone())
}

/// A snapshot of the selected ids.
#[must_use]
pub fn selection_ids() -> Vec<String> {
    with_host(|host| host.selection.borrow().clone()).unwrap_or_default()
}

/// How many ids are selected.
#[must_use]
pub fn selection_len() -> usize {
    with_host(|host| host.selection.borrow().len()).unwrap_or(0)
}

/// Replace the selected ids.
pub fn set_selection_ids(ids: Vec<String>) {
    with_host(|host| *host.selection.borrow_mut() = ids);
}

/// Drop from the selection every id the live document no longer holds, decided by `live` — the
/// caller's answer to "what is selectable now", read from the POST-change document.
///
/// One prune, over one universe, at every post-change site. Two copies is how one of them gets
/// widened and the other does not.
pub fn retain_selected(live: &dyn Fn(&str) -> bool) {
    with_host(|host| host.selection.borrow_mut().retain(|id| live(id)));
}
