//! Role: a canonical, order-independent fingerprint of a document's materialized slots.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none; pure over the document it is handed.
//! Invariants: two documents holding the same slot data produce the same string, whatever order
//! their rows materialize in and whatever order their string dictionaries interned in. Floats are
//! compared bit-exactly, so the fingerprint never hides a coordinate that merely rounds the same.

use crate::data::store::MissionDocCore;

/// A canonical fingerprint of the materialized slots: one sorted line per slot, every interned
/// index resolved to its string, every float as its exact bits.
///
/// This is what a reload comparison asks about — cold document against warm — and **not** the
/// encode bytes. `encode_state_as_update_v1` is deterministic for the same document but is not
/// byte-identical between a document and a fresh peer that replayed its update: only the
/// *materialization* is equal. A byte compare would therefore report a difference where none
/// exists; this fingerprint reports one only when the slot data actually differs.
///
/// Rows are sorted rather than emitted in materialize order, and indices are resolved rather than
/// printed, because both orders are arbitrary: the row order is the map's and the dictionary order
/// is first-seen. Either one leaking into the string would make two identical documents disagree.
#[must_use]
pub fn slots_digest(core: &MissionDocCore) -> String {
    let soa = core.materialize();
    let get = |dict: &[String], idx: u32| {
        dict.get(idx as usize)
            .map_or("", String::as_str)
            .to_string()
    };
    let mut rows: Vec<String> = (0..soa.ids.len())
        .map(|i| {
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                soa.ids[i],
                soa.xs[i].to_bits(),
                soa.ys[i].to_bits(),
                soa.zs[i].to_bits(),
                soa.rotations[i].to_bits(),
                soa.stance[i],
                get(&soa.roles, soa.role_idx[i]),
                get(&soa.tags, soa.tag_idx[i]),
                get(&soa.squads, soa.squad_idx[i]),
                get(&soa.layers, soa.layer_idx[i]),
            )
        })
        .collect();
    // Canonical: each row begins with a unique slot id, so a plain sort is a total order that
    // neither materialize order nor dictionary intern order can perturb.
    rows.sort();
    rows.join("\n")
}

#[cfg(test)]
#[path = "tests/slot_fingerprint.rs"]
mod tests;
