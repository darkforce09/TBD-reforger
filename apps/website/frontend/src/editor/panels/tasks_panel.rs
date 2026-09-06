//! T-936.2 — the **Tasks** panel: a list of primary / secondary / optional assignments with
//! trigger and marker pickers, every write one undo step.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.tasks` through [`operations::update_environment`], the
//! same one-patch-one-undo-step path the T-224 flow controls and the T-936.1 win-conditions card
//! use. `map_engine_core::mission::extensions` copies the key onto the compiled payload root.
//!
//! ══ Where this renders ══════════════════════════════════════════════════════════════════════
//! [`tasks_panel`] is a section in the same shape as `win_conditions_card`. It belongs in the
//! Mission Settings dialog immediately after that card (`settings_modal.rs`'s
//! `{tasks_panel(ctrl)}`). This slice does not own `settings_modal.rs`; T-936.1's card waited for
//! the wave bookkeeping commit to land its one-line mount for the same reason. A panel that is
//! built, registered and tested but never mounted is a mechanism that cannot fire — the mount is
//! on the human checklist, not silently dropped.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::Value;

use map_engine_core::mission::tasks::{validate_schedule, STATES, TIERS};

#[cfg(target_arch = "wasm32")]
use super::env::read_flow_seconds;
use super::env::FLOW_DEFAULT_TIMELIMIT_S;

/// The reader chain for `meta.environment.tasks`, end to end.
pub const TASKS_READERS: &[(&str, &str)] = &[
    (
        "compile",
        "map_engine_core::mission::compile::compile_payload → the saved payload's top-level \
         `tasks` (via mission::extensions::copy_authored_blocks)",
    ),
    (
        "flatten",
        "map_engine_core::mission::flatten::EditorPayload.authored_blocks_root → \
         ExtensionBlocks::from_payload → the compiled document's `tasks` block",
    ),
    (
        "mod",
        "TBD_TaskStateMachine (server-authoritative transitions driven by T-676 trigger \
         completion) and TBD_TaskHud (one HUD marker per assigned task)",
    ),
    (
        "editor",
        "this panel, which reads the key back on every open through operations::read_env_value",
    ),
];

/// The words a tier gets in the picker.
#[must_use]
pub fn tier_label(tier: &str) -> &'static str {
    match tier {
        "primary" => "Primary",
        "secondary" => "Secondary",
        "optional" => "Optional",
        _ => "Unknown tier",
    }
}

/// The words a state gets in the picker.
#[must_use]
pub fn state_label(state: &str) -> &'static str {
    match state {
        "assigned" => "Assigned",
        "succeeded" => "Succeeded",
        "failed" => "Failed",
        _ => "Unknown state",
    }
}

/// A new task the Add button authors. `id` is unique against `existing`.
#[must_use]
pub fn default_task(existing: &[Value]) -> Value {
    serde_json::json!({
        "id": next_task_id(existing),
        "title": "New task",
        "tier": "primary",
        "state": "assigned",
    })
}

/// `task-1`, `task-2`, … skipping ids already in the list.
#[must_use]
pub fn next_task_id(existing: &[Value]) -> String {
    let mut n = existing.len() + 1;
    loop {
        let candidate = format!("task-{n}");
        let taken = existing
            .iter()
            .any(|t| t.get("id").and_then(Value::as_str) == Some(&candidate));
        if !taken {
            return candidate;
        }
        n += 1;
    }
}

/// The authored array as the document holds it, or empty when the mission authors no tasks.
#[must_use]
pub fn tasks_from_block(block: Option<&Value>) -> Vec<Value> {
    block
        .and_then(Value::as_array)
        .map(|a| a.clone())
        .unwrap_or_default()
}

/// Append one default task. One new array, one undo step at commit.
#[must_use]
pub fn add_task(existing: &[Value]) -> Vec<Value> {
    let mut next = existing.to_vec();
    next.push(default_task(existing));
    next
}

/// Remove the row at `index`, or return the list unchanged when the index is out of range.
#[must_use]
pub fn remove_task(existing: &[Value], index: usize) -> Vec<Value> {
    let mut next = existing.to_vec();
    if index < next.len() {
        next.remove(index);
    }
    next
}

/// Move the row at `index` up or down by one. Out-of-range and already-at-end are no-ops.
#[must_use]
pub fn move_task(existing: &[Value], index: usize, delta: i32) -> Vec<Value> {
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
    let row = from.clone();
    next.remove(index);
    next.insert(to, row);
    next
}

