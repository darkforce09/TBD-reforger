//! the **Audio** panel: positional emitters and music cues, every write one undo step.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.audio` through [`operations::update_environment`],
//! the same one-patch-one-undo-step path the .4 cards use.
//! `map_engine_core::mission::extensions` copies the key onto the compiled payload root.
//!
//! ══ Place on map ════════════════════════════════════════════════════════════════════════════
//! Placement reuses the existing marker gesture: [`arm_place_on_map`] calls
//! `begin_place_marker` (no new code in gestures.rs). After the canvas click lands a marker,
//! [`xz_from_last_marker`] copies that point onto the emitter. Numeric x/z still work without it.
//!
//! ══ Where this renders ══════════════════════════════════════════════════════════════════════
//! [`audio_emitters_panel`] belongs in Mission Settings
//! (`settings_modal.rs`'s `{audio_emitters_panel(ctrl)}`). This slice does not own
//! `settings_modal.rs`; .4 waited for the wave bookkeeping commit to land the one-line
//! mount. A panel that is built, registered and tested but never mounted is a mechanism that
//! cannot fire — the mount is on the human checklist, not silently dropped.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::{Map, Value};

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::armed_placement;
use website_map_engine::data::scenario::audio::validate;
use website_map_engine::data::scenario::audio::MUSIC_EVENTS;

/// The reader chain for `meta.environment.audio`, end to end.
pub const AUDIO_READERS: &[(&str, &str)] = &[
    (
        "compile",
        "map_engine_core::mission::compile::compile_payload → the saved payload's top-level \
         `audio` (via mission::extensions::copy_authored_blocks)",
    ),
    (
        "flatten",
        "map_engine_core::mission::flatten::EditorPayload.authored_blocks_root → \
         ExtensionBlocks::from_payload → the compiled document's `audio` block \
         (read through its named EditorPayload field; without that field the artifact document drops the key)",
    ),
    (
        "mod",
        "TBD_AudioEmitter (server arms emitters and fires cues; each client spawns one sound \
         source per emitter and honours radiusM / loop)",
    ),
    (
        "editor",
        "ui/inspector/audio_emitters.rs — this panel, via operations::update_environment",
    ),
];

/// Marker icon `begin_place_marker` arms for the place-on-map gesture.
pub const PLACE_MARKER_ICON: &str = "point_of_interest";

const EMITTER_KEYS: &[&str] = &["id", "x", "z", "y", "sound", "radiusM", "loop", "triggerId"];
const CUE_KEYS: &[&str] = &["id", "event", "track"];

/// Human labels for [`MUSIC_EVENTS`].
#[must_use]
pub fn event_label(event: &str) -> &'static str {
    match event {
        "mission_start" => "Mission start",
        "task_succeeded" => "Task succeeded",
        "task_failed" => "Task failed",
        "mission_end" => "Mission end",
        _ => "Unknown event",
    }
}

