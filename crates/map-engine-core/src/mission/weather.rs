//! T-936.4 — the authored `weatherTimeline` block: keyframes, ordering, preset vocabulary.
//!
//! ══ Why this module exists ══════════════════════════════════════════════════════════════════
//! Weather is one static `environment.weatherPreset`. Nothing changes it over mission time; no
//! script under `Scripts/Game/TBD/Gamemode` drives the world's weather manager. This block is the
//! authored timeline: `{keyframes: [{atMinutes, weatherPreset, windDirDeg?, fog?}]}`. The editor
//! writes it into `meta.environment.weatherTimeline`; [`crate::mission::extensions::AUTHORED_BLOCKS`]
//! copies it onto the compiled payload root; `TBD_WeatherRuntime.c` applies each keyframe at its
//! offset. A mission that authors no timeline still compiles to today's bytes — the carrier emits
//! nothing when the key is absent.
//!
//! ══ Ordering ════════════════════════════════════════════════════════════════════════════════
//! `atMinutes` is strictly increasing. Equal offsets are refused — two keyframes at the same
//! minute would race, and the runtime applies in array order so a tie is an authored contradiction.
//! [`minutes_strictly_increase`] is the perturbation target: widening it from `>` to `>=` turns
//! [`tests::equal_at_minutes_are_refused`] red.
//!
//! ══ Preset vocabulary ═══════════════════════════════════════════════════════════════════════
//! Shared with `environment.weatherPreset` / `flatten.rs`'s `WEATHER_PRESETS` / the top-strip
//! weather `<select>`: [`WEATHER_PRESETS`]. A keyframe that names anything else is refused here
//! so the mod never receives a string `ForceWeatherTo` cannot resolve.

use serde_json::{Map, Value};

/// The four weather strings the editor already authors as `environment.weatherPreset`.
///
/// Order matches `flatten.rs` `WEATHER_PRESETS` and `top_strip.rs` `WEATHER_OPTIONS`.
pub const WEATHER_PRESETS: &[&str] = &["clear", "overcast", "heavy_rain", "dense_fog"];

/// Keys `$defs/weatherKeyframe` declares. Anything else is `additionalProperties: false`.
const KEYFRAME_KEYS: &[&str] = &["atMinutes", "weatherPreset", "windDirDeg", "fog"];

/// One authored keyframe. Optional wind/fog ride the same axes T-682 already applies statically.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredKeyframe {
    pub at_minutes: i64,
    pub weather_preset: String,
    pub wind_dir_deg: Option<f64>,
    pub fog: Option<f64>,
}

/// The authored timeline. `keyframes` is never empty — an empty timeline is omitted, not stored.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredWeatherTimeline {
    pub keyframes: Vec<AuthoredKeyframe>,
}

/// Strictly increasing minute offsets. Equal is refused.
///
/// Widening `>` to `>=` is the T-936.4 perturbation: [`tests::equal_at_minutes_are_refused`]
/// goes red, restore + `touch` goes green.
#[must_use]
pub fn minutes_strictly_increase(prev: i64, next: i64) -> bool {
    next > prev
}

/// Parse an authored `weatherTimeline` object.
///
/// # Errors
/// Wrong type, unknown keys, empty `keyframes`, a keyframe the schema would refuse, a preset
/// outside [`WEATHER_PRESETS`], or `atMinutes` that is not strictly increasing.
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

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's
/// validator.
///
/// # Errors
/// Every error [`parse`] returns.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

