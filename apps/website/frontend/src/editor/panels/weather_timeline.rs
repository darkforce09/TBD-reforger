//! T-936.4 — the **Weather timeline** panel: keyframes (`atMinutes`, preset, optional wind/fog),
//! every write one undo step.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.weatherTimeline` through [`operations::update_environment`],
//! the same one-patch-one-undo-step path the T-224 flow controls and the T-936.1/.2/.3 cards use.
//! `map_engine_core::mission::extensions` copies the key onto the compiled payload root.
//!
//! ══ Where this renders ══════════════════════════════════════════════════════════════════════
//! [`weather_timeline_panel`] is a section in the same shape as `win_conditions_card` /
//! `tasks_panel` / `radio_panel`. It belongs in the Mission Settings dialog
//! (`settings_modal.rs`'s `{weather_timeline_panel(ctrl)}`). This slice does not own
//! `settings_modal.rs`; T-936.1/.2/.3 waited for the wave bookkeeping commit to land the one-line
//! mount for the same reason. A panel that is built, registered and tested but never mounted is a
//! mechanism that cannot fire — the mount is on the human checklist, not silently dropped.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::{Map, Value};

use map_engine_core::mission::weather::{validate, WEATHER_PRESETS};

/// The reader chain for `meta.environment.weatherTimeline`, end to end.
pub const WEATHER_TIMELINE_READERS: &[(&str, &str)] = &[
    (
        "compile",
        "map_engine_core::mission::compile::compile_payload → the saved payload's top-level \
         `weatherTimeline` (via mission::extensions::copy_authored_blocks)",
    ),
    (
        "flatten",
        "map_engine_core::mission::flatten::EditorPayload.authored_blocks_root → \
         ExtensionBlocks::from_payload → the compiled document's `weatherTimeline` block \
         (needs the named EditorPayload field T-291 owns; without it /compiled drops the key)",
    ),
    (
        "mod",
        "TBD_WeatherRuntime (server applies each keyframe at atMinutes through \
         TimeAndWeatherManagerEntity.ForceWeatherTo; clients follow weather replication)",
    ),
    (
        "editor",
        "panels/weather_timeline.rs — this panel, via operations::update_environment",
    ),
];

const EDITABLE_KEYS: &[&str] = &["atMinutes", "weatherPreset", "windDirDeg", "fog"];

/// Human labels for [`WEATHER_PRESETS`], same words as `top_strip.rs` `WEATHER_OPTIONS`.
#[must_use]
pub fn preset_label(preset: &str) -> &'static str {
    match preset {
        "clear" => "Clear",
        "overcast" => "Overcast",
        "heavy_rain" => "Heavy Rain",
        "dense_fog" => "Dense Fog",
        _ => "Unknown preset",
    }
}

/// One default keyframe: fifteen minutes after the last, or T+0 on an empty list, Clear.
#[must_use]
pub fn default_keyframe(existing: &[Value]) -> Value {
    let at = existing
        .iter()
        .filter_map(|row| json_i64(row.get("atMinutes")))
        .max()
        .map(|n| n + 15)
        .unwrap_or(0);
    serde_json::json!({
        "atMinutes": at,
        "weatherPreset": "clear"
    })
}

