//! T-936.5 — the authored `audio` block: positional emitters and music cues.
//!
//! ══ Why this module exists ══════════════════════════════════════════════════════════════════
//! Missions carry no authored sound. There is no `audio` vocabulary in mission.schema.json, no
//! editor panel, and no script under `Scripts/Game/TBD`. This block is
//! `{emitters: [{id, x, z, y?, sound, radiusM, loop, triggerId?}], musicCues: [{id, event, track}]}`.
//! The editor writes it into `meta.environment.audio`; [`crate::mission::extensions::AUTHORED_BLOCKS`]
//! copies it onto the compiled payload root; `TBD_AudioEmitter.c` plays it. A mission that authors
//! no audio still compiles to today's bytes — the carrier emits nothing when the key is absent.
//!
//! ══ Radius ══════════════════════════════════════════════════════════════════════════════════
//! `radiusM` must be strictly greater than zero. A zero (or negative) radius is a silent source,
//! which is indistinguishable from "the emitter was never authored" at the listener. [`radius_above_zero`]
//! is the perturbation target: widening it from `>` to `>=` turns
//! [`tests::radius_zero_is_refused`] red.
//!
//! ══ Events ══════════════════════════════════════════════════════════════════════════════════
//! Cue `event` is the closed set [`MUSIC_EVENTS`]. Anything else is refused here so the mod never
//! receives a string it cannot dispatch.

use serde_json::{Map, Value};

/// Cue events `$defs/musicCue.event` declares.
pub const MUSIC_EVENTS: &[&str] = &[
    "mission_start",
    "task_succeeded",
    "task_failed",
    "mission_end",
];

const AUDIO_KEYS: &[&str] = &["emitters", "musicCues"];
const EMITTER_KEYS: &[&str] = &["id", "x", "z", "y", "sound", "radiusM", "loop", "triggerId"];
const CUE_KEYS: &[&str] = &["id", "event", "track"];

/// One positional emitter.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredEmitter {
    pub id: String,
    pub x: f64,
    pub z: f64,
    pub y: Option<f64>,
    pub sound: String,
    pub radius_m: f64,
    pub loop_sound: bool,
    pub trigger_id: Option<String>,
}

/// One music cue.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredMusicCue {
    pub id: String,
    pub event: String,
    pub track: String,
}

/// The authored audio block. At least one emitter or one cue.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredAudio {
    pub emitters: Vec<AuthoredEmitter>,
    pub music_cues: Vec<AuthoredMusicCue>,
}

/// Strictly positive hearable radius. Zero is refused.
///
/// Widening `>` to `>=` is the T-936.5 perturbation: [`tests::radius_zero_is_refused`]
/// goes red, restore + `touch` goes green.
#[must_use]
pub fn radius_above_zero(n: f64) -> bool {
    n > 0.0
}

/// Parse an authored `audio` object.
///
/// # Errors
/// Wrong type, unknown keys, empty block, a row the schema would refuse, a duplicate id,
/// an unknown cue event, or `radiusM` that is not strictly positive.
pub fn parse(value: &Value) -> Result<AuthoredAudio, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`audio` must be an object, not {}",
            type_name(value)
        ));
    };

    for key in obj.keys() {
        if !AUDIO_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "`audio` carries {key:?}, which `$defs/audio` does not declare \
                 (additionalProperties is false)"
            ));
        }
    }

    let emitters = required_array(obj, "emitters")?;
    let cues = required_array(obj, "musicCues")?;
    if emitters.is_empty() && cues.is_empty() {
        return Err(
            "`audio` has no emitters and no musicCues — an empty block is omitted rather than authored"
                .into(),
        );
    }

    let mut out_emitters = Vec::with_capacity(emitters.len());
    let mut seen: Vec<String> = Vec::with_capacity(emitters.len() + cues.len());
    for (index, item) in emitters.iter().enumerate() {
        let row = parse_emitter(item, index)?;
        refuse_duplicate(&seen, &row.id, "emitters", index)?;
        seen.push(row.id.clone());
        out_emitters.push(row);
    }

    let mut out_cues = Vec::with_capacity(cues.len());
    for (index, item) in cues.iter().enumerate() {
        let row = parse_cue(item, index)?;
        refuse_duplicate(&seen, &row.id, "musicCues", index)?;
        seen.push(row.id.clone());
        out_cues.push(row);
    }

    Ok(AuthoredAudio {
        emitters: out_emitters,
        music_cues: out_cues,
    })
}

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's
/// validator.
///
/// # Errors
/// Every error [`parse`] returns.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

fn required_array<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a Vec<Value>, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!("`audio.{key}` is required and is missing"));
    };
    raw.as_array()
        .ok_or_else(|| format!("`audio.{key}` must be an array, not {}", type_name(raw)))
}

fn refuse_duplicate(seen: &[String], id: &str, bag: &str, index: usize) -> Result<(), String> {
    if seen.iter().any(|s| s == id) {
        return Err(format!(
            "`audio.{bag}[{index}]`.id is {} — each audio id must be unique",
            quote(id)
        ));
    }
    Ok(())
}