/// The first `prefix-N` identifier, counting from 1, that no row in `existing` already claims.
///
/// Callers pool the emitter and cue rows together before calling, so the two lists never mint
/// the same id and an operator reading a refusal can tell which row it names.
#[must_use]
pub fn next_id(prefix: &str, existing: &[Value]) -> String {
    let taken: Vec<String> = existing
        .iter()
        .filter_map(|r| r.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();
    let mut n: u32 = 1;
    loop {
        let id = format!("{prefix}-{n}");
        if !taken.iter().any(|t| t == &id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// A fresh emitter row: a free id, the world origin, a placeholder sound, a 25 m radius and
/// looping on.
///
/// `cue_ids` joins `existing` only to widen the pool the id is minted against.
#[must_use]
pub fn default_emitter(existing: &[Value], cue_ids: &[Value]) -> Value {
    let mut pool = existing.to_vec();
    pool.extend_from_slice(cue_ids);
    serde_json::json!({
        "id": next_id("ae", &pool),
        "x": 0.0,
        "z": 0.0,
        "sound": "SOUND_HINT",
        "radiusM": 25.0,
        "loop": true
    })
}

/// A fresh music cue row: a free id, the mission-start event and a placeholder track.
///
/// `emitter_ids` joins `existing` only to widen the pool the id is minted against.
#[must_use]
pub fn default_cue(existing: &[Value], emitter_ids: &[Value]) -> Value {
    let mut pool = existing.to_vec();
    pool.extend_from_slice(emitter_ids);
    serde_json::json!({
        "id": next_id("mc", &pool),
        "event": "mission_start",
        "track": "SOUND_HINT"
    })
}

/// The emitter rows inside an authored audio block, or an empty list when the block is absent or
/// holds no emitter array.
#[must_use]
pub fn emitters_from_block(block: Option<&Value>) -> Vec<Value> {
    array_from(block, "emitters")
}

/// The music cue rows inside an authored audio block, or an empty list when the block is absent
/// or holds no cue array.
#[must_use]
pub fn cues_from_block(block: Option<&Value>) -> Vec<Value> {
    array_from(block, "musicCues")
}

fn array_from(block: Option<&Value>, key: &str) -> Vec<Value> {
    block
        .and_then(Value::as_object)
        .and_then(|o| o.get(key))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Wrap arrays into the authored object, or `None` when both lists are empty (omit, don't store).
#[must_use]
pub fn block_from_parts(emitters: &[Value], cues: &[Value]) -> Option<Value> {
    if emitters.is_empty() && cues.is_empty() {
        None
    } else {
        Some(serde_json::json!({ "emitters": emitters, "musicCues": cues }))
    }
}

/// `emitters` with one default emitter appended, or the schema's refusal clause when the
/// resulting block would not validate.
///
/// Validation runs over the whole block rather than the new row alone, so a duplicate id or a
/// cross-row rule refuses the addition before it can become an undo step.
#[must_use]
pub fn add_emitter(emitters: &[Value], cues: &[Value]) -> Result<Vec<Value>, String> {
    let mut next = emitters.to_vec();
    next.push(default_emitter(emitters, cues));
    refuse_block(&next, cues)?;
    Ok(next)
}

/// `cues` with one default music cue appended, or the schema's refusal clause when the resulting
/// block would not validate.
#[must_use]
pub fn add_cue(cues: &[Value], emitters: &[Value]) -> Result<Vec<Value>, String> {
    let mut next = cues.to_vec();
    next.push(default_cue(cues, emitters));
    refuse_block(emitters, &next)?;
    Ok(next)
}

/// `rows` without the row at `index`, order preserved. An `index` past the end removes nothing.
///
/// Deletion is never refused: a block the operator has emptied out is always a legal block.
#[must_use]
pub fn remove_at(rows: &[Value], index: usize) -> Vec<Value> {
    rows.iter()
        .enumerate()
        .filter_map(|(i, r)| if i == index { None } else { Some(r.clone()) })
        .collect()
}

/// Copy a marker's world point onto emitter `index`.
pub fn apply_marker_xz(
    emitters: &[Value],
    cues: &[Value],
    index: usize,
    x: f64,
    z: f64,
) -> Result<Vec<Value>, String> {
    if !x.is_finite() || !z.is_finite() {
        return Err("marker position must be finite".into());
    }
    let mut next = with_emitter_number(emitters, cues, index, "x", x)?;
    next = with_emitter_number(&next, cues, index, "z", z)?;
    Ok(next)
}

/// Last marker's (x, z), if any — the point the place-on-map gesture just dropped.
#[must_use]
pub fn xz_from_last_marker(markers: &[(f64, f64)]) -> Option<(f64, f64)> {
    markers.last().copied()
}

/// `emitters` with one authored field of the row at `index` set from the operator's text, or a
/// refusal clause explaining why the edit was not taken.
///
/// Every field arrives as a string because every editor of it is a text input or a toggle, and
/// the key decides how that string is read: identifier and sound are required strings, the
/// trigger reference is an optional one, the position and radius are required numbers, the
/// vertical offset is an optional one, and the loop flag takes only the two boolean spellings.
/// An unknown key, a vanished row, an unreadable value and a block that fails the schema all
/// refuse, and a refusal leaves `emitters` untouched — the caller commits the returned list or
/// shows the clause, never both.
pub fn with_emitter_field(
    emitters: &[Value],
    cues: &[Value],
    index: usize,
    key: &str,
    value: &str,
) -> Result<Vec<Value>, String> {
    if !EMITTER_KEYS.contains(&key) {
        return Err(format!("{key:?} is not an authored emitter field"));
    }
    let Some(row) = emitters.get(index).and_then(Value::as_object) else {
        return Err("that emitter is no longer in the list".into());
    };
    let mut obj = row.clone();
    let trimmed = value.trim();
    match key {
        "id" | "sound" => set_required_string(&mut obj, key, trimmed)?,
        "triggerId" => set_optional_string(&mut obj, key, trimmed)?,
        "x" | "z" | "radiusM" => set_required_number(&mut obj, key, trimmed)?,
        "y" => set_optional_number(&mut obj, key, trimmed)?,
        "loop" => {
            let on = match trimmed {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => return Err("loop must be true or false".into()),
            };
            obj.insert(key.to_string(), Value::Bool(on));
        }
        _ => unreachable!("EMITTER_KEYS"),
    }
    let mut next = emitters.to_vec();
    next[index] = Value::Object(obj);
    refuse_block(&next, cues)?;
    Ok(next)
}

/// `emitters` with the loop flag of the row at `index` set to `on`.
///
/// A named wrapper over the boolean spelling [`with_emitter_field`] expects, so the checkbox's
/// call site never has to know it.
pub fn with_emitter_loop(
    emitters: &[Value],
    cues: &[Value],
    index: usize,
    on: bool,
) -> Result<Vec<Value>, String> {
    with_emitter_field(
        emitters,
        cues,
        index,
        "loop",
        if on { "true" } else { "false" },
    )
}

fn with_emitter_number(
    emitters: &[Value],
    cues: &[Value],
    index: usize,
    key: &str,
    n: f64,
) -> Result<Vec<Value>, String> {
    with_emitter_field(emitters, cues, index, key, &format!("{n}"))
}

/// `cues` with one authored field of the row at `index` set from the operator's text, or a
/// refusal clause explaining why the edit was not taken.
///
/// The cue's identifier and track are required strings and its event must be one of the known
/// music events. Like [`with_emitter_field`], an unknown key, a vanished row, an unreadable value
/// or a block that fails the schema refuses and leaves `cues` untouched.
pub fn with_cue_field(
    cues: &[Value],
    emitters: &[Value],
    index: usize,
    key: &str,
    value: &str,
) -> Result<Vec<Value>, String> {
    if !CUE_KEYS.contains(&key) {
        return Err(format!("{key:?} is not an authored cue field"));
    }
    let Some(row) = cues.get(index).and_then(Value::as_object) else {
        return Err("that cue is no longer in the list".into());
    };
    let mut obj = row.clone();
    let trimmed = value.trim();
    match key {
        "id" | "track" => set_required_string(&mut obj, key, trimmed)?,
        "event" => {
            if !MUSIC_EVENTS.contains(&trimmed) {
                return Err(format!(
                    "event {trimmed:?} is not one of {}",
                    MUSIC_EVENTS.join(", ")
                ));
            }
            obj.insert(key.to_string(), Value::String(trimmed.to_string()));
        }
        _ => unreachable!("CUE_KEYS"),
    }
    let mut next = cues.to_vec();
    next[index] = Value::Object(obj);
    refuse_block(emitters, &next)?;
    Ok(next)
}

fn set_required_string(
    obj: &mut Map<String, Value>,
    key: &str,
    trimmed: &str,
) -> Result<(), String> {
    if trimmed.is_empty() {
        return Err(format!("{key} cannot be blank"));
    }
    obj.insert(key.to_string(), Value::String(trimmed.to_string()));
    Ok(())
}

fn set_optional_string(
    obj: &mut Map<String, Value>,
    key: &str,
    trimmed: &str,
) -> Result<(), String> {
    if trimmed.is_empty() {
        obj.remove(key);
        return Ok(());
    }
    obj.insert(key.to_string(), Value::String(trimmed.to_string()));
    Ok(())
}

fn set_required_number(
    obj: &mut Map<String, Value>,
    key: &str,
    trimmed: &str,
) -> Result<(), String> {
    let n: f64 = trimmed
        .parse()
        .map_err(|_| format!("{key} must be a number"))?;
    if !n.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    obj.insert(key.to_string(), json_f64(n));
    Ok(())
}

fn set_optional_number(
    obj: &mut Map<String, Value>,
    key: &str,
    trimmed: &str,
) -> Result<(), String> {
    if trimmed.is_empty() {
        obj.remove(key);
        return Ok(());
    }
    set_required_number(obj, key, trimmed)
}

fn json_f64(n: f64) -> Value {
    Value::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()))
}

fn json_num(v: Option<&Value>) -> String {
    v.and_then(Value::as_f64)
        .map(|n| format!("{n}"))
        .unwrap_or_default()
}

fn refuse_block(emitters: &[Value], cues: &[Value]) -> Result<(), String> {
    match block_from_parts(emitters, cues) {
        None => Ok(()),
        Some(block) => validate(&block),
    }
}

/// The environment patch that stores `block`, as the JSON text the update path takes.
///
/// `None` patches the key to null rather than omitting it, because omitting a key leaves whatever
/// was stored before in place — clearing the panel has to actually clear the document.
#[must_use]
pub fn env_patch(block: Option<&Value>) -> String {
    match block {
        None => serde_json::json!({ "audio": null }).to_string(),
        Some(b) => serde_json::json!({ "audio": b }).to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_block() -> Option<Value> {
    crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value("audio")
        .filter(|v| v.is_object())
}

#[cfg(target_arch = "wasm32")]
fn commit(block: Option<&Value>) {
    if let Some(b) = block {
        if let Err(clause) = validate(b) {
            leptos::logging::warn!("audio is not yet complete: {clause}");
        }
    }
    crate::v2::apps::editor::bridge::host_state::editor_context::update_environment(env_patch(
        block,
    ));
}

/// Renders and wires the audio emitter inspector controls.
mod view;
#[cfg(target_arch = "wasm32")]
pub use view::arm_place_on_map;
pub use view::audio_emitters_panel;

#[cfg(test)]
#[path = "tests/audio_emitters/emitter_authoring.rs"]
mod tests;
