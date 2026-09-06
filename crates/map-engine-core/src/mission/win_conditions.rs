//! T-936.1 — the authored `winConditions` block: parse, validate, and the vocabulary both ends of
//! the wire agree on.
//!
//! ══ What this closes ═════════════════════════════════════════════════════════════════════════
//! `mission.schema.json` has REQUIRED `winConditions` since the beginning and `$defs/winConditions`
//! declared `mode` as a free `minLength: 1` string. `flatten_to_mod_document` built the block from
//! a struct literal with `mode: "attrition"` in it and derived `endOn` from the sides that hold
//! slots, so a payload could carry `{"mode": "vip", "vipSlotId": "s1"}` and the compiled document
//! still said `attrition` — the author's choice dropped on the floor between `compile_payload` and
//! the emitter, in silence, with nothing downstream able to notice because the field admitted any
//! string at all.
//!
//! ══ The two vocabularies, and why they are not the same size ═════════════════════════════════
//! [`AUTHORED_MODES`] is the five the EDITOR may author, and it is what `mission-editor-payload
//! .schema.json` pins. `mission.schema.json`'s enum is seven wide: it also grandfathers
//! `points_then_attrition` and `defender_holds_or_attacker_destroys`, which four committed
//! hand-authored goldens carry and which no editor payload can produce. That is the same LAYER
//! distinction `$defs/editorFaction` draws for `key` (uppercase author graph, slugged wire), and
//! it is why [`parse`] rejects a golden's mode rather than accepting it: this module is the
//! AUTHORING gate, not the wire reader.
//!
//! ══ Absent means absent ══════════════════════════════════════════════════════════════════════
//! A payload that authors NO `winConditions` must compile to the bytes it compiled to before this
//! module existed — the whole Class-R byte-parity claim rests on it. So there is no default
//! constructed here and no "empty" [`AuthoredWinConditions`]: absence is `Option::None` at the one
//! call site in `flatten.rs`, which then runs exactly the code it ran before.

use serde::Serialize;
use serde_json::Value;

/// The five win rules the editor may author, in the order the mode picker lists them.
///
/// `mission.schema.json`'s `$defs/winConditions.mode` enum is a SUPERSET of this (see the module
/// header). Keep this list and `mission-editor-payload.schema.json`'s five-value enum in step —
/// [`tests::the_authored_modes_are_the_editor_payload_schema_s_enum`] is what holds them there, so
/// adding a sixth mode fails in the same commit rather than in a later one.
pub const AUTHORED_MODES: &[&str] = &["attrition", "objective", "extraction", "vip", "timeout"];

/// The five `endOn` triggers, `mission.schema.json#/$defs/winConditions/properties/endOn`.
///
/// UNCHANGED by T-936.1 and deliberately so: three are driven by
/// `TBD_ObjectiveRegistry.EvaluateEndTriggers` and two by `TBD_FrameworkManager.TickWinConditions`,
/// and this ticket adds a win RULE on top of them rather than a sixth way to end a round.
pub const END_ON_TRIGGERS: &[&str] = &[
    "time_limit",
    "all_objectives_captured",
    "faction_eliminated",
    "objective_destroyed",
    "hold_expired",
];

/// `endOn` when the author checked nothing the mission can honour — see [`AuthoredWinConditions`].
///
/// Not a policy choice: `endOn` is `minItems: 1`, so an empty array is an invalid document, and
/// `time_limit` is the one trigger every mission can serve (`flow.timeLimitSeconds` always has a
/// value, authored or defaulted). It is also what the pre-T-936.1 derivation always emitted first.
pub const FALLBACK_TRIGGER: &str = "time_limit";

/// `mode: timeout` — the smallest authorable round, in minutes. `$defs/winConditions
/// .timeoutMinutes` `minimum: 1`.
pub const TIMEOUT_MINUTES_MIN: i64 = 1;

/// `mode: timeout` — the largest authorable round, in minutes (24 h). `$defs/winConditions
/// .timeoutMinutes` `maximum: 1440`.
///
/// The ceiling exists for the reason `$defs/zoneRules.captureSeconds` has one: a typo'd `600`
/// where `60` was meant is a round nobody present can finish, and a value the schema would take is
/// not the same as a value an author meant.
pub const TIMEOUT_MINUTES_MAX: i64 = 1440;

