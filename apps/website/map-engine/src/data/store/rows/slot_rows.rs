//! Role: slot rows.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::ENTITY_IDS;
use super::HashSet;
use super::MapRef;
use super::Out;
use super::ReadTxn;
use super::RemintMap;
use super::SLOT_IDS;
use super::TransactionMut;
use super::position_any_merged;
use super::read_id_array;
use super::read_position;
use super::read_position_map;
use super::read_str;
use super::retain_ids;
use super::retain_in;
use super::slot_is_transform_locked;
use yrs::Map;

/// Canonical paste known slot keys value.
pub(super) const PASTE_KNOWN_SLOT_KEYS: &[&str] = &[
    "id",
    "squadId",
    "index",
    "role",
    "tag",
    "assetId",
    "position",
    "stance",
    "loadoutId",
    "loadout",
];

/// Set leader in txn using the supplied domain data.
pub(super) fn set_leader_in_txn(
    txn: &mut TransactionMut,
    squads: &MapRef,
    squad_id: &str,
    slot_id: &str,
) {
    let ids = read_id_array(txn, squads, squad_id, "slotIds");
    if !ids
        .iter()
        .any(|a| matches!(a, Any::String(s) if s.as_ref() == slot_id))
    {
        return;
    }
    if let Some(Out::YMap(sq)) = squads.get(txn, squad_id) {
        sq.insert(txn, "leaderSlotId", slot_id);
    }
}

/// Rewrite slot indices using the supplied domain data.
pub(super) fn rewrite_slot_indices(
    txn: &mut TransactionMut,
    slots: &MapRef,
    squads: &MapRef,
    squad_id: &str,
) {
    let ids = read_id_array(txn, squads, squad_id, "slotIds");
    for (i, any) in ids.iter().enumerate() {
        let Any::String(sid) = any else {
            continue;
        };
        if let Some(Out::YMap(slot)) = slots.get(txn, sid.as_ref()) {
            slot.insert(txn, "index", Any::BigInt(i as i64));
        }
    }
}

/// Garbage collect squad in txn using the supplied domain data.
pub(super) fn garbage_collect_squad_in_txn(
    txn: &mut TransactionMut,
    squads: &MapRef,
    factions: &MapRef,
    vehicles: &MapRef,
    squad_id: &str,
) {
    if squads.get(txn, squad_id).is_none() {
        return;
    }
    let faction_id = match squads.get(txn, squad_id).and_then(|o| match o {
        Out::YMap(sq) => sq.get(txn, "factionId"),
        _ => None,
    }) {
        Some(Out::Any(Any::String(f))) => f.to_string(),
        _ => String::new(),
    };
    let vehicle_ids = read_id_array(txn, squads, squad_id, "vehicleIds");
    for vid in &vehicle_ids {
        if let Any::String(id) = vid {
            vehicles.remove(txn, id.as_ref());
        }
    }
    if !faction_id.is_empty()
        && let Some(Out::YMap(f)) = factions.get(txn, faction_id.as_str())
    {
        let arr = read_id_array(txn, factions, faction_id.as_str(), "squadIds");
        let remove: HashSet<&str> = HashSet::from([squad_id]);
        let kept = retain_ids(&arr, &remove);
        f.insert(txn, "squadIds", Any::Array(kept.into()));
    }
    squads.remove(txn, squad_id);
}

/// Ensure leader invariant in txn using the supplied domain data.
pub(super) fn ensure_leader_invariant_in_txn(
    txn: &mut TransactionMut,
    squads: &MapRef,
    factions: &MapRef,
    vehicles: &MapRef,
    squad_id: &str,
) {
    let ids = read_id_array(txn, squads, squad_id, "slotIds");
    if ids.is_empty() {
        garbage_collect_squad_in_txn(txn, squads, factions, vehicles, squad_id);
        return;
    }
    let leader_ok = match squads.get(txn, squad_id).and_then(|o| match o {
        Out::YMap(sq) => sq.get(txn, "leaderSlotId"),
        _ => None,
    }) {
        Some(Out::Any(Any::String(l))) => ids
            .iter()
            .any(|a| matches!(a, Any::String(s) if s.as_ref() == l.as_ref())),
        _ => false,
    };
    if !leader_ok && let Some(Any::String(first)) = ids.first() {
        set_leader_in_txn(txn, squads, squad_id, first.as_ref());
    }
}

/// Squad or faction is merged using the supplied domain data.
pub(super) fn squad_or_faction_is_merged(remint: &RemintMap, old_id: &str) -> bool {
    remint.merged.contains(old_id)
}

/// Update slot in txn using the supplied domain data.
pub(super) fn update_slot_in_txn(
    txn: &mut TransactionMut,
    slots: &MapRef,
    id: &str,
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
) {
    if let Some(Out::YMap(slot)) = slots.get(&*txn, id) {
        if let Some(r) = role {
            slot.insert(&mut *txn, "role", r);
        }
        if let Some(t) = tag {
            slot.insert(&mut *txn, "tag", t);
        }
        if let Some(s) = stance {
            slot.insert(&mut *txn, "stance", s);
        }
    }
}

