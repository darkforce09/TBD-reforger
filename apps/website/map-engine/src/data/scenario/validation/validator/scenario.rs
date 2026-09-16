//! Role: scenario.
//! Position: `mission/validation/validator` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    EvalContext, Primitive, Rule, Severity, Value, editor_factions, editor_slots, no_trip_context,
    terrain_bounds, terrain_key,
};

/// Declares players using the supplied domain data.
pub(super) fn declares_players(payload: &Value, _ctx: &EvalContext) -> bool {
    !editor_factions(payload).is_empty()
}

/// Rule v1 player spawn using the supplied domain data.
pub(super) fn rule_v1_player_spawn() -> Rule {
    Rule {
        id: "V1-PLAYER-SPAWN",
        severity: Severity::Error,
        primitive: Primitive::RequiredEntity,

        applies: declares_players,
        eval: |rule, payload, _ctx| {
            if editor_slots(payload).is_empty() {
                vec![rule.finding(
                    "This mission declares a faction but has no slots — there is nowhere for a \
                     player to spawn. Add at least one slot to a squad."
                        .to_string(),
                    "/editor/slots".to_string(),
                )]
            } else {
                Vec::new()
            }
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
                    "squads": [{"id": "sq1", "callsign": "Alpha", "slotIds": []}],
                    "slots": []
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/// The canonical side count. `mission-editor-payload.schema.json` `editorFaction`: `FACTION_SIDES = [BLUFOR, OPFOR, INDFOR, CIV]` — four, and the editor mints at most one row per side.
pub(super) const MAX_FACTIONS: usize = 4;

/// Rule v2 faction max using the supplied domain data.
pub(super) fn rule_v2_faction_max() -> Rule {
    Rule {
        id: "V2-FACTION-MAX",
        severity: Severity::Warning,
        primitive: Primitive::Cardinality,
        applies: |_, _| true,
        eval: |rule, payload, _ctx| {
            let n = editor_factions(payload).len();
            if n > MAX_FACTIONS {
                vec![rule.finding(
                    format!(
                        "{n} factions declared, but a mission has at most {MAX_FACTIONS} sides \
                         (BLUFOR / OPFOR / INDFOR / CIV) — extra faction rows will not map to a \
                         side."
                    ),
                    "/editor/factions".to_string(),
                )]
            } else {
                Vec::new()
            }
        },

        trip_fixture: || {
            let factions: Vec<Value> = (0..MAX_FACTIONS + 1)
                .map(
                    |i| serde_json::json!({"key": format!("SIDE{i}"), "name": format!("Side {i}")}),
                )
                .collect();
            serde_json::json!({ "editor": { "factions": factions } })
        },
        trip_context: no_trip_context,
    }
}

/// Rule v3 slot in bounds using the supplied domain data.
pub(super) fn rule_v3_slot_in_bounds() -> Rule {
    Rule {
        id: "V3-SLOT-IN-BOUNDS",
        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,
        applies: |_, _| true,
        eval: |rule, payload, _ctx| {
            let [min_x, min_y, max_x, max_y] = terrain_bounds(terrain_key(payload));
            let mut out = Vec::new();

            for (i, slot) in editor_slots(payload).iter().enumerate() {
                let Some(pos) = slot.get("position") else {
                    continue;
                };
                let x = pos.get("x").and_then(Value::as_f64);
                let y = pos.get("y").and_then(Value::as_f64);
                let (Some(x), Some(y)) = (x, y) else {
                    continue;
                };
                if x < min_x || x > max_x || y < min_y || y > max_y {
                    out.push(rule.finding(
                        format!(
                            "slot position ({x:.1}, {y:.1}) is outside the {} terrain bounds \
                             [{min_x:.0}, {min_y:.0}]–[{max_x:.0}, {max_y:.0}] — it would spawn off \
                             the playable map.",
                            terrain_key(payload),
                        ),
                        format!("/editor/slots/{i}/position"),
                    ));
                }
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "map": {"terrain": "everon"},
                "editor": {
                    "slots": [
                        {"id": "s1", "role": "RFL", "position": {"x": 20000.0, "y": 20000.0, "z": 0.0}}
                    ]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/// Rule v4 schema version using the supplied domain data.
pub(super) fn rule_v4_schema_version() -> Rule {
    Rule {
        id: "V4-SCHEMA-VERSION",
        severity: Severity::Error,
        primitive: Primitive::FieldShape,
        applies: |_, _| true,
        eval: |rule, payload, _ctx| {
            let Some(raw) = payload.get("schemaVersion") else {
                return Vec::new();
            };

            let ok = raw.as_u64().is_some_and(|v| v >= 1);
            if ok {
                Vec::new()
            } else {
                vec![rule.finding(
                    format!(
                        "schemaVersion must be a positive integer (the editor-payload format \
                         version); got {raw}. A string or fractional value here is the \
                         canonical-vs-editor namespace confusion."
                    ),
                    "/schemaVersion".to_string(),
                )]
            }
        },

        trip_fixture: || serde_json::json!({ "schemaVersion": "1" }),
        trip_context: no_trip_context,
    }
}
