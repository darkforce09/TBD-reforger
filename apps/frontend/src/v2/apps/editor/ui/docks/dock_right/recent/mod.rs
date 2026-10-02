//! Right dock recent behavior.

use super::*;

/// one recently-placed entry: the asset id (`resource_name`) and the label to show. Same
/// `asset_id` a leaf carries, so a recent row arms the identical place a fresh palette leaf would.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentPlaced {
    pub asset_id: String,
    pub label: String,
}

/// the pure list transform behind [`record_recent`]: move `asset_id` to the head
/// (most-recent-first), dedup by id so a re-place bumps the existing entry rather than duplicating it,
/// and cap at [`FAVOURITES_MAX`]. Split out from the signal write so the ordering/dedup/cap contract
/// is native-testable without a reactive runtime.
pub(super) fn push_recent_into(list: &mut Vec<RecentPlaced>, asset_id: String, label: String) {
    list.retain(|r| r.asset_id != asset_id);
    list.insert(0, RecentPlaced { asset_id, label });
    list.truncate(FAVOURITES_MAX);
}

/// push an asset to the head of the session recently-placed list. Thin signal wrapper over
/// [`push_recent_into`]; no storage, no document (session-scoped, per the UX-review summary).
pub(super) fn record_recent(recent: RwSignal<Vec<RecentPlaced>>, asset_id: String, label: String) {
    recent.update(|list| push_recent_into(list, asset_id, label));
}

/// the registered recently-placed recorder: `(asset_id, label)`, the exact
/// [`push_recent_into`] key/label. Set once at [`DockRight`] mount; `None` on the host / pre-mount /
/// while the dock is unmounted. Peer of [`ZoneSelectHook`].
type RecentRecorder = std::rc::Rc<dyn Fn(String, String)>;

thread_local! {
 /// The recently-placed recorder hook. Peer of [`SELECT_ZONE`]; thread_local for the same reason —
 /// it closes over `recent`, a `!Send` `RwSignal` owned by `DockRight` that no off-dock caller can
 /// hold.
    static RECENT_RECORDER: std::cell::RefCell<Option<RecentRecorder>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the recently-placed recorder (called once at [`DockRight`] mount).
///
/// Prefer [`install_recent_recorder`] from inside a component: a bare register with no matching
/// unregister is the F2 defect [`install_select_zone`] documents.
pub(super) fn register_recent_recorder(f: RecentRecorder) {
    RECENT_RECORDER.with(|c| *c.borrow_mut() = Some(f));
}

/// Unregister the recorder at [`DockRight`] unmount — but ONLY if `f` is still the LIVE registration.
/// The `Rc::ptr_eq` guard is [`unregister_select_zone`]'s: a remount can install its newer recorder
/// before the old component's cleanup runs, and an unconditional clear would delete the live one.
pub(super) fn unregister_recent_recorder(f: &RecentRecorder) -> bool {
    let taken = RECENT_RECORDER.with(|c| {
        let mut slot = c.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|live| std::rc::Rc::ptr_eq(live, f))
        {
            slot.take()
        } else {
            None
        }
    });
    taken.is_some()
}

/// Install the recorder for the CURRENT reactive owner: register now, unregister at unmount. Mirrors
/// [`install_select_zone`] — the `StoredValue` clone keeps the `Rc` alive so the `ptr_eq` identity is
/// meaningful, and `on_cleanup` drops the registration when Backspace hide-chrome (or a mission
/// switch) unmounts the dock, so a later placement finds `None` and no-ops rather than writing into a
/// disposed `recent` signal.
pub(super) fn install_recent_recorder(f: RecentRecorder) {
    let mine = StoredValue::new_local(std::rc::Rc::clone(&f));
    register_recent_recorder(f);
    on_cleanup(move || {
        let _ = mine.try_with_value(unregister_recent_recorder);
    });
}

/// record a placement made OUTSIDE this dock (composition stamp / ORBAT Add-Vehicle) into the
/// session recently-placed list, through the mount-registered recorder. A no-op when no dock is
/// mounted (host / pre-mount / hidden chrome) — that is not a dropped ack (see the seam note): the
/// placement itself already committed, and the recent list is a convenience with nothing to add to
/// when nothing is listening. Routes through [`record_recent`], so the head/dedup/cap contract holds.
pub(crate) fn record_placed(asset_id: String, label: String) {
    let hook = RECENT_RECORDER.with(|c| c.borrow().clone());
    if let Some(f) = hook {
        f(asset_id, label);
    }
}
