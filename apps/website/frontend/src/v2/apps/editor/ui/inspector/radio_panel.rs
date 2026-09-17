//! the **Radio nets** panel: an authored `radioPlan` (id, label, freqMHz, faction
//! assignment, range), every write one undo step, and a Reset-to-derived action.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.radioPlan` through [`operations::update_environment`],
//! the same one-patch-one-undo-step path the  flow controls and the /.2 cards use.
//! `map_engine_core::mission::extensions` copies the key onto the compiled payload root;
//! flatten honours the authored nets when present and derives only when this key is absent.
//!
//! ══ Where this renders ══════════════════════════════════════════════════════════════════════
//! [`radio_panel`] is a section in the same shape as `win_conditions_card` / `tasks_panel`. It
//! belongs in the Mission Settings dialog (`settings_modal.rs`'s `{radio_panel(ctrl)}`). This
//! slice does not own `settings_modal.rs` ( is the later mount), same as /.2.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::Value;

use website_map_engine::data::scenario::radio_plan::freq_key;
use website_map_engine::data::scenario::radio_plan::validate;
use website_map_engine::data::scenario::radio_plan::FREQ_MAX_MHZ;
use website_map_engine::data::scenario::radio_plan::FREQ_MIN_MHZ;
use website_map_engine::data::scenario::radio_plan::MAX_NETS;
use website_map_engine::data::scenario::radio_plan::RANGES;

/// The reader chain for `meta.environment.radioPlan`, end to end.
pub const RADIO_READERS: &[(&str, &str)] = &[
    (
        "compile",
        "map_engine_core::mission::compile::compile_payload → the saved payload's top-level \
         `radioPlan` (via mission::extensions::copy_authored_blocks)",
    ),
    (
        "flatten",
        "map_engine_core::mission::flatten::resolve_radio_plan → the compiled document's \
         `radioPlan` block (authored nets unchanged, or T-203 derivation when absent)",
    ),
    (
        "mod",
        "TBD_RadioPlan.Parse (first 32 nets in document order; Fault rejects out-of-band \
         frequencies)",
    ),
    (
        "editor",
        "this panel, which reads the key back on every open through operations::read_env_value",
    ),
];

/// The words a range value gets in the picker.
#[must_use]
pub fn range_label(range: &str) -> &'static str {
    match range {
        "short" => "Handheld (short)",
        "long" => "Backpack (long)",
        _ => "Unknown range",
    }
}

/// A new net the Add button authors. `id` and `freqMHz` are unique against `existing`.
#[must_use]
pub fn default_net(existing: &[Value]) -> Value {
    serde_json::json!({
        "id": next_net_id(existing),
        "label": "Net",
        "freqMHz": next_freq_mhz(existing),
    })
}

/// `net:ch_1`, `net:ch_2`, … skipping ids already in the list.
#[must_use]
pub fn next_net_id(existing: &[Value]) -> String {
    let mut n = existing.len() + 1;
    loop {
        let candidate = format!("net:ch_{n}");
        let taken = existing
            .iter()
            .any(|t| t.get("id").and_then(Value::as_str) == Some(&candidate));
        if !taken {
            return candidate;
        }
        n += 1;
    }
}

/// Next free allocation on the  0.5 MHz grid, starting at the schema floor.
#[must_use]
pub fn next_freq_mhz(existing: &[Value]) -> f64 {
    let used: Vec<i64> = existing
        .iter()
        .filter_map(|n| n.get("freqMHz").and_then(Value::as_f64).map(freq_key))
        .collect();
    let mut i = 0u32;
    loop {
        let freq = FREQ_MIN_MHZ + 0.5 * f64::from(i);
        if freq > FREQ_MAX_MHZ {
            return FREQ_MIN_MHZ;
        }
        if !used.contains(&freq_key(freq)) {
            return freq;
        }
        i += 1;
    }
}

/// The authored `nets[]` as the document holds them, or empty when the mission authors no plan.
#[must_use]
pub fn nets_from_block(block: Option<&Value>) -> Vec<Value> {
    block
        .and_then(|b| b.get("nets"))
        .and_then(Value::as_array)
        .map(|a| a.clone())
        .unwrap_or_default()
}

/// The `{nets: [...]}` object the compile validates, or `None` when the list is empty
/// (absence, not an empty array — schema `minItems: 1`).
#[must_use]
pub fn plan_from_nets(nets: &[Value]) -> Option<Value> {
    if nets.is_empty() {
        None
    } else {
        Some(serde_json::json!({ "nets": nets }))
    }
}

/// Append one default net. Refuses when the cap would be exceeded.
///
/// # Errors
/// Returns a sentence when [`MAX_NETS`] is already filled.
pub fn add_net(existing: &[Value]) -> Result<Vec<Value>, String> {
    if existing.len() >= MAX_NETS {
        return Err(format!(
            "a radio plan can hold at most {MAX_NETS} nets — the mod would drop the rest"
        ));
    }
    let mut next = existing.to_vec();
    next.push(default_net(existing));
    Ok(next)
}

/// Remove the row at `index`, or return the list unchanged when the index is out of range.
#[must_use]
pub fn remove_net(existing: &[Value], index: usize) -> Vec<Value> {
    let mut next = existing.to_vec();
    if index < next.len() {
        next.remove(index);
    }
    next
}

/// Move the row at `index` by `delta` (−1 up, +1 down). Out-of-range is a no-op.
#[must_use]
pub fn move_net(existing: &[Value], index: usize, delta: i32) -> Vec<Value> {
    let mut next = existing.to_vec();
    let Some(new_index) = index.checked_add_signed(delta as isize) else {
        return next;
    };
    if index >= next.len() || new_index >= next.len() {
        return next;
    }
    next.swap(index, new_index);
    next
}

