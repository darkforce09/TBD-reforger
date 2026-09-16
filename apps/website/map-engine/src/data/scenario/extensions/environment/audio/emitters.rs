//! Role: emitters.
//! Position: `mission/extensions/environment/audio` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Map, Value};

/// Cue events `$defs/musicCue.event` declares.
pub const MUSIC_EVENTS: &[&str] = &[
    "mission_start",
    "task_succeeded",
    "task_failed",
    "mission_end",
];

/// Canonical audio keys value.
pub(super) const AUDIO_KEYS: &[&str] = &["emitters", "musicCues"];

/// Canonical emitter keys value.
pub(super) const EMITTER_KEYS: &[&str] =
    &["id", "x", "z", "y", "sound", "radiusM", "loop", "triggerId"];

/// Canonical cue keys value.
pub(super) const CUE_KEYS: &[&str] = &["id", "event", "track"];

/// One positional emitter.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredEmitter {
    /// Id.
    pub id: String,
    /// X.
    pub x: f64,
    /// Z.
    pub z: f64,
    /// Y.
    pub y: Option<f64>,
    /// Sound.
    pub sound: String,
    /// Radius m.
    pub radius_m: f64,
    /// Loop sound.
    pub loop_sound: bool,
    /// Trigger id.
    pub trigger_id: Option<String>,
}

/// One music cue.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredMusicCue {
    /// Id.
    pub id: String,
    /// Event.
    pub event: String,
    /// Track.
    pub track: String,
}

/// The authored audio block. At least one emitter or one cue.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredAudio {
    /// Emitters.
    pub emitters: Vec<AuthoredEmitter>,
    /// Music cues.
    pub music_cues: Vec<AuthoredMusicCue>,
}

/// Strictly positive hearable radius. Zero is refused.
#[must_use]
pub fn radius_above_zero(n: f64) -> bool {
    n > 0.0
}

/// Parse an authored `audio` object.
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

/// [`parse`] with the value discarded — the [`crate::data::scenario::extensions::AUTHORED_BLOCKS`] row's validator.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

/// Required array using the supplied domain data.
pub(super) fn required_array<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Vec<Value>, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!("`audio.{key}` is required and is missing"));
    };
    raw.as_array()
        .ok_or_else(|| format!("`audio.{key}` must be an array, not {}", type_name(raw)))
}

/// Refuse duplicate using the supplied domain data.
pub(super) fn refuse_duplicate(
    seen: &[String],
    id: &str,
    bag: &str,
    index: usize,
) -> Result<(), String> {
    if seen.iter().any(|s| s == id) {
        return Err(format!(
            "`audio.{bag}[{index}]`.id is {} — each audio id must be unique",
            quote(id)
        ));
    }
    Ok(())
}

/// Parse emitter using the supplied domain data.
pub(super) fn parse_emitter(value: &Value, index: usize) -> Result<AuthoredEmitter, String> {
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

/// Parse cue using the supplied domain data.
pub(super) fn parse_cue(value: &Value, index: usize) -> Result<AuthoredMusicCue, String> {
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

/// Optional nonempty using the supplied domain data.
pub(super) fn optional_nonempty(
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

/// Required bool using the supplied domain data.
pub(super) fn required_bool(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<bool, String> {
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

/// Required finite using the supplied domain data.
pub(super) fn required_finite(
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

/// Optional finite using the supplied domain data.
pub(super) fn optional_finite(
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
