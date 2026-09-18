//! The zone vocabulary, enforced at the SAVE boundary.
//!
//! [`scan_authored_zones`] is the entry point; everything above it is the projection that turns an
//! AUTHORED zone into the compiled `zones[]` row the flatten stage will emit, so the row that gets
//! validated is the row a game server would receive rather than the one the author typed.

use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::{Value, json};
use website_map_engine::data::scenario::wire_safety::MAX_REPORTED;

use super::schema_validators::MISSION_SCHEMA;

/// The zone subschema, lifted out of the **already-embedded** [`MISSION_SCHEMA`] so the save
/// boundary and the serve boundary read the same bytes.
///
/// `$defs` is borrowed whole rather than copied, so `#/$defs/zone`'s own internal refs
/// (`shape` → `circle`/`polygon`, `wireSafeString`, `zoneRules` → `alias`) resolve inside the
/// same document. The wrapper deliberately carries **no `$id`**: the base URI stays the default
/// one, so `#/$defs/zone` points at this wrapper's root and nothing has to be fetched.
fn compile_zone_schema() -> Result<Validator, String> {
    let mission: Value = serde_json::from_str(MISSION_SCHEMA).map_err(|e| e.to_string())?;
    let defs = mission
        .get("$defs")
        .ok_or_else(|| "mission.schema.json has no $defs".to_string())?;
    let wrapper = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": "#/$defs/zone",
        "$defs": defs,
    });
    jsonschema::validator_for(&wrapper).map_err(|e| e.to_string())
}

