//! Right dock recent behavior.

#[cfg(any(test, target_arch = "wasm32"))]
use super::*;

/// one recently-placed entry: the asset id (`resource_name`) and the label to show. Same
/// `asset_id` a leaf carries, so a recent row arms the identical place a fresh palette leaf would.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentPlaced {
    /// The placed asset.
    pub asset_id: mission_validation::AssetId,
    /// The label the list shows.
    pub label: String,
}

/// the pure list transform behind [`record_recent`]: move `asset_id` to the head
/// (most-recent-first), dedup by id so a re-place bumps the existing entry rather than duplicating it,
/// and cap at [`FAVOURITES_MAX`]. Split out from the signal write so the ordering/dedup/cap contract
/// is native-testable without a reactive runtime.
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) fn push_recent_into(
    list: &mut Vec<RecentPlaced>,
    asset_id: mission_validation::AssetId,
    label: String,
) {
    list.retain(|r| r.asset_id != asset_id);
    list.insert(0, RecentPlaced { asset_id, label });
    list.truncate(FAVOURITES_MAX);
}

/// push an asset to the head of the session recently-placed list. Thin signal wrapper over
/// [`push_recent_into`]; no storage, no document (session-scoped, per the UX-review summary).
#[cfg(target_arch = "wasm32")]
pub(super) fn record_recent(
    recent: RwSignal<Vec<RecentPlaced>>,
    asset_id: mission_validation::AssetId,
    label: String,
) {
    recent.update(|list| push_recent_into(list, asset_id, label));
}

#[cfg(target_arch = "wasm32")]
use mission_creator_state::recent_placements::{
    RecentRecorder, register_recent_recorder, unregister_recent_recorder,
};

/// Install the recorder for the CURRENT reactive owner: register now, unregister at unmount. Mirrors
/// [`install_select_zone`] — the `StoredValue` clone keeps the `Rc` alive so the `ptr_eq` identity is
/// meaningful, and `on_cleanup` drops the registration when Backspace hide-chrome (or a mission
/// switch) unmounts the dock, so a later placement finds `None` and no-ops rather than writing into a
/// disposed `recent` signal.
#[cfg(target_arch = "wasm32")]
pub(super) fn install_recent_recorder(f: RecentRecorder) {
    let mine = StoredValue::new_local(std::rc::Rc::clone(&f));
    register_recent_recorder(f);
    on_cleanup(move || {
        let _ = mine.try_with_value(unregister_recent_recorder);
    });
}
