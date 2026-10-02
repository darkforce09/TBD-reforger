//! Role: rules.
//! Position: `mission/validation/validator` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    EvalContext, Finding, Registry, Value, rule_asset_resolves, rule_cargo_over_capacity,
    rule_loadout_has_equipment, rule_loadout_has_uniform, rule_loadout_has_vest,
    rule_loadout_mag_count, rule_orbat_callsign_unique, rule_orbat_identity_filled,
    rule_orbat_slot_resolves, rule_orbat_squad_has_leader, rule_orbat_template_coverage,
    rule_v1_player_spawn, rule_v2_faction_max, rule_v3_slot_in_bounds, rule_v4_schema_version,
    rule_vehicle_cargo_policy,
};

/// The seed registry: one rule per primitive, each on a payload shape the editor produces today.
#[must_use]
pub fn default_registry() -> Registry {
    Registry::new(vec![
        rule_v1_player_spawn(),
        rule_v2_faction_max(),
        rule_v3_slot_in_bounds(),
        rule_v4_schema_version(),
        rule_orbat_slot_resolves(),
        rule_orbat_identity_filled(),
        rule_orbat_squad_has_leader(),
        rule_orbat_callsign_unique(),
        rule_orbat_template_coverage(),
        rule_asset_resolves(),
        rule_loadout_has_uniform(),
        rule_loadout_has_vest(),
        rule_loadout_mag_count(),
        rule_loadout_has_equipment(),
        rule_vehicle_cargo_policy(),
        rule_cargo_over_capacity(),
    ])
}

/// No trip context using the supplied domain data.
pub(super) fn no_trip_context() -> Option<EvalContext> {
    None
}

/// Convenience: `default_registry().evaluate(payload)`. The one-call entry the API/SPA use.
#[must_use]
pub fn validate_editor_payload(payload: &Value) -> Vec<Finding> {
    default_registry().evaluate(payload)
}

/// Editor factions using the supplied domain data.
pub(super) fn editor_factions(payload: &Value) -> &[Value] {
    payload
        .get("editor")
        .and_then(|e| e.get("factions"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Editor slots using the supplied domain data.
pub(super) fn editor_slots(payload: &Value) -> &[Value] {
    payload
        .get("editor")
        .and_then(|e| e.get("slots"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Editor squads using the supplied domain data.
pub(super) fn editor_squads(payload: &Value) -> &[Value] {
    payload
        .get("editor")
        .and_then(|e| e.get("squads"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Top level array using the supplied domain data.
pub(super) fn top_level_array<'a>(payload: &'a Value, key: &str) -> &'a [Value] {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A string field on an object as `&str`, or `""` — a missing key, a null, or a non-string are all "absent" here. Total: never panics, so an `eval` calling it is a total function over any payload.
pub(super) fn str_field<'a>(obj: &'a Value, key: &str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or("")
}

/// A string-array field as an iterator of `&str`, skipping any non-string element. Total over any payload shape (a missing / non-array field yields an empty iterator; a `[1, "s1"]` yields `s1`).
pub(super) fn str_array<'a>(obj: &'a Value, key: &str) -> impl Iterator<Item = &'a str> {
    obj.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_str)
}

/// A slot's stable id (`slots[].id`), or `""` when absent — the `subject_id` a slot-scoped finding carries. Ids are minted by the document core (`slot-...`) and are non-empty in practice; a blank id is itself a malformed row the rule still reports (with an empty `subject_id`), never a panic.
pub(super) fn slot_id(slot: &Value) -> &str {
    str_field(slot, "id")
}

/// A squad's stable id (`squads[].id`), or `""` when absent.
pub(super) fn squad_id(squad: &Value) -> &str {
    str_field(squad, "id")
}

/// The authored terrain key (`map.terrain`), defaulting to `everon` exactly as the compiler does (`compile.rs`: `meta.terrain ?? 'everon'`). Feeds [`terrain_bounds`].
pub(super) fn terrain_key(payload: &Value) -> &str {
    payload
        .get("map")
        .and_then(|m| m.get("terrain"))
        .and_then(Value::as_str)
        .unwrap_or("everon")
}
