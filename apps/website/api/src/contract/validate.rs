//! Runtime JSON-Schema validation — Rust port of `internal/contract/{validate,mission}.go`,
//! using the `jsonschema` crate (draft 2020-12) in place of santhosh-tekuri.
//!
//! Schemas are embedded directly from the canonical `packages/tbd-schema/schema/`
//! (no copy step). Contract mirrors Go: `Ok(empty)` = valid, `Ok(details)` = schema
//! violations (advisory strings), `Err` = internal schema-compile failure only.
//!
//! NOTE: the `details` *strings* differ from Go's santhosh-tekuri wording (different
//! library) — a documented bounded deviation. Status + top-level error message match;
//! `details` are advisory (never matched by the client).

use std::sync::OnceLock;

use jsonschema::Validator;
use map_engine_core::mission::wire_safety::{self, CargoPhysCatalog, MAX_REPORTED};
use serde_json::{Value, json};

const EDITOR_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/mission-editor-payload.schema.json");
const MISSION_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/mission.schema.json");
const REGISTRY_ITEMS_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/registry-items.schema.json");
const REGISTRY_COMPAT_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/registry-compat.schema.json");
const FACTION_LIBRARY_SCHEMA: &str =
    include_str!("../../../../../packages/tbd-schema/schema/faction-library.schema.json");

/// Internal schema-compile failure (never returned for merely-invalid input).
#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error("schema compile failed: {0}")]
    Compile(String),
}

fn compile(src: &str) -> Result<Validator, String> {
    let schema: Value = serde_json::from_str(src).map_err(|e| e.to_string())?;
    jsonschema::validator_for(&schema).map_err(|e| e.to_string())
}

fn run(
    cell: &'static OnceLock<Result<Validator, String>>,
    schema_src: &str,
    raw: &[u8],
    bad_json: &str,
) -> Result<Vec<String>, ContractError> {
    run_parsed(cell, schema_src, raw, bad_json, |_instance| Vec::new())
}

/// Schema pass plus a code-side walk over the SAME parsed instance. Reuses the parse rather than
/// taking `&[u8]`, because re-parsing a save payload (hundreds of MB at editor scale) to run a
/// second check would cost orders of magnitude more than the check.
fn run_parsed(
    cell: &'static OnceLock<Result<Validator, String>>,
    schema_src: &str,
    raw: &[u8],
    bad_json: &str,
    extra: impl FnOnce(&Value) -> Vec<String>,
) -> Result<Vec<String>, ContractError> {
    let compiled = cell.get_or_init(|| compile(schema_src));
    let validator = compiled
        .as_ref()
        .map_err(|e| ContractError::Compile(e.clone()))?;

    let Ok(instance) = serde_json::from_slice::<Value>(raw) else {
        return Ok(vec![bad_json.to_string()]);
    };

    let mut details: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| {
            let loc = e.instance_path().to_string();
            let loc = if loc.is_empty() { "/".to_string() } else { loc };
            format!("{loc}: {e}")
        })
        .collect();
    details.extend(extra(&instance));
    Ok(details)
}

/// Validate a raw mission-version payload against `mission-editor-payload.schema.json`
/// (the write-side editor superset). Used by CreateMission + CreateVersion.
///
/// Two code-side passes after the schema, one parse:
/// * [`wire_safety::scan_editor_payload`] (T-181.44) — control characters in authored strings;
/// * [`wire_safety::scan_cargo_capacity`] (T-416) — over-capacity cargo when a phys catalog is
///   supplied via [`validate_mission_editor_payload_with_catalog`]. This entry point passes an
///   **empty** catalog so existing callers keep their signature; without phys attrs the cargo
///   walk is a no-op (never invent capacity). Wire Save/compile refusal by loading
///   `registry_items` into a [`CargoPhysCatalog`] and calling the `_with_catalog` variant.
///
/// @contract mission-editor-payload.schema.json#/ + mission.schema.json#/$defs/wireSafeString
pub fn validate_mission_editor_payload(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    validate_mission_editor_payload_with_catalog(raw, &CargoPhysCatalog::new())
}

