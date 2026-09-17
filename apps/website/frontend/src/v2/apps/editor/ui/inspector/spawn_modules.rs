//! the **Spawn modules** panel: waves and garrisons, every write one undo step.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.spawnModules` through [`operations::update_environment`],
//! the same one-patch-one-undo-step path the .5 cards use.
//! `map_engine_core::mission::extensions` copies the key onto the compiled payload root.
//!
//! ══ Where this renders ══════════════════════════════════════════════════════════════════════
//! [`spawn_modules_panel`] belongs in the Mission Settings dialog
//! (`settings_modal.rs`'s `{spawn_modules_panel(ctrl)}`).  registered audio and never
//! mounted it; this slice owns `settings_modal.rs` and mounts this panel there.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::Value;

use website_map_engine::data::scenario::spawn_modules::validate;
use website_map_engine::data::scenario::spawn_modules::FACTION_KEYS;
use website_map_engine::data::scenario::spawn_modules::KINDS;
use website_map_engine::data::scenario::spawn_modules::MAX_ALIVE;

/// The reader chain for `meta.environment.spawnModules`, end to end.
pub const SPAWN_MODULES_READERS: &[(&str, &str)] = &[
    (
        "compile",
        "map_engine_core::mission::compile::compile_payload → the saved payload's top-level \
         `spawnModules` (via mission::extensions::copy_authored_blocks)",
    ),
    (
        "flatten",
        "map_engine_core::mission::flatten::EditorPayload.authored_blocks_root → \
         ExtensionBlocks::from_payload → the compiled document's `spawnModules` block",
    ),
    (
        "mod",
        "TBD_DynamicSpawner (server spawns wave groups on interval or trigger up to maxAlive; \
         garrison spawns once and holds; cleanup on mission end)",
    ),
    (
        "editor",
        "ui/inspector/spawn_modules.rs — this panel, via operations::update_environment",
    ),
];

const EDITABLE_KEYS: &[&str] = &[
    "id",
    "kind",
    "factionKey",
    "groupTemplate",
    "x",
    "z",
    "zoneId",
    "count",
    "intervalSeconds",
    "maxAlive",
    "triggerId",
];

const OPTIONAL_KEYS: &[&str] = &[
    "intervalSeconds",
    "maxAlive",
    "triggerId",
    "zoneId",
    "x",
    "z",
];

/// Human labels for [`KINDS`].
#[must_use]
pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "wave" => "Wave",
        "garrison" => "Garrison",
        _ => "Unknown kind",
    }
}

/// Human labels for [`FACTION_KEYS`].
#[must_use]
pub fn faction_label(key: &str) -> &'static str {
    match key {
        "blufor" => "BLUFOR",
        "opfor" => "OPFOR",
        "indfor" => "INDFOR",
        "civ" => "Civilian",
        _ => "Unknown faction",
    }
}

/// Next `sm-N` id not already in `existing`.
#[must_use]
pub fn next_id(existing: &[Value]) -> String {
    let mut n = existing.len() + 1;
    loop {
        let id = format!("sm-{n}");
        if !existing
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(id.as_str()))
        {
            return id;
        }
        n += 1;
    }
}

/// One default wave at the origin, one group, 60 s interval.
#[must_use]
pub fn default_module(existing: &[Value]) -> Value {
    serde_json::json!({
        "id": next_id(existing),
        "kind": "wave",
        "factionKey": "opfor",
        "groupTemplate": "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et",
        "x": 0.0,
        "z": 0.0,
        "count": 1,
        "intervalSeconds": 60.0,
        "maxAlive": 1
    })
}

/// The modules array, or empty when the block is absent.
#[must_use]
pub fn modules_from_block(block: Option<&Value>) -> Vec<Value> {
    block.and_then(Value::as_array).cloned().unwrap_or_default()
}

/// Wrap modules into the authored array, or `None` when the list is empty (omit, don't store).
#[must_use]
pub fn block_from_modules(rows: &[Value]) -> Option<Value> {
    if rows.is_empty() {
        None
    } else {
        Some(Value::Array(rows.to_vec()))
    }
}

/// Append one default module. One new array, one undo step at commit.
pub fn add_module(existing: &[Value]) -> Result<Vec<Value>, String> {
    let mut next = existing.to_vec();
    next.push(default_module(existing));
    refuse_list(&next)?;
    Ok(next)
}

/// Remove the row at `index`, or return the list unchanged when the index is out of range.
#[must_use]
pub fn remove_module(existing: &[Value], index: usize) -> Vec<Value> {
    let mut next = existing.to_vec();
    if index < next.len() {
        next.remove(index);
    }
    next
}

