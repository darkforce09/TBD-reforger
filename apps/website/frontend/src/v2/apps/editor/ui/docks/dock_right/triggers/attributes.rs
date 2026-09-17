//! Right dock triggers behavior.

use super::*;

/// Render the selected trigger's fields, owner link choice, rules, and reshape actions.
#[cfg(target_arch = "wasm32")]
pub(super) fn trigger_attributes(
    t: engine_ops::TriggerRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::{
        humanize_token, DrawTarget, ZoneShape,
    };

    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let tid = t.id.clone();
    let input_class = "mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60";
    let field_label =
        "mt-2 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant";

    let (id_name, id_activation, id_owner, id_delete) =
        (tid.clone(), tid.clone(), tid.clone(), tid.clone());
    let rules = t.rules.clone();
    let owner_opts = engine_ops::placed_owner_options();
    let current_owner = t.owner_id.clone();
    let current_owner_dangling = current_owner
        .as_ref()
        .is_some_and(|o| !owner_opts.iter().any(|opt| &opt.id == o));

    view! {
        <div class="mt-3 border-t border-white/10 pt-2">
            <h4 class="text-label-md font-semibold text-on-surface">
                {format!("Attributes — {}", t.id)}
            </h4>

            <label class=field_label>"Name"</label>
            <input
                type="text"
                aria-label="Trigger name"
                placeholder="(unnamed)"
                class=input_class
                prop:value=t.name.clone().unwrap_or_default()
                on:change=move |ev| {
                    let v = event_target_value(&ev);
                    let next = (!v.trim().is_empty()).then_some(v);
                    engine_ops::set_trigger_name(&id_name, next);
                    bump();
                }
            />

            <label class=field_label>"Activation"</label>
            <select
                aria-label="Trigger activation"
                class=input_class
                on:change=move |ev| {
                    engine_ops::set_trigger_activation(&id_activation, &event_target_value(&ev));
                    bump();
                }
            >
                {
                    let current = t.activation.clone();
                    engine_ops::TRIGGER_ACTIVATIONS
                        .iter()
                        .map(|a| {
                            let a = (*a).to_string();
                            let is = a == current;
                            let label = humanize_token(&a);
                            view! { <option value=a selected=is>{label}</option> }
                        })
                        .collect_view()
                }
            </select>

            <label class=field_label>"Owner"</label>
            <select
                aria-label="Trigger owner"
                class=input_class
                on:change=move |ev| {
                    let v = event_target_value(&ev);
                    let next = (!v.is_empty()).then_some(v);
                    engine_ops::set_trigger_owner(&id_owner, next);
                    bump();
                }
            >
                <option value="" selected=current_owner.is_none()>
                    "(unowned)"
                </option>
                {current_owner_dangling
                    .then(|| {
                        let o = current_owner.clone().unwrap_or_default();
                        view! {
                            <option value=o.clone() selected=true>
                                {format!("{o} (deleted)")}
                            </option>
                        }
                    })}
                {
                    let current_owner = current_owner.clone();
                    owner_opts
                        .into_iter()
                        .map(|opt| {
                            let is = current_owner.as_deref() == Some(opt.id.as_str());
                            view! { <option value=opt.id selected=is>{opt.label}</option> }
                        })
                        .collect_view()
                }
            </select>

            <label class=field_label>"Shape"</label>
            <div class="flex gap-1.5">
                {
                    let (a, b) = (tid.clone(), tid.clone());
                    view! {
                        <button
                            type="button"
                            title="Redraw this trigger as a circle — click the centre, then the rim"
                            class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                armed_placement::begin_zone_reshape(&a, ZoneShape::Circle, DrawTarget::Trigger);
                                bump();
                            }
                        >
                            "Redraw circle"
                        </button>
                        <button
                            type="button"
                            title="Redraw this trigger as a polygon — click each vertex, then Close"
                            class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                armed_placement::begin_zone_reshape(&b, ZoneShape::Polygon, DrawTarget::Trigger);
                                bump();
                            }
                        >
                            "Redraw polygon"
                        </button>
                    }
                }
            </div>

            <h4 class="mt-3 text-label-md font-semibold text-on-surface">"Rules"</h4>
            <p class="mt-0.5 text-label-sm normal-case text-outline">
                "Reuses the mission schema's zoneRules vocabulary — the same controls the Zones panel draws. Blank means the key is not authored and the mod's default applies."
            </p>
            {crate::v2::apps::editor::ui::inspector::zones_panel::zone_rule_fields()
                .into_iter()
                .map(|f| trigger_rule_control(tid.clone(), f, rules.clone(), doc_tick))
                .collect_view()}

            <button
                type="button"
                class="mt-3 w-full rounded-md border border-error/40 px-2 py-1.5 text-label-sm text-error transition-colors hover:bg-error/15"
                on:click=move |_| {
                    engine_ops::delete_trigger(&id_delete);
                    selected.set(None);
                    bump();
                }
            >
                "Delete trigger"
            </button>
        </div>
    }
    .into_any()
}

