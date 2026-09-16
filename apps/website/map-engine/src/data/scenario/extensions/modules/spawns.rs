//! Role: spawns.
//! Position: `mission/extensions/modules` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Map, Value};

/// `kind` vocabulary `$defs/spawnModule.kind` declares.
pub const KINDS: &[&str] = &["wave", "garrison"];

/// Faction keys the spawner can resolve. Order matches SpawnManager's EngineFactionKey cases.
pub const FACTION_KEYS: &[&str] = &["blufor", "opfor", "indfor", "civ"];

/// Canonical max alive value.
pub const MAX_ALIVE: i64 = 32;

/// Canonical module keys value.
pub(super) const MODULE_KEYS: &[&str] = &[
    "id",
    "kind",
    "factionKey",
    "groupTemplate",
    "x",
    "z",
    "zoneId",
    "count",
    "intervalSeconds",
    "maxAlive",
    "triggerId",
];

/// One authored spawn module.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredSpawnModule {
    /// Id.
    pub id: String,
    /// Kind.
    pub kind: String,
    /// Faction key.
    pub faction_key: String,
    /// Group template.
    pub group_template: String,
    /// X.
    pub x: Option<f64>,
    /// Z.
    pub z: Option<f64>,
    /// Zone id.
    pub zone_id: Option<String>,
    /// Count.
    pub count: i64,
    /// Interval seconds.
    pub interval_seconds: Option<f64>,
    /// Max alive.
    pub max_alive: Option<i64>,
    /// Trigger id.
    pub trigger_id: Option<String>,
}

/// Exclusive placement: position XOR zone, and at least one side.
#[must_use]
pub fn placement_is_exclusive(has_position: bool, has_zone: bool) -> bool {
    has_position != has_zone
}

/// Parse an authored `spawnModules[]` array.
pub fn parse(value: &Value) -> Result<Vec<AuthoredSpawnModule>, String> {
    let Some(arr) = value.as_array() else {
        return Err(format!(
            "`spawnModules` must be an array, not {}",
            type_name(value)
        ));
    };
    if arr.is_empty() {
        return Err(
            "`spawnModules` is empty — an empty list is omitted rather than authored".into(),
        );
    }

    let mut out = Vec::with_capacity(arr.len());
    let mut seen: Vec<String> = Vec::with_capacity(arr.len());
    for (index, item) in arr.iter().enumerate() {
        let row = parse_module(item, index)?;
        if seen.iter().any(|s| s == &row.id) {
            return Err(format!(
                "`spawnModules[{index}]`.id is {} — each spawn module id must be unique",
                quote(&row.id)
            ));
        }
        seen.push(row.id.clone());
        out.push(row);
    }
    Ok(out)
}

/// [`parse`] with the value discarded — the [`crate::data::scenario::extensions::AUTHORED_BLOCKS`] row's validator.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

