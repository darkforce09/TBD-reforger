//! Role: cargo.
//! Position: `mission/validation/validator` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    EvalContext, HashSet, LoadoutPolicy, Primitive, Rule, Severity, Value, declares_loadout,
    editor_slots, loadout_of, slot_id, str_field, top_level_array,
};

/// The equipment kinds a slot's authored loadout carries, lowercased.
pub(super) fn equipment_kinds(loadout: &Value) -> HashSet<String> {
    loadout
        .get("equipment")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter(|(_, v)| v.as_str().is_some_and(|s| !s.trim().is_empty()))
                .map(|(k, _)| k.to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

/// Rule loadout has equipment using the supplied domain data.
pub(super) fn rule_loadout_has_equipment() -> Rule {
    Rule {
        id: "LOADOUT-HAS-EQUIPMENT",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,

        applies: |payload, ctx| {
            declares_loadout(payload, ctx)
                && ctx
                    .loadout_policy
                    .as_ref()
                    .and_then(|p| p.required_equipment.as_ref())
                    .is_some_and(|s| !s.is_empty())
        },
        eval: |rule, payload, ctx| {
            let Some(required) = ctx
                .loadout_policy
                .as_ref()
                .and_then(|p| p.required_equipment.as_ref())
                .filter(|s| !s.is_empty())
            else {
                return Vec::new();
            };
            let mut out = Vec::new();
            for (i, slot) in editor_slots(payload).iter().enumerate() {
                let Some(lo) = loadout_of(slot) else {
                    continue;
                };
                let have = equipment_kinds(lo);
                let mut missing: Vec<&str> = required
                    .iter()
                    .filter(|k| !have.contains(*k))
                    .map(String::as_str)
                    .collect();
                if missing.is_empty() {
                    continue;
                }
                missing.sort_unstable();
                let id = slot_id(slot);
                out.push(rule.finding_id(
                    format!(
                        "slot {} is missing required equipment [{}] — mission policy requires every \
                         player to carry it. (Equipment micro-slots land with the Arsenal equipment \
                         slice; until then no loadout authors them.)",
                        if id.is_empty() { "(no id)" } else { id },
                        missing.join(", "),
                    ),
                    format!("/editor/slots/{i}/loadout/equipment"),
                    id.to_string(),
                ));
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {"slots": [
                    {"id": "s1", "role": "RFL", "loadout": {"version": 2,
                        "wear": {"jacket": "{A}U.et", "vest": "{A}V.et"}, "weapons": [],
                        "equipment": {"map": "{A}Map.et", "compass": null}}}
                ]}
            })
        },
        trip_context: || {
            let kinds: HashSet<String> = ["map", "compass", "radio"]
                .into_iter()
                .map(str::to_string)
                .collect();
            Some(
                EvalContext::default()
                    .with_loadout_policy(LoadoutPolicy::default().with_required_equipment(kinds)),
            )
        },
    }
}

/// Total cargo ITEMS in a vehicle's authored inventory (summed `qty` over `vehicles[].cargo[]`, each row `{item, qty}` — the `EntityInventoryIn` shape, flatten.rs:786-791). A row with a non-positive/absent qty contributes 0. Total over any JSON.
pub(super) fn vehicle_cargo_items(vehicle: &Value) -> u64 {
    vehicle
        .get("cargo")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .map(|r| {
            r.get("qty")
                .and_then(Value::as_i64)
                .filter(|q| *q >= 1)
                .unwrap_or(0) as u64
        })
        .sum()
}

/// A payload "places vehicles" iff the top-level `vehicles[]` array is non-empty. With none, R9 has nothing to check. Shape-only gate; `ctx` unused.
pub(super) fn places_vehicles(payload: &Value, _ctx: &EvalContext) -> bool {
    !top_level_array(payload, "vehicles").is_empty()
}

/// Rule vehicle cargo policy using the supplied domain data.
pub(super) fn rule_vehicle_cargo_policy() -> Rule {
    Rule {
        id: "VEHICLE-CARGO-POLICY",

        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,

        applies: |payload, ctx| {
            places_vehicles(payload, ctx)
                && ctx
                    .loadout_policy
                    .as_ref()
                    .and_then(|p| p.max_vehicle_cargo_items)
                    .is_some()
        },
        eval: |rule, payload, ctx| {
            let Some(max) = ctx
                .loadout_policy
                .as_ref()
                .and_then(|p| p.max_vehicle_cargo_items)
            else {
                return Vec::new();
            };
            let mut out = Vec::new();

            for (i, veh) in top_level_array(payload, "vehicles").iter().enumerate() {
                let items = vehicle_cargo_items(veh);
                if items > max {
                    let id = str_field(veh, "id");
                    out.push(rule.finding_id(
                        format!(
                            "vehicle {} carries {items} cargo item(s), over the policy ceiling of \
                             {max} — a pre-stuffed vehicle inventory is a PvP fairness problem.",
                            if id.is_empty() { "(no id)" } else { id },
                        ),
                        format!("/vehicles/{i}/cargo"),
                        id.to_string(),
                    ));
                }
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "vehicles": [
                    {"id": "v1", "resourceName": "{A}Truck.et",
                     "cargo": [{"item": "{A}Mag.et", "qty": 30}, {"item": "{A}Bandage.et", "qty": 5}]}
                ]
            })
        },
        trip_context: || {
            Some(
                EvalContext::default()
                    .with_loadout_policy(LoadoutPolicy::default().with_max_vehicle_cargo_items(10)),
            )
        },
    }
}

