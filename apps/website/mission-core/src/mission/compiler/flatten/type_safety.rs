//! Role: type safety.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::EditorPayload;

/// Does this payload deserialise into the editor graph [`flatten_to_mod_document`] compiles? Empty result = it does, and `/compiled` cannot answer [`CompileError::Parse`] for these bytes.
#[must_use]
pub fn scan_editor_payload_types(payload: &[u8]) -> Vec<String> {
    let Err(err) = serde_json::from_slice::<EditorPayload>(payload) else {
        return Vec::new();
    };

    if !err.is_data() {
        return Vec::new();
    }

    let located = serde_json::from_slice::<serde_json::Value>(payload)
        .ok()
        .map(|v| locate_briefing_type_errors(&v))
        .unwrap_or_default();
    if !located.is_empty() {
        return located;
    }
    vec![format!(
        "/editor: this payload does not match the shape the mission compiler reads, so it cannot be \
         compiled — {err}"
    )]
}

/// Name a JSON node the way an author reading an error can match it to what they typed.
pub(super) fn json_type_of(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

/// Locate briefing type errors using the supplied domain data.
pub(super) fn locate_briefing_type_errors(payload: &serde_json::Value) -> Vec<String> {
    let Some(factions) = payload
        .pointer("/editor/factions")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();

    for (i, f) in factions.iter().enumerate() {
        let Some(briefing) = f.get("briefing") else {
            continue;
        };
        match briefing {

            serde_json::Value::Null => {}
            serde_json::Value::Object(_) => {
                locate_one_briefing(&mut out, i, briefing);
            }

            serde_json::Value::Array(_) => {}
            other => out.push(format!(
                "/editor/factions/{i}/briefing: expected an object of this faction's orders, got {}. \
                 On a faction row `briefing` is the per-faction briefing OBJECT \
                 (`situation`/`mission`/`execution`/`markers`); the mission's one-line library blurb \
                 is a string on the MISSION, set by POST/PATCH /missions/:id — not this key",
                json_type_of(other)
            )),
        }
        if out.len() >= crate::mission::wire_safety::MAX_REPORTED {
            out.truncate(crate::mission::wire_safety::MAX_REPORTED);
            out.push(format!(
                "/editor/factions: stopped after {} findings — fix these and save again to see the \
                 rest",
                crate::mission::wire_safety::MAX_REPORTED
            ));
            break;
        }
    }
    out
}

/// The three prose fields and the marker rows of one object-form `briefing`.
pub(super) fn locate_one_briefing(out: &mut Vec<String>, i: usize, briefing: &serde_json::Value) {
    for field in ["situation", "mission", "execution"] {
        if let Some(v) = briefing.get(field)
            && !v.is_string()
            && !v.is_null()
        {
            out.push(format!(
                "/editor/factions/{i}/briefing/{field}: expected a string of briefing prose or null, \
                 got {}",
                json_type_of(v)
            ));
        }
    }

    let Some(markers) = briefing.get("markers") else {
        return;
    };
    let Some(rows) = markers.as_array() else {
        out.push(format!(
            "/editor/factions/{i}/briefing/markers: expected an array of map markers, got {}",
            json_type_of(markers)
        ));
        return;
    };
    for (j, m) in rows.iter().enumerate() {
        if !m.is_object() {
            if !m.is_array() {
                out.push(format!(
                    "/editor/factions/{i}/briefing/markers/{j}: expected a marker object with \
                     `x`/`z`/`icon`/`label`, got {}",
                    json_type_of(m)
                ));
            }
            continue;
        }
        for field in ["x", "z"] {
            if let Some(v) = m.get(field)
                && !v.is_number()
            {
                out.push(format!(
                    "/editor/factions/{i}/briefing/markers/{j}/{field}: expected a number of metres, \
                     got {}",
                    json_type_of(v)
                ));
            }
        }
        for field in ["icon", "label"] {
            if let Some(v) = m.get(field)
                && !v.is_string()
            {
                out.push(format!(
                    "/editor/factions/{i}/briefing/markers/{j}/{field}: expected a string, got {}",
                    json_type_of(v)
                ));
            }
        }
    }
}