fn parse_keyframe(value: &Value, index: usize) -> Result<AuthoredKeyframe, String> {
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

fn required_minutes(obj: &Map<String, Value>, index: usize) -> Result<i64, String> {
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

fn as_i64_minutes(raw: &Value) -> Option<i64> {
    if let Some(n) = raw.as_i64() {
        return Some(n);
    }
    let f = raw.as_f64()?;
    if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        return Some(f as i64);
    }
    None
}

fn required_preset(obj: &Map<String, Value>, index: usize) -> Result<String, String> {
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

fn optional_unit_interval(
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

fn quote(s: &str) -> String {
    format!("{s:?}")
}

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
    use crate::mission::compile::compile_payload;
    use crate::mission::extensions::{ExtensionBlocks, copy_authored_blocks, is_authored_block};
    use serde_json::json;

    fn three_keyframes() -> Value {
        json!({
            "keyframes": [
                {"atMinutes": 0, "weatherPreset": "clear"},
                {"atMinutes": 15, "weatherPreset": "overcast", "windDirDeg": 90.0},
                {"atMinutes": 40, "weatherPreset": "heavy_rain", "fog": 0.4}
            ]
        })
    }

    fn compile_env_with_timeline(timeline: &Value) -> Value {
        compile_payload(
            &json!({
                "meta": {
                    "terrain": "everon",
                    "environment": { "weather": "clear", "weatherTimeline": timeline }
                }
            })
            .to_string(),
            "{}",
            false,
        )
    }

    #[test]
    fn a_three_keyframe_block_parses() {
        let got = parse(&three_keyframes()).expect("parses");
        assert_eq!(got.keyframes.len(), 3);
        assert_eq!(got.keyframes[0].at_minutes, 0);
        assert_eq!(got.keyframes[0].weather_preset, "clear");
        assert!(got.keyframes[0].wind_dir_deg.is_none());
        assert!(got.keyframes[0].fog.is_none());
        assert_eq!(got.keyframes[1].wind_dir_deg, Some(90.0));
        assert_eq!(got.keyframes[2].fog, Some(0.4));
        assert_eq!(got.keyframes[2].weather_preset, "heavy_rain");
    }

    #[test]
    fn the_preset_vocabulary_matches_environment() {
        assert_eq!(
            WEATHER_PRESETS,
            ["clear", "overcast", "heavy_rain", "dense_fog"]
        );
    }

    /// **The perturbation target.** Widening [`minutes_strictly_increase`] from `>` to `>=`
    /// accepts equal offsets, and this test goes red.
    #[test]
    fn equal_at_minutes_are_refused() {
        let err = parse(&json!({
            "keyframes": [
                {"atMinutes": 10, "weatherPreset": "clear"},
                {"atMinutes": 10, "weatherPreset": "overcast"}
            ]
        }))
        .expect_err("equal atMinutes must be refused");
        assert!(err.contains("strictly increasing"), "{err}");
        assert!(err.contains("10"), "{err}");
        assert!(
            !minutes_strictly_increase(10, 10),
            "the predicate itself must refuse equal offsets"
        );
        validate(&json!({
            "keyframes": [
                {"atMinutes": 10, "weatherPreset": "clear"},
                {"atMinutes": 10, "weatherPreset": "overcast"}
            ]
        }))
        .expect_err("equal atMinutes");
    }

    #[test]
    fn out_of_order_at_minutes_are_refused() {
        let err = parse(&json!({
            "keyframes": [
                {"atMinutes": 20, "weatherPreset": "clear"},
                {"atMinutes": 5, "weatherPreset": "overcast"}
            ]
        }))
        .expect_err("descending");
        assert!(err.contains("strictly increasing"), "{err}");
        assert!(minutes_strictly_increase(5, 20));
        assert!(!minutes_strictly_increase(20, 5));
    }

    #[test]
    fn a_negative_offset_is_refused() {
        let err = parse(&json!({
            "keyframes": [{"atMinutes": -1, "weatherPreset": "clear"}]
        }))
        .expect_err("negative");
        assert!(err.contains("atMinutes"), "{err}");
        assert!(err.contains("negative"), "{err}");
    }

    #[test]
    fn an_unknown_preset_is_refused() {
        let err = parse(&json!({
            "keyframes": [{"atMinutes": 0, "weatherPreset": "hailstorm"}]
        }))
        .expect_err("unknown preset");
        assert!(err.contains("hailstorm"), "{err}");
        assert!(err.contains("clear"), "{err}");
    }

    #[test]
    fn fog_outside_unit_interval_is_refused() {
        let err = parse(&json!({
            "keyframes": [{"atMinutes": 0, "weatherPreset": "clear", "fog": 1.5}]
        }))
        .expect_err("fog");
        assert!(err.contains("fog"), "{err}");
        assert!(err.contains("0..=1"), "{err}");
    }

    #[test]
    fn wind_dir_outside_circle_is_refused() {
        let err = parse(&json!({
            "keyframes": [{"atMinutes": 0, "weatherPreset": "clear", "windDirDeg": 361.0}]
        }))
        .expect_err("wind");
        assert!(err.contains("windDirDeg"), "{err}");
        assert!(err.contains("0..=360"), "{err}");
    }

    #[test]
    fn an_empty_keyframes_array_is_refused() {
        let err = parse(&json!({"keyframes": []})).expect_err("empty");
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let err = parse(&json!({
            "keyframes": [{"atMinutes": 0, "weatherPreset": "clear", "thunder": true}]
        }))
        .expect_err("unknown key");
        assert!(err.contains("thunder"), "{err}");
    }

    #[test]
    fn weather_timeline_is_registered_on_the_carrier() {
        assert!(
            is_authored_block("weatherTimeline"),
            "T-936.4's row must be in AUTHORED_BLOCKS or the carrier never emits it"
        );
        assert!(is_authored_block("spawnModules"));
    }

    #[test]
    fn a_three_keyframe_mission_copies_to_the_payload_root() {
        let timeline = three_keyframes();
        let p = compile_env_with_timeline(&timeline);
        assert_eq!(
            p["weatherTimeline"], timeline,
            "AUTHORED_BLOCKS must promote weatherTimeline out of the env bag: {p:#}"
        );
        assert_eq!(
            p["weatherTimeline"]["keyframes"]
                .as_array()
                .expect("array")
                .len(),
            3
        );

        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert_eq!(carried.get("weatherTimeline"), Some(&timeline));
    }

    #[test]
    fn an_unauthored_payload_still_omits_the_weather_timeline_key() {
        let p = compile_payload(
            &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}})
                .to_string(),
            "{}",
            false,
        );
        assert!(
            p.get("weatherTimeline").is_none(),
            "parity: no weatherTimeline authored ⇒ no weatherTimeline key: {p:#}"
        );
        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(carried.get("weatherTimeline").is_none());
    }

    #[test]
    fn copy_authored_blocks_carries_weather_timeline_and_leaves_weather() {
        let timeline = three_keyframes();
        let env = json!({"weather": "clear", "weatherTimeline": timeline});
        let mut dst = serde_json::Map::new();
        let copied = copy_authored_blocks(&env, &mut dst);
        assert_eq!(copied, ["weatherTimeline"]);
        assert_eq!(dst["weatherTimeline"], timeline);
        assert!(
            !dst.contains_key("weather"),
            "the bag's own keys stay in the bag"
        );
    }
}