/// T-416 — same as [`validate_mission_editor_payload`], but the cargo-capacity walk uses `catalog`
/// (`resource_name →` weight/volume/garment maxima). Build it from `registry_items`; do not put the
/// registry inside `map-engine-core` (see `wire_safety` module header).
pub fn validate_mission_editor_payload_with_catalog(
    raw: &[u8],
    catalog: &CargoPhysCatalog,
) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run_parsed(
        &V,
        EDITOR_SCHEMA,
        raw,
        "payload is not valid JSON",
        |instance| {
            let mut d = wire_safety::scan_editor_payload(instance);
            d.extend(wire_safety::scan_cargo_capacity(instance, catalog));
            d.extend(scan_authored_zones(instance));
            d
        },
    )
}

// ---- T-581: zone vocabulary, enforced at the SAVE boundary ----

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
/// a copy plus the quantisation — no restatement of the vocabulary, which stays where T-241 put it.
///
/// **`faction` is deliberately omitted**, and this is the T-357 layer distinction, not an
/// oversight. The compiled key is `$defs/factionKey` (`^[a-z][a-z0-9_]*$`); the AUTHORED key is
/// uppercase by construction (`BLUFOR`/`OPFOR`/`INDFOR`/`CIV`), and flatten runs it through
/// `slug_key`, whose output matches that pattern for every possible input. So validating the
/// authored value here would reject documents that compile perfectly, and validating the slugged
/// one can never fail. There is nothing to check, and the key is optional — omitting it is exact.
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