/// Set one string field on the row at `index`. Blank optional keys are removed rather than stored
/// as `""` (the compile refuses a blank `triggerId` / `markerId` / `description`).
pub fn with_field(
    existing: &[Value],
    index: usize,
    key: &str,
    value: &str,
) -> Result<Vec<Value>, String> {
    if !EDITABLE_KEYS.contains(&key) {
        return Err(format!("{key:?} is not an authored task field"));
    }
    let Some(row) = existing.get(index).and_then(Value::as_object) else {
        return Err("that task is no longer in the list".to_string());
    };
    if key == "tier" && !TIERS.contains(&value) {
        return Err(format!("tier {value:?} is not one of {}", TIERS.join(", ")));
    }
    if key == "state" && !STATES.contains(&value) {
        return Err(format!(
            "state {value:?} is not one of {}",
            STATES.join(", ")
        ));
    }
    if matches!(key, "id" | "title") && value.trim().is_empty() {
        return Err(format!("{key} cannot be blank"));
    }
    if key == "id" {
        let trimmed = value.trim();
        let clash = existing
            .iter()
            .enumerate()
            .any(|(i, t)| i != index && t.get("id").and_then(Value::as_str) == Some(trimmed));
        if clash {
            return Err(format!("id {trimmed:?} is already used by another task"));
        }
    }

    let mut obj = row.clone();
    let trimmed = value.trim();
    if trimmed.is_empty() && OPTIONAL_KEYS.contains(&key) {
        obj.remove(key);
    } else {
        obj.insert(key.to_string(), Value::String(trimmed.to_string()));
    }
    let mut next = existing.to_vec();
    next[index] = Value::Object(obj);
    Ok(next)
}

/// Seconds currently authored on `schedule.<key>`, or empty when the task is untimed.
#[must_use]
pub fn schedule_seconds(row: &Value, key: &str) -> String {
    row.get("schedule")
        .and_then(|s| s.get(key))
        .and_then(Value::as_i64)
        .map(|n| n.to_string())
        .unwrap_or_default()
}

/// Set or clear `tasks[index].schedule`. Empty-empty removes the key (untimed). One empty field
/// is refused so a half-typed row cannot ride the wire. `mission_length_s` is
/// `flow.timeLimitSeconds` (0 = no limit).
///
/// # Errors
/// The reason shown in the panel: window, start, or mission-length refusal copy.
pub fn with_schedule(
    existing: &[Value],
    index: usize,
    start_after_s: &str,
    window_s: &str,
    mission_length_s: i64,
) -> Result<Vec<Value>, String> {
    let Some(row) = existing.get(index).and_then(Value::as_object) else {
        return Err("that task is no longer in the list".to_string());
    };
    let start_raw = start_after_s.trim();
    let window_raw = window_s.trim();
    let mut obj = row.clone();
    if start_raw.is_empty() && window_raw.is_empty() {
        obj.remove("schedule");
        let mut next = existing.to_vec();
        next[index] = Value::Object(obj);
        return Ok(next);
    }
    if start_raw.is_empty() {
        return Err("startAfterS is required when a schedule is authored".to_string());
    }
    if window_raw.is_empty() {
        return Err("windowS is required when a schedule is authored".to_string());
    }
    let start: i64 = start_raw
        .parse()
        .map_err(|_| format!("startAfterS {start_raw:?} is not an integer"))?;
    let window: i64 = window_raw
        .parse()
        .map_err(|_| format!("windowS {window_raw:?} is not an integer"))?;
    validate_schedule(start, window, Some(mission_length_s))?;
    obj.insert(
        "schedule".to_string(),
        serde_json::json!({"startAfterS": start, "windowS": window}),
    );
    let mut next = existing.to_vec();
    next[index] = Value::Object(obj);
    Ok(next)
}

const EDITABLE_KEYS: &[&str] = &[
    "id",
    "title",
    "tier",
    "state",
    "triggerId",
    "markerId",
    "description",
];
const OPTIONAL_KEYS: &[&str] = &["triggerId", "markerId", "description"];

