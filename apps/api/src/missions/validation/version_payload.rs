//! Predicates over a raw mission-version payload string: the title the row mirrors out of it, and
//! whether the payload carries enough authored content to become `current_version_id`.

use serde_json::Value;

use crate::core::error_handling::api_error::ApiError;
use crate::missions::validation::mission_fields::validated_mission_title;

/// Non-blank trimmed top-level `title` from a Save payload.
///
/// `create_version` uses it to mirror the authored title onto `missions.title`. Reuses
/// [`validated_mission_title`] so the row write has the same non-blank trim guard as CREATE/PATCH.
/// Absent / non-string / whitespace-only → `None` (leave the row title alone).
pub(crate) fn payload_title_for_row_mirror(payload_str: &str) -> Option<String> {
    let v: Value = serde_json::from_str(payload_str).ok()?;
    let raw = v.get("title")?.as_str()?;
    validated_mission_title(raw).ok()
}

/// Whether a version payload is vacuous — schema-valid but useless as `current_version_id`.
///
/// `mission-editor-payload.schema.json` has no top-level `required` / `minProperties`, so `{}`
/// passes the schema pass in `missions::contract`. `create_mission` deliberately stores that stub
/// as `0.1.0`. Promoting the same shape (or any object with no editor graph and no placed content)
/// through `create_version` would move `current_version_id` with no recovery path — `/compiled`
/// 409s (`CompileError::NoSlots`), orbat attach materialises zero slots, and the ingest roster
/// omits the mission. Prior version rows survive in the DB, and `set_current_version` lets an
/// author or admin re-point the tip at one of them.
///
/// "Empty" here is measured against the same surfaces that break:
/// - `editor.slots` array (flatten / `ingest_list_missions` `jsonb_array_length` census)
/// - non-empty top-level `orbat` (explicit ORBAT wins in `parse_orbat_template`)
/// - non-empty `objectives` / `vehicles` / `markers` / `entities` (peers of
///   `MissionDocCore::has_content`)
/// - non-empty `editor.factions` / `squads` / `editorLayers` (authored structure without slots yet)
///
/// Explicit `editor.slots: []` is **not** vacuous — it is the draft skeleton integration tests and
/// WIP saves use before anything is placed. Schema `minItems` is deliberately **not** the fix: a
/// naive tightening there would 400 live missions. Non-JSON returns `false` so the schema pass owns
/// that message.
pub(crate) fn version_payload_is_vacuous(payload: &str) -> bool {
    let Ok(v) = serde_json::from_str::<Value>(payload) else {
        return false;
    };
    let Some(obj) = v.as_object() else {
        return true;
    };
    if let Some(orbat) = obj.get("orbat").and_then(Value::as_array)
        && !orbat.is_empty()
    {
        return false;
    }
    for key in ["objectives", "vehicles", "markers", "entities"] {
        if obj
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
        {
            return false;
        }
    }
    match obj.get("editor").and_then(Value::as_object) {
        Some(ed) => {
            if ed.get("slots").is_some_and(Value::is_array) {
                return false;
            }
            for key in ["factions", "squads", "editorLayers"] {
                if ed
                    .get(key)
                    .and_then(Value::as_array)
                    .is_some_and(|a| !a.is_empty())
                {
                    return false;
                }
            }
            true
        }
        None => true,
    }
}

/// Write-time gate for `create_version` — 400 before INSERT / `current_version_id` update.
pub(crate) fn reject_vacuous_version_payload(payload: &str) -> Result<(), ApiError> {
    if version_payload_is_vacuous(payload) {
        return Err(ApiError::bad_request(
            "payload must include editor content (refusing empty payload as current version)",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/version_payload.rs"]
mod tests;
