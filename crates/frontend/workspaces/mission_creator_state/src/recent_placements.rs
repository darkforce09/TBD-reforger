//! The recently-placed recorder cell: the one place a placement made outside the right dock
//! records itself into the dock's session list.
//!
//! **Role:** holds the recorder the right dock registers at mount — `(asset_id, label)`, the key
//! and label of one recently-placed row — and runs it for a placement made elsewhere (a
//! composition stamp, an ORBAT Add-Vehicle).
//! **Position:** part of the editor's state layer. The right dock's recent list registers into it
//! (`ui::docks::dock_right`); the bridge's map release and the ORBAT manager record through
//! [`record_placed`](crate::recent_placements::record_placed).
//! **Signals & state:** one thread-local cell holding the mounted recorder, `None` before the dock
//! mounts and after it unmounts.
//! **Invariants:** only the registration a dock installed is cleared by that dock's cleanup, so a
//! remount's newer recorder survives the old dock's unmount; recording with no dock mounted is a
//! no-op, since the placement itself has already committed.

/// The registered recently-placed recorder: `(asset_id, label)`. Closes over the dock's `!Send`
/// list signal, which no off-dock caller can hold.
pub type RecentRecorder = std::rc::Rc<dyn Fn(mission_validation::AssetId, String)>;

thread_local! {
    /// The recently-placed recorder hook. Thread-local because it closes over a `!Send`
    /// `RwSignal` owned by the mounted dock.
    static RECENT_RECORDER: std::cell::RefCell<Option<RecentRecorder>> =
        const { std::cell::RefCell::new(None) };
}

/// Registers the recently-placed recorder (the right dock calls this once at mount, paired with
/// [`unregister_recent_recorder`] at unmount).
pub fn register_recent_recorder(f: RecentRecorder) {
    RECENT_RECORDER.with(|c| *c.borrow_mut() = Some(f));
}

/// Unregisters the recorder at the dock's unmount — but only if `f` is still the live
/// registration. A remount can install its newer recorder before the old component's cleanup
/// runs, and an unconditional clear would delete the live one.
pub fn unregister_recent_recorder(f: &RecentRecorder) -> bool {
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

/// Records a placement made outside the right dock (a composition stamp, an ORBAT Add-Vehicle)
/// into the session recently-placed list, through the mount-registered recorder. A no-op when no
/// dock is mounted: the placement itself already committed, and the recent list is a convenience
/// with nothing to add to when nothing is listening.
pub fn record_placed(asset_id: mission_validation::AssetId, label: String) {
    let hook = RECENT_RECORDER.with(|c| c.borrow().clone());
    if let Some(f) = hook {
        f(asset_id, label);
    }
}