/// ONE `$defs/zoneRules` property as a control for a TRIGGER, writing through
/// `set_trigger_rule`. Reuses `eden_zones`'s vocabulary machinery ([`ZoneRuleField`] /
/// [`ZoneRuleKind`], read from the schema by `zone_rule_fields`) — the load-bearing "no second
/// vocabulary" reuse — and mirrors `eden_zones::zone_rule_control`'s rendering, differing only in the
/// mutator it calls. Clearing a control removes the key (the mod's default returns), exactly as the
/// zone control does.
#[cfg(target_arch = "wasm32")]
pub(super) fn trigger_rule_control(
    trigger_id: String,
    f: crate::v2::apps::editor::ui::inspector::zones_panel::ZoneRuleField,
    rules: serde_json::Value,
    doc_tick: RwSignal<u64>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::{
        humanize_key, humanize_token, ZoneRuleKind,
    };

    let current = rules.get(&f.key).cloned();
    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let label = humanize_key(&f.key);
    let doc = f.doc.clone();
    let key = f.key.clone();
    let row = "mt-2";
    let ctl = "mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface outline-none focus:border-primary/60";

    let body = match f.kind {
        ZoneRuleKind::Bool { default } => {
            let checked = current.as_ref().and_then(serde_json::Value::as_bool);
            let k = key.clone();
            view! {
                <label class="mt-2 flex items-center gap-2 text-label-sm text-on-surface">
                    <input
                        type="checkbox"
                        aria-label=label.clone()
                        prop:checked=checked.unwrap_or(default)
                        prop:indeterminate=checked.is_none()
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            engine_ops::set_trigger_rule(&trigger_id, &k, Some(serde_json::Value::Bool(on)));
                            bump();
                        }
                    />
                    <span>{label.clone()}</span>
                    <span class="ml-auto font-mono text-code-md text-outline">
                        {format!("default {default}")}
                    </span>
                </label>
            }
            .into_any()
        }
        ZoneRuleKind::Choice { options, default } => {
            let cur = current
                .as_ref()
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
            let k = key.clone();
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <select
                        aria-label=label.clone()
                        class=ctl
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let next = (!v.is_empty()).then(|| serde_json::Value::String(v));
                            engine_ops::set_trigger_rule(&trigger_id, &k, next);
                            bump();
                        }
                    >
                        <option value="" selected=cur.is_none()>
                            {default
                                .as_ref()
                                .map_or_else(
                                    || "(not authored)".to_string(),
                                    |d| format!("(not authored — default {d})"),
                                )}
                        </option>
                        {options
                            .into_iter()
                            .map(|o| {
                                let is = cur.as_deref() == Some(o.as_str());
                                let l = humanize_token(&o);
                                view! { <option value=o selected=is>{l}</option> }
                            })
                            .collect_view()}
                    </select>
                </div>
            }
            .into_any()
        }
        ZoneRuleKind::Number {
            default,
            minimum,
            exclusive_minimum,
            maximum,
            integer,
        } => {
            let cur = current.as_ref().and_then(serde_json::Value::as_f64);
            let k = key.clone();
            let step = if integer { 1.0 } else { 0.1 };
            let min_attr = minimum.or_else(|| exclusive_minimum.map(|m| m + step));
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <input
                        type="number"
                        aria-label=label.clone()
                        class=ctl
                        step=step
                        min=min_attr.map(|m| m.to_string())
                        max=maximum.map(|m| m.to_string())
                        placeholder=default
                            .map_or_else(
                                || "(not authored)".to_string(),
                                |d| format!("(not authored — default {d})"),
                            )
                        prop:value=cur.map(|v| v.to_string()).unwrap_or_default()
                        on:change=move |ev| {
                            let raw = event_target_value(&ev);
                            let next = if raw.trim().is_empty() {
                                None
                            } else {
                                raw.trim()
                                    .parse::<f64>()
                                    .ok()
                                    .and_then(serde_json::Number::from_f64)
                                    .map(serde_json::Value::Number)
                            };
                            if next.is_some() || raw.trim().is_empty() {
                                engine_ops::set_trigger_rule(&trigger_id, &k, next);
                                bump();
                            }
                        }
                    />
                </div>
            }
            .into_any()
        }
        ZoneRuleKind::Text { default, pattern } => {
            let cur = current
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let k = key.clone();
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <input
                        type="text"
                        aria-label=label.clone()
                        class=ctl
                        pattern=pattern
                        placeholder=default.unwrap_or_else(|| "(not authored)".to_string())
                        prop:value=cur
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let next = (!v.trim().is_empty())
                                .then(|| serde_json::Value::String(v.trim().to_string()));
                            engine_ops::set_trigger_rule(&trigger_id, &k, next);
                            bump();
                        }
                    />
                </div>
            }
            .into_any()
        }
    };
    view! {
        <div title=doc>{body}</div>
    }
    .into_any()
}