/// Set one authored field on the net at `index`. Duplicate frequencies and out-of-range
/// values are refused here — that is the panel message the ticket asks for.
///
/// # Errors
/// Returns a sentence the panel shows when the value is not one this compile would carry.
pub fn with_field(
    existing: &[Value],
    index: usize,
    key: &str,
    value: &str,
) -> Result<Vec<Value>, String> {
    if !EDITABLE_KEYS.contains(&key) {
        return Err(format!("{key:?} is not an authored net field"));
    }
    let Some(row) = existing.get(index).and_then(Value::as_object) else {
        return Err("that net is no longer in the list".to_string());
    };

    let mut obj = row.clone();
    let trimmed = value.trim();

    match key {
        "id" => {
            if trimmed.is_empty() {
                return Err("id cannot be blank".into());
            }
            if !trimmed.starts_with("net:")
                || trimmed.len() <= 4
                || !trimmed[4..]
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                return Err(format!(
                    "id {trimmed:?} is not a net id (`net:` then lowercase letters, digits, underscores)"
                ));
            }
            let clash = existing
                .iter()
                .enumerate()
                .any(|(i, t)| i != index && t.get("id").and_then(Value::as_str) == Some(trimmed));
            if clash {
                return Err(format!("id {trimmed:?} is already used by another net"));
            }
            obj.insert(key.to_string(), Value::String(trimmed.to_string()));
        }
        "label" => {
            if trimmed.is_empty() {
                return Err("label cannot be blank".into());
            }
            if trimmed.chars().count()
                > website_map_engine::data::scenario::radio_plan::MAX_LABEL_CHARS
            {
                return Err(format!(
                    "label is longer than {} characters — the mod would truncate it",
                    website_map_engine::data::scenario::radio_plan::MAX_LABEL_CHARS
                ));
            }
            obj.insert(key.to_string(), Value::String(trimmed.to_string()));
        }
        "freqMHz" => {
            let freq: f64 = trimmed
                .parse()
                .map_err(|_| format!("frequency {trimmed:?} is not a number"))?;
            if !freq.is_finite() || !(FREQ_MIN_MHZ..=FREQ_MAX_MHZ).contains(&freq) {
                return Err(format!(
                    "frequency {freq} is outside {FREQ_MIN_MHZ}–{FREQ_MAX_MHZ} MHz"
                ));
            }
            let key_hz = freq_key(freq);
            let clash = existing.iter().enumerate().find(|(i, t)| {
                *i != index
                    && t.get("freqMHz")
                        .and_then(Value::as_f64)
                        .is_some_and(|f| freq_key(f) == key_hz)
            });
            if let Some((other, n)) = clash {
                let other_id = n.get("id").and_then(Value::as_str).unwrap_or("?");
                return Err(format!(
                    "frequency {freq} is already used by nets[{other}] ({other_id}) — two \
                     channels on one frequency would hear each other"
                ));
            }
            obj.insert(
                key.to_string(),
                serde_json::Number::from_f64(freq)
                    .map(Value::Number)
                    .unwrap_or(json_f64(freq)),
            );
        }
        "faction" => {
            if trimmed.is_empty() {
                obj.remove(key);
            } else {
                let mut chars = trimmed.chars();
                let ok = match chars.next() {
                    Some(c) if c.is_ascii_lowercase() => {
                        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                    }
                    _ => false,
                };
                if !ok {
                    return Err(format!(
                        "faction {trimmed:?} is not a faction key (lowercase, starting with a letter)"
                    ));
                }
                obj.insert(key.to_string(), Value::String(trimmed.to_string()));
            }
        }
        "range" => {
            if trimmed.is_empty() {
                obj.remove(key);
            } else if !RANGES.contains(&trimmed) {
                return Err(format!(
                    "range {trimmed:?} is not one of {}",
                    RANGES.join(", ")
                ));
            } else {
                obj.insert(key.to_string(), Value::String(trimmed.to_string()));
            }
        }
        _ => unreachable!("EDITABLE_KEYS filtered"),
    }

    let mut next = existing.to_vec();
    next[index] = Value::Object(obj);
    if let Some(plan) = plan_from_nets(&next) {
        validate(&plan)?;
    }
    Ok(next)
}

fn json_f64(freq: f64) -> Value {
    Value::from(freq)
}

const EDITABLE_KEYS: &[&str] = &["id", "label", "freqMHz", "faction", "range"];

/// The `meta.environment` merge patch for a plan, or for CLEARING one (Reset-to-derived).
///
/// `None` / empty writes an explicit `null` rather than omitting the key, because
/// `update_environment` MERGES: an omitted key leaves the previous value in place.
#[must_use]
pub fn env_patch(plan: Option<&Value>) -> String {
    match plan {
        None => serde_json::json!({ "radioPlan": null }).to_string(),
        Some(block) => serde_json::json!({ "radioPlan": block }).to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_block() -> Option<Value> {
    crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value("radioPlan")
        .filter(|v| v.is_object())
}

#[cfg(target_arch = "wasm32")]
fn commit(plan: Option<&Value>) {
    if let Some(block) = plan {
        if let Err(clause) = validate(block) {
            leptos::logging::warn!("radioPlan is not yet complete: {clause}");
        }
    }
    crate::v2::apps::editor::bridge::host_state::editor_context::update_environment(env_patch(
        plan,
    ));
}

/// The **Radio nets** panel. `ctrl` is the dialog's shared control class.
///
/// Inert on the native view shell (no document), like every sibling panel.
mod view;
pub use view::radio_panel;

#[cfg(test)]
#[path = "tests/radio_panel/radio_network_authoring.rs"]
mod tests;