/// One authored block, parsed and checked.
///
/// `params` is FLATTENED into the emitted `winConditions` object rather than nested under a
/// `params` key, because that is the shape `$defs/winConditions` declares
/// (`{mode, endOn, vipSlotId}`, not `{mode, endOn, params: {vipSlotId}}`) and the shape
/// `JsonLoadContext` binds member-by-name on the mod side. An absent param serialises to nothing,
/// which is what keeps a mission with no params byte-identical to one compiled before the params
/// existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredWinConditions {
    /// One of [`AUTHORED_MODES`].
    pub mode: String,
    /// One or more of [`END_ON_TRIGGERS`], deduplicated, in the author's order.
    pub end_on: Vec<String>,
    /// The one param the authored mode takes, or all-`None` for `attrition` / `objective`.
    pub params: WinConditionParams,
}

/// The per-mode params, serde-flattened into the emitted `winConditions` object.
///
/// Exactly one of these is `Some` on a well-formed block, and WHICH one is decided by `mode` —
/// [`parse`] refuses a param belonging to a different mode rather than carrying it to a wire
/// nothing will read it from. Kept as three named `Option`s rather than an enum so the serde
/// emission is one derive and the schema property names are visible at the field they name.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinConditionParams {
    /// `mode: extraction` — the `zones[].id` the extracting side must reach.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_zone_id: Option<String>,
    /// `mode: vip` — the `slots[].uid` of the protected player.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vip_slot_id: Option<String>,
    /// `mode: timeout` — the round length in whole minutes. Projected onto `flow.timeLimitSeconds`
    /// by the emitter; it never becomes a second clock.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_minutes: Option<i64>,
}

impl WinConditionParams {
    /// True when the author set nothing — the shape `attrition` and `objective` always have, and
    /// therefore the shape whose emitted bytes are identical to a pre-T-936.1 document's.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.extraction_zone_id.is_none()
            && self.vip_slot_id.is_none()
            && self.timeout_minutes.is_none()
    }
}

/// The param key each mode takes, or `None` for a mode that takes none.
///
/// One table, read by both the "you must author this" gate and the "this belongs to another mode"
/// gate, so the two can never disagree about which param goes with which rule.
#[must_use]
pub fn param_key_for_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "extraction" => Some("extractionZoneId"),
        "vip" => Some("vipSlotId"),
        "timeout" => Some("timeoutMinutes"),
        _ => None,
    }
}

/// Every param key, in `$defs/winConditions` property order.
const PARAM_KEYS: &[&str] = &["extractionZoneId", "vipSlotId", "timeoutMinutes"];

