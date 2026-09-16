//! Role: loadout.
//! Position: `mission/validation/validator` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    EvalContext, LoadoutPolicy, Primitive, Rule, Severity, Value, editor_slots, no_trip_context,
    slot_id,
};

/// Loadout of using the supplied domain data.
pub(super) fn loadout_of(slot: &Value) -> Option<&Value> {
    slot.get("loadout").filter(|v| v.is_object())
}

/// A `wear{}` garment `resource_name` for `key`, trimmed, or `""` when absent/blank/null/non-string. The `wear` map is `SlotLoadoutV2.wear` (keys = `WEAR_PICK_KEYS`); a `null` value (the Arsenal's "empty slot" marker, `picks_to_loadout` sticky()) reads as absent. Total.
pub(super) fn wear_garment<'a>(loadout: &'a Value, key: &str) -> &'a str {
    loadout
        .get("wear")
        .and_then(|w| w.get(key))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
}

/// The slot's primary-weapon `weapons[]` entry, if one is authored. A primary is the entry whose `slotIndex == 0` (arsenal_rules.rs:53-57 — the "primary" row maps to `(0, "primary")`); the magazine and optic ride it (`picks_to_loadout` writes `optic`/`magazine` only on the primary, arsenal.rs:450-456). Returns the first `slotIndex == 0` object, or `None`. Total over any JSON.
pub(super) fn primary_weapon(loadout: &Value) -> Option<&Value> {
    loadout
        .get("weapons")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .find(|w| w.get("slotIndex").and_then(Value::as_i64) == Some(0))
}

/// A loadout "declares a slot loadout" iff at least one editor slot carries a `loadout` object. With none, the loadout policy rules have nothing to check (a slotless / loadout-free draft is not "below" a policy), so — like V1 — they are conditional on that shape. Shape-only; `ctx` unused.
pub(super) fn declares_loadout(payload: &Value, _ctx: &EvalContext) -> bool {
    editor_slots(payload)
        .iter()
        .any(|s| loadout_of(s).is_some())
}

/// Rule loadout has uniform using the supplied domain data.
pub(super) fn rule_loadout_has_uniform() -> Rule {
    Rule {
        id: "LOADOUT-HAS-UNIFORM",

        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,

        applies: declares_loadout,
        eval: |rule, payload, _ctx| {
            let mut out = Vec::new();
            for (i, slot) in editor_slots(payload).iter().enumerate() {
                let Some(lo) = loadout_of(slot) else {
                    continue;
                };

                if wear_garment(lo, "jacket").is_empty() {
                    let id = slot_id(slot);
                    out.push(rule.finding_id(
                        format!(
                            "slot {} has an authored loadout but no uniform (wear.jacket is empty) \
                             — a player spawning on it wears no fatigues unless the kit prefab \
                             supplies them.",
                            if id.is_empty() { "(no id)" } else { id },
                        ),
                        format!("/editor/slots/{i}/loadout/wear/jacket"),
                        id.to_string(),
                    ));
                }
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {"slots": [
                    {"id": "s1", "role": "RFL",
                     "loadout": {"version": 2, "wear": {"jacket": "", "vest": "{A}Vest.et"}, "weapons": []}}
                ]}
            })
        },
        trip_context: no_trip_context,
    }
}

/// Rule loadout has vest using the supplied domain data.
pub(super) fn rule_loadout_has_vest() -> Rule {
    Rule {
        id: "LOADOUT-HAS-VEST",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_loadout,
        eval: |rule, payload, _ctx| {
            let mut out = Vec::new();
            for (i, slot) in editor_slots(payload).iter().enumerate() {
                let Some(lo) = loadout_of(slot) else {
                    continue;
                };

                let has_vest = !wear_garment(lo, "vest").is_empty()
                    || !wear_garment(lo, "armoredVest").is_empty();
                if !has_vest {
                    let id = slot_id(slot);
                    out.push(rule.finding_id(
                        format!(
                            "slot {} has an authored loadout but no vest (neither wear.vest nor \
                             wear.armoredVest is set) — no chest rig or plate carrier for the \
                             player's magazines and gear.",
                            if id.is_empty() { "(no id)" } else { id },
                        ),
                        format!("/editor/slots/{i}/loadout/wear/vest"),
                        id.to_string(),
                    ));
                }
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {"slots": [
                    {"id": "s1", "role": "RFL",
                     "loadout": {"version": 2, "wear": {"jacket": "{A}U.et"}, "weapons": []}}
                ]}
            })
        },
        trip_context: no_trip_context,
    }
}

/// Returns the count as `u64` (saturating at the row `qty`, which the Arsenal writes ≥ 1). Total.
pub(super) fn magazine_count(loadout: &Value) -> u64 {
    let mag_rn = primary_weapon(loadout)
        .and_then(|w| w.get("magazine"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(mag_rn) = mag_rn else {
        return 0;
    };

    let mut count: u64 = 1;

    for row in loadout
        .get("cargo")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
    {
        if row.get("item").and_then(Value::as_str) == Some(mag_rn) {
            let qty = row
                .get("qty")
                .and_then(Value::as_i64)
                .filter(|q| *q >= 1)
                .unwrap_or(0);
            count = count.saturating_add(qty as u64);
        }
    }
    count
}

/// Rule loadout mag count using the supplied domain data.
pub(super) fn rule_loadout_mag_count() -> Rule {
    Rule {
        id: "LOADOUT-MAG-COUNT",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,

        applies: |payload, ctx| {
            declares_loadout(payload, ctx)
                && ctx
                    .loadout_policy
                    .as_ref()
                    .and_then(|p| p.min_magazines)
                    .is_some()
        },
        eval: |rule, payload, ctx| {
            let Some(min) = ctx.loadout_policy.as_ref().and_then(|p| p.min_magazines) else {
                return Vec::new();
            };
            let mut out = Vec::new();
            for (i, slot) in editor_slots(payload).iter().enumerate() {
                let Some(lo) = loadout_of(slot) else {
                    continue;
                };

                if primary_weapon(lo)
                    .and_then(|w| w.get("weapon"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .is_none()
                {
                    continue;
                }
                let have = magazine_count(lo);
                if have < min {
                    let id = slot_id(slot);
                    out.push(rule.finding_id(
                        format!(
                            "slot {} carries {have} magazine(s) but mission policy requires at least \
                             {min} for a slot with a primary weapon — a below-standard basic load.",
                            if id.is_empty() { "(no id)" } else { id },
                        ),
                        format!("/editor/slots/{i}/loadout"),
                        id.to_string(),
                    ));
                }
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {"slots": [
                    {"id": "s1", "role": "RFL", "loadout": {"version": 2,
                        "wear": {"jacket": "{A}U.et", "vest": "{A}V.et"},
                        "weapons": [{"slotIndex": 0, "slotType": "primary",
                                     "weapon": "{A}Rifle.et", "magazine": "{A}Mag.et"}],
                        "cargo": []}}
                ]}
            })
        },
        trip_context: || {
            Some(
                EvalContext::default()
                    .with_loadout_policy(LoadoutPolicy::default().with_min_magazines(3)),
            )
        },
    }
}
