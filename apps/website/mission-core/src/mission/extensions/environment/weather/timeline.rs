//! Role: timeline.
//! Position: `mission/extensions/environment/weather` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Map, Value};

/// The four weather strings the editor already authors as `environment.weatherPreset`.
pub const WEATHER_PRESETS: &[&str] = &["clear", "overcast", "heavy_rain", "dense_fog"];

/// Keys `$defs/weatherKeyframe` declares. Anything else is `additionalProperties: false`.
pub(super) const KEYFRAME_KEYS: &[&str] = &["atMinutes", "weatherPreset", "windDirDeg", "fog"];

/// Domain representation of authored keyframe.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredKeyframe {
    /// At minutes.
    pub at_minutes: i64,
    /// Weather preset.
    pub weather_preset: String,
    /// Wind dir deg.
    pub wind_dir_deg: Option<f64>,
    /// Fog.
    pub fog: Option<f64>,
}

/// The authored timeline. `keyframes` is never empty — an empty timeline is omitted, not stored.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredWeatherTimeline {
    /// Keyframes.
    pub keyframes: Vec<AuthoredKeyframe>,
}

/// Strictly increasing minute offsets. Equal is refused.
#[must_use]
pub fn minutes_strictly_increase(prev: i64, next: i64) -> bool {
    next > prev
}

/// Parse an authored `weatherTimeline` object.
pub fn parse(value: &Value) -> Result<AuthoredWeatherTimeline, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`weatherTimeline` must be an object, not {}",
            type_name(value)
        ));
    };

    for key in obj.keys() {
        if key != "keyframes" {
            return Err(format!(
                "`weatherTimeline` carries {key:?}, which `$defs/weatherTimeline` does not declare \
                 (additionalProperties is false)"
            ));
        }
    }

    let Some(arr) = obj.get("keyframes").and_then(Value::as_array) else {
        return Err(
            "`weatherTimeline.keyframes` is required and must be an array of one or more keyframes"
                .into(),
        );
    };
    if arr.is_empty() {
        return Err(
            "`weatherTimeline.keyframes` is empty — an empty timeline is omitted rather than authored"
                .into(),
        );
    }

    let mut out: Vec<AuthoredKeyframe> = Vec::with_capacity(arr.len());
    for (index, item) in arr.iter().enumerate() {
        let kf = parse_keyframe(item, index)?;
        if let Some(prev) = out.last()
            && !minutes_strictly_increase(prev.at_minutes, kf.at_minutes)
        {
            return Err(format!(
                "`weatherTimeline.keyframes[{index}]`.atMinutes is {} — atMinutes must be \
                 strictly increasing (previous was {})",
                kf.at_minutes, prev.at_minutes
            ));
        }
        out.push(kf);
    }

    Ok(AuthoredWeatherTimeline { keyframes: out })
}

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's validator.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

/// Parse keyframe using the supplied domain data.
pub(super) fn parse_keyframe(value: &Value, index: usize) -> Result<AuthoredKeyframe, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };

    for key in obj.keys() {
        if !KEYFRAME_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "`weatherTimeline.keyframes[{index}]` carries {key:?}, which \
                 `$defs/weatherKeyframe` does not declare (additionalProperties is false)"
            ));
        }
    }

    let at_minutes = required_minutes(obj, index)?;
    let weather_preset = required_preset(obj, index)?;
    let wind_dir_deg =
        optional_unit_interval(obj, index, "windDirDeg", 0.0, 360.0, "outside 0..=360")?;
    let fog = optional_unit_interval(obj, index, "fog", 0.0, 1.0, "outside 0..=1")?;

    Ok(AuthoredKeyframe {
        at_minutes,
        weather_preset,
        wind_dir_deg,
        fog,
    })
}

/// Required minutes using the supplied domain data.
pub(super) fn required_minutes(obj: &Map<String, Value>, index: usize) -> Result<i64, String> {
    let Some(raw) = obj.get("atMinutes") else {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}]`.atMinutes is required and is missing"
        ));
    };
    let Some(n) = as_i64_minutes(raw) else {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}]`.atMinutes must be an integer number of minutes, \
             not {}",
            type_name(raw)
        ));
    };
    if n < 0 {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}]`.atMinutes is {n} — offsets cannot be negative"
        ));
    }
    Ok(n)
}

/// As i64 minutes using the supplied domain data.
pub(super) fn as_i64_minutes(raw: &Value) -> Option<i64> {
    if let Some(n) = raw.as_i64() {
        return Some(n);
    }
    let f = raw.as_f64()?;
    if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        return Some(f as i64);
    }
    None
}

/// Required preset using the supplied domain data.
pub(super) fn required_preset(obj: &Map<String, Value>, index: usize) -> Result<String, String> {
    let Some(raw) = obj.get("weatherPreset") else {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}]`.weatherPreset is required and is missing"
        ));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}]`.weatherPreset must be a string, not {}",
            type_name(raw)
        ));
    };
    if !WEATHER_PRESETS.contains(&s) {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}]`.weatherPreset is {} — must be one of {}",
            quote(s),
            WEATHER_PRESETS.join(", ")
        ));
    }
    Ok(s.to_string())
}

/// Optional unit interval using the supplied domain data.
pub(super) fn optional_unit_interval(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
    min: f64,
    max: f64,
    range_clause: &str,
) -> Result<Option<f64>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(n) = raw.as_f64() else {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}].{key}` must be a number, not {}",
            type_name(raw)
        ));
    };
    if !(min..=max).contains(&n) {
        return Err(format!(
            "`weatherTimeline.keyframes[{index}].{key}` is {n} — {range_clause}"
        ));
    }
    Ok(Some(n))
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