/// Parse module using the supplied domain data.
pub(super) fn parse_module(value: &Value, index: usize) -> Result<AuthoredSpawnModule, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`spawnModules[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };
    refuse_unknown(obj, MODULE_KEYS, &format!("spawnModules[{index}]"))?;

    let id = required_nonempty(obj, index, "id")?;
    let kind = required_nonempty(obj, index, "kind")?;
    if !KINDS.contains(&kind.as_str()) {
        return Err(format!(
            "`spawnModules[{index}]`.kind is {} — must be one of {}",
            quote(&kind),
            KINDS.join(", ")
        ));
    }
    let faction_key = required_nonempty(obj, index, "factionKey")?;
    if !FACTION_KEYS.contains(&faction_key.as_str()) {
        return Err(format!(
            "`spawnModules[{index}]`.factionKey is {} — must be a known faction ({})",
            quote(&faction_key),
            FACTION_KEYS.join(", ")
        ));
    }
    let group_template = required_nonempty(obj, index, "groupTemplate")?;

    let x = optional_finite(obj, index, "x")?;
    let z = optional_finite(obj, index, "z")?;
    if x.is_some() != z.is_some() {
        return Err(format!(
            "`spawnModules[{index}]` has incomplete position — x and z must be authored together"
        ));
    }
    let zone_id = optional_nonempty(obj, index, "zoneId")?;
    let has_position = x.is_some() && z.is_some();
    let has_zone = zone_id.is_some();
    if !placement_is_exclusive(has_position, has_zone) {
        return Err(format!(
            "`spawnModules[{index}]` must name either x+z or zoneId, never both and never neither"
        ));
    }

    let count = required_count(obj, index, "count")?;
    let max_alive = optional_count(obj, index, "maxAlive")?;
    let interval_seconds = optional_positive_seconds(obj, index, "intervalSeconds")?;
    let trigger_id = optional_nonempty(obj, index, "triggerId")?;

    Ok(AuthoredSpawnModule {
        id,
        kind,
        faction_key,
        group_template,
        x,
        z,
        zone_id,
        count,
        interval_seconds,
        max_alive,
        trigger_id,
    })
}

/// Refuse unknown using the supplied domain data.
pub(super) fn refuse_unknown(
    obj: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), String> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "`{path}` carries {key:?}, which the schema does not declare \
                 (additionalProperties is false)"
            ));
        }
    }
    Ok(())
}

/// Required nonempty using the supplied domain data.
pub(super) fn required_nonempty(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<String, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is required and is missing"
        ));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be a string, not {}",
            type_name(raw)
        ));
    };
    if s.is_empty() {
        return Err(format!("`spawnModules[{index}]`.{key} is blank"));
    }
    Ok(s.to_string())
}

/// Optional nonempty using the supplied domain data.
pub(super) fn optional_nonempty(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be a string, not {}",
            type_name(raw)
        ));
    };
    if s.is_empty() {
        return Err(format!("`spawnModules[{index}]`.{key} is blank"));
    }
    Ok(Some(s.to_string()))
}

/// Optional finite using the supplied domain data.
pub(super) fn optional_finite(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<f64>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(n) = raw.as_f64() else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be a number, not {}",
            type_name(raw)
        ));
    };
    if !n.is_finite() {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is {n} — must be a finite number"
        ));
    }
    Ok(Some(n))
}

/// Required count using the supplied domain data.
pub(super) fn required_count(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<i64, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is required and is missing"
        ));
    };
    parse_count(raw, index, key)
}

/// Optional count using the supplied domain data.
pub(super) fn optional_count(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<i64>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    parse_count(raw, index, key).map(Some)
}

/// Parse count using the supplied domain data.
pub(super) fn parse_count(raw: &Value, index: usize, key: &str) -> Result<i64, String> {
    let Some(n) = as_i64(raw) else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be an integer, not {}",
            type_name(raw)
        ));
    };
    if n <= 0 || n > MAX_ALIVE {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is {n} — must be in 1..={MAX_ALIVE}"
        ));
    }
    Ok(n)
}

/// Optional positive seconds using the supplied domain data.
pub(super) fn optional_positive_seconds(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<f64>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(n) = raw.as_f64() else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be a number, not {}",
            type_name(raw)
        ));
    };
    if !n.is_finite() || n <= 0.0 {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is {n} — intervalSeconds must be above zero"
        ));
    }
    Ok(Some(n))
}

/// As i64 using the supplied domain data.
pub(super) fn as_i64(raw: &Value) -> Option<i64> {
    if let Some(n) = raw.as_i64() {
        return Some(n);
    }
    if let Some(n) = raw.as_u64() {
        return i64::try_from(n).ok();
    }
    let n = raw.as_f64()?;
    if n.is_finite() && n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        return Some(n as i64);
    }
    None
}

/// Quote using the supplied domain data.
pub(super) fn quote(s: &str) -> String {
    format!("{s:?}")
}

/// Type name using the supplied domain data.
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