/// The keyframes array inside a `weatherTimeline` object, or empty when the block is absent.
#[must_use]
pub fn keyframes_from_block(block: Option<&Value>) -> Vec<Value> {
    block
        .and_then(Value::as_object)
        .and_then(|o| o.get("keyframes"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Wrap keyframes into the authored object, or `None` when the list is empty (omit, don't store).
#[must_use]
pub fn timeline_from_keyframes(rows: &[Value]) -> Option<Value> {
    if rows.is_empty() {
        None
    } else {
        Some(serde_json::json!({ "keyframes": rows }))
    }
}

/// Append one default keyframe. One new array, one undo step at commit.
#[must_use]
pub fn add_keyframe(existing: &[Value]) -> Result<Vec<Value>, String> {
    let mut next = existing.to_vec();
    next.push(default_keyframe(existing));
    refuse_order(&next)?;
    Ok(next)
}

/// Remove the row at `index`, or return the list unchanged when the index is out of range.
#[must_use]
pub fn remove_keyframe(existing: &[Value], index: usize) -> Vec<Value> {
    let mut next = existing.to_vec();
    if index < next.len() {
        next.remove(index);
    }
    next
}

/// Reorder: swap the weather (preset / wind / fog) with the neighbour, leave `atMinutes` on the
/// slot so the timeline stays strictly increasing. A pure object-swap would invert times and the
/// panel would have to refuse the only reorder the ticket asks for.
#[must_use]
pub fn move_keyframe(existing: &[Value], index: usize, delta: i32) -> Vec<Value> {
    if existing.is_empty() {
        return existing.to_vec();
    }
    let Some(from) = existing.get(index) else {
        return existing.to_vec();
    };
    let to = (index as i32 + delta).clamp(0, (existing.len() - 1) as i32) as usize;
    if to == index {
        return existing.to_vec();
    }
    let mut next = existing.to_vec();
    let moved = from.clone();
    next.remove(index);
    next.insert(to, moved);
    let times: Vec<Option<Value>> = existing
        .iter()
        .map(|row| row.get("atMinutes").cloned())
        .collect();
    for (i, row) in next.iter_mut().enumerate() {
        let Some(obj) = row.as_object_mut() else {
            continue;
        };
        match times.get(i).cloned().flatten() {
            Some(t) => {
                obj.insert("atMinutes".into(), t);
            }
            None => {
                obj.remove("atMinutes");
            }
        }
    }
    next
}

/// Set one field on the row at `index`. Blank optional keys are removed rather than stored as
/// `""`. Out-of-order `atMinutes` and unknown presets are refused here so the panel never authors
/// a block the compile rejects.
pub fn with_field(
    existing: &[Value],
    index: usize,
    key: &str,
    value: &str,
) -> Result<Vec<Value>, String> {
    if !EDITABLE_KEYS.contains(&key) {
        return Err(format!("{key:?} is not an authored keyframe field"));
    }
    let Some(row) = existing.get(index).and_then(Value::as_object) else {
        return Err("that keyframe is no longer in the list".to_string());
    };

    let mut obj = row.clone();
    let trimmed = value.trim();

    match key {
        "weatherPreset" => {
            if !WEATHER_PRESETS.contains(&trimmed) {
                return Err(format!(
                    "weatherPreset {trimmed:?} is not one of {}",
                    WEATHER_PRESETS.join(", ")
                ));
            }
            obj.insert(key.to_string(), Value::String(trimmed.to_string()));
        }
        "atMinutes" => {
            let n = parse_i64(trimmed, "atMinutes")?;
            if n < 0 {
                return Err("atMinutes cannot be negative".into());
            }
            obj.insert(key.to_string(), json_i64_value(n));
        }
        "windDirDeg" => optional_number(&mut obj, key, trimmed, 0.0, 360.0, "outside 0..=360")?,
        "fog" => optional_number(&mut obj, key, trimmed, 0.0, 1.0, "outside 0..=1")?,
        _ => unreachable!("EDITABLE_KEYS"),
    }

    let mut next = existing.to_vec();
    next[index] = Value::Object(obj);
    refuse_order(&next)?;
    Ok(next)
}

fn optional_number(
    obj: &mut Map<String, Value>,
    key: &str,
    trimmed: &str,
    min: f64,
    max: f64,
    range_clause: &str,
) -> Result<(), String> {
    if trimmed.is_empty() {
        obj.remove(key);
        return Ok(());
    }
    let n: f64 = trimmed
        .parse()
        .map_err(|_| format!("{key} must be a number"))?;
    if !(min..=max).contains(&n) {
        return Err(format!("{key} is {n} — {range_clause}"));
    }
    obj.insert(key.to_string(), json_f64(n));
    Ok(())
}

fn refuse_order(rows: &[Value]) -> Result<(), String> {
    match timeline_from_keyframes(rows) {
        None => Ok(()),
        Some(block) => validate(&block),
    }
}

fn parse_i64(raw: &str, key: &str) -> Result<i64, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{key} cannot be blank"));
    }
    trimmed
        .parse::<i64>()
        .or_else(|_| {
            trimmed
                .parse::<f64>()
                .ok()
                .filter(|f| f.fract() == 0.0)
                .map(|f| f as i64)
                .ok_or(())
        })
        .map_err(|_| format!("{key} must be a whole number of minutes"))
}

fn json_i64(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    v.as_i64()
        .or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
}

fn json_i64_value(n: i64) -> Value {
    Value::Number(n.into())
}

fn json_f64(n: f64) -> Value {
    Value::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()))
}