/// Update slot object in txn using the supplied domain data.
pub(super) fn update_slot_object_in_txn(
    txn: &mut TransactionMut,
    slots: &MapRef,
    id: &str,
    asset_id: Option<String>,
    description: Option<String>,
) {
    if let Some(Out::YMap(slot)) = slots.get(&*txn, id) {
        for (key, val) in [("assetId", asset_id), ("description", description)] {
            let Some(v) = val else { continue };
            if v.is_empty() {
                slot.remove(&mut *txn, key);
            } else {
                slot.insert(&mut *txn, key, v);
            }
        }
    }
}

/// Update slot position in txn using the supplied domain data.
#[allow(clippy::too_many_arguments)]
pub(super) fn update_slot_position_in_txn(
    txn: &mut TransactionMut,
    slots: &MapRef,
    editor_layers: &MapRef,
    id: &str,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
    width: f64,
    height: f64,
) -> bool {
    if slot_is_transform_locked(&*txn, editor_layers, id) {
        return false;
    }
    let Some(Out::YMap(slot)) = slots.get(&*txn, id) else {
        return false;
    };
    let (mut px, mut py, mut pz, mut prot) = read_position(txn, &slot);
    if let Some(nx) = x.filter(|v| v.is_finite()) {
        px = nx.clamp(0.0, width);
    }
    if let Some(ny) = y.filter(|v| v.is_finite()) {
        py = ny.clamp(0.0, height);
    }
    if let Some(nr) = rotation.filter(|v| v.is_finite()) {
        prot = ((nr % 360.0) + 360.0) % 360.0;
    }
    if let Some(nz) = z.filter(|v| v.is_finite()) {
        pz = nz;
    } else if x.is_some() || y.is_some() {
        pz = 0.0;
    }
    let existing = read_position_map(txn, &slot);
    slot.insert(
        &mut *txn,
        "position",
        position_any_merged(existing, px, py, pz, prot),
    );
    true
}

/// Remove slots in txn using the supplied domain data.
pub(super) fn remove_slots_in_txn(
    txn: &mut TransactionMut,
    slots: &MapRef,
    squads: &MapRef,
    editor_layers: &MapRef,
    ids: &[String],
) {
    if ids.is_empty() {
        return;
    }
    let id_set: HashSet<&str> = ids.iter().map(String::as_str).collect();

    let mut affected: HashSet<String> = HashSet::new();
    for id in ids {
        if let Some(Out::YMap(slot)) = slots.get(&*txn, id.as_str())
            && let Some(Out::Any(Any::String(sid))) = slot.get(&*txn, "squadId")
        {
            affected.insert(sid.to_string());
        }
    }
    for sid in &affected {
        if let Some(Out::YMap(squad)) = squads.get(&*txn, sid) {
            retain_in(&mut *txn, &squad, SLOT_IDS, &id_set);
        }
    }

    let layer_ids: Vec<String> = editor_layers
        .iter(&*txn)
        .map(|(k, _)| k.to_string())
        .collect();
    for lid in &layer_ids {
        if let Some(Out::YMap(layer)) = editor_layers.get(&*txn, lid) {
            retain_in(&mut *txn, &layer, ENTITY_IDS, &id_set);
        }
    }

    for id in ids {
        slots.remove(&mut *txn, id.as_str());
    }
}

/// Set slot editor hidden in txn using the supplied domain data.
pub(super) fn set_slot_editor_hidden_in_txn(
    txn: &mut TransactionMut,
    slots: &MapRef,
    id: &str,
    hidden: bool,
) {
    if let Some(Out::YMap(slot)) = slots.get(&*txn, id) {
        if hidden {
            slot.insert(&mut *txn, "editorHidden", true);
        } else {
            slot.remove(&mut *txn, "editorHidden");
        }
    }
}

/// Resolve slot side key using the supplied domain data.
pub(super) fn resolve_slot_side_key<T: ReadTxn>(
    txn: &T,
    squads: &MapRef,
    factions: &MapRef,
    squad_id: &str,
) -> String {
    if squad_id.is_empty() {
        return String::from("BLUFOR");
    }
    let Some(Out::YMap(sq)) = squads.get(txn, squad_id) else {
        return String::from("BLUFOR");
    };
    let Some(faction_id) = read_str(txn, &sq, "factionId") else {
        return String::from("BLUFOR");
    };
    if faction_id.is_empty() {
        return String::from("BLUFOR");
    }
    let Some(Out::YMap(f)) = factions.get(txn, faction_id.as_str()) else {
        return String::from("BLUFOR");
    };
    match read_str(txn, &f, "key") {
        Some(k) if !k.is_empty() => k,
        _ => String::from("BLUFOR"),
    }
}
