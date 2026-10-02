//! Attributes modal field inputs behavior.

use super::*;

/// Selects the finest active keyboard modifier step.
pub(super) fn nudge_step(ctrl: bool, shift: bool, alt: bool) -> f64 {
    if ctrl {
        0.1
    } else if shift {
        10.0
    } else if alt {
        100.0
    } else {
        1.0
    }
}

/// Calculates a finite numeric nudge from a truthful base value.
pub(super) fn nudged(from: Option<f64>, up: bool, step: f64) -> Option<f64> {
    let from = from.filter(|v| v.is_finite())?;
    let raw = from + if up { step } else { -step };
    let quantised = (raw * 1000.0).round() / 1000.0;
    quantised.is_finite().then_some(quantised)
}

/// Formats an unfocused numeric value for presentation.
pub(super) fn field_display(value: f64) -> String {
    if !value.is_finite() {
        return format!("{value}");
    }
    let s = format!("{value:.3}");
    let t = s.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" {
        "0".to_string()
    } else {
        t.to_string()
    }
}

/// Accepts only finite changed values or deliberate multi-edit stamps.
pub(super) fn should_commit(differs: bool, n: f64, settled: f64) -> bool {
    n.is_finite() && (differs || n != settled)
}

/// Renders a numeric draft that commits on blur or Enter.
#[cfg(target_arch = "wasm32")]
pub(super) fn number_field(
    label: &'static str,
    value: f64,
    suffix: Option<&'static str>,
    gate: Gate,
    on_commit: impl Fn(f64) + Copy + 'static,
) -> impl IntoView {
    let draft = RwSignal::new(String::new());
    let focused = RwSignal::new(false);
    let shown = StoredValue::new(field_display(value));
    let exact = StoredValue::new(format!("{value}"));
    let display = move || {
        if gate.differs() {
            String::new()
        } else {
            shown.get_value()
        }
    };
    let seed = move || {
        if gate.differs() {
            String::new()
        } else {
            exact.get_value()
        }
    };
    let commit = move || {
        focused.set(false);
        if let Ok(n) = draft.get_untracked().parse::<f64>() {
            if should_commit(gate.differs(), n, value) {
                on_commit(n);
            }
        }
    };
    view! {
        <div class="flex flex-col gap-1">
            {field_label(label, gate)}
            <div class="relative">
                <input
                    type="number"
                    step="any"
                    aria-label=label
                    disabled=move || gate.locked()
                    placeholder=if gate.differs() { "—" } else { "" }
                    prop:value=move || { if focused.get() { draft.get() } else { display() } }
                    on:focus=move |_| {
                        draft.set(seed());
                        focused.set(true);
                    }
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:blur=move |_| commit()
                    on:keydown=move |ev| {
                        let key = ev.key();
                        if key == "Escape" {
                            ev.stop_propagation();
                            focused.set(false);
                            draft.set(seed());
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                            return;
                        }
                        if key == "Enter" {
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                            return;
                        }
                        let up = match key.as_str() {
                            "PageUp" | "ArrowUp" => true,
                            "PageDown" | "ArrowDown" => false,
                            _ => return,
                        };
                        ev.prevent_default();
                        if gate.locked_now() {
                            return;
                        }
                        let Some(next) = nudged(
                            draft.get_untracked().parse::<f64>().ok(),
                            up,
                            nudge_step(ev.ctrl_key(), ev.shift_key(), ev.alt_key()),
                        ) else {
                            return;
                        };
                        draft.set(format!("{next}"));
                    }
                    class=move || {
                        let pad = if suffix.is_some() { " pr-7" } else { "" };
                        let lock = if gate.locked() { CONTROL_LOCKED } else { "" };
                        format!("{CONTROL} font-mono{pad}{lock}")
                    }
                />
                {suffix
                    .map(|s| {
                        view! {
                            <span class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 font-mono text-label-sm text-outline">
                                {s}
                            </span>
                        }
                    })}
            </div>
        </div>
    }
}

/// Renders a text draft that commits on blur or Enter.
#[cfg(target_arch = "wasm32")]
pub(super) fn text_field(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    gate: Gate,
    on_change: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    let draft = RwSignal::new(String::new());
    let focused = RwSignal::new(false);
    let settled = StoredValue::new(value);
    let ph = if gate.differs() {
        "Multiple values"
    } else {
        placeholder
    };
    let text_display = move || {
        if gate.differs() {
            String::new()
        } else {
            settled.get_value()
        }
    };
    let edited = RwSignal::new(false);
    let text_commit = move || {
        focused.set(false);
        if !edited.get_untracked() {
            return;
        }
        let next = draft.get_untracked();
        if gate.differs() || next != settled.get_value() {
            on_change(next);
        }
    };
    view! {
        <div class="flex flex-col gap-1">
            {field_label(label, gate)}
            <input
                type="text"
                aria-label=label
                disabled=move || gate.locked()
                placeholder=ph
                prop:value=move || { if focused.get() { draft.get() } else { text_display() } }
                on:focus=move |_| {
                    draft.set(text_display());
                    edited.set(false);
                    focused.set(true);
                }
                on:input=move |ev| {
                    edited.set(true);
                    draft.set(event_target_value(&ev));
                }
                on:blur=move |_| text_commit()
                on:keydown=move |ev| {
                    match ev.key().as_str() {
                        "Enter" => {
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                        }
                        "Escape" => {
                            ev.stop_propagation();
                            edited.set(false);
                            focused.set(false);
                            draft.set(text_display());
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                        }
                        _ => {}
                    }
                }
                                class=move || {
                    let lock = if gate.locked() { CONTROL_LOCKED } else { "" };
                    format!("{CONTROL}{lock}")
                }
            />
        </div>
    }
}
