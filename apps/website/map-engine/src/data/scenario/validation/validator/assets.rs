//! Role: assets.
//! Position: `mission/validation/validator` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    EvalContext, HashSet, Primitive, Rule, Severity, Value, editor_slots, slot_id, str_field,
    top_level_array,
};

/// Canonical asset alias prefixes value.
pub(super) const ASSET_ALIAS_PREFIXES: &[&str] = &["veh:", "prop:", "comp:"];

/// Whether `id` looks like an alias form (`veh:` / `prop:` / `comp:` …) rather than a bare `resource_name`. Purely a shape test on the string; resolution is still membership in the supplied catalogue set.
pub(super) fn is_alias_form(id: &str) -> bool {
    ASSET_ALIAS_PREFIXES.iter().any(|p| id.starts_with(p))
}

/// Every placed-asset reference in the payload the resolution rule must check, as `(subject_pointer, subject_id, asset_id)` triples. The asset-id vocabulary is fixed by the document core's writers and carried verbatim by `compile::compile_payload`:.
pub(super) fn placed_asset_refs(payload: &Value) -> Vec<(String, String, String)> {
    let mut refs = Vec::new();

    for (i, slot) in editor_slots(payload).iter().enumerate() {
        let asset = str_field(slot, "assetId");
        if asset.is_empty() {
            continue;
        }
        let id = slot_id(slot);
        refs.push((
            format!("/editor/slots/{i}/assetId"),
            id.to_string(),
            asset.to_string(),
        ));
    }

    for (i, veh) in top_level_array(payload, "vehicles").iter().enumerate() {
        let asset = str_field(veh, "resourceName");
        if asset.is_empty() {
            continue;
        }
        let id = str_field(veh, "id");
        refs.push((
            format!("/vehicles/{i}/resourceName"),
            id.to_string(),
            asset.to_string(),
        ));
    }

    for (i, ent) in top_level_array(payload, "entities").iter().enumerate() {
        let alias = str_field(ent, "alias");
        let (field, asset) = if alias.is_empty() {
            ("resourceName", str_field(ent, "resourceName"))
        } else {
            ("alias", alias)
        };
        if asset.is_empty() {
            continue;
        }
        let id = str_field(ent, "id");
        refs.push((
            format!("/entities/{i}/{field}"),
            id.to_string(),
            asset.to_string(),
        ));
    }

    refs
}

/// Asset resolves using the supplied domain data.
pub(super) fn asset_resolves(asset_id: &str, known: &HashSet<String>) -> bool {
    known.contains(asset_id)
}

/// Rule asset resolves using the supplied domain data.
pub(super) fn rule_asset_resolves() -> Rule {
    Rule {
        id: "ASSET-RESOLVES",

        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,

        applies: |_payload, ctx| ctx.known_asset_ids.is_some(),
        eval: |rule, payload, ctx| {
            let Some(known) = ctx.known_asset_ids.as_ref() else {
                return Vec::new();
            };
            let mut out = Vec::new();

            for (subject, subject_id, asset_id) in placed_asset_refs(payload) {
                if asset_resolves(&asset_id, known) {
                    continue;
                }

                let kind = if is_alias_form(&asset_id) {
                    "alias"
                } else {
                    "prefab"
                };
                out.push(rule.finding_id(
                    format!(
                        "placed asset {kind} {asset_id:?} does not resolve in the live catalogue — \
                         the entry is missing (modset drift), so this placement will not spawn. \
                         Re-pick it from the palette or restore the mod that provides it."
                    ),
                    subject,
                    subject_id,
                ));
            }
            out
        },

        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "slots": [
                        {"id": "s1", "role": "RFL", "assetId": "{ABC}Prefabs/Characters/Ghost.et"}
                    ]
                }
            })
        },

        trip_context: || {
            Some(
                EvalContext::default().with_known_asset_ids(
                    ["{XYZ}Prefabs/Characters/SomethingElse.et".to_string()]
                        .into_iter()
                        .collect(),
                ),
            )
        },
    }
}
