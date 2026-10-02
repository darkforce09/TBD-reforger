//! Role: nets.
//! Position: `mission/extensions/radio` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Map, Value};

/// Canonical freq min mhz value.
pub const FREQ_MIN_MHZ: f64 = 30.0;

/// `$defs/net/freqMHz` `maximum: 512`.
pub const FREQ_MAX_MHZ: f64 = 512.0;

/// `$defs/radioPlan.nets` `maxItems` and `TBD_RadioPlan.MAX_NETS`.
pub const MAX_NETS: usize = 32;

/// `$defs/net.label` `maxLength` and `TBD_RadioPlan.MAX_LABEL_CHARS`.
pub const MAX_LABEL_CHARS: usize = 48;

/// `$defs/net.range` vocabulary.
pub const RANGES: &[&str] = &["short", "long"];

/// One authored net, parsed and checked.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredNet {
    /// Id.
    pub id: String,
    /// Label.
    pub label: String,
    /// Freq mhz.
    pub freq_mhz: f64,
    /// Faction.
    pub faction: Option<String>,
    /// Range.
    pub range: Option<String>,
}

/// One authored `radioPlan` object (`{ nets: [...] }`).
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredRadioPlan {
    /// Nets.
    pub nets: Vec<AuthoredNet>,
}

/// Compare two frequencies at 1 kHz so `30` and `30.0` collide and `30.0` vs `30.5` do not.
#[must_use]
pub fn freq_key(mhz: f64) -> i64 {
    (mhz * 1000.0).round() as i64
}

/// Refuse a net whose frequency is already used by an earlier net.
pub fn refuse_duplicate_frequency(
    nets: &[AuthoredNet],
    candidate: f64,
    index: usize,
) -> Result<(), String> {
    let key = freq_key(candidate);
    if let Some((other, n)) = nets
        .iter()
        .enumerate()
        .find(|(_, n)| freq_key(n.freq_mhz) == key)
    {
        return Err(format!(
            "`radioPlan.nets[{index}]`.freqMHz is {candidate} — the same frequency as \
             nets[{other}] ({}) so the two channels would hear each other",
            n.id
        ));
    }
    Ok(())
}

/// Parse and validate one authored `radioPlan` value.
pub fn parse(value: &Value) -> Result<AuthoredRadioPlan, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`radioPlan` must be an object, not {}",
            type_name(value)
        ));
    };

    for key in obj.keys() {
        if key != "nets" {
            return Err(format!(
                "`radioPlan` carries {key:?}, which `$defs/radioPlan` does not declare \
                 (additionalProperties is false)"
            ));
        }
    }

    let Some(arr) = obj.get("nets").and_then(Value::as_array) else {
        return Err("`radioPlan.nets` is required and must be an array of one or more nets".into());
    };
    if arr.is_empty() {
        return Err(
            "`radioPlan.nets` is empty — the schema requires at least one net, and an empty \
             plan is omitted rather than authored"
                .into(),
        );
    }
    if arr.len() > MAX_NETS {
        return Err(format!(
            "`radioPlan.nets` has {} entries — `TBD_RadioPlan.MAX_NETS` is {MAX_NETS} and the \
             mod would silently drop the rest",
            arr.len()
        ));
    }

    let mut out = Vec::with_capacity(arr.len());
    let mut seen_ids: Vec<String> = Vec::with_capacity(arr.len());

    for (index, item) in arr.iter().enumerate() {
        let net = parse_net(item, index)?;
        refuse_duplicate_frequency(&out, net.freq_mhz, index)?;
        if seen_ids.iter().any(|id| id == &net.id) {
            return Err(format!(
                "`radioPlan.nets[{index}]`.id is {} — each net id must be unique",
                quote(&net.id)
            ));
        }
        seen_ids.push(net.id.clone());
        out.push(net);
    }

    Ok(AuthoredRadioPlan { nets: out })
}

/// [`parse`] with the value discarded — the [`crate::data::scenario::extensions::AUTHORED_BLOCKS`] row's validator.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

/// Parse net using the supplied domain data.
pub(super) fn parse_net(value: &Value, index: usize) -> Result<AuthoredNet, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`radioPlan.nets[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };

    for key in obj.keys() {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "`radioPlan.nets[{index}]` carries {key:?}, which `$defs/net` does not declare \
                 (additionalProperties is false)"
            ));
        }
    }

    let id = required_string(obj, index, "id")?;
    if !is_net_id(&id) {
        return Err(format!(
            "`radioPlan.nets[{index}]`.id is {} — a net id is `net:` then lowercase letters, \
             digits and underscores",
            quote(&id)
        ));
    }

    let label = required_string(obj, index, "label")?;
    let label_chars = label.chars().count();
    if !(1..=MAX_LABEL_CHARS).contains(&label_chars) {
        return Err(format!(
            "`radioPlan.nets[{index}]`.label is {label_chars} characters — `$defs/net.label` \
             allows 1..={MAX_LABEL_CHARS} (the mod truncates past that without a word)"
        ));
    }

    let freq_mhz = required_freq(obj, index)?;
    let faction = optional_string(obj, index, "faction")?;
    if let Some(ref f) = faction
        && !is_faction_key(f)
    {
        return Err(format!(
            "`radioPlan.nets[{index}]`.faction is {} — a faction key starts with a letter and \
             is lowercase letters, digits and underscores",
            quote(f)
        ));
    }

    let range = optional_string(obj, index, "range")?;
    if let Some(ref r) = range
        && !RANGES.contains(&r.as_str())
    {
        return Err(format!(
            "`radioPlan.nets[{index}]`.range is {} — the editor authors one of {}",
            quote(r),
            RANGES.join(", ")
        ));
    }

    Ok(AuthoredNet {
        id,
        label,
        freq_mhz,
        faction,
        range,
    })
}

/// Canonical known keys value.
pub(super) const KNOWN_KEYS: &[&str] = &["id", "label", "freqMHz", "faction", "range"];

/// `^net:[a-z0-9_]+$`.
pub(super) fn is_net_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("net:") else {
        return false;
    };
    !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// `^[a-z][a-z0-9_]*$`.
pub(super) fn is_faction_key(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {
            chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        }
        _ => false,
    }
}

/// Required string using the supplied domain data.
pub(super) fn required_string(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<String, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` is required and is missing"
        ));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` must be a string, not {}",
            type_name(raw)
        ));
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` is blank — a net needs a {key} the radio UI can name"
        ));
    }
    Ok(trimmed.to_string())
}

/// Optional string using the supplied domain data.
pub(super) fn optional_string(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` must be a string, not {}",
            type_name(raw)
        ));
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` is blank — omit the key rather than authoring an \
             empty {key}"
        ));
    }
    Ok(Some(trimmed.to_string()))
}

/// Required freq using the supplied domain data.
pub(super) fn required_freq(obj: &Map<String, Value>, index: usize) -> Result<f64, String> {
    let Some(raw) = obj.get("freqMHz") else {
        return Err(format!(
            "`radioPlan.nets[{index}].freqMHz` is required and is missing"
        ));
    };
    let Some(freq) = raw.as_f64() else {
        return Err(format!(
            "`radioPlan.nets[{index}].freqMHz` must be a number, not {}",
            type_name(raw)
        ));
    };
    if !freq.is_finite() || !(FREQ_MIN_MHZ..=FREQ_MAX_MHZ).contains(&freq) {
        return Err(format!(
            "`radioPlan.nets[{index}].freqMHz` is {freq} — `$defs/net.freqMHz` allows \
             {FREQ_MIN_MHZ}..={FREQ_MAX_MHZ}"
        ));
    }
    Ok(freq)
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
