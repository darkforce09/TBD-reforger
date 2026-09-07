//! T-936.6 — the **Spawn modules** panel: waves and garrisons, every write one undo step.
//!
//! ══ Transport ═══════════════════════════════════════════════════════════════════════════════
//! Every write goes to `meta.environment.spawnModules` through [`operations::update_environment`],
//! the same one-patch-one-undo-step path the T-936.1–.5 cards use.
//! `map_engine_core::mission::extensions` copies the key onto the compiled payload root.
//!
//! ══ Where this renders ══════════════════════════════════════════════════════════════════════
//! [`spawn_modules_panel`] belongs in the Mission Settings dialog
//! (`settings_modal.rs`'s `{spawn_modules_panel(ctrl)}`). T-936.5 registered audio and never
//! mounted it; this slice owns `settings_modal.rs` and mounts this panel there.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated).
#![allow(dead_code)]
use leptos::prelude::*;
use serde_json::Value;

use map_engine_core::mission::spawn_modules::{validate, FACTION_KEYS, KINDS, MAX_ALIVE};

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
        "panels/spawn_modules.rs — this panel, via operations::update_environment",
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
    crate::editor::state::operations::read_env_value("spawnModules").filter(|v| v.is_array())
}

#[cfg(target_arch = "wasm32")]
fn commit(block: Option<&Value>) {
    if let Some(block) = block {
        if let Err(clause) = validate(block) {
            leptos::logging::warn!("spawnModules is not yet complete: {clause}");
        }
    }
    crate::editor::state::operations::update_environment(env_patch(block));
}

/// The **Spawn modules** panel. `ctrl` is the dialog's shared control class.
///
/// Inert on the native view shell (no document), like every sibling panel.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn spawn_modules_panel(ctrl: &'static str) -> AnyView {
    let _ = ctrl;
    ().into_any()
}

/// The **Spawn modules** panel — see the native sibling for the signature contract.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn spawn_modules_panel(ctrl: &'static str) -> AnyView {
    let sect = "text-label-sm uppercase tracking-wider text-outline";
    let hint = "text-label-sm normal-case text-outline";

    let rows = modules_from_block(read_block().as_ref());
    let refusal = RwSignal::new(String::new());
    let rows_for_add = rows.clone();

    let list = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let id = row
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let kind = row
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("wave")
                .to_string();
            let faction = row
                .get("factionKey")
                .and_then(Value::as_str)
                .unwrap_or("opfor")
                .to_string();
            let template = row
                .get("groupTemplate")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let x = number_display(row.get("x"));
            let z = number_display(row.get("z"));
            let zone = row
                .get("zoneId")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let count = number_display(row.get("count"));
            let interval = number_display(row.get("intervalSeconds"));
            let max_alive = number_display(row.get("maxAlive"));
            let trigger = row
                .get("triggerId")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            let rows_for_kind = rows.clone();
            let rows_for_faction = rows.clone();
            let rows_for_template = rows.clone();
            let rows_for_x = rows.clone();
            let rows_for_z = rows.clone();
            let rows_for_zone = rows.clone();
            let rows_for_count = rows.clone();
            let rows_for_interval = rows.clone();
            let rows_for_max = rows.clone();
            let rows_for_trigger = rows.clone();
            let rows_for_del = rows.clone();

            view! {
                <div class="flex flex-col gap-2 border border-outline-variant/30 p-2">
                    <div class="flex items-center gap-2">
                        <span class=sect>{id}</span>
                        <button
                            type="button"
                            class="text-label-sm"
                            on:click=move |_| {
                                refusal.set(String::new());
                                commit(block_from_modules(&remove_module(&rows_for_del, index)).as_ref());
                            }
                        >
                            "Delete"
                        </button>
                    </div>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Kind"</span>
                        <select
                            prop:value=kind.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_kind,
                                    index,
                                    "kind",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        >
                            {KINDS
                                .iter()
                                .map(|k| view! { <option value=*k>{kind_label(k)}</option> })
                                .collect::<Vec<_>>()}
                        </select>
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Faction"</span>
                        <select
                            prop:value=faction.clone()
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_faction,
                                    index,
                                    "factionKey",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        >
                            {FACTION_KEYS
                                .iter()
                                .map(|k| view! { <option value=*k>{faction_label(k)}</option> })
                                .collect::<Vec<_>>()}
                        </select>
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Group template"</span>
                        <input
                            type="text"
                            prop:value=template
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_template,
                                    index,
                                    "groupTemplate",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"X"</span>
                        <input
                            type="text"
                            prop:value=x
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_x, index, "x", &event_target_value(&ev)) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Z"</span>
                        <input
                            type="text"
                            prop:value=z
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(&rows_for_z, index, "z", &event_target_value(&ev)) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Zone id"</span>
                        <input
                            type="text"
                            prop:value=zone
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_zone,
                                    index,
                                    "zoneId",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Count"</span>
                        <input
                            type="text"
                            prop:value=count
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_count,
                                    index,
                                    "count",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Interval (s)"</span>
                        <input
                            type="text"
                            prop:value=interval
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_interval,
                                    index,
                                    "intervalSeconds",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Max alive"</span>
                        <input
                            type="text"
                            prop:value=max_alive
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_max,
                                    index,
                                    "maxAlive",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
                                    Err(err) => refusal.set(err),
                                }
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class=sect>"Trigger id"</span>
                        <input
                            type="text"
                            prop:value=trigger
                            class=ctrl
                            on:change=move |ev| {
                                refusal.set(String::new());
                                match with_field(
                                    &rows_for_trigger,
                                    index,
                                    "triggerId",
                                    &event_target_value(&ev),
                                ) {
                                    Ok(next) => commit(block_from_modules(&next).as_ref()),
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
        <section class="flex flex-col gap-3">
            <h3 class=sect>"Spawn modules"</h3>
            <p class=hint>
                "Waves restock on an interval up to max alive. Garrisons spawn once and hold. \
                 Placement is x+z or a zone, never both."
            </p>
            <div class="flex flex-col gap-2">{list}</div>
            <button
                type="button"
                class="text-label-sm"
                on:click=move |_| {
                    refusal.set(String::new());
                    match add_module(&rows_for_add) {
                        Ok(next) => commit(block_from_modules(&next).as_ref()),
                        Err(err) => refusal.set(err),
                    }
                }
            >
                "Add module"
            </button>
            <Show when=move || !refusal.get().is_empty()>
                <p class="text-label-sm text-error">{move || refusal.get()}</p>
            </Show>
        </section>
    }
    .into_any()
}