/// Set one field on the row at `index`. Blank optional keys are removed rather than stored as
/// `""`. Switching to `zoneId` strips `x`/`z`; switching to `x` or `z` strips `zoneId`.
pub fn with_field(
    existing: &[Value],
    index: usize,
    key: &str,
    value: &str,
) -> Result<Vec<Value>, String> {
    if !EDITABLE_KEYS.contains(&key) {
        return Err(format!("{key:?} is not an authored spawn-module field"));
    }
    let Some(row) = existing.get(index).and_then(Value::as_object) else {
        return Err("that spawn module is no longer in the list".to_string());
    };

    let mut obj = row.clone();
    let trimmed = value.trim();

    match key {
        "kind" => {
            if !KINDS.contains(&trimmed) {
                return Err(format!(
                    "kind {trimmed:?} is not one of {}",
                    KINDS.join(", ")
                ));
            }
            obj.insert("kind".into(), Value::String(trimmed.to_string()));
        }
        "factionKey" => {
            if !FACTION_KEYS.contains(&trimmed) {
                return Err(format!(
                    "factionKey {trimmed:?} is not a known faction ({})",
                    FACTION_KEYS.join(", ")
                ));
            }
            obj.insert("factionKey".into(), Value::String(trimmed.to_string()));
        }
        "id" | "groupTemplate" => {
            if trimmed.is_empty() {
                return Err(format!("{key} is required"));
            }
            obj.insert(key.to_string(), Value::String(trimmed.to_string()));
        }
        "count" | "maxAlive" => {
            if trimmed.is_empty() {
                if OPTIONAL_KEYS.contains(&key) {
                    obj.remove(key);
                } else {
                    return Err(format!("{key} is required"));
                }
            } else {
                let n = parse_i64(trimmed, key)?;
                if n <= 0 || n > MAX_ALIVE {
                    return Err(format!("{key} must be in 1..={MAX_ALIVE}"));
                }
                obj.insert(key.to_string(), json_i64(n));
            }
        }
        "intervalSeconds" => {
            if trimmed.is_empty() {
                obj.remove(key);
            } else {
                let n = parse_f64(trimmed, key)?;
                if n <= 0.0 {
                    return Err("intervalSeconds must be above zero".into());
                }
                obj.insert(key.to_string(), json_f64(n));
            }
        }
        "x" | "z" => {
            if trimmed.is_empty() {
                obj.remove(key);
            } else {
                let n = parse_f64(trimmed, key)?;
                obj.insert(key.to_string(), json_f64(n));
                obj.remove("zoneId");
                let other = if key == "x" { "z" } else { "x" };
                if obj.get(other).is_none() {
                    obj.insert(other.to_string(), json_f64(0.0));
                }
            }
        }
        "zoneId" => {
            if trimmed.is_empty() {
                obj.remove(key);
            } else {
                obj.insert("zoneId".into(), Value::String(trimmed.to_string()));
                obj.remove("x");
                obj.remove("z");
            }
        }
        "triggerId" => {
            if trimmed.is_empty() {
                obj.remove(key);
            } else {
                obj.insert("triggerId".into(), Value::String(trimmed.to_string()));
            }
        }
        _ => unreachable!("EDITABLE_KEYS guards the match"),
    }

    let mut next = existing.to_vec();
    next[index] = Value::Object(obj);
    refuse_list(&next)?;
    Ok(next)
}

fn refuse_list(rows: &[Value]) -> Result<(), String> {
    match block_from_modules(rows) {
        None => Ok(()),
        Some(block) => validate(&block),
    }
}

fn parse_i64(raw: &str, key: &str) -> Result<i64, String> {
    raw.parse::<i64>()
        .map_err(|_| format!("{key} must be an integer"))
}

fn parse_f64(raw: &str, key: &str) -> Result<f64, String> {
    let n: f64 = raw.parse().map_err(|_| format!("{key} must be a number"))?;
    if !n.is_finite() {
        return Err(format!("{key} must be a finite number"));
    }
    Ok(n)
}

fn json_i64(n: i64) -> Value {
    Value::Number(n.into())
}

fn json_f64(n: f64) -> Value {
    Value::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()))
}

/// The environment patch string `update_environment` consumes. Empty list → `null` (omit).
#[must_use]
pub fn env_patch(block: Option<&Value>) -> String {
    match block {
        None => serde_json::json!({ "spawnModules": null }).to_string(),
        Some(block) => serde_json::json!({ "spawnModules": block }).to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_block() -> Option<Value> {
    crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value("spawnModules")
        .filter(|v| v.is_array())
}

#[cfg(target_arch = "wasm32")]
fn commit(block: Option<&Value>) {
    if let Some(block) = block {
        if let Err(clause) = validate(block) {
            leptos::logging::warn!("spawnModules is not yet complete: {clause}");
        }
    }
    crate::v2::apps::editor::bridge::host_state::editor_context::update_environment(env_patch(
        block,
    ));
}

/// The **Spawn modules** panel. `ctrl` is the dialog's shared control class.
///
/// Inert on the native view shell (no document), like every sibling panel.
mod view;
pub use view::spawn_modules_panel;

#[cfg(test)]
#[path = "tests/spawn_modules/spawn_module_authoring.rs"]
mod tests;
