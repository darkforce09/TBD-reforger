//! the **Tasks** panel: a list of primary / secondary / optional assignments with
//! trigger and marker pickers, every write one undo step.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.tasks` through [`operations::update_environment`], the
//! same one-patch-one-undo-step path the  flow controls and the  win-conditions card
//! use. `map_engine_core::mission::extensions` copies the key onto the compiled payload root.
//!
//! ══ Where this renders ══════════════════════════════════════════════════════════════════════
//! [`tasks_panel`] is a section in the same shape as `win_conditions_card`. It belongs in the
//! Mission Settings dialog immediately after that card (`settings_modal.rs`'s
//! `{tasks_panel(ctrl)}`). This slice does not own `settings_modal.rs`; 's card waited for
//! the wave bookkeeping commit to land its one-line mount for the same reason. A panel that is
//! built, registered and tested but never mounted is a mechanism that cannot fire — the mount is
//! on the human checklist, not silently dropped.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::Value;

use website_map_engine::data::scenario::tasks::validate_schedule;
use website_map_engine::data::scenario::tasks::STATES;
use website_map_engine::data::scenario::tasks::TIERS;

#[cfg(target_arch = "wasm32")]
use super::env::read_flow_seconds;
use super::env::FLOW_DEFAULT_TIMELIMIT_S;
use website_map_engine::editing::hosted_commands as engine_ops;

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
    crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value("tasks")
        .filter(|v| v.is_array())
}

#[cfg(target_arch = "wasm32")]
fn commit(tasks: Option<&[Value]>) {
    if let Some(rows) = tasks {
        let value = Value::Array(rows.to_vec());
        if let Err(clause) = website_map_engine::data::scenario::tasks::validate(&value) {
            leptos::logging::warn!("tasks is not yet complete: {clause}");
        }
    }
    crate::v2::apps::editor::bridge::host_state::editor_context::update_environment(env_patch(
        tasks,
    ));
}

#[cfg(target_arch = "wasm32")]
fn trigger_options() -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = engine_ops::trigger_rows()
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
    website_map_engine::editing::hosted_commands::marker_rows()
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
mod view;
pub use view::tasks_panel;

#[cfg(test)]
#[path = "tests/tasks_panel/task_authoring.rs"]
mod tests;