fn number_display(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_engine_core::mission::spawn_modules::placement_is_exclusive;
    use serde_json::json;

    fn wave() -> Value {
        json!({
            "id": "sm-wave",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "Group_Base",
            "x": 1.0,
            "z": 2.0,
            "count": 2,
            "intervalSeconds": 30.0,
            "maxAlive": 4
        })
    }

    #[test]
    fn add_appends_a_valid_default_wave() {
        let next = add_module(&[]).expect("add");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0]["kind"], "wave");
        assert_eq!(next[0]["id"], "sm-1");
        validate(&block_from_modules(&next).expect("block")).expect("valid");
    }

    #[test]
    fn remove_drops_one_row_and_clearing_the_last_writes_null() {
        let next = remove_module(&[wave()], 0);
        assert!(next.is_empty());
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"spawnModules": null}));
    }

    #[test]
    fn both_position_and_zone_are_refused_in_the_panel() {
        let with_zone = with_field(&[wave()], 0, "zoneId", "z1").expect("zone strips xz");
        assert!(with_zone[0].get("x").is_none());
        assert!(with_zone[0].get("z").is_none());
        assert_eq!(with_zone[0]["zoneId"], "z1");
        validate(&block_from_modules(&with_zone).expect("block")).expect("xor");

        let back = with_field(&with_zone, 0, "x", "10").expect("x strips zone");
        assert!(back[0].get("zoneId").is_none());
        assert_eq!(back[0]["x"], 10.0);
        assert_eq!(back[0]["z"], 0.0);
        validate(&block_from_modules(&back).expect("block")).expect("position");
        assert!(!placement_is_exclusive(true, true));
    }

    #[test]
    fn an_unknown_faction_is_refused() {
        let err = with_field(&[wave()], 0, "factionKey", "navy").expect_err("faction");
        assert!(err.contains("navy"), "{err}");
    }

    #[test]
    fn over_cap_max_alive_is_refused() {
        let err = with_field(&[wave()], 0, "maxAlive", "99").expect_err("cap");
        assert!(err.contains("32"), "{err}");
    }

    #[test]
    fn blank_optional_interval_is_stripped() {
        let next = with_field(&[wave()], 0, "intervalSeconds", "  ").expect("blank");
        assert!(next[0].get("intervalSeconds").is_none());
        validate(&block_from_modules(&next).expect("block")).expect("valid");
    }

    #[test]
    fn the_pickers_offer_exactly_the_schema_vocabulary() {
        assert_eq!(KINDS, ["wave", "garrison"]);
        assert_eq!(FACTION_KEYS, ["blufor", "opfor", "indfor", "civ"]);
        for k in KINDS {
            assert_ne!(kind_label(k), "Unknown kind");
        }
        for k in FACTION_KEYS {
            assert_ne!(faction_label(k), "Unknown faction");
        }
    }

    #[test]
    fn env_patch_sets_and_clears() {
        let set: Value =
            serde_json::from_str(&env_patch(block_from_modules(&[wave()]).as_ref())).expect("json");
        assert_eq!(set["spawnModules"][0]["kind"], "wave");
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"spawnModules": null}));
    }

    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = SPAWN_MODULES_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in SPAWN_MODULES_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }
}
