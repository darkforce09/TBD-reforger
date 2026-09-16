//! Role: the live authored document, and the one borrow chain every editing command reaches it by.
//! Position: `editing` in the map engine.
//! Signals & state: one installed host per thread — the document handle, the selected-id set, the
//! in-flight placement, and the id minter.
//! Invariants: the handle is the SAME shared cell a restore or a hydrate swaps into, so a command
//! always sees the live document rather than a snapshot taken at mount. Every entry point below
//! opens exactly one borrow and drops it before returning, so a caller can never hold a read
//! across a write. Nothing here is a reactive signal: what a host chooses to mirror into its own
//! surface is the host's business and is not modelled on this side of the wall.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::data::store::MissionDocCore;
use crate::data::store::operations::assets::PlacePayload;
use crate::data::store::operations::zones::{DrawTarget, ZoneShape};

/// The hosted document. A shared cell rather than an owned value: a restore or a server hydrate
/// replaces the document wholesale, and every command must see the replacement without being
/// re-registered.
pub type DocHandle = Rc<RefCell<Option<MissionDocCore>>>;

/// The app-side selected-id set. Selection is not document state — a mission is the same mission
/// whatever is highlighted.
pub type SelectionHandle = Rc<RefCell<Vec<String>>>;

/// The in-flight placement: `Some` between the moment an operator picks something up and the
/// moment the map commits it.
///
/// The discriminant lives on the ARMED VALUE rather than on a separate "current tab" reading: the
/// tab can change, or the surface holding it can go away, between the pick-up and the commit, and
/// a placement must commit the entity the operator actually picked up.
#[derive(Clone, Debug, PartialEq)]
pub enum Pending {
    /// A character.
    Character(PlacePayload),

    /// A vehicle.
    Vehicle(PlacePayload),

    /// A world object.
    Object(PlacePayload),

    /// A saved composition, by id.
    Composition(String),

    /// A map marker, by icon key.
    Marker(String),

    /// A zone or trigger being drawn.
    Zone(ZoneDraft),
}

/// A zone draw in progress. Lives on the armed placement rather than in a reading of its own,
/// because "is a draw in flight" is what routes a map release to the draw instead of the select
/// machine — and re-deriving that from a second source is how the two get out of step.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneDraft {
    /// The authored `zone.type`, taken from the schema by whoever armed the draw — never typed.
    pub kind: String,

    /// Circle or polygon.
    pub shape: ZoneShape,

    /// Circle: the centre, set by the first click. `None` until then.
    pub centre: Option<(f64, f64)>,

    /// Polygon: the ring so far, one vertex per click.
    pub verts: Vec<(f64, f64)>,

    /// The id being reshaped, when this draw is editing an existing row rather than minting one.
    pub target: Option<String>,

    /// Which authored collection the draw commits into.
    pub collection: DrawTarget,
}

/// Everything a document command reaches that is not presentation.
pub struct EditingHost {
    /// The live document.
    pub doc: DocHandle,

    /// The selected ids.
    pub selection: SelectionHandle,

    /// The in-flight placement, if any.
    pub pending: RefCell<Option<Pending>>,

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
            pending: RefCell::new(None),
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