/// The `meta.environment` merge patch for a list, or for CLEARING one.
///
/// `None` / empty writes an explicit `null` rather than omitting the key, because
/// `update_environment` MERGES: an omitted key leaves the previous value in place.
#[must_use]
pub fn env_patch(tasks: Option<&[Value]>) -> String {
    match tasks {
        None | Some(&[]) => serde_json::json!({ "tasks": null }).to_string(),
        Some(rows) => serde_json::json!({ "tasks": rows }).to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_block() -> Option<Value> {
    crate::editor::state::operations::read_env_value("tasks").filter(|v| v.is_array())
}

#[cfg(target_arch = "wasm32")]
fn commit(tasks: Option<&[Value]>) {
    if let Some(rows) = tasks {
        let value = Value::Array(rows.to_vec());
        if let Err(clause) = map_engine_core::mission::tasks::validate(&value) {
            leptos::logging::warn!("tasks is not yet complete: {clause}");
        }
    }
    crate::editor::state::operations::update_environment(env_patch(tasks));
}

#[cfg(target_arch = "wasm32")]
fn trigger_options() -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = crate::editor::state::operations::trigger_rows()
        .into_iter()
        .map(|r| {
            let label = r.name.unwrap_or_else(|| r.id.clone());
            (r.id, label)
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

#[cfg(target_arch = "wasm32")]
fn marker_options() -> Vec<(String, String)> {
    crate::editor::state::operations::marker_rows()
        .into_iter()
        .map(|r| {
            let label = if r.label.is_empty() {
                r.id.clone()
            } else {
                format!("{} ({})", r.label, r.id)
            };
            (r.id, label)
        })
        .collect()
}

/// The **Tasks** panel. `ctrl` is the dialog's shared control class.
///
/// Inert on the native view shell (no document), like every sibling panel.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn tasks_panel(ctrl: &'static str) -> AnyView {
    let _ = ctrl;
    ().into_any()
}

/// The **Tasks** panel — see the native sibling for the signature contract.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn tasks_panel(ctrl: &'static str) -> AnyView {
    let sect = "text-label-sm uppercase tracking-wider text-outline";
    let hint = "text-label-sm normal-case text-outline";

    let rows = tasks_from_block(read_block().as_ref());
    let triggers = trigger_options();
    let markers = marker_options();
    let refusal = RwSignal::new(String::new());

    let list = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let id = row.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let title = row.get("title").and_then(Value::as_str).unwrap_or("").to_string();
            let tier = row.get("tier").and_then(Value::as_str).unwrap_or("primary").to_string();
            let state = row.get("state").and_then(Value::as_str).unwrap_or("assigned").to_string();
            let trigger_id = row
                .get("triggerId")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let marker_id = row
                .get("markerId")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let description = row
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let start_after = schedule_seconds(&row, "startAfterS");
            let window_s = schedule_seconds(&row, "windowS");
            let window_for_start = window_s.clone();
            let start_for_window = start_after.clone();
            let rows_for_edit = rows.clone();
            let rows_for_tier = rows.clone();
            let rows_for_state = rows.clone();
            let rows_for_trigger = rows.clone();
            let rows_for_marker = rows.clone();
            let rows_for_desc = rows.clone();
            let rows_for_start = rows.clone();
            let rows_for_window = rows.clone();
            let rows_for_up = rows.clone();
            let rows_for_down = rows.clone();
            let rows_for_del = rows.clone();
            let trigger_opts = triggers.clone();
            let marker_opts = markers.clone();

            view! {
                <div class="flex flex-col gap-2 border border-outline-variant/30 p-2">
                    <div class="flex items-center gap-2">
                        <input
                            type="text"
                            prop:value=title.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_edit, index, "title", &event_target_value(&ev)) {
                                    Ok(next) => commit(Some(&next)),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                commit(Some(&move_task(&rows_for_up, index, -1)));
                            }
                        >
                            "Up"
                        </button>
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                commit(Some(&move_task(&rows_for_down, index, 1)));
                            }
                        >
                            "Down"
                        </button>
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                let next = remove_task(&rows_for_del, index);
                                if next.is_empty() {
                                    commit(None);
                                } else {
                                    commit(Some(&next));
                                }
                            }
                        >
                            "Remove"
                        </button>
                    </div>
                    <span class=hint>{format!("id {id}")}</span>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Tier"</span>
                        <select
                            prop:value=tier.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_tier, index, "tier", &event_target_value(&ev)) {
                                    Ok(next) => commit(Some(&next)),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        >
                            {TIERS
                                .iter()
                                .map(|t| view! { <option value=*t>{tier_label(t)}</option> })
                                .collect::<Vec<_>>()}
                        </select>
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"State"</span>
                        <select
                            prop:value=state.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_state, index, "state", &event_target_value(&ev)) {
                                    Ok(next) => commit(Some(&next)),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        >
                            {STATES
                                .iter()
                                .map(|s| view! { <option value=*s>{state_label(s)}</option> })
                                .collect::<Vec<_>>()}
                        </select>
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Trigger"</span>
                        <select
                            prop:value=trigger_id.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_trigger, index, "triggerId", &event_target_value(&ev)) {
                                    Ok(next) => commit(Some(&next)),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        >
                            <option value="">"None — this task does not auto-complete"</option>
                            {trigger_opts
                                .into_iter()
                                .map(|(value, label)| view! { <option value=value>{label}</option> })
                                .collect::<Vec<_>>()}
                        </select>
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Marker"</span>
                        <select
                            prop:value=marker_id.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_marker, index, "markerId", &event_target_value(&ev)) {
                                    Ok(next) => commit(Some(&next)),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        >
                            <option value="">"None — HUD uses objective_marker"</option>
                            {marker_opts
                                .into_iter()
                                .map(|(value, label)| view! { <option value=value>{label}</option> })
                                .collect::<Vec<_>>()}
                        </select>
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Description"</span>
                        <input
                            type="text"
                            prop:value=description.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_desc, index, "description", &event_target_value(&ev)) {
                                    Ok(next) => commit(Some(&next)),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Start after (s)"</span>
                        <input
                            type="text"
                            prop:value=start_after.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                let length = read_flow_seconds("timeLimitSeconds", FLOW_DEFAULT_TIMELIMIT_S);
                                match with_schedule(
                                    &rows_for_start,
                                    index,
                                    &event_target_value(&ev),
                                    &window_for_start,
                                    length,
                                ) {
                                    Ok(next) => commit(Some(&next)),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Window (s)"</span>
                        <input
                            type="text"
                            prop:value=window_s.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                let length = read_flow_seconds("timeLimitSeconds", FLOW_DEFAULT_TIMELIMIT_S);
                                match with_schedule(
                                    &rows_for_window,
                                    index,
                                    &start_for_window,
                                    &event_target_value(&ev),
                                    length,
                                ) {
                                    Ok(next) => commit(Some(&next)),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                </div>
            }
        })
        .collect::<Vec<_>>();

    let rows_for_add = rows.clone();

    view! {
        <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
            <span class=sect>"Tasks"</span>
            <span class=hint>
                "Primary, secondary and optional assignments. Completing a linked trigger succeeds \
                 the task in-game; the HUD shows only assigned tasks. A schedule keeps the task \
                 inactive until startAfterS and evaluates only inside windowS."
            </span>
            {list}
            <button
                type="button"
                class=ctrl
                on:click=move |_| {
                    refusal.set(String::new());
                    commit(Some(&add_task(&rows_for_add)));
                }
            >
                "Add task"
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

    fn pri() -> Value {
        json!({"id": "task-1", "title": "Seize", "tier": "primary", "state": "assigned"})
    }

    #[test]
    fn add_appends_a_primary_assigned_task_with_a_fresh_id() {
        let next = add_task(&[pri()]);
        assert_eq!(next.len(), 2);
        assert_eq!(next[1]["id"], "task-2");
        assert_eq!(next[1]["tier"], "primary");
        assert_eq!(next[1]["state"], "assigned");
        map_engine_core::mission::tasks::validate(&Value::Array(next))
            .expect("the panel must not author a block the compile refuses");
    }

    #[test]
    fn remove_drops_one_row_and_clearing_the_last_writes_null() {
        let next = remove_task(&[pri()], 0);
        assert!(next.is_empty());
        let cleared: Value = serde_json::from_str(&env_patch(Some(&next))).expect("json");
        assert_eq!(cleared, json!({"tasks": null}));
    }

    #[test]
    fn reorder_swaps_neighbours_and_is_undoable_as_one_array() {
        let a = pri();
        let b = json!({"id": "task-2", "title": "Cache", "tier": "secondary", "state": "assigned"});
        let up = move_task(&[a.clone(), b.clone()], 1, -1);
        assert_eq!(up[0]["id"], "task-2");
        assert_eq!(up[1]["id"], "task-1");
        let down = move_task(&up, 0, 1);
        assert_eq!(down[0]["id"], "task-1");
        // Moving past the ends is a no-op, not a wrap.
        assert_eq!(move_task(&down, 0, -1), down);
        assert_eq!(move_task(&down, 1, 1), down);
    }

    #[test]
    fn the_pickers_offer_exactly_the_schema_vocabularies() {
        assert_eq!(TIERS, ["primary", "secondary", "optional"]);
        assert_eq!(STATES, ["assigned", "succeeded", "failed"]);
        for t in TIERS {
            assert_ne!(tier_label(t), "Unknown tier");
        }
        for s in STATES {
            assert_ne!(state_label(s), "Unknown state");
        }
    }

    #[test]
    fn with_field_sets_tier_trigger_and_marker_and_strips_blank_optionals() {
        let base = vec![pri()];
        let next = with_field(&base, 0, "tier", "secondary").expect("tier");
        assert_eq!(next[0]["tier"], "secondary");
        let next = with_field(&next, 0, "triggerId", "trg-hill").expect("trigger");
        assert_eq!(next[0]["triggerId"], "trg-hill");
        let next = with_field(&next, 0, "markerId", "attack").expect("marker");
        assert_eq!(next[0]["markerId"], "attack");
        let next = with_field(&next, 0, "triggerId", "  ").expect("blank optional");
        assert!(next[0].get("triggerId").is_none());
        map_engine_core::mission::tasks::validate(&Value::Array(next)).expect("valid");
    }

    #[test]
    fn a_blank_title_is_refused() {
        let err = with_field(&[pri()], 0, "title", "   ").expect_err("blank title");
        assert!(err.contains("blank"), "{err}");
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let rows = vec![
            pri(),
            json!({"id": "task-2", "title": "B", "tier": "optional", "state": "assigned"}),
        ];
        let err = with_field(&rows, 1, "id", "task-1").expect_err("clash");
        assert!(err.contains("already used"), "{err}");
    }

    #[test]
    fn a_full_authoring_pass_produces_a_block_the_compile_accepts() {
        let mut rows = add_task(&[]);
        rows = with_field(&rows, 0, "title", "Seize the hill").expect("title");
        rows = with_field(&rows, 0, "tier", "primary").expect("tier");
        rows = with_field(&rows, 0, "triggerId", "trg-hill").expect("trigger");
        rows = with_field(&rows, 0, "markerId", "attack").expect("marker");
        rows = add_task(&rows);
        rows = with_field(&rows, 1, "title", "Find the cache").expect("t2");
        rows = with_field(&rows, 1, "tier", "secondary").expect("sec");
        rows = add_task(&rows);
        rows = with_field(&rows, 2, "title", "Radio check").expect("t3");
        rows = with_field(&rows, 2, "tier", "optional").expect("opt");
        rows = move_task(&rows, 2, -1);
        assert_eq!(rows[1]["tier"], "optional");
        map_engine_core::mission::tasks::validate(&Value::Array(rows))
            .expect("the panel must not author a block the compile refuses");
    }

    #[test]
    fn clearing_writes_an_explicit_null_patch() {
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"tasks": null}));
        let set: Value = serde_json::from_str(&env_patch(Some(&[pri()]))).expect("json");
        assert_eq!(set["tasks"][0]["id"], "task-1");
    }

    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = TASKS_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in TASKS_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }

    #[test]
    fn with_schedule_writes_start_and_window() {
        let next = with_schedule(&[pri()], 0, "600", "300", 5400).expect("legal");
        assert_eq!(next[0]["schedule"]["startAfterS"], 600);
        assert_eq!(next[0]["schedule"]["windowS"], 300);
        assert_eq!(schedule_seconds(&next[0], "startAfterS"), "600");
        assert_eq!(schedule_seconds(&next[0], "windowS"), "300");
        assert_eq!(schedule_seconds(&pri(), "startAfterS"), "");
        map_engine_core::mission::tasks::validate(&Value::Array(next))
            .expect("the panel must not author a block the compile refuses");
        assert_eq!(FLOW_DEFAULT_TIMELIMIT_S, 5400);
    }

    #[test]
    fn a_zero_window_is_refused_with_the_reason() {
        let err = with_schedule(&[pri()], 0, "10", "0", 5400).expect_err("window 0");
        assert!(err.contains("windowS"), "{err}");
        assert!(err.contains("> 0"), "{err}");
    }

    #[test]
    fn start_past_mission_length_is_refused_with_the_reason() {
        let err = with_schedule(&[pri()], 0, "5400", "60", 5400).expect_err("at end");
        assert!(err.contains("within mission length"), "{err}");
        assert!(err.contains("5400"), "{err}");
    }

    #[test]
    fn clearing_both_fields_removes_the_schedule() {
        let timed = with_schedule(&[pri()], 0, "120", "60", 5400).expect("set");
        let cleared = with_schedule(&timed, 0, "", "", 5400).expect("clear");
        assert!(cleared[0].get("schedule").is_none());
    }

    #[test]
    fn a_half_filled_schedule_is_refused() {
        let err = with_schedule(&[pri()], 0, "120", "", 5400).expect_err("window missing");
        assert!(err.contains("windowS"), "{err}");
        let err = with_schedule(&[pri()], 0, "", "60", 5400).expect_err("start missing");
        assert!(err.contains("startAfterS"), "{err}");
    }
}