/// The environment patch string `update_environment` consumes. Empty list → `null` (omit).
#[must_use]
pub fn env_patch(timeline: Option<&Value>) -> String {
    match timeline {
        None => serde_json::json!({ "weatherTimeline": null }).to_string(),
        Some(block) => serde_json::json!({ "weatherTimeline": block }).to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_block() -> Option<Value> {
    crate::editor::state::operations::read_env_value("weatherTimeline").filter(|v| v.is_object())
}

#[cfg(target_arch = "wasm32")]
fn commit(timeline: Option<&Value>) {
    if let Some(block) = timeline {
        if let Err(clause) = validate(block) {
            leptos::logging::warn!("weatherTimeline is not yet complete: {clause}");
        }
    }
    crate::editor::state::operations::update_environment(env_patch(timeline));
}

/// The **Weather timeline** panel. `ctrl` is the dialog's shared control class.
///
/// Inert on the native view shell (no document), like every sibling panel.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn weather_timeline_panel(ctrl: &'static str) -> AnyView {
    let _ = ctrl;
    ().into_any()
}

/// The **Weather timeline** panel — see the native sibling for the signature contract.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn weather_timeline_panel(ctrl: &'static str) -> AnyView {
    let sect = "text-label-sm uppercase tracking-wider text-outline";
    let hint = "text-label-sm normal-case text-outline";

    let rows = keyframes_from_block(read_block().as_ref());
    let refusal = RwSignal::new(String::new());
    let rows_for_add = rows.clone();

    let list = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let at = json_i64(row.get("atMinutes"))
                .map(|n| n.to_string())
                .unwrap_or_default();
            let preset = row
                .get("weatherPreset")
                .and_then(Value::as_str)
                .unwrap_or("clear")
                .to_string();
            let wind = row
                .get("windDirDeg")
                .and_then(Value::as_f64)
                .map(|n| format!("{n}"))
                .unwrap_or_default();
            let fog = row
                .get("fog")
                .and_then(Value::as_f64)
                .map(|n| format!("{n}"))
                .unwrap_or_default();

            let rows_for_edit = rows.clone();
            let rows_for_up = rows.clone();
            let rows_for_down = rows.clone();
            let rows_for_del = rows.clone();
            let rows_for_preset = rows.clone();
            let rows_for_wind = rows.clone();
            let rows_for_fog = rows.clone();

            view! {
                <div class="flex flex-col gap-2 border border-outline-variant/30 p-2">
                    <div class="flex items-center gap-2">
                        <label class="flex flex-col gap-1">
                            <span class=sect>"At (min)"</span>
                            <input
                                type="text"
                                prop:value=at.clone()
                                class=ctrl
                                on:change=move |ev| {
                                    refusal.set(String::new());
                                    match with_field(
                                        &rows_for_edit,
                                        index,
                                        "atMinutes",
                                        &event_target_value(&ev),
                                    ) {
                                        Ok(next) => commit(timeline_from_keyframes(&next).as_ref()),
                                        Err(err) => refusal.set(err),
                                    }
                                }
                            />
                        </label>
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                commit(timeline_from_keyframes(&move_keyframe(&rows_for_up, index, -1)).as_ref());
                            }
                        >
                            "Up"
                        </button>
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                commit(
                                    timeline_from_keyframes(&move_keyframe(&rows_for_down, index, 1)).as_ref(),
                                );
                            }
                        >
                            "Down"
                        </button>
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                let next = remove_keyframe(&rows_for_del, index);
                                commit(timeline_from_keyframes(&next).as_ref());
                            }
                        >
                            "Delete"
                        </button>
                    </div>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Weather"</span>
                        <select
                            prop:value=preset.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_preset,
                                    index,
                                    "weatherPreset",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(timeline_from_keyframes(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        >
                            {WEATHER_PRESETS
                                .iter()
                                .map(|p| view! { <option value=*p>{preset_label(p)}</option> })
                                .collect::<Vec<_>>()}
                        </select>
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Wind dir (deg)"</span>
                        <input
                            type="text"
                            prop:value=wind.clone()
                            class=ctrl
                            placeholder="optional"
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_wind,
                                    index,
                                    "windDirDeg",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(timeline_from_keyframes(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Fog"</span>
                        <input
                            type="text"
                            prop:value=fog.clone()
                            class=ctrl
                            placeholder="optional 0..1"
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_fog, index, "fog", &event_target_value(&ev)) {
                                    Ok(next) => commit(timeline_from_keyframes(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                </div>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
            <span class=sect>"Weather timeline"</span>
            <span class=hint>
                "Keyframes change weather over mission time. atMinutes is minutes from LIVE start, \
                 strictly increasing. Wind and fog are optional; blank omits them. A mission with \
                 no timeline keeps today's static weatherPreset."
            </span>
            {list}
            <button
                type="button"
                class=ctrl
                on:click=move |_| {
                    refusal.set(String::new());
                    match add_keyframe(&rows_for_add) {
                        Ok(next) => commit(timeline_from_keyframes(&next).as_ref()),
                        Err(err) => refusal.set(err),
                    }
                }
            >
                "Add keyframe"
            </button>
            <p class="text-label-sm text-error">{move || refusal.get()}</p>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn t0() -> Value {
        json!({"atMinutes": 0, "weatherPreset": "clear"})
    }

    fn t15() -> Value {
        json!({"atMinutes": 15, "weatherPreset": "overcast", "windDirDeg": 90.0})
    }

    #[test]
    fn add_appends_fifteen_minutes_after_the_last() {
        let next = add_keyframe(&[t0()]).expect("add");
        assert_eq!(next.len(), 2);
        assert_eq!(next[1]["atMinutes"], 15);
        assert_eq!(next[1]["weatherPreset"], "clear");
        validate(&timeline_from_keyframes(&next).expect("block")).expect("valid");
    }

    #[test]
    fn remove_drops_one_row_and_clearing_the_last_writes_null() {
        let next = remove_keyframe(&[t0()], 0);
        assert!(next.is_empty());
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"weatherTimeline": null}));
    }

    #[test]
    fn reorder_swaps_weather_and_keeps_times_increasing() {
        let a = t0();
        let b = t15();
        let up = move_keyframe(&[a.clone(), b.clone()], 1, -1);
        assert_eq!(up[0]["atMinutes"], 0);
        assert_eq!(up[0]["weatherPreset"], "overcast");
        assert_eq!(up[1]["atMinutes"], 15);
        assert_eq!(up[1]["weatherPreset"], "clear");
        validate(&timeline_from_keyframes(&up).expect("block")).expect("still increasing");
        let down = move_keyframe(&up, 0, 1);
        assert_eq!(down[0]["weatherPreset"], "clear");
        assert_eq!(down[1]["weatherPreset"], "overcast");
        assert_eq!(move_keyframe(&down, 0, -1), down);
        assert_eq!(move_keyframe(&down, 1, 1), down);
    }

    #[test]
    fn equal_at_minutes_are_refused_in_the_panel() {
        let err = with_field(&[t0(), t15()], 1, "atMinutes", "0").expect_err("equal");
        assert!(err.contains("strictly increasing"), "{err}");
    }

    #[test]
    fn out_of_order_at_minutes_are_refused_in_the_panel() {
        let err = with_field(&[t0(), t15()], 1, "atMinutes", "-1").expect_err("neg");
        assert!(err.contains("negative"), "{err}");
        let err = with_field(&[t0(), t15()], 0, "atMinutes", "20").expect_err("invert");
        assert!(err.contains("strictly increasing"), "{err}");
    }

    #[test]
    fn an_unknown_preset_is_refused() {
        let err = with_field(&[t0()], 0, "weatherPreset", "hailstorm").expect_err("preset");
        assert!(err.contains("hailstorm"), "{err}");
    }

    #[test]
    fn blank_optional_wind_and_fog_are_stripped() {
        let next = with_field(&[t15()], 0, "windDirDeg", "  ").expect("blank wind");
        assert!(next[0].get("windDirDeg").is_none());
        let next = with_field(&next, 0, "fog", "0.25").expect("fog");
        assert_eq!(next[0]["fog"], 0.25);
        let next = with_field(&next, 0, "fog", "").expect("blank fog");
        assert!(next[0].get("fog").is_none());
        validate(&timeline_from_keyframes(&next).expect("block")).expect("valid");
    }

    #[test]
    fn the_pickers_offer_exactly_the_schema_vocabulary() {
        assert_eq!(
            WEATHER_PRESETS,
            ["clear", "overcast", "heavy_rain", "dense_fog"]
        );
        for p in WEATHER_PRESETS {
            assert_ne!(preset_label(p), "Unknown preset");
        }
    }

    #[test]
    fn env_patch_sets_and_clears() {
        let set: Value =
            serde_json::from_str(&env_patch(timeline_from_keyframes(&[t0()]).as_ref()))
                .expect("json");
        assert_eq!(
            set["weatherTimeline"]["keyframes"][0]["weatherPreset"],
            "clear"
        );
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"weatherTimeline": null}));
    }

    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = WEATHER_TIMELINE_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in WEATHER_TIMELINE_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }
}