fn parse_emitter(value: &Value, index: usize) -> Result<AuthoredEmitter, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`audio.emitters[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };
    refuse_unknown(obj, EMITTER_KEYS, &format!("audio.emitters[{index}]"))?;

    let id = required_nonempty(obj, index, "emitters", "id")?;
    let x = required_finite(obj, index, "emitters", "x")?;
    let z = required_finite(obj, index, "emitters", "z")?;
    let y = optional_finite(obj, index, "emitters", "y")?;
    let sound = required_nonempty(obj, index, "emitters", "sound")?;
    let radius_m = required_finite(obj, index, "emitters", "radiusM")?;
    if !radius_above_zero(radius_m) {
        return Err(format!(
            "`audio.emitters[{index}]`.radiusM is {radius_m} — radiusM must be above zero"
        ));
    }
    let loop_sound = required_bool(obj, index, "loop")?;
    let trigger_id = optional_nonempty(obj, index, "emitters", "triggerId")?;

    Ok(AuthoredEmitter {
        id,
        x,
        z,
        y,
        sound,
        radius_m,
        loop_sound,
        trigger_id,
    })
}

fn parse_cue(value: &Value, index: usize) -> Result<AuthoredMusicCue, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`audio.musicCues[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };
    refuse_unknown(obj, CUE_KEYS, &format!("audio.musicCues[{index}]"))?;

    let id = required_nonempty(obj, index, "musicCues", "id")?;
    let event = required_nonempty(obj, index, "musicCues", "event")?;
    if !MUSIC_EVENTS.contains(&event.as_str()) {
        return Err(format!(
            "`audio.musicCues[{index}]`.event is {} — must be one of {}",
            quote(&event),
            MUSIC_EVENTS.join(", ")
        ));
    }
    let track = required_nonempty(obj, index, "musicCues", "track")?;
    Ok(AuthoredMusicCue { id, event, track })
}

fn refuse_unknown(obj: &Map<String, Value>, allowed: &[&str], path: &str) -> Result<(), String> {
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

fn required_nonempty(
    obj: &Map<String, Value>,
    index: usize,
    bag: &str,
    key: &str,
) -> Result<String, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} is required and is missing"
        ));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} must be a string, not {}",
            type_name(raw)
        ));
    };
    if s.is_empty() {
        return Err(format!("`audio.{bag}[{index}]`.{key} is blank"));
    }
    Ok(s.to_string())
}

fn optional_nonempty(
    obj: &Map<String, Value>,
    index: usize,
    bag: &str,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} must be a string, not {}",
            type_name(raw)
        ));
    };
    if s.is_empty() {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} is blank — omit the key rather than store empty"
        ));
    }
    Ok(Some(s.to_string()))
}

fn required_bool(obj: &Map<String, Value>, index: usize, key: &str) -> Result<bool, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`audio.emitters[{index}]`.{key} is required and is missing"
        ));
    };
    raw.as_bool().ok_or_else(|| {
        format!(
            "`audio.emitters[{index}]`.{key} must be a boolean, not {}",
            type_name(raw)
        )
    })
}

fn required_finite(
    obj: &Map<String, Value>,
    index: usize,
    bag: &str,
    key: &str,
) -> Result<f64, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} is required and is missing"
        ));
    };
    let Some(n) = raw.as_f64() else {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} must be a number, not {}",
            type_name(raw)
        ));
    };
    if !n.is_finite() {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} is {n} — must be finite"
        ));
    }
    Ok(n)
}

