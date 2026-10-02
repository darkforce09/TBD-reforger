//! Role: orbat.
//! Position: `mission/validation/validator` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    EvalContext, Primitive, Rule, Severity, Value, editor_factions, editor_slots, editor_squads,
    no_trip_context, slot_id, squad_id, str_array, str_field,
};

/// Declares orbat using the supplied domain data.
pub(super) fn declares_orbat(payload: &Value, _ctx: &EvalContext) -> bool {
    !editor_squads(payload).is_empty()
}

/// The set of slot ids referenced by *some* squad's `slotIds`. A slot whose id is absent from this set is not filed under any squad — it resolves no squad.
pub(super) fn attached_slot_ids(payload: &Value) -> std::collections::HashSet<&str> {
    let mut set = std::collections::HashSet::new();
    for sq in editor_squads(payload) {
        for id in str_array(sq, "slotIds") {
            set.insert(id);
        }
    }
    set
}

/// Rule orbat slot resolves using the supplied domain data.
pub(super) fn rule_orbat_slot_resolves() -> Rule {
    Rule {
        id: "ORBAT-SLOT-RESOLVES",

        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,

        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            let attached = attached_slot_ids(payload);
            let mut out = Vec::new();

            for (i, slot) in editor_slots(payload).iter().enumerate() {
                let id = slot_id(slot);
                let role = str_field(slot, "role").trim();
                let has_role = !role.is_empty();

                let has_squad = !id.is_empty() && attached.contains(id);
                if has_role && has_squad {
                    continue;
                }
                let missing = match (has_role, has_squad) {
                    (false, false) => "resolves neither a role nor a squad",
                    (true, false) => "is not filed under any squad",
                    (false, true) => "has no role",
                    (true, true) => unreachable!(),
                };
                out.push(rule.finding_id(
                    format!(
                        "slot {} {missing} — every slot must name a role and belong to a squad, \
                         or it compiles to no usable seat.",
                        if id.is_empty() { "(no id)" } else { id },
                    ),
                    format!("/editor/slots/{i}"),
                    id.to_string(),
                ));
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "squads": [{"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": []}],
                    "slots": [{"id": "s1", "role": ""}]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/// Rule orbat identity filled using the supplied domain data.
pub(super) fn rule_orbat_identity_filled() -> Rule {
    Rule {
        id: "ORBAT-IDENTITY-FILLED",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            let mut out = Vec::new();
            for (i, sq) in editor_squads(payload).iter().enumerate() {
                let id = squad_id(sq);
                let callsign_blank = str_field(sq, "callsign").trim().is_empty();
                let name_blank = str_field(sq, "name").trim().is_empty();

                if callsign_blank && name_blank {
                    out.push(rule.finding_id(
                        format!(
                            "squad {} has no callsign and no name — give it an identity so it is \
                             addressable in the ORBAT and the roster.",
                            if id.is_empty() { "(no id)" } else { id },
                        ),
                        format!("/editor/squads/{i}"),
                        id.to_string(),
                    ));
                }
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "squads": [{"id": "sq1", "callsign": "", "name": "", "slotIds": []}]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/// Rule orbat squad has leader using the supplied domain data.
pub(super) fn rule_orbat_squad_has_leader() -> Rule {
    Rule {
        id: "ORBAT-SQUAD-HAS-LEADER",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            let mut out = Vec::new();
            for (i, sq) in editor_squads(payload).iter().enumerate() {
                let id = squad_id(sq);
                let members: Vec<&str> = str_array(sq, "slotIds").collect();
                if members.is_empty() {
                    continue;
                }
                let leader = str_field(sq, "leaderSlotId");

                let has_leader = !leader.is_empty() && members.contains(&leader);
                if !has_leader {
                    out.push(rule.finding_id(
                        format!(
                            "squad {} has {} slot(s) but no leader — one of its slots must be the \
                             leader (leaderSlotId).",
                            if id.is_empty() { "(no id)" } else { id },
                            members.len(),
                        ),
                        format!("/editor/squads/{i}"),
                        id.to_string(),
                    ));
                }
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "squads": [{"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": ["s1"]}],
                    "slots": [{"id": "s1", "role": "SL"}]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/// Rule orbat callsign unique using the supplied domain data.
pub(super) fn rule_orbat_callsign_unique() -> Rule {
    Rule {
        id: "ORBAT-CALLSIGN-UNIQUE",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            use std::collections::HashMap;

            let squads_by_id: HashMap<&str, (usize, &Value)> = editor_squads(payload)
                .iter()
                .enumerate()
                .map(|(i, s)| (squad_id(s), (i, s)))
                .collect();
            let mut out = Vec::new();
            for faction in editor_factions(payload) {
                let mut seen: HashMap<String, &str> = HashMap::new();
                for member_id in str_array(faction, "squadIds") {
                    let Some(&(idx, sq)) = squads_by_id.get(member_id) else {
                        continue;
                    };
                    let callsign = str_field(sq, "callsign").trim();
                    if callsign.is_empty() {
                        continue;
                    }
                    let key = callsign.to_lowercase();
                    if let Some(&first) = seen.get(&key) {
                        let id = squad_id(sq);
                        out.push(rule.finding_id(
                            format!(
                                "callsign {callsign:?} is used by more than one squad on side {:?} \
                                 (also squad {first}) — callsigns must be unique within a side.",
                                str_field(faction, "key"),
                            ),
                            format!("/editor/squads/{idx}"),
                            id.to_string(),
                        ));
                    } else {
                        seen.insert(key, member_id);
                    }
                }
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1", "sq2"]}],
                    "squads": [
                        {"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": []},
                        {"id": "sq2", "callsign": "Alpha", "name": "Alpha 1-2", "slotIds": []}
                    ]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/// Required roles using the supplied domain data.
pub(super) fn required_roles(squad: &Value) -> Vec<&str> {
    squad
        .get("template")
        .and_then(|t| t.get("requiredRoles"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .collect()
}

/// Rule orbat template coverage using the supplied domain data.
pub(super) fn rule_orbat_template_coverage() -> Rule {
    Rule {
        id: "ORBAT-TEMPLATE-COVERAGE",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            use std::collections::HashSet;

            let role_of: std::collections::HashMap<&str, &str> = editor_slots(payload)
                .iter()
                .map(|s| (slot_id(s), str_field(s, "role").trim()))
                .collect();
            let mut out = Vec::new();
            for (i, sq) in editor_squads(payload).iter().enumerate() {
                let required = required_roles(sq);
                if required.is_empty() {
                    continue;
                }

                let filled: HashSet<String> = str_array(sq, "slotIds")
                    .filter_map(|id| role_of.get(id))
                    .map(|r| r.to_lowercase())
                    .filter(|r| !r.is_empty())
                    .collect();
                let mut missing: Vec<&str> = required
                    .iter()
                    .copied()
                    .filter(|r| !filled.contains(&r.to_lowercase()))
                    .collect();
                if missing.is_empty() {
                    continue;
                }
                missing.dedup();
                let id = squad_id(sq);
                out.push(rule.finding_id(
                    format!(
                        "squad {} is missing required role(s) [{}] for its template — a squad \
                         instantiated from a template must fill every role the template requires.",
                        if id.is_empty() { "(no id)" } else { id },
                        missing.join(", "),
                    ),
                    format!("/editor/squads/{i}"),
                    id.to_string(),
                ));
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "squads": [{
                        "id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1",
                        "slotIds": ["s1"],
                        "leaderSlotId": "s1",
                        "template": {"requiredRoles": ["SL", "MED"]}
                    }],
                    "slots": [{"id": "s1", "role": "SL"}]
                }
            })
        },
        trip_context: no_trip_context,
    }
}
