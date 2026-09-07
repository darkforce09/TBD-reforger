//! T-936.5 — the **Audio** panel: positional emitters and music cues, every write one undo step.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.audio` through [`operations::update_environment`],
//! the same one-patch-one-undo-step path the T-936.1–.4 cards use.
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
//! `settings_modal.rs`; T-936.1–.4 waited for the wave bookkeeping commit to land the one-line
//! mount. A panel that is built, registered and tested but never mounted is a mechanism that
//! cannot fire — the mount is on the human checklist, not silently dropped.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::{Map, Value};

use map_engine_core::mission::audio::{validate, MUSIC_EVENTS};

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
         (needs the named EditorPayload field T-291 owns; without it /compiled drops the key)",
    ),
    (
        "mod",
        "TBD_AudioEmitter (server arms emitters and fires cues; each client spawns one sound \
         source per emitter and honours radiusM / loop)",
    ),
    (
        "editor",
        "panels/audio_emitters.rs — this panel, via operations::update_environment",
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

#[must_use]
pub fn emitters_from_block(block: Option<&Value>) -> Vec<Value> {
    array_from(block, "emitters")
}

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

#[must_use]
pub fn add_emitter(emitters: &[Value], cues: &[Value]) -> Result<Vec<Value>, String> {
    let mut next = emitters.to_vec();
    next.push(default_emitter(emitters, cues));
    refuse_block(&next, cues)?;
    Ok(next)
}

#[must_use]
pub fn add_cue(cues: &[Value], emitters: &[Value]) -> Result<Vec<Value>, String> {
    let mut next = cues.to_vec();
    next.push(default_cue(cues, emitters));
    refuse_block(emitters, &next)?;
    Ok(next)
}

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

#[must_use]
pub fn env_patch(block: Option<&Value>) -> String {
    match block {
        None => serde_json::json!({ "audio": null }).to_string(),
        Some(b) => serde_json::json!({ "audio": b }).to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_block() -> Option<Value> {
    crate::editor::state::operations::read_env_value("audio").filter(|v| v.is_object())
}

#[cfg(target_arch = "wasm32")]
fn commit(block: Option<&Value>) {
    if let Some(b) = block {
        if let Err(clause) = validate(b) {
            leptos::logging::warn!("audio is not yet complete: {clause}");
        }
    }
    crate::editor::state::operations::update_environment(env_patch(block));
}

/// Arms the existing marker placement gesture. No new code in gestures.rs.
#[cfg(target_arch = "wasm32")]
pub fn arm_place_on_map() {
    crate::editor::state::operations::begin_place_marker(PLACE_MARKER_ICON.to_string());
}

#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn audio_emitters_panel(ctrl: &'static str) -> AnyView {
    let _ = ctrl;
    ().into_any()
}

#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn audio_emitters_panel(ctrl: &'static str) -> AnyView {
    let sect = "text-label-sm uppercase tracking-wider text-outline";
    let hint = "text-label-sm normal-case text-outline";
    let block = read_block();
    let emitters = emitters_from_block(block.as_ref());
    let cues = cues_from_block(block.as_ref());
    let refusal = RwSignal::new(String::new());

    let emitters_for_add = emitters.clone();
    let cues_for_add_e = cues.clone();
    let cues_for_add = cues.clone();
    let emitters_for_add_c = emitters.clone();

    let emitter_list = emitters
        .iter()
        .enumerate()
        .map(|(index, row)| {
            emitter_row(
                ctrl,
                sect,
                hint,
                index,
                row,
                emitters.clone(),
                cues.clone(),
                refusal,
            )
        })
        .collect::<Vec<_>>();

    let cue_list = cues
        .iter()
        .enumerate()
        .map(|(index, row)| {
            cue_row(
                ctrl,
                sect,
                hint,
                index,
                row,
                emitters.clone(),
                cues.clone(),
                refusal,
            )
        })
        .collect::<Vec<_>>();

    view! {
        <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
            <span class=sect>"Audio emitters"</span>
            <span class=hint>
                "Place on map reuses the marker gesture: arm, click the canvas, then Use last marker. \
                 radiusM must be above zero. Loop repeats inside the radius. Trigger id is optional."
            </span>
            {emitter_list}
            <button type="button" class=ctrl
                on:click=move |_| {
                    refusal.set(String::new());
                    match add_emitter(&emitters_for_add, &cues_for_add_e) {
                        Ok(next) => commit(block_from_parts(&next, &cues_for_add_e).as_ref()),
                        Err(err) => refusal.set(err),
                    }
                }
            >"Add emitter"</button>
            <span class=sect>"Music cues"</span>
            {cue_list}
            <button type="button" class=ctrl
                on:click=move |_| {
                    refusal.set(String::new());
                    match add_cue(&cues_for_add, &emitters_for_add_c) {
                        Ok(next) => commit(block_from_parts(&emitters_for_add_c, &next).as_ref()),
                        Err(err) => refusal.set(err),
                    }
                }
            >"Add cue"</button>
            <p class="text-label-sm text-error">{move || refusal.get()}</p>
        </div>
    }
    .into_any()
}

#[cfg(target_arch = "wasm32")]
fn emitter_row(
    ctrl: &'static str,
    sect: &'static str,
    hint: &'static str,
    index: usize,
    row: &Value,
    emitters: Vec<Value>,
    cues: Vec<Value>,
    refusal: RwSignal<String>,
) -> AnyView {
    let id = row
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let x = json_num(row.get("x"));
    let z = json_num(row.get("z"));
    let y = json_num(row.get("y"));
    let sound = row
        .get("sound")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let radius = json_num(row.get("radiusM"));
    let loop_on = row.get("loop").and_then(Value::as_bool).unwrap_or(false);
    let trigger = row
        .get("triggerId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let e_mark = emitters.clone();
    let c_mark = cues.clone();
    let e_del = emitters.clone();
    let c_del = cues.clone();
    let e_sound = emitters.clone();
    let c_sound = cues.clone();
    let e_loop = emitters.clone();
    let c_loop = cues.clone();
    let e_trig = emitters.clone();
    let c_trig = cues.clone();

    view! {
        <div class="flex flex-col gap-2 border border-outline-variant/30 p-2">
            <div class="flex items-center gap-2">
                <span class=hint>{format!("id {id}")}</span>
                <button type="button" class="text-label-sm"
                    on:click=move |_| {
                        refusal.set(String::new());
                        arm_place_on_map();
                    }
                >"Place on map"</button>
                <button type="button" class="text-label-sm"
                    on:click=move |_| {
                        refusal.set(String::new());
                        let markers: Vec<(f64, f64)> = crate::editor::state::operations::marker_rows()
                            .into_iter()
                            .map(|m| (m.x, m.z))
                            .collect();
                        match xz_from_last_marker(&markers) {
                            Some((mx, mz)) => match apply_marker_xz(&e_mark, &c_mark, index, mx, mz) {
                                Ok(next) => commit(block_from_parts(&next, &c_mark).as_ref()),
                                Err(err) => refusal.set(err),
                            },
                            None => refusal.set(
                                "place a marker on the map first (Place on map, then click the canvas)"
                                    .into(),
                            ),
                        }
                    }
                >"Use last marker"</button>
                <button type="button" class="text-label-sm"
                    on:click=move |_| {
                        refusal.set(String::new());
                        let next = remove_at(&e_del, index);
                        commit(block_from_parts(&next, &c_del).as_ref());
                    }
                >"Remove"</button>
            </div>
            <div class="flex flex-wrap items-end gap-2">
                {num_input(ctrl, sect, "X", x, emitters.clone(), cues.clone(), index, "x", refusal)}
                {num_input(ctrl, sect, "Z", z, emitters.clone(), cues.clone(), index, "z", refusal)}
                {num_input(ctrl, sect, "Y", y, emitters.clone(), cues.clone(), index, "y", refusal)}
                {num_input(ctrl, sect, "Radius m", radius, emitters.clone(), cues.clone(), index, "radiusM", refusal)}
            </div>
            <label class="flex flex-col gap-1">
                <span class=sect>"Sound"</span>
                <input type="text" prop:value=sound class=ctrl
                    on:change=move |ev| {
                        refusal.set(String::new());
                        match with_emitter_field(&e_sound, &c_sound, index, "sound", &event_target_value(&ev)) {
                            Ok(next) => commit(block_from_parts(&next, &c_sound).as_ref()),
                            Err(err) => refusal.set(err),
                        }
                    }
                />
            </label>
            <label class="flex items-center gap-2">
                <span class=sect>"Loop"</span>
                <input type="checkbox" prop:checked=loop_on class="accent-primary"
                    on:change=move |ev| {
                        refusal.set(String::new());
                        match with_emitter_loop(&e_loop, &c_loop, index, event_target_checked(&ev)) {
                            Ok(next) => commit(block_from_parts(&next, &c_loop).as_ref()),
                            Err(err) => refusal.set(err),
                        }
                    }
                />
            </label>
            <label class="flex flex-col gap-1">
                <span class=sect>"Trigger id"</span>
                <input type="text" prop:value=trigger class=ctrl placeholder="optional"
                    on:change=move |ev| {
                        refusal.set(String::new());
                        match with_emitter_field(&e_trig, &c_trig, index, "triggerId", &event_target_value(&ev)) {
                            Ok(next) => commit(block_from_parts(&next, &c_trig).as_ref()),
                            Err(err) => refusal.set(err),
                        }
                    }
                />
            </label>
        </div>
    }
    .into_any()
}

#[cfg(target_arch = "wasm32")]
fn cue_row(
    ctrl: &'static str,
    sect: &'static str,
    hint: &'static str,
    index: usize,
    row: &Value,
    emitters: Vec<Value>,
    cues: Vec<Value>,
    refusal: RwSignal<String>,
) -> AnyView {
    let id = row
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let event = row
        .get("event")
        .and_then(Value::as_str)
        .unwrap_or("mission_start")
        .to_string();
    let track = row
        .get("track")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let e_del = emitters.clone();
    let c_del = cues.clone();
    let e_ev = emitters.clone();
    let c_ev = cues.clone();
    let e_tr = emitters.clone();
    let c_tr = cues.clone();

    view! {
        <div class="flex flex-col gap-2 border border-outline-variant/30 p-2">
            <div class="flex items-center gap-2">
                <span class=hint>{format!("id {id}")}</span>
                <button type="button" class="text-label-sm"
                    on:click=move |_| {
                        refusal.set(String::new());
                        let next = remove_at(&c_del, index);
                        commit(block_from_parts(&e_del, &next).as_ref());
                    }
                >"Remove"</button>
            </div>
            <label class="flex flex-col gap-1">
                <span class=sect>"Event"</span>
                <select prop:value=event class=ctrl
                    on:change=move |ev| {
                        refusal.set(String::new());
                        match with_cue_field(&c_ev, &e_ev, index, "event", &event_target_value(&ev)) {
                            Ok(next) => commit(block_from_parts(&e_ev, &next).as_ref()),
                            Err(err) => refusal.set(err),
                        }
                    }
                >
                    {MUSIC_EVENTS.iter().map(|e| view! {
                        <option value=*e>{event_label(e)}</option>
                    }).collect::<Vec<_>>()}
                </select>
            </label>
            <label class="flex flex-col gap-1">
                <span class=sect>"Track"</span>
                <input type="text" prop:value=track class=ctrl
                    on:change=move |ev| {
                        refusal.set(String::new());
                        match with_cue_field(&c_tr, &e_tr, index, "track", &event_target_value(&ev)) {
                            Ok(next) => commit(block_from_parts(&e_tr, &next).as_ref()),
                            Err(err) => refusal.set(err),
                        }
                    }
                />
            </label>
        </div>
    }
    .into_any()
}

#[cfg(target_arch = "wasm32")]
fn num_input(
    ctrl: &'static str,
    sect: &'static str,
    label: &'static str,
    value: String,
    emitters: Vec<Value>,
    cues: Vec<Value>,
    index: usize,
    key: &'static str,
    refusal: RwSignal<String>,
) -> AnyView {
    view! {
        <label class="flex flex-col gap-1">
            <span class=sect>{label}</span>
            <input type="text" prop:value=value class=ctrl
                on:change=move |ev| {
                    refusal.set(String::new());
                    match with_emitter_field(&emitters, &cues, index, key, &event_target_value(&ev)) {
                        Ok(next) => commit(block_from_parts(&next, &cues).as_ref()),
                        Err(err) => refusal.set(err),
                    }
                }
            />
        </label>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ae() -> Value {
        json!({"id": "ae-1", "x": 1.0, "z": 2.0, "sound": "SOUND_HINT", "radiusM": 10.0, "loop": true})
    }

    #[test]
    fn add_appends_a_valid_emitter() {
        let next = add_emitter(&[], &[]).expect("add");
        assert_eq!(next.len(), 1);
        validate(&block_from_parts(&next, &[]).expect("block")).expect("valid");
    }

    #[test]
    fn remove_drops_and_clearing_the_last_writes_null() {
        let next = remove_at(&[ae()], 0);
        assert!(next.is_empty());
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"audio": null}));
    }

    #[test]
    fn radius_zero_is_refused_in_the_panel() {
        let err = with_emitter_field(&[ae()], &[], 0, "radiusM", "0").expect_err("zero");
        assert!(err.contains("above zero"), "{err}");
    }

    #[test]
    fn an_unknown_event_is_refused() {
        let cue = default_cue(&[], &[]);
        let err = with_cue_field(&[cue], &[], 0, "event", "round_pause").expect_err("event");
        assert!(err.contains("round_pause"), "{err}");
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let cue = json!({"id": "ae-1", "event": "mission_end", "track": "SOUND_HINT"});
        let err = refuse_block(&[ae()], &[cue]).expect_err("dup");
        assert!(err.contains("unique"), "{err}");
    }

    #[test]
    fn last_marker_fills_xz() {
        let next = apply_marker_xz(&[ae()], &[], 0, 6400.0, 1200.0).expect("xz");
        assert_eq!(next[0]["x"], 6400.0);
        assert_eq!(next[0]["z"], 1200.0);
        assert_eq!(
            xz_from_last_marker(&[(1.0, 2.0), (9.0, 8.0)]),
            Some((9.0, 8.0))
        );
        assert!(xz_from_last_marker(&[]).is_none());
    }

    #[test]
    fn the_pickers_offer_exactly_the_schema_vocabulary() {
        assert_eq!(
            MUSIC_EVENTS,
            [
                "mission_start",
                "task_succeeded",
                "task_failed",
                "mission_end"
            ]
        );
        for e in MUSIC_EVENTS {
            assert_ne!(event_label(e), "Unknown event");
        }
    }

    #[test]
    fn env_patch_sets_and_clears() {
        let set: Value = serde_json::from_str(&env_patch(block_from_parts(&[ae()], &[]).as_ref()))
            .expect("json");
        assert_eq!(set["audio"]["emitters"][0]["id"], "ae-1");
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"audio": null}));
    }

    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = AUDIO_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in AUDIO_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }

    #[test]
    fn place_on_map_reuses_the_marker_gesture() {
        const SRC: &str = include_str!("audio_emitters.rs");
        assert!(
            SRC.contains("begin_place_marker"),
            "placement must call begin_place_marker, not a new gesture"
        );
        assert!(SRC.contains(PLACE_MARKER_ICON));
        assert!(SRC.contains("arm_place_on_map"));
    }
}
