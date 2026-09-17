//! Win conditions card inspector view.

use super::*;

/// Native placeholder for the Win conditions inspector panel.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn win_conditions_card(ctrl: &'static str) -> AnyView {
    let _ = ctrl;
    ().into_any()
}

/// The **Win conditions** card — see the native sibling for the signature contract.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn win_conditions_card(ctrl: &'static str) -> AnyView {
    let sect = "text-label-sm uppercase tracking-wider text-outline";
    let hint = "text-label-sm normal-case text-outline";

    let block = read_block();
    let mode = block
        .as_ref()
        .and_then(|b| b.get("mode"))
        .and_then(serde_json::Value::as_str)
        .filter(|m| AUTHORED_MODES.contains(m))
        .unwrap_or("")
        .to_string();

    let refusal = RwSignal::new(String::new());

    let picker = {
        let selected = mode.clone();
        let block_for_pick = block.clone();
        view! {
            <label class="flex flex-col gap-1">
                <span class=sect>"Win rule"</span>
                <select
                    prop:value=selected
                    on:change=move |ev| {
                        refusal.set(String::new());
                        let picked = event_target_value(&ev);
                        if picked.is_empty() {
                            commit(None);
                        } else if AUTHORED_MODES.contains(&picked.as_str()) {
                            commit(Some(&with_mode(block_for_pick.as_ref(), &picked)));
                        } else {
                            leptos::logging::error!(
                                "refusing winConditions.mode = {picked:?}: not an authored mode"
                            );
                        }
                    }
                    class=ctrl
                >
                    {mode_options()
                        .into_iter()
                        .map(|(value, label)| view! { <option value=value>{label}</option> })
                        .collect::<Vec<_>>()}
                </select>
                <span class=hint>
                    "None leaves the mission on the derived attrition rule — the last side with living players wins."
                </span>
            </label>
        }
    };

    if mode.is_empty() {
        return view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Win conditions"</span>
                {picker}
            </div>
        }
        .into_any();
    }

    let param_rows = param_fields(&mode)
        .into_iter()
        .map(|(key, label, hint_text)| {
            let committed = block
                .as_ref()
                .and_then(|b| b.get(key))
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default();
            let mode_for_row = mode.clone();
            let block_for_row = block.clone();
            let numeric = key == "timeoutMinutes";
            view! {
                <label class="flex flex-col gap-1">
                    <span class=sect>{label}</span>
                    <input
                        type=if numeric { "number" } else { "text" }
                        min=if numeric { TIMEOUT_MINUTES_MIN.to_string() } else { String::new() }
                        max=if numeric { TIMEOUT_MINUTES_MAX.to_string() } else { String::new() }
                        step="1"
                        value=committed
                        on:change=move |ev| {
                            let raw = event_target_value(&ev);
                            match with_param(block_for_row.as_ref(), &mode_for_row, key, &raw) {
                                Ok(next) => {
                                    refusal.set(String::new());
                                    commit(Some(&next));
                                }
                                Err(clause) => refusal.set(clause),
                            }
                        }
                        class=ctrl
                    />
                    <span class=hint>{hint_text}</span>
                </label>
            }
        })
        .collect::<Vec<_>>();

    let checked: Vec<String> = block
        .as_ref()
        .and_then(|b| b.get("endOn"))
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let checklist = END_ON_TRIGGERS
        .iter()
        .map(|trigger| {
            let (label, why) = trigger_label(trigger);
            let is_on = checked.iter().any(|t| t == trigger);
            let mode_for_row = mode.clone();
            let block_for_row = block.clone();
            let trigger = *trigger;
            view! {
                <div class="flex flex-col gap-0.5">
                    <label class="flex items-center justify-between py-0.5">
                        <span class="text-label-md text-on-surface-variant">{label}</span>
                        <input
                            type="checkbox"
                            prop:checked=is_on
                            on:change=move |ev| {
                                let on = event_target_checked(&ev);
                                match with_trigger(block_for_row.as_ref(), &mode_for_row, trigger, on) {
                                    Ok(next) => {
                                        refusal.set(String::new());
                                        commit(Some(&next));
                                    }
                                    Err(clause) => {
                                        refusal.set(clause);
                                        if let Some(input) = ev
                                            .target()
                                            .and_then(|t| {
                                                wasm_bindgen::JsCast::dyn_into::<
                                                    web_sys::HtmlInputElement,
                                                >(t)
                                                    .ok()
                                            })
                                        {
                                            input.set_checked(true);
                                        }
                                    }
                                }
                            }
                            class="accent-primary"
                        />
                    </label>
                    <span class=hint>{why}</span>
                </div>
            }
        })
        .collect::<Vec<_>>();

    let incomplete = block
        .as_ref()
        .and_then(|b| website_map_engine::data::scenario::win_conditions::validate(b).err());

    view! {
        <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
            <span class=sect>"Win conditions"</span>
            {picker}
            {param_rows}
            <div class="flex flex-col gap-2">
                <span class=sect>"Ends the round on"</span>
                {checklist}
            </div>
            {move || {
                let r = refusal.get();
                (!r.is_empty()).then(|| view! { <p class="text-label-sm text-error">{r}</p> })
            }}
            {incomplete
                .map(|clause| {
                    view! {
                        <p class="text-label-sm text-error">
                            {format!(
                                "This rule is not complete, so the mission will run on the derived attrition rule: {clause}",
                            )}
                        </p>
                    }
                })}
        </div>
    }
    .into_any()
}
