//! T-936.3 — the **Radio nets** panel: an authored `radioPlan` (id, label, freqMHz, faction
//! assignment, range), every write one undo step, and a Reset-to-derived action.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.radioPlan` through [`operations::update_environment`],
//! the same one-patch-one-undo-step path the T-224 flow controls and the T-936.1/.2 cards use.
//! `map_engine_core::mission::extensions` copies the key onto the compiled payload root;
//! flatten honours the authored nets when present and derives only when this key is absent.
//!
//! ══ Where this renders ══════════════════════════════════════════════════════════════════════
//! [`radio_panel`] is a section in the same shape as `win_conditions_card` / `tasks_panel`. It
//! belongs in the Mission Settings dialog (`settings_modal.rs`'s `{radio_panel(ctrl)}`). This
//! slice does not own `settings_modal.rs` (T-946.33 is the later mount), same as T-936.1/.2.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::Value;

use map_engine_core::mission::radio_plan::{
    freq_key, validate, FREQ_MAX_MHZ, FREQ_MIN_MHZ, MAX_NETS, RANGES,
};

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

/// Next free allocation on the T-203 0.5 MHz grid, starting at the schema floor.
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
            if trimmed.chars().count() > map_engine_core::mission::radio_plan::MAX_LABEL_CHARS {
                return Err(format!(
                    "label is longer than {} characters — the mod would truncate it",
                    map_engine_core::mission::radio_plan::MAX_LABEL_CHARS
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
    crate::editor::state::operations::read_env_value("radioPlan").filter(|v| v.is_object())
}

#[cfg(target_arch = "wasm32")]
fn commit(plan: Option<&Value>) {
    if let Some(block) = plan {
        if let Err(clause) = validate(block) {
            leptos::logging::warn!("radioPlan is not yet complete: {clause}");
        }
    }
    crate::editor::state::operations::update_environment(env_patch(plan));
}

/// The **Radio nets** panel. `ctrl` is the dialog's shared control class.
///
/// Inert on the native view shell (no document), like every sibling panel.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn radio_panel(ctrl: &'static str) -> AnyView {
    let _ = ctrl;
    ().into_any()
}

/// The **Radio nets** panel — see the native sibling for the signature contract.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn radio_panel(ctrl: &'static str) -> AnyView {
    let sect = "text-label-sm uppercase tracking-wider text-outline";
    let hint = "text-label-sm normal-case text-outline";

    let rows = nets_from_block(read_block().as_ref());
    let refusal = RwSignal::new(String::new());
    let authored = !rows.is_empty();

    let list = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let id = row.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let label = row
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let freq = row
                .get("freqMHz")
                .and_then(Value::as_f64)
                .map(|f| format!("{f}"))
                .unwrap_or_default();
            let faction = row
                .get("faction")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let range = row
                .get("range")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let rows_for_id = rows.clone();
            let rows_for_label = rows.clone();
            let rows_for_freq = rows.clone();
            let rows_for_faction = rows.clone();
            let rows_for_range = rows.clone();
            let rows_for_up = rows.clone();
            let rows_for_down = rows.clone();
            let rows_for_del = rows.clone();

            view! {
                <div class="flex flex-col gap-2 border border-outline-variant/30 p-2">
                    <div class="flex items-center gap-2">
                        <input
                            type="text"
                            prop:value=label.clone()
                            class=ctrl
                            aria-label="Net label"
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_label, index, "label", &event_target_value(&ev)) {
                                    Ok(next) => commit(plan_from_nets(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                commit(plan_from_nets(&move_net(&rows_for_up, index, -1)).as_ref());
                            }
                        >
                            "Up"
                        </button>
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                commit(plan_from_nets(&move_net(&rows_for_down, index, 1)).as_ref());
                            }
                        >
                            "Down"
                        </button>
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                commit(plan_from_nets(&remove_net(&rows_for_del, index)).as_ref());
                            }
                        >
                            "Remove"
                        </button>
                    </div>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Id"</span>
                        <input
                            type="text"
                            prop:value=id.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_id, index, "id", &event_target_value(&ev)) {
                                    Ok(next) => commit(plan_from_nets(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Frequency (MHz)"</span>
                        <input
                            type="number"
                            min=FREQ_MIN_MHZ.to_string()
                            max=FREQ_MAX_MHZ.to_string()
                            step="0.5"
                            prop:value=freq.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_freq, index, "freqMHz", &event_target_value(&ev)) {
                                    Ok(next) => commit(plan_from_nets(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Faction assignment"</span>
                        <input
                            type="text"
                            prop:value=faction.clone()
                            class=ctrl
                            placeholder="blufor — blank is unscoped"
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_faction, index, "faction", &event_target_value(&ev)) {
                                    Ok(next) => commit(plan_from_nets(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Range"</span>
                        <select
                            prop:value=range.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_range, index, "range", &event_target_value(&ev)) {
                                    Ok(next) => commit(plan_from_nets(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        >
                            <option value="">"Default (handheld)"</option>
                            {RANGES
                                .iter()
                                .map(|r| view! { <option value=*r>{range_label(r)}</option> })
                                .collect::<Vec<_>>()}
                        </select>
                    </label>
                </div>
            }
        })
        .collect::<Vec<_>>();

    let rows_for_add = rows.clone();

    view! {
        <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
            <span class=sect>"Radio nets"</span>
            <span class=hint>
                {if authored {
                    "This mission authors its own nets and frequencies. Reset restores the derived plan (base + 0.5 MHz × index)."
                } else {
                    "No authored plan — compile derives one net per side (command, long) plus one per squad. Add a net to author frequencies."
                }}
            </span>
            {list}
            <div class="flex items-center gap-2">
                <button
                    type="button"
                    class="text-label-sm"
                    on:click=move |_| {
                        refusal.set(String::new());
                        match add_net(&rows_for_add) {
                            Ok(next) => commit(plan_from_nets(&next).as_ref()),
                            Err(err) => refusal.set(err),
                        }
                    }
                >
                    "Add net"
                </button>
                <button
                    type="button"
                    class="text-label-sm"
                    on:click=move |_| {
                        refusal.set(String::new());
                        commit(None);
                    }
                >
                    "Reset to derived"
                </button>
            </div>
            <p class="text-label-sm text-error">{move || refusal.get()}</p>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cmd() -> Value {
        json!({
            "id": "net:blufor_cmd",
            "label": "Command",
            "freqMHz": 30.0,
            "faction": "blufor",
            "range": "long"
        })
    }

    #[test]
    fn add_appends_a_unique_id_and_frequency() {
        let next = add_net(&[cmd()]).expect("add");
        assert_eq!(next.len(), 2);
        assert_eq!(next[1]["id"], "net:ch_2");
        assert_eq!(next[1]["freqMHz"], 30.5);
        validate(&plan_from_nets(&next).expect("plan"))
            .expect("the panel must not author a refused block");
    }

    #[test]
    fn remove_drops_one_row_and_clearing_the_last_writes_null() {
        let next = remove_net(&[cmd()], 0);
        assert!(next.is_empty());
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"radioPlan": null}));
    }

    #[test]
    fn a_duplicate_frequency_is_refused_in_the_panel() {
        let rows = vec![
            cmd(),
            json!({"id": "net:blufor_alpha", "label": "Alpha", "freqMHz": 31.0}),
        ];
        let err = with_field(&rows, 1, "freqMHz", "30").expect_err("clash");
        assert!(err.contains("already used"), "{err}");
        assert!(err.contains("30"), "{err}");
    }

    #[test]
    fn an_out_of_range_frequency_is_refused_in_the_panel() {
        let err = with_field(&[cmd()], 0, "freqMHz", "20").expect_err("low");
        assert!(err.contains("outside"), "{err}");
        let err = with_field(&[cmd()], 0, "freqMHz", "900").expect_err("high");
        assert!(err.contains("outside"), "{err}");
    }

    #[test]
    fn reset_to_derived_writes_an_explicit_null_patch() {
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"radioPlan": null}));
        let set: Value =
            serde_json::from_str(&env_patch(plan_from_nets(&[cmd()]).as_ref())).expect("json");
        assert_eq!(set["radioPlan"]["nets"][0]["id"], "net:blufor_cmd");
    }

    #[test]
    fn a_full_authoring_pass_produces_a_block_the_compile_accepts() {
        let mut rows = add_net(&[]).expect("first");
        rows = with_field(&rows, 0, "label", "Command").expect("label");
        rows = with_field(&rows, 0, "faction", "blufor").expect("faction");
        rows = with_field(&rows, 0, "range", "long").expect("range");
        rows = add_net(&rows).expect("second");
        rows = with_field(&rows, 1, "label", "Alpha").expect("alpha");
        rows = with_field(&rows, 1, "faction", "blufor").expect("fac2");
        validate(&plan_from_nets(&rows).expect("plan")).expect("valid");
    }

    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = RADIO_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in RADIO_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }

    #[test]
    fn the_pickers_offer_exactly_the_schema_vocabulary() {
        assert_eq!(RANGES, ["short", "long"]);
        for r in RANGES {
            assert_ne!(range_label(r), "Unknown range");
        }
    }
}