/// Parse and validate one authored `winConditions` value, or say exactly what is wrong with it.
///
/// The error string is a whole sentence naming the value it refused, because it reaches the author
/// through the compile's refusal path and "invalid winConditions" tells nobody which of five modes
/// and three params was the problem.
///
/// # Errors
/// Returns the refusal clause when the value is not an object, `mode` is missing or outside
/// [`AUTHORED_MODES`], `endOn` is missing / empty / carries a trigger outside [`END_ON_TRIGGERS`],
/// the mode's own param is missing or out of range, or a param belonging to a DIFFERENT mode is
/// present.
pub fn parse(value: &Value) -> Result<AuthoredWinConditions, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`winConditions` must be an object, not {}",
            type_name(value)
        ));
    };

    // `mode` — the whole point of the block, so it is checked first and by identity, not by
    // "non-empty". A misspelled mode is a rule the game cannot honour, and admitting it is exactly
    // the free-string hole T-936.1 closes.
    let Some(mode) = obj.get("mode").and_then(Value::as_str) else {
        return Err(
            "`winConditions.mode` is required and must be a string — one of attrition, objective, \
             extraction, vip, timeout"
                .to_string(),
        );
    };
    if !AUTHORED_MODES.contains(&mode) {
        return Err(format!(
            "`winConditions.mode` is {} — the editor authors one of {}. (mission.schema.json also \
             admits two grandfathered hand-authored golden modes; the editor cannot produce them.)",
            quote(mode),
            AUTHORED_MODES.join(", ")
        ));
    }

    // `endOn` — schema `minItems: 1`, closed vocabulary. Deduplicated in the author's order: the
    // card is a checklist so a repeat cannot be typed, but a hand-staged payload can carry one and
    // a document that declares `time_limit` twice reads as a document nobody checked.
    let Some(raw_end_on) = obj.get("endOn").and_then(Value::as_array) else {
        return Err(format!(
            "`winConditions.endOn` is required and must be an array of one or more of {}",
            END_ON_TRIGGERS.join(", ")
        ));
    };
    if raw_end_on.is_empty() {
        return Err(
            "`winConditions.endOn` is empty — a mission that declares no end trigger runs until an \
             admin ends it, which is why the schema requires at least one"
                .to_string(),
        );
    }
    let mut end_on: Vec<String> = Vec::with_capacity(raw_end_on.len());
    for t in raw_end_on {
        let Some(t) = t.as_str() else {
            return Err(format!(
                "`winConditions.endOn` carries {}, and every entry must be one of {}",
                type_name(t),
                END_ON_TRIGGERS.join(", ")
            ));
        };
        if !END_ON_TRIGGERS.contains(&t) {
            return Err(format!(
                "`winConditions.endOn` declares {} — the round can only end on {}",
                quote(t),
                END_ON_TRIGGERS.join(", ")
            ));
        }
        if !end_on.iter().any(|kept| kept == t) {
            end_on.push(t.to_string());
        }
    }

    // Params. The mode decides which key is REQUIRED and which keys are REFUSED, off one table
    // ([`param_key_for_mode`]), so "vip needs vipSlotId" and "vip must not carry timeoutMinutes"
    // cannot drift apart.
    let owned = param_key_for_mode(mode);
    for key in PARAM_KEYS {
        if obj.contains_key(*key) && owned != Some(*key) {
            return Err(format!(
                "`winConditions.{key}` belongs to mode {}, not to the authored mode {} — a param \
                 the mode does not read would ride the wire and change nothing",
                quote(mode_for_param_key(key)),
                quote(mode)
            ));
        }
    }

    let mut params = WinConditionParams::default();
    match owned {
        Some("extractionZoneId") => {
            params.extraction_zone_id = Some(required_id(obj, "extractionZoneId")?);
        }
        Some("vipSlotId") => {
            params.vip_slot_id = Some(required_id(obj, "vipSlotId")?);
        }
        Some("timeoutMinutes") => {
            params.timeout_minutes = Some(required_timeout_minutes(obj)?);
        }
        // `attrition` and `objective` take no param — the registry's endOn triggers and the
        // faction-elimination check are already everything those two rules need.
        _ => {}
    }

    Ok(AuthoredWinConditions {
        mode: mode.to_string(),
        end_on,
        params,
    })
}

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's
/// validator, which needs the verdict and not the parse.
///
/// # Errors
/// Every error [`parse`] returns.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

/// A required non-blank string param.
fn required_id(obj: &serde_json::Map<String, Value>, key: &str) -> Result<String, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`winConditions.{key}` is required by mode {} and is missing",
            quote(mode_for_param_key(key))
        ));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`winConditions.{key}` must be a string, not {}",
            type_name(raw)
        ));
    };
    // Trimmed and refused-when-blank rather than trimmed-and-carried: the schema says
    // `minLength: 1`, and a whitespace id resolves against nothing on either side of the wire.
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "`winConditions.{key}` is blank — mode {} needs an id it can resolve",
            quote(mode_for_param_key(key))
        ));
    }
    Ok(trimmed.to_string())
}