/// A payload "declares cargo capacity to check" iff it authors at least one slot loadout — the same shape gate the other loadout rules use. Shape-only; the CATALOGUE gate (the real inertness) is on `applies` below (`ctx.cargo_phys.is_some()`).
pub(super) fn declares_cargo(payload: &Value, ctx: &EvalContext) -> bool {
    declares_loadout(payload, ctx)
}

/// Rule cargo over capacity using the supplied domain data.
pub(super) fn rule_cargo_over_capacity() -> Rule {
    Rule {
        id: "CARGO-OVER-CAPACITY",

        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,

        applies: |payload, ctx| declares_cargo(payload, ctx) && ctx.cargo_phys.is_some(),
        eval: |rule, payload, ctx| {
            let Some(catalog) = ctx.cargo_phys.as_ref() else {
                return Vec::new();
            };

            let lines = crate::mission::wire_safety::scan_cargo_capacity(payload, catalog);
            let slots = editor_slots(payload);
            let mut out = Vec::new();
            for line in lines {
                let (subject, subject_id) = parse_cargo_line_subject(&line, slots);
                out.push(rule.finding_id_opt(line, subject, subject_id));
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {"slots": [
                    {"id": "s1", "role": "RFL", "loadout": {"version": 2,
                        "wear": {"vest": "vest_rn"}, "weapons": [],
                        "cargo": [{"container": "vest", "item": "mag", "qty": 4}]}}
                ]}
            })
        },
        trip_context: || {
            let mut catalog = crate::mission::wire_safety::CargoPhysCatalog::new();
            catalog.insert(
                "mag".to_string(),
                crate::mission::wire_safety::CargoPhys {
                    display_name: "Mag".to_string(),
                    weight_kg: Some(0.5),
                    volume_cm3: Some(60.0),
                    ..crate::mission::wire_safety::CargoPhys::default()
                },
            );
            catalog.insert(
                "vest_rn".to_string(),
                crate::mission::wire_safety::CargoPhys {
                    display_name: "Plate Carrier".to_string(),
                    max_weight_kg: Some(5.0),
                    max_volume_cm3: Some(200.0),
                    ..crate::mission::wire_safety::CargoPhys::default()
                },
            );
            Some(EvalContext::default().with_cargo_phys(catalog))
        },
    }
}

/// Parse cargo line subject using the supplied domain data.
pub(super) fn parse_cargo_line_subject(line: &str, slots: &[Value]) -> (String, Option<String>) {
    let prefix = line.split(':').next().unwrap_or(line);

    let idx = prefix
        .strip_prefix("/editor/slots/")
        .and_then(|rest| rest.split('/').next())
        .and_then(|n| n.parse::<usize>().ok());
    let subject_id = idx
        .and_then(|i| slots.get(i))
        .map(slot_id)
        .filter(|id| !id.is_empty())
        .map(str::to_string);
    (prefix.to_string(), subject_id)
}
