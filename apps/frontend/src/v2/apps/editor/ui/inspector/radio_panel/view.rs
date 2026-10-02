//! Radio panel inspector view.

use super::*;

/// Native placeholder for the Radio nets inspector panel.
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
