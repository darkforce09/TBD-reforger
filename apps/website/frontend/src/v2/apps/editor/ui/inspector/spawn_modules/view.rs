//! Spawn modules inspector view.

use super::*;

/// Native placeholder for the Spawn modules inspector panel.
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
