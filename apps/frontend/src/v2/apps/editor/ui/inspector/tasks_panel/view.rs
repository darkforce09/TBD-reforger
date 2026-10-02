//! Tasks panel inspector view.

use super::*;

/// Native placeholder for the Tasks inspector panel.
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
