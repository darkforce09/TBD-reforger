//! Role: conditions.
//! Position: `mission/extensions/objectives/win_conditions` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Serialize, Value};

/// The five win rules the editor may author, in the order the mode picker lists them.
pub const AUTHORED_MODES: &[&str] = &["attrition", "objective", "extraction", "vip", "timeout"];

/// The five `endOn` triggers, `mission.schema.json#/$defs/winConditions/properties/endOn`.
pub const END_ON_TRIGGERS: &[&str] = &[
    "time_limit",
    "all_objectives_captured",
    "faction_eliminated",
    "objective_destroyed",
    "hold_expired",
];

/// `endOn` when the author checked nothing the mission can honour — see [`AuthoredWinConditions`].
pub const FALLBACK_TRIGGER: &str = "time_limit";

/// `mode: timeout` — the smallest authorable round, in minutes. `$defs/winConditions .timeoutMinutes` `minimum: 1`.
pub const TIMEOUT_MINUTES_MIN: i64 = 1;

/// `mode: timeout` — the largest authorable round, in minutes (24 h). `$defs/winConditions .timeoutMinutes` `maximum: 1440`.
pub const TIMEOUT_MINUTES_MAX: i64 = 1440;

/// One authored block, parsed and checked.
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinConditionParams {
    /// `mode: extraction` — the `zones[].id` the extracting side must reach.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_zone_id: Option<String>,

    /// `mode: vip` — the `slots[].uid` of the protected player.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vip_slot_id: Option<String>,

    /// `mode: timeout` — the round length in whole minutes. Projected onto `flow.timeLimitSeconds` by the emitter; it never becomes a second clock.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_minutes: Option<i64>,
}

impl WinConditionParams {
    /// Is empty using the supplied domain data.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.extraction_zone_id.is_none()
            && self.vip_slot_id.is_none()
            && self.timeout_minutes.is_none()
    }
}

/// The param key each mode takes, or `None` for a mode that takes none.
#[must_use]
pub fn param_key_for_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "extraction" => Some("extractionZoneId"),
        "vip" => Some("vipSlotId"),
        "timeout" => Some("timeoutMinutes"),
        _ => None,
    }
}

/// The param keys a mode may carry WITHOUT being required to, or `&[]`.
#[must_use]
pub fn optional_param_keys_for_mode(mode: &str) -> &'static [&'static str] {
    match mode {
        "vip" => &["extractionZoneId"],
        _ => &[],
    }
}

/// May `mode` carry `key` at all — as its required param or as an optional one?.
pub(super) fn mode_may_carry(mode: &str, key: &str) -> bool {
    param_key_for_mode(mode) == Some(key) || optional_param_keys_for_mode(mode).contains(&key)
}

/// Every param key, in `$defs/winConditions` property order.
pub(super) const PARAM_KEYS: &[&str] = &["extractionZoneId", "vipSlotId", "timeoutMinutes"];

/// Parse and validate one authored `winConditions` value, or say exactly what is wrong with it.
pub fn parse(value: &Value) -> Result<AuthoredWinConditions, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`winConditions` must be an object, not {}",
            type_name(value)
        ));
    };

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

    let owned = param_key_for_mode(mode);
    for key in PARAM_KEYS {
        if obj.contains_key(*key) && !mode_may_carry(mode, key) {
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

        _ => {}
    }

    for key in optional_param_keys_for_mode(mode) {
        if !obj.contains_key(*key) {
            continue;
        }
        match *key {
            "extractionZoneId" => {
                params.extraction_zone_id = Some(required_id(obj, "extractionZoneId")?);
            }
            other => {
                return Err(format!(
                    "`winConditions.{other}` is listed as optional for mode {} and this parser has \
                     no branch for it — add one rather than dropping the author's value",
                    quote(mode)
                ));
            }
        }
    }

    Ok(AuthoredWinConditions {
        mode: mode.to_string(),
        end_on,
        params,
    })
}

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's validator, which needs the verdict and not the parse.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

/// A required non-blank string param.
pub(super) fn required_id(
    obj: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, String> {
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
pub(super) fn required_timeout_minutes(
    obj: &serde_json::Map<String, Value>,
) -> Result<i64, String> {
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

/// Which mode owns a param key — the inverse of [`param_key_for_mode`], used only to write the refusal sentence. Unreachable for anything outside [`PARAM_KEYS`]; it answers `"?"` rather than panicking, because a diagnostic is never worth a compile failure.
pub(super) fn mode_for_param_key(key: &str) -> &'static str {
    match key {
        "extractionZoneId" => "extraction",
        "vipSlotId" => "vip",
        "timeoutMinutes" => "timeout",
        _ => "?",
    }
}

/// `"value"` — quoting for a refusal sentence, so an empty or space-bearing value is visible.
pub(super) fn quote(s: &str) -> String {
    format!("{s:?}")
}

/// The JSON type name for a refusal sentence.
pub(super) fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}