/// The `timeout` param, range-checked against the schema's own bounds.
///
/// **The two comparisons below are the ones the ticket's perturbation inverts.** They are written
/// as two separate one-sided tests rather than one `RangeInclusive::contains` for two reasons: each
/// bound gets the sentence that fits it (a five-second round and a five-day round are different
/// mistakes), and either comparison can be inverted on its own into a program that still compiles
/// and means the opposite — which is what makes a perturbation proof worth running at all.
fn required_timeout_minutes(obj: &serde_json::Map<String, Value>) -> Result<i64, String> {
    let Some(raw) = obj.get("timeoutMinutes") else {
        return Err(
            "`winConditions.timeoutMinutes` is required by mode \"timeout\" and is missing"
                .to_string(),
        );
    };
    let Some(minutes) = raw.as_i64() else {
        return Err(format!(
            "`winConditions.timeoutMinutes` must be a whole number of minutes, not {}",
            type_name(raw)
        ));
    };
    if minutes < TIMEOUT_MINUTES_MIN {
        return Err(format!(
            "`winConditions.timeoutMinutes` is {minutes} — the shortest authorable round is \
             {TIMEOUT_MINUTES_MIN} minute. Zero or negative is not a round that ends immediately, \
             it is a round the clock cannot express."
        ));
    }
    if minutes > TIMEOUT_MINUTES_MAX {
        return Err(format!(
            "`winConditions.timeoutMinutes` is {minutes} — the longest authorable round is \
             {TIMEOUT_MINUTES_MAX} minutes (24 h). A round nobody present can finish is the same \
             defect as a `captureSeconds` typo."
        ));
    }
    Ok(minutes)
}

/// Which mode owns a param key — the inverse of [`param_key_for_mode`], used only to write the
/// refusal sentence. Unreachable for anything outside [`PARAM_KEYS`]; it answers `"?"` rather than
/// panicking, because a diagnostic is never worth a compile failure.
fn mode_for_param_key(key: &str) -> &'static str {
    match key {
        "extractionZoneId" => "extraction",
        "vipSlotId" => "vip",
        "timeoutMinutes" => "timeout",
        _ => "?",
    }
}

/// `"value"` — quoting for a refusal sentence, so an empty or space-bearing value is visible.
fn quote(s: &str) -> String {
    format!("{s:?}")
}

