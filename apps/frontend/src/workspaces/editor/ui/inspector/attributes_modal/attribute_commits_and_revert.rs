//! Attributes modal attribute commits and revert behavior.

#[cfg(target_arch = "wasm32")]
use super::*;

/// Updates one or several selected slot positions through the engine.
#[cfg(target_arch = "wasm32")]
pub(super) fn commit_position(
    targets: StoredValue<Vec<String>>,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) {
    let ids = targets.get_value();
    if ids.len() > 1 {
        engine_ops::attrs_update_position_multi(&ids, x, y, z, rotation);
    } else if let Some(id) = ids.first() {
        engine_ops::attrs_update_position(id.as_str(), x, y, z, rotation);
    }
}

/// Updates one or several selected slot identity fields through the engine.
#[cfg(target_arch = "wasm32")]
pub(super) fn commit_slot(
    targets: StoredValue<Vec<String>>,
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) {
    let ids = targets.get_value();
    if ids.len() > 1 {
        engine_ops::attrs_update_slot_multi(
            &ids,
            role,
            tag,
            stance,
            asset_id.map(Into::into),
            description,
        );
    } else if let Some(id) = ids.first() {
        engine_ops::attrs_update_slot(
            id.as_str(),
            role,
            tag,
            stance,
            asset_id.map(Into::into),
            description,
        );
    }
}

/// Restores each selected slot from its value captured when the modal opened.
#[cfg(target_arch = "wasm32")]
pub(super) fn revert_to_snapshot(snapshot: StoredValue<Vec<mission_operations::attrs::SlotAttrs>>) {
    for snap in snapshot.get_value() {
        engine_ops::attrs_update_position(
            snap.id.as_str(),
            Some(snap.x),
            Some(snap.y),
            Some(snap.z),
            Some(snap.rotation),
        );
        engine_ops::attrs_update_slot(
            snap.id.as_str(),
            Some(snap.role.clone()),
            Some(snap.tag.clone()),
            Some(snap.stance.clone()),
            Some(snap.asset_id.clone()),
            Some(snap.description.clone()),
        );
    }
    engine_ops::restore_slot_squads(&snapshot.get_value());
}