fn optional_finite(
    obj: &Map<String, Value>,
    index: usize,
    bag: &str,
    key: &str,
) -> Result<Option<f64>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(n) = raw.as_f64() else {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} must be a number, not {}",
            type_name(raw)
        ));
    };
    if !n.is_finite() {
        return Err(format!(
            "`audio.{bag}[{index}]`.{key} is {n} — must be finite"
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

    fn two_and_one() -> Value {
        json!({
            "emitters": [
                {
                    "id": "ae-gen",
                    "x": 100.0,
                    "z": 200.0,
                    "sound": "SOUND_HINT",
                    "radiusM": 25.0,
                    "loop": true
                },
                {
                    "id": "ae-shot",
                    "x": 300.0,
                    "z": 400.0,
                    "y": 12.5,
                    "sound": "SOUND_FEVER",
                    "radiusM": 8.0,
                    "loop": false,
                    "triggerId": "tr-door"
                }
            ],
            "musicCues": [
                {
                    "id": "mc-start",
                    "event": "mission_start",
                    "track": "SOUND_HINT"
                }
            ]
        })
    }

    fn compile_env_with_audio(block: &Value) -> Value {
        compile_payload(
            &json!({
                "meta": {
                    "terrain": "everon",
                    "environment": { "weather": "clear", "audio": block }
                }
            })
            .to_string(),
            "{}",
            false,
        )
    }

    #[test]
    fn a_two_emitter_one_cue_block_parses() {
        let got = parse(&two_and_one()).expect("parses");
        assert_eq!(got.emitters.len(), 2);
        assert_eq!(got.emitters[0].id, "ae-gen");
        assert_eq!(got.emitters[0].radius_m, 25.0);
        assert!(got.emitters[0].loop_sound);
        assert!(got.emitters[0].y.is_none());
        assert_eq!(got.emitters[1].y, Some(12.5));
        assert_eq!(got.emitters[1].trigger_id.as_deref(), Some("tr-door"));
        assert_eq!(got.music_cues.len(), 1);
        assert_eq!(got.music_cues[0].event, "mission_start");
    }

    /// **The perturbation target.** Widening [`radius_above_zero`] from `>` to `>=`
    /// accepts radius 0, and this test goes red.
    #[test]
    fn radius_zero_is_refused() {
        let err = parse(&json!({
            "emitters": [{
                "id": "ae-zero",
                "x": 1.0,
                "z": 2.0,
                "sound": "SOUND_HINT",
                "radiusM": 0.0,
                "loop": false
            }],
            "musicCues": []
        }))
        .expect_err("radius 0 must be refused");
        assert!(err.contains("above zero"), "{err}");
        assert!(err.contains("0"), "{err}");
        assert!(
            !radius_above_zero(0.0),
            "the predicate itself must refuse radius 0"
        );
        assert!(radius_above_zero(0.1));
        validate(&json!({
            "emitters": [{
                "id": "ae-zero",
                "x": 1.0,
                "z": 2.0,
                "sound": "SOUND_HINT",
                "radiusM": 0.0,
                "loop": false
            }],
            "musicCues": []
        }))
        .expect_err("radius 0");
    }

    #[test]
    fn a_negative_radius_is_refused() {
        let err = parse(&json!({
            "emitters": [{
                "id": "ae-neg",
                "x": 1.0,
                "z": 2.0,
                "sound": "SOUND_HINT",
                "radiusM": -4.0,
                "loop": false
            }],
            "musicCues": []
        }))
        .expect_err("negative");
        assert!(err.contains("above zero"), "{err}");
    }

    #[test]
    fn an_unknown_event_is_refused() {
        let err = parse(&json!({
            "emitters": [],
            "musicCues": [{
                "id": "mc-bad",
                "event": "round_pause",
                "track": "SOUND_HINT"
            }]
        }))
        .expect_err("unknown event");
        assert!(err.contains("round_pause"), "{err}");
        assert!(err.contains("mission_start"), "{err}");
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let err = parse(&json!({
            "emitters": [{
                "id": "same",
                "x": 1.0,
                "z": 2.0,
                "sound": "SOUND_HINT",
                "radiusM": 5.0,
                "loop": false
            }],
            "musicCues": [{
                "id": "same",
                "event": "mission_end",
                "track": "SOUND_HINT"
            }]
        }))
        .expect_err("duplicate");
        assert!(err.contains("unique"), "{err}");
    }

    #[test]
    fn an_empty_block_is_refused() {
        let err = parse(&json!({"emitters": [], "musicCues": []})).expect_err("empty");
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let err = parse(&json!({
            "emitters": [{
                "id": "ae-1",
                "x": 1.0,
                "z": 2.0,
                "sound": "SOUND_HINT",
                "radiusM": 5.0,
                "loop": false,
                "volume": 2
            }],
            "musicCues": []
        }))
        .expect_err("unknown key");
        assert!(err.contains("volume"), "{err}");
    }

    #[test]
    fn audio_is_registered_on_the_carrier() {
        assert!(
            is_authored_block("audio"),
            "T-936.5's row must be in AUTHORED_BLOCKS or the carrier never emits it"
        );
        assert!(!is_authored_block("spawnModules"));
    }

    #[test]
    fn a_two_emitter_mission_copies_to_the_payload_root() {
        let block = two_and_one();
        let p = compile_env_with_audio(&block);
        assert_eq!(
            p["audio"], block,
            "AUTHORED_BLOCKS must promote audio out of the env bag: {p:#}"
        );
        assert_eq!(p["audio"]["emitters"].as_array().expect("array").len(), 2);
        assert_eq!(p["audio"]["musicCues"].as_array().expect("array").len(), 1);

        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert_eq!(carried.get("audio"), Some(&block));
    }

    #[test]
    fn an_unauthored_payload_still_omits_the_audio_key() {
        let p = compile_payload(
            &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}})
                .to_string(),
            "{}",
            false,
        );
        assert!(
            p.get("audio").is_none(),
            "parity: no audio authored ⇒ no audio key: {p:#}"
        );
        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(carried.get("audio").is_none());
    }

    #[test]
    fn copy_authored_blocks_carries_audio_and_leaves_weather() {
        let block = two_and_one();
        let env = json!({"weather": "clear", "audio": block});
        let mut dst = serde_json::Map::new();
        let copied = copy_authored_blocks(&env, &mut dst);
        assert!(copied.contains(&"audio"), "{copied:?}");
        assert_eq!(dst["audio"], block);
        assert!(
            !dst.contains_key("weather"),
            "the bag's own keys stay in the bag"
        );
    }
}
