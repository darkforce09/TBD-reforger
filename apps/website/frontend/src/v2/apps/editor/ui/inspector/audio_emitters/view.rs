//! Audio emitters inspector view.

use super::*;

/// Arms marker placement from the audio emitter panel.
#[cfg(target_arch = "wasm32")]
pub fn arm_place_on_map() {
    armed_placement::begin_place_marker(PLACE_MARKER_ICON.to_string());
}

/// The Audio panel's native stand-in: an empty view.
///
/// Every control here reads and writes the live document through wasm-only paths, so off the
/// browser target there is nothing to render and the panel renders nothing rather than a shell
/// that cannot work.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn audio_emitters_panel(ctrl: &'static str) -> AnyView {
    let _ = ctrl;
    ().into_any()
}

/// The Audio panel: the positional emitter list and the music cue list, built from the document's
/// current audio block.
///
/// **Signals & state:** the lists are read once per build from the document, and every accepted
/// edit commits the whole rebuilt block in one environment update, which is one undo step. A
/// refused edit is written to a local refusal signal and shown beside the controls instead; the
/// document is not touched. `ctrl` is the shared control class the surrounding settings surface
/// styles its inputs with.
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
                        let markers: Vec<(f64, f64)> = website_map_engine::editing::hosted_commands::marker_rows()
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