/// The JSON type name for a refusal sentence.
fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vip() -> Value {
        json!({"mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "s-12"})
    }

    #[test]
    fn a_vip_block_parses_with_its_param() {
        let got = parse(&vip()).expect("parses");
        assert_eq!(got.mode, "vip");
        assert_eq!(got.end_on, ["faction_eliminated"]);
        assert_eq!(got.params.vip_slot_id.as_deref(), Some("s-12"));
        assert!(got.params.extraction_zone_id.is_none());
        assert!(got.params.timeout_minutes.is_none());
    }

    #[test]
    fn attrition_and_objective_take_no_param() {
        for mode in ["attrition", "objective"] {
            let got = parse(&json!({"mode": mode, "endOn": ["time_limit"]})).expect("parses");
            assert!(got.params.is_empty(), "{mode} must carry no param");
        }
    }

    #[test]
    fn a_mode_outside_the_editor_vocabulary_is_refused() {
        // Including the two `mission.schema.json` grandfathers — this is the AUTHORING gate, and
        // the editor cannot produce them (see the module header's layer note).
        for mode in [
            "points_then_attrition",
            "defender_holds_or_attacker_destroys",
            "vip_hunt",
        ] {
            let err = parse(&json!({"mode": mode, "endOn": ["time_limit"]}))
                .expect_err("must refuse {mode}");
            assert!(err.contains("winConditions.mode"), "{err}");
            assert!(err.contains(mode), "the refusal must name the value: {err}");
        }
    }

    #[test]
    fn a_missing_mode_param_is_refused_and_names_the_mode() {
        for (mode, key) in [
            ("extraction", "extractionZoneId"),
            ("vip", "vipSlotId"),
            ("timeout", "timeoutMinutes"),
        ] {
            let err = parse(&json!({"mode": mode, "endOn": ["time_limit"]}))
                .expect_err("must refuse a missing param");
            assert!(err.contains(key), "{err}");
            assert!(err.contains(mode), "{err}");
        }
    }

    #[test]
    fn a_param_belonging_to_another_mode_is_refused() {
        let err = parse(&json!({
            "mode": "vip",
            "endOn": ["time_limit"],
            "vipSlotId": "s1",
            "timeoutMinutes": 30,
        }))
        .expect_err("timeoutMinutes does not belong to vip");
        assert!(err.contains("timeoutMinutes"), "{err}");
        assert!(err.contains("\"timeout\""), "{err}");
        assert!(err.contains("\"vip\""), "{err}");
    }

    /// **The perturbation target.** Inverting the two-sided comparison in
    /// [`required_timeout_minutes`] flips every row below.
    #[test]
    fn the_timeout_range_is_two_sided_and_inclusive() {
        let at = |m: i64| {
            parse(&json!({"mode": "timeout", "endOn": ["time_limit"], "timeoutMinutes": m}))
        };

        // Both bounds are INCLUSIVE and both accept.
        assert_eq!(
            at(TIMEOUT_MINUTES_MIN)
                .expect("the minimum is authorable")
                .params
                .timeout_minutes,
            Some(TIMEOUT_MINUTES_MIN)
        );
        assert_eq!(
            at(TIMEOUT_MINUTES_MAX)
                .expect("the maximum is authorable")
                .params
                .timeout_minutes,
            Some(TIMEOUT_MINUTES_MAX)
        );
        // A plain middle value, so an inversion that somehow kept the bounds still goes red.
        assert_eq!(
            at(90)
                .expect("90 minutes is authorable")
                .params
                .timeout_minutes,
            Some(90)
        );

        // One minute under and one minute over are both refused, and each refusal states the bound
        // it broke — which is what makes an inversion of ONE of the two comparisons visible.
        for bad in [TIMEOUT_MINUTES_MIN - 1, 0, -30] {
            let err = at(bad).expect_err("a round under the floor must be refused");
            assert!(err.contains("timeoutMinutes"), "{err}");
            assert!(
                err.contains(&format!(
                    "shortest authorable round is {TIMEOUT_MINUTES_MIN}"
                )),
                "the refusal must state the floor: {err}"
            );
        }
        for bad in [TIMEOUT_MINUTES_MAX + 1, 100_000] {
            let err = at(bad).expect_err("a round over the ceiling must be refused");
            assert!(err.contains("timeoutMinutes"), "{err}");
            assert!(
                err.contains(&format!(
                    "longest authorable round is {TIMEOUT_MINUTES_MAX}"
                )),
                "the refusal must state the ceiling: {err}"
            );
        }
    }

    #[test]
    fn end_on_is_a_closed_vocabulary_and_deduplicates_in_authored_order() {
        let got = parse(&json!({
            "mode": "attrition",
            "endOn": ["faction_eliminated", "time_limit", "faction_eliminated"],
        }))
        .expect("parses");
        assert_eq!(got.end_on, ["faction_eliminated", "time_limit"]);

        let err = parse(&json!({"mode": "attrition", "endOn": ["admin_ended"]}))
            .expect_err("must refuse an unknown trigger");
        assert!(err.contains("admin_ended"), "{err}");

        let err = parse(&json!({"mode": "attrition", "endOn": []}))
            .expect_err("must refuse an empty endOn");
        assert!(err.contains("endOn"), "{err}");
    }

    #[test]
    fn a_blank_or_wrong_typed_param_is_refused() {
        let err = parse(&json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "   "}))
            .expect_err("blank is not an id");
        assert!(err.contains("blank"), "{err}");

        let err = parse(&json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": 12}))
            .expect_err("a number is not an id");
        assert!(err.contains("must be a string"), "{err}");

        let err = parse(&json!({
            "mode": "timeout", "endOn": ["time_limit"], "timeoutMinutes": 12.5
        }))
        .expect_err("a fraction is not a whole minute");
        assert!(err.contains("whole number"), "{err}");
    }

    #[test]
    fn a_param_is_trimmed_before_it_reaches_the_wire() {
        let got = parse(&json!({
            "mode": "extraction", "endOn": ["time_limit"], "extractionZoneId": "  z-lz  "
        }))
        .expect("parses");
        assert_eq!(got.params.extraction_zone_id.as_deref(), Some("z-lz"));
    }

    #[test]
    fn a_non_object_block_is_refused_rather_than_defaulted() {
        for bad in [json!(null), json!("attrition"), json!([]), json!(7)] {
            assert!(parse(&bad).is_err(), "{bad} must not parse");
        }
    }

    /// Absent params serialise to NOTHING. This is the byte-parity claim at its smallest: a block
    /// with no params must emit exactly `{"mode":…,"endOn":…}`, the two keys a pre-T-936.1
    /// document carried.
    #[test]
    fn empty_params_serialise_to_no_keys_at_all() {
        let empty = WinConditionParams::default();
        assert!(empty.is_empty());
        assert_eq!(serde_json::to_string(&empty).expect("serialises"), "{}");

        let one = WinConditionParams {
            vip_slot_id: Some("s-12".to_string()),
            ..WinConditionParams::default()
        };
        assert!(!one.is_empty());
        assert_eq!(
            serde_json::to_string(&one).expect("serialises"),
            r#"{"vipSlotId":"s-12"}"#
        );
    }

    /// The two lists this module owns must be the schema's own, or the compile accepts a mode the
    /// document then fails validation on (or refuses one it would have accepted).
    ///
    /// Read out of the committed schema bytes rather than restated, so a schema edit that widens
    /// either vocabulary without touching this file fails HERE — the `url_guard` pattern.
    #[test]
    fn the_authored_modes_are_the_editor_payload_schema_s_enum() {
        const PAYLOAD_SCHEMA: &str = include_str!(
            "../../../../packages/tbd-schema/schema/mission-editor-payload.schema.json"
        );
        const MISSION_SCHEMA: &str =
            include_str!("../../../../packages/tbd-schema/schema/mission.schema.json");

        let payload: Value = serde_json::from_str(PAYLOAD_SCHEMA).expect("payload schema parses");
        let modes = payload["properties"]["winConditions"]["properties"]["mode"]["enum"]
            .as_array()
            .expect("the editor payload schema declares winConditions.mode as an enum");
        let modes: Vec<&str> = modes.iter().filter_map(Value::as_str).collect();
        assert_eq!(
            modes, AUTHORED_MODES,
            "mission-editor-payload.schema.json's mode enum and AUTHORED_MODES have drifted"
        );

        let mission: Value = serde_json::from_str(MISSION_SCHEMA).expect("mission schema parses");
        let wire = mission["$defs"]["winConditions"]["properties"]["mode"]["enum"]
            .as_array()
            .expect("mission.schema.json declares winConditions.mode as an enum");
        let wire: Vec<&str> = wire.iter().filter_map(Value::as_str).collect();
        for m in AUTHORED_MODES {
            assert!(
                wire.contains(m),
                "the wire enum must admit every authored mode; {m} is missing"
            );
        }

        let triggers = mission["$defs"]["winConditions"]["properties"]["endOn"]["items"]["enum"]
            .as_array()
            .expect("endOn items are an enum");
        let triggers: Vec<&str> = triggers.iter().filter_map(Value::as_str).collect();
        assert_eq!(
            triggers, END_ON_TRIGGERS,
            "END_ON_TRIGGERS and $defs/winConditions.endOn have drifted"
        );
    }

    /// The schema's own numeric bounds, read from the committed bytes for the same reason as
    /// above: the perturbation this ticket names inverts the comparison, and a constant that had
    /// quietly drifted from the schema would make the resulting red mean nothing.
    #[test]
    fn the_timeout_bounds_are_the_schema_s_own() {
        const MISSION_SCHEMA: &str =
            include_str!("../../../../packages/tbd-schema/schema/mission.schema.json");
        let mission: Value = serde_json::from_str(MISSION_SCHEMA).expect("mission schema parses");
        let t = &mission["$defs"]["winConditions"]["properties"]["timeoutMinutes"];
        assert_eq!(t["minimum"].as_i64(), Some(TIMEOUT_MINUTES_MIN));
        assert_eq!(t["maximum"].as_i64(), Some(TIMEOUT_MINUTES_MAX));
        assert_eq!(t["type"].as_str(), Some("integer"));
    }

    #[test]
    fn every_mode_that_takes_a_param_names_it_both_ways() {
        for mode in AUTHORED_MODES {
            if let Some(key) = param_key_for_mode(mode) {
                assert_eq!(mode_for_param_key(key), *mode, "{mode} round-trips");
                assert!(PARAM_KEYS.contains(&key), "{key} must be in PARAM_KEYS");
            }
        }
        // ...and no param key is orphaned.
        for key in PARAM_KEYS {
            assert!(
                AUTHORED_MODES.contains(&mode_for_param_key(key)),
                "{key} names a mode that is not authorable"
            );
        }
    }
}