/// One-decimal metre quantisation. **Mirrors `flatten::round_coord`** — the one line of that
/// module this file restates, pinned against its source by `zone_quantisation_mirrors_flatten`.
///
/// This is the whole reason the check cannot be written as a rule on the AUTHORED document:
/// `round_coord(0.04) == 0.0`, and `0.0` violates `$defs/circle.r`'s `exclusiveMinimum: 0`, so a
/// radius that is schema-VALID on the way in becomes schema-INVALID on the way out.
fn round_coord(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Quantise one authored coordinate the way the compile will.
///
/// `None` in (key absent) is `0.0`, matching `#[serde(default)]` on `CircleIn`. A non-number bails
/// (`None` out) rather than guessing: that payload is a hard `serde` type error, which
/// `flatten::scan_editor_payload_types` already reports on this same `details` array, and a second
/// wording of it here would be noise. Non-finite out serialises as `null` in the compiled
/// document, so it is reported as `null` here — that is what the game server would receive.
fn quantised(v: Option<&Value>) -> Option<Value> {
    let n = match v {
        None | Some(Value::Null) => 0.0,
        Some(other) => other.as_f64()?,
    };
    let r = round_coord(n);
    Some(serde_json::Number::from_f64(r).map_or(Value::Null, Value::Number))
}

/// The `shape` the compile will emit, or `None` when it emits none — in which case
/// `flatten_authored_zone` drops the whole zone and it can never reach a game server.
///
/// Mirrors `flatten::shape_from_input` branch for branch, INCLUDING the `r > 0.0` test being made
/// on the authored radius rather than the rounded one. Getting that order wrong here would hide
/// exactly the defect this function exists to catch.
fn projected_shape(shape: &Value) -> Option<Value> {
    let polygon = shape.get("polygon").and_then(Value::as_array);
    if polygon.is_some_and(|p| p.len() >= 3) {
        let mut ring: Vec<Value> = Vec::new();
        for pair in polygon.expect("checked above") {
            let Some(pair) = pair.as_array() else {
                continue;
            };
            if pair.len() != 2 {
                continue;
            }
            ring.push(json!([quantised(pair.first())?, quantised(pair.get(1))?]));
        }
        if ring.len() >= 3 {
            return Some(json!({ "polygon": ring }));
        }
    }

    let circle = shape.get("circle")?;
    // The authored radius, pre-rounding — `shape_from_input`'s own gate.
    if circle.get("r").and_then(Value::as_f64).unwrap_or(0.0) <= 0.0 {
        return None;
    }
    Some(json!({ "circle": {
        "x": quantised(circle.get("x"))?,
        "z": quantised(circle.get("z"))?,
        "r": quantised(circle.get("r"))?,
    }}))
}

/// The compiled `zones[]` row this authored zone becomes, or `None` when the compile drops it.
///
/// Everything except `shape` is carried VERBATIM by `flatten_authored_zone`, so the projection is
/// a copy plus the quantisation — no restatement of the vocabulary, which stays in the schema.
///
/// **`faction` is deliberately omitted**, and this is a layer distinction, not an oversight. The
/// compiled key is `$defs/factionKey` (`^[a-z][a-z0-9_]*$`); the AUTHORED key is uppercase by
/// construction (`BLUFOR`/`OPFOR`/`INDFOR`/`CIV`), and flatten runs it through `slug_key`, whose
/// output matches that pattern for every possible input. So validating the authored value here
/// would reject documents that compile perfectly, and validating the slugged one can never fail.
/// There is nothing to check, and the key is optional — omitting it is exact.
fn projected_zone(zone: &Value) -> Option<Value> {
    let shape = projected_shape(zone.get("shape")?)?;
    let id = zone.get("id").and_then(Value::as_str).unwrap_or_default();
    let kind = zone.get("type").and_then(Value::as_str).unwrap_or_default();
    // `flatten_authored_zone` drops a zone with an empty id or type before it reaches the document.
    if id.is_empty() || kind.is_empty() {
        return None;
    }
    let mut out = json!({ "id": id, "type": kind, "shape": shape });
    let obj = out.as_object_mut().expect("built as an object");
    // `ModZone` skips both when empty / absent, so the compiled row would not carry the key.
    match zone.get("label").and_then(Value::as_str) {
        Some(label) if !label.is_empty() => {
            obj.insert("label".into(), json!(label));
        }
        _ => {}
    }
    if let Some(rules) = zone.get("rules")
        && !rules.is_null()
    {
        obj.insert("rules".into(), rules.clone());
    }
    Some(out)
}

/// Name the quantisation when it is what broke the shape, instead of quoting a radius the author
/// never typed.
///
/// Wording only, and only ever reached for a row the schema has ALREADY rejected — so, exactly as
/// in `flatten::locate_briefing_type_errors`, a wrong guess here cannot change what is accepted.
/// The raw finding is `"{\"circle\":{…,\"r\":0.0}} is not valid under any of the schemas listed in
/// the 'oneOf' keyword"`, and an author who wrote `0.04` has no way to connect that `0.0` to
/// anything they did.
fn sharpen_quantised_radius(zone: &Value, loc: &str) -> Option<String> {
    if !loc.starts_with("/shape") {
        return None;
    }
    let r = zone
        .get("shape")?
        .get("circle")?
        .get("r")
        .and_then(Value::as_f64)?;
    if r <= 0.0 || round_coord(r) != 0.0 {
        return None;
    }
    Some(format!(
        "circle radius {r} m rounds to 0 m and the zone would have no area. Mission documents are \
         quantised to a 0.1 m grid, so the smallest radius that survives the compile is 0.05 m — \
         below that the compiled `r` is 0, which `mission.schema.json` $defs/circle refuses \
         (exclusiveMinimum: 0). If this came from a click without a drag, drag out a radius"
    ))
}

/// Refuse a zone at SAVE that `GET /missions/:id/compiled` would refuse at SERVE.
///
/// ## The defect this closes
///
/// Without this pass, zone vocabulary is enforced at serve time only. `doc/store.rs`
/// `set_zone_rules` stores `rules` OPAQUE (deliberately — a typed Rust mirror of the rule names
/// would be the SECOND vocabulary the schema exists to prevent), `flatten.rs` carries it verbatim,
/// and the first thing to look at it is `validated_compiled_body`. MEASURED on the real HTTP path,
/// on a scratch instance:
///
/// ```text
/// POST /missions/:id/versions  zones[0].rules = {"notInVocabulary": 1}  -> 201 Created
/// GET  /missions/:id/compiled                                           -> 500
///      "/zones/0/rules: Additional properties are not allowed ('notInVocabulary' was unexpected)"
/// POST /missions/:id/versions  zones[0].type  = "capture"               -> 201 Created
/// GET  /missions/:id/compiled                                           -> 500
///      "/zones/0/type: \"capture\" is not one of \"spawn\", \"objective_capture\" or 4 other candidates"
/// ```
///
/// …and forever, because a `mission_versions` row is immutable. The author sees success; the
/// failure surfaces in front of a game server that supplied nothing but a mission id and can do
/// nothing about it. Every one of those payloads deserialises cleanly, because `ZoneIn.rules` is
/// `Option<serde_json::Value>` — the serde precheck cannot see any of it.
///
/// ## Why the verdict is the SERVE schema and not a new rule
///
/// The accept/reject decision below is `mission.schema.json#/$defs/zone` — lifted out of the same
/// embedded bytes [`super::schema_validators::validate_mission_document`] validates the compiled
/// document against. "Save accepted it" and "`/compiled` will validate" are therefore one sentence
/// by construction, the same property `flatten::scan_editor_payload_types` gets by running the
/// compiler's own deserialiser. A per-zone subschema written into
/// `mission-editor-payload.schema.json` would be a SECOND declaration of a vocabulary whose entire
/// design premise (see `$defs/zoneRules`) is that there is exactly one place a misspelled rule key
/// can be caught.
///
/// ## Why the POST-QUANTISATION shape
///
/// The compile rounds every zone coordinate to 0.1 m, and `round_coord(0.04) = 0.0` violates
/// `$defs/circle.r`'s `exclusiveMinimum: 0` — while `0.04` itself is perfectly schema-valid.
/// MEASURED: `circle r: 0.04` → save **201** → `/compiled` **500**
/// `"/zones/0/shape: {\"circle\":{...,\"r\":0.0}} is not valid under any of the schemas listed in
/// the 'oneOf' keyword"`. A click without a drag in the draw tool produces exactly that radius, so
/// a check that validated only the authored document would pass it straight through.
/// [`projected_zone`] therefore validates the row the compile will EMIT, not the one authored.
///
/// ## Why it cannot reject a payload that compiles today
///
/// [`projected_zone`] returns `None` for every zone `flatten_authored_zone` drops (no usable
/// shape, empty `id`, empty `type`). A dropped zone never reaches the document, so it cannot 500,
/// so refusing it here would break the invariant this boundary holds: the accept set at save must
/// not be narrower than the compile set.
///
/// ## Cost
///
/// O(zones), and zones are the BOUNDED array in this payload — a play area plus a handful of
/// objectives, unlike `editor.slots` where 367k is a measured live size and why that array carries
/// no per-item subschema. This walks the already-parsed instance and adds no parse.
pub(super) fn scan_authored_zones(payload: &Value) -> Vec<String> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    let Some(zones) = payload.get("zones").and_then(Value::as_array) else {
        // Absent, or not an array at all — `scan_editor_payload_types` owns the latter, since
        // `EditorPayload.zones` is a `Vec<ZoneIn>` and anything else is a hard serde type error.
        return Vec::new();
    };
    let Ok(validator) = V.get_or_init(compile_zone_schema) else {
        // A schema that will not compile is an internal fault, not this payload's. The schema pass
        // in `run_parsed` reports it through `ContractError::Compile`; staying silent here avoids
        // turning it into a bogus 400 against the author.
        return Vec::new();
    };

    let mut out: Vec<String> = Vec::new();
    for (i, zone) in zones.iter().enumerate() {
        let Some(projected) = projected_zone(zone) else {
            continue; // the compile drops this row — it can never reach a game server.
        };
        for e in validator.iter_errors(&projected) {
            let loc = e.instance_path().to_string();
            let msg = sharpen_quantised_radius(zone, &loc).unwrap_or_else(|| e.to_string());
            out.push(format!("/zones/{i}{loc}: {msg}"));
            if out.len() >= MAX_REPORTED {
                out.push(format!(
                    "/zones: stopped after {MAX_REPORTED} findings — fix these and save again to \
                     see the rest"
                ));
                return out;
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "tests/zone_quantisation.rs"]
mod tests;