/// **T-581** — refuse a zone at SAVE that `GET /missions/:id/compiled` would refuse at SERVE.
///
/// ## The defect this closes
///
/// Zone vocabulary was enforced only at serve time. `doc/store.rs` `set_zone_rules` stores `rules`
/// OPAQUE (deliberately — a typed Rust mirror of the rule names would be the SECOND vocabulary
/// T-241 exists to prevent), `flatten.rs` carries it verbatim, and the first thing that looked at
/// it was `validated_compiled_body`. MEASURED on the real HTTP path before this landed, on a
/// scratch instance:
///
/// ```text
/// POST /missions/:id/versions  zones[0].rules = {"notInT241Vocabulary": 1}  -> 201 Created
/// GET  /missions/:id/compiled                                               -> 500
///      "/zones/0/rules: Additional properties are not allowed ('notInT241Vocabulary' was unexpected)"
/// POST /missions/:id/versions  zones[0].type  = "capture"                   -> 201 Created
/// GET  /missions/:id/compiled                                               -> 500
///      "/zones/0/type: \"capture\" is not one of \"spawn\", \"objective_capture\" or 4 other candidates"
/// ```
///
/// …and forever, because a `mission_versions` row is immutable. The author sees success; the
/// failure surfaces in front of a game server that supplied nothing but a mission id and can do
/// nothing about it. That is the T-367 class exactly, arriving through the T-367 precheck itself:
/// `ZoneIn.rules` is `Option<serde_json::Value>`, so every one of these deserialises cleanly.
///
/// ## Why the verdict is the SERVE schema and not a new rule
///
/// The accept/reject decision below is `mission.schema.json#/$defs/zone` — lifted out of the same
/// embedded bytes [`validate_mission_document`] validates the compiled document against. "Save
/// accepted it" and "`/compiled` will validate" are therefore one sentence by construction, the
/// same property `flatten::scan_editor_payload_types` gets by running the compiler's own
/// deserialiser. A per-zone subschema written into `mission-editor-payload.schema.json` would be a
/// SECOND declaration of a vocabulary whose entire design premise (see `$defs/zoneRules`) is that
/// there is exactly one place a misspelled rule key can be caught.
///
/// ## Why the POST-QUANTISATION shape
///
/// The compile rounds every zone coordinate to 0.1 m, and `round_coord(0.04) = 0.0` violates
/// `$defs/circle.r`'s `exclusiveMinimum: 0` — while `0.04` itself is perfectly schema-valid.
/// MEASURED: `circle r: 0.04` → save **201** → `/compiled` **500**
/// `"/zones/0/shape: {\"circle\":{...,\"r\":0.0}} is not valid under any of the schemas listed in
/// the 'oneOf' keyword"`. A click without a drag in the coming draw tool produces exactly that
/// radius, so a check that validated only the authored document would pass it straight through.
/// [`projected_zone`] therefore validates the row the compile will EMIT, not the one authored.
///
/// ## Why it cannot reject a payload that compiles today
///
/// [`projected_zone`] returns `None` for every zone `flatten_authored_zone` drops (no usable
/// shape, empty `id`, empty `type`). A dropped zone never reaches the document, so it cannot 500,
/// so refusing it here would break the one invariant T-367 pinned: the accept set at save must not
/// be narrower than the compile set.
///
/// ## Cost
///
/// O(zones), and zones are the BOUNDED array in this payload — a play area plus a handful of
/// objectives, unlike `editor.slots` where 367k is a measured live size and why that array carries
/// no per-item subschema. This walks the already-parsed instance and adds no parse.
fn scan_authored_zones(payload: &Value) -> Vec<String> {
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

/// T-450 — mirrors `TBD_MissionLoader.MISSION_FILE_MAX_BYTES` (`8 * 1024 * 1024`) and
/// `mission.schema.json` `x-tbd-missionFileMaxBytes`. JSON Schema cannot express whole-document
/// byte size on an object, so this code-side check is the enforceable pin that keeps a
/// schema-valid (but oversized) document from reaching the mod and dying at
/// `LoadFromProfileFile`.
const MISSION_FILE_MAX_BYTES: usize = 8 * 1024 * 1024;

/// Validate a compiled mod mission document against `mission.schema.json` (the
/// game-server contract served at `/missions/:id/compiled`).
///
/// @contract mission.schema.json#/
pub fn validate_mission_document(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    if raw.len() > MISSION_FILE_MAX_BYTES {
        return Ok(vec![format!(
            "/: document exceeds MISSION_FILE_MAX_BYTES ({} B > {} B) — \
             TBD_MissionLoader.c LoadFromProfileFile would refuse this file",
            raw.len(),
            MISSION_FILE_MAX_BYTES
        )]);
    }
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(&V, MISSION_SCHEMA, raw, "document is not valid JSON")
}

/// Validate a raw T-150 items envelope against `registry-items.schema.json`
/// (the Workbench export ingested by `import-registry`, T-068.9).
///
/// @contract registry-items.schema.json#/
pub fn validate_registry_items_envelope(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(&V, REGISTRY_ITEMS_SCHEMA, raw, "envelope is not valid JSON")
}

/// Validate a faction-library document against `faction-library.schema.json`
/// (the jsonb doc of a user_factions row, T-153).
///
/// @contract faction-library.schema.json#/
pub fn validate_faction_library_doc(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(&V, FACTION_LIBRARY_SCHEMA, raw, "doc is not valid JSON")
}

/// Validate a raw T-150 compat envelope against `registry-compat.schema.json`
/// (the Workbench edge export ingested by `import-registry`, T-068.9).
///
/// @contract registry-compat.schema.json#/
pub fn validate_registry_compat_envelope(raw: &[u8]) -> Result<Vec<String>, ContractError> {
    static V: OnceLock<Result<Validator, String>> = OnceLock::new();
    run(
        &V,
        REGISTRY_COMPAT_SCHEMA,
        raw,
        "envelope is not valid JSON",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_engine_core::mission::wire_safety::CargoPhys;

    #[test]
    fn editor_schema_compiles_and_accepts_minimal_payload() {
        // A minimal valid editor payload (schemaVersion int + editor block).
        let ok = br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}"#;
        let details = validate_mission_editor_payload(ok).expect("compiles");
        assert!(details.is_empty(), "expected valid, got {details:?}");
    }

    #[test]
    fn invalid_json_reports_detail_not_error() {
        let details = validate_mission_editor_payload(b"not json").expect("compiles");
        assert_eq!(details, vec!["payload is not valid JSON".to_string()]);
    }

    /// T-181.44 — the save-time catch, on the channel `create_version` already answers 400 with.
    /// Before this, the same payload validated CLEAN here and only failed later at `/compiled`,
    /// as a 500 the author never saw.
    #[test]
    fn control_character_in_an_authored_slot_string_is_a_save_time_finding() {
        let bad = br#"{"schemaVersion":1,"editor":{"factions":[],
            "squads":[{"id":"sq1","callsign":"AL\tPHA","slotIds":["s1"]}],
            "slots":[{"id":"s1","role":"SL"}],"editorLayers":[]}}"#;
        let details = validate_mission_editor_payload(bad).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(
            details[0].starts_with("/editor/squads/0/callsign:"),
            "{details:?}"
        );
        assert!(details[0].contains("TAB (U+0009)"), "{details:?}");

        // The schema half of the same call is untouched: a payload that is merely wire-safe still
        // has to satisfy `mission-editor-payload.schema.json`.
        let ok = br#"{"schemaVersion":1,"editor":{"factions":[],
            "squads":[{"id":"sq1","callsign":"ALPHA","slotIds":["s1"]}],
            "slots":[{"id":"s1","role":"SL"}],"editorLayers":[]}}"#;
        assert!(
            validate_mission_editor_payload(ok)
                .expect("compiles")
                .is_empty()
        );
    }

    /// T-416 — over-capacity cargo joins the same `details` channel as wire-safety, when the
    /// caller supplies a phys catalog. Without a catalog the walk is silent (never invent).
    #[test]
    fn over_capacity_cargo_is_a_save_time_finding_with_catalog() {
        let mut catalog = CargoPhysCatalog::new();
        catalog.insert(
            "mag".into(),
            CargoPhys {
                display_name: "Mag".into(),
                weight_kg: Some(0.5),
                volume_cm3: Some(60.0),
                ..CargoPhys::default()
            },
        );
        catalog.insert(
            "vest_rn".into(),
            CargoPhys {
                display_name: "Plate Carrier".into(),
                max_weight_kg: Some(5.0),
                max_volume_cm3: Some(200.0),
                ..CargoPhys::default()
            },
        );
        // 4 × 60 = 240 > 200 cm³.
        let bad = br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":4}]}}]}}"#;
        let details =
            validate_mission_editor_payload_with_catalog(bad, &catalog).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(
            details[0].starts_with("/editor/slots/0/loadout/wear/vest:"),
            "{details:?}"
        );
        assert!(details[0].contains("240 / 200 cm³"), "{details:?}");

        // One magazine fewer: under capacity → clean.
        let ok = br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":3}]}}]}}"#;
        assert!(
            validate_mission_editor_payload_with_catalog(ok, &catalog)
                .expect("compiles")
                .is_empty(),
            "under-capacity must stay clean"
        );

        // Same over-capacity bytes, empty catalog → silent (signature-compatible entry point).
        assert!(
            validate_mission_editor_payload(bad)
                .expect("compiles")
                .is_empty(),
            "empty catalog must not invent a limit"
        );
    }

    #[test]
    fn mission_document_schema_compiles() {
        // Empty object violates the required keys → non-empty details, but compiles.
        let details = validate_mission_document(b"{}").expect("compiles");
        assert!(!details.is_empty(), "empty doc should be schema-invalid");
    }

    /// T-450 — a document that is schema-shaped can still be refused solely on raw byte size.
    /// Pads `meta.author` (no maxLength) past `MISSION_FILE_MAX_BYTES`; the size finding must
    /// fire before (or instead of) any schema finding, and must name the mod constant.
    #[test]
    fn oversized_mission_document_is_rejected_on_byte_ceiling() {
        let mut doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../packages/tbd-schema/golden-missions/last-stand-at-montfort.json"
        ))
        .expect("golden");
        // Keep the document structurally valid so a missing size check would pass schema alone.
        let pad = "x".repeat(MISSION_FILE_MAX_BYTES);
        doc["meta"]["author"] = serde_json::Value::String(pad);
        let raw = serde_json::to_vec(&doc).expect("serialize");
        assert!(
            raw.len() > MISSION_FILE_MAX_BYTES,
            "pad must push past the ceiling (got {} B)",
            raw.len()
        );

        let details = validate_mission_document(&raw).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(
            details[0].starts_with("/: document exceeds MISSION_FILE_MAX_BYTES"),
            "{details:?}"
        );
        assert!(
            details[0].contains(&MISSION_FILE_MAX_BYTES.to_string()),
            "{details:?}"
        );
    }

    // ---- T-581: zone vocabulary at the save boundary ----

    /// A payload carrying one zone, wrapped so it also satisfies the rest of the editor schema.
    fn payload_with_zones(zones: &str) -> Vec<u8> {
        format!(
            r#"{{"schemaVersion":1,"zones":{zones},
                "editor":{{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}}}"#
        )
        .into_bytes()
    }

    /// **DEFECT 1** — the executed repro, now a save-time finding.
    ///
    /// RED without `scan_authored_zones`: `details` is empty, `create_version` answers 201, and
    /// `GET /compiled` answers 500 forever because a `mission_versions` row is immutable.
    #[test]
    fn undeclared_zone_rule_key_is_a_save_time_finding() {
        let bad = payload_with_zones(
            r#"[{"id":"z1","type":"objective_capture",
                 "shape":{"circle":{"x":100,"z":200,"r":50}},
                 "rules":{"notInT241Vocabulary":1}}]"#,
        );
        let details = validate_mission_editor_payload(&bad).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(details[0].starts_with("/zones/0/rules:"), "{details:?}");
        assert!(details[0].contains("notInT241Vocabulary"), "{details:?}");

        // A rule key that IS in the T-241 vocabulary still saves clean.
        let ok = payload_with_zones(
            r#"[{"id":"z1","type":"objective_capture",
                 "shape":{"circle":{"x":100,"z":200,"r":50}},
                 "rules":{"captureSeconds":90}}]"#,
        );
        assert!(
            validate_mission_editor_payload(&ok)
                .expect("compiles")
                .is_empty(),
            "a declared rule must still be accepted"
        );
    }

    /// **DEFECT 1, second mechanism** — `type` outside the six-value enum, same 201-then-500.
    #[test]
    fn zone_type_outside_the_enum_is_a_save_time_finding() {
        let bad = payload_with_zones(
            r#"[{"id":"z1","type":"capture","shape":{"circle":{"x":1,"z":2,"r":50}}}]"#,
        );
        let details = validate_mission_editor_payload(&bad).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(details[0].starts_with("/zones/0/type:"), "{details:?}");

        for kind in [
            "spawn",
            "objective_capture",
            "objective_destroy",
            "objective_hold_until",
            "boundary",
            "base_protection",
        ] {
            let ok = payload_with_zones(&format!(
                r#"[{{"id":"z1","type":"{kind}","shape":{{"circle":{{"x":1,"z":2,"r":50}}}}}}]"#
            ));
            assert!(
                validate_mission_editor_payload(&ok)
                    .expect("compiles")
                    .is_empty(),
                "the declared type {kind} must still be accepted"
            );
        }
    }

    /// **DEFECT 2 — the post-quantisation case, and the reason this check cannot be a rule on the
    /// authored document.**
    ///
    /// `r: 0.04` satisfies `$defs/circle.r` `exclusiveMinimum: 0` exactly as written, and
    /// `shape_from_input`'s own `r > 0.0` gate passes it. Only `round_coord` makes it `0.0`, and
    /// only the EMITTED row shows that. A click without a drag in the draw tool produces this
    /// radius. RED if `projected_shape` is changed to carry the authored radius through unrounded.
    #[test]
    fn click_without_drag_radius_is_caught_after_quantisation() {
        let bad = payload_with_zones(
            r#"[{"id":"z1","type":"boundary","shape":{"circle":{"x":100,"z":200,"r":0.04}}}]"#,
        );
        let details = validate_mission_editor_payload(&bad).expect("compiles");
        assert_eq!(details.len(), 1, "{details:?}");
        assert!(details[0].starts_with("/zones/0/shape:"), "{details:?}");
        // The author typed 0.04; the raw schema finding quotes an r of 0.0 they never wrote.
        assert!(details[0].contains("0.04"), "{details:?}");
        assert!(details[0].contains("0.05 m"), "{details:?}");

        // 0.05 is the first radius that survives the 0.1 m grid — it must still be accepted.
        let ok = payload_with_zones(
            r#"[{"id":"z1","type":"boundary","shape":{"circle":{"x":100,"z":200,"r":0.05}}}]"#,
        );
        assert!(
            validate_mission_editor_payload(&ok)
                .expect("compiles")
                .is_empty(),
            "the smallest radius that survives quantisation must be accepted"
        );
    }

    /// The invariant T-367 pinned: the accept set at save must not be narrower than the compile
    /// set. `flatten_authored_zone` DROPS each of these before the document exists, so none can
    /// reach a game server and none may be refused here.
    #[test]
    fn zones_the_compile_drops_are_not_refused() {
        for dropped in [
            r#"[{}]"#,                                                              // nothing at all
            r#"[{"id":"z1","type":"boundary"}]"#,                                   // no shape
            r#"[{"id":"","type":"boundary","shape":{"circle":{"r":9}}}]"#,          // empty id
            r#"[{"id":"z1","type":"","shape":{"circle":{"r":9}}}]"#,                // empty type
            r#"[{"id":"z1","type":"boundary","shape":{"circle":{"r":0}}}]"#,        // r not > 0
            r#"[{"id":"z1","type":"boundary","shape":{"polygon":[[1,2],[3,4]]}}]"#, // < 3 vertices
            r#"[]"#,                                                                // no zones
        ] {
            let details =
                validate_mission_editor_payload(&payload_with_zones(dropped)).expect("compiles");
            assert!(
                details.is_empty(),
                "a zone the compile drops must not be refused at save: {dropped} -> {details:?}"
            );
        }
    }

    /// A polygon zone round-trips, and its vertices are quantised the same way.
    #[test]
    fn polygon_zone_is_accepted_and_quantised() {
        let ok = payload_with_zones(
            r#"[{"id":"z_bounds","type":"boundary","label":"Play area",
                 "faction":"BLUFOR",
                 "shape":{"polygon":[[0,0],[1000.256,0],[1000.256,175.789],[0,175.789]]}}]"#,
        );
        assert!(
            validate_mission_editor_payload(&ok)
                .expect("compiles")
                .is_empty(),
            "an uppercase authored faction must NOT be validated against $defs/factionKey (T-357)"
        );
    }

    /// The quantisation this file mirrors is `flatten::round_coord`, which is private there.
    /// Pin it against that source so the mirror cannot drift silently — the drift T-346 named as
    /// the real bug class. RED if `flatten.rs` changes its grid without this file following.
    #[test]
    fn zone_quantisation_mirrors_flatten() {
        let flatten = include_str!("../../../../../crates/map-engine-core/src/mission/flatten.rs");
        let body = flatten
            .split("fn round_coord(v: f64) -> f64 {")
            .nth(1)
            .expect("flatten::round_coord must exist");
        let expr = body.split('}').next().expect("body").trim();
        assert_eq!(
            expr, "(v * 10.0).round() / 10.0",
            "flatten::round_coord changed — update contract::validate::round_coord to match"
        );
        // And the mirror agrees on the value that produced the defect.
        assert_eq!(round_coord(0.04), 0.0);
        assert_eq!(round_coord(0.05), 0.1);
        assert_eq!(round_coord(1000.256), 1000.3);
    }

    /// The verdict must come from `mission.schema.json` itself, not from a copy of it. If the
    /// wrapper stopped resolving `#/$defs/zone`, every zone would validate against nothing and
    /// this whole pass would go silently vacuous — the signature defect.
    #[test]
    fn zone_schema_resolves_the_real_defs() {
        let v = compile_zone_schema().expect("wrapper must compile");
        assert!(
            v.is_valid(
                &json!({"id":"z1","type":"boundary","shape":{"circle":{"x":0,"z":0,"r":5}}})
            ),
            "a good zone must validate"
        );
        // Each of these can only fail if a DIFFERENT $def was reached: zone → shape → circle,
        // zone → zoneRules.
        assert!(
            !v.is_valid(&json!({"id":"z1","type":"nope","shape":{"circle":{"x":0,"z":0,"r":5}}}))
        );
        assert!(!v.is_valid(
            &json!({"id":"z1","type":"boundary","shape":{"circle":{"x":0,"z":0,"r":0}}})
        ));
        assert!(!v.is_valid(
            &json!({"id":"z1","type":"boundary","shape":{"circle":{"x":0,"z":0,"r":5}},
                    "rules":{"graceSeconds":-1}})
        ));
    }

    #[test]
    fn schema_x_tbd_mission_file_max_bytes_matches_mod_constant() {
        let schema: serde_json::Value =
            serde_json::from_str(MISSION_SCHEMA).expect("mission.schema.json");
        let pinned = schema["x-tbd-missionFileMaxBytes"]
            .as_u64()
            .expect("x-tbd-missionFileMaxBytes must be present on mission.schema.json");
        assert_eq!(
            pinned as usize, MISSION_FILE_MAX_BYTES,
            "schema keyword drifted from validate_mission_document / TBD_MissionLoader"
        );
    }
}
