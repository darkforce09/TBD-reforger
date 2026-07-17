//! Attributes modal — the AttributesModal.tsx + RightInspector/fields.tsx port (T-159.26, spec
//! `t159_23_attributes_modal.md`). Opened by dbl-clicking a slot on the map or activating an
//! outliner row (multi-select suppresses — A1); Esc / backdrop / ✕ close. Tabs: **Transform**
//! (X/Y/Z/Rotation NumberFields committing on blur/Enter via `update_slot_position`, plus a Stance
//! select) and **Identity** (Role/Tag TextFields committing per input like the React `TextField`,
//! with a readonly Squad) live; **States** is the React trait stub; **Arsenal** is a disabled stub
//! (A3 — the Forge rides T-159.27). Commits run `editor_ops::attrs_update_*` → `after_local_edit`
//! (rebind + persist + one undo step per commit — A4).
//!
//! The field values re-read from the doc on every `doc_ver` bump, so an undo while the modal is
//! open refreshes the fields — and if the slot itself was undone away, the modal closes.
#![allow(dead_code)]
use leptos::prelude::*;

const CONTROL: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
const TABS: [&str; 4] = ["Transform", "Identity", "States", "Arsenal"];

/// The modal host. Renders nothing while closed (`attrs_open == None`) — V-capture-safe like the
/// suite Dialog. `doc_ver` is the re-read trigger (the doc has no change subscription).
#[component]
pub fn AttributesModal(
    attrs_open: RwSignal<Option<String>>,
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    // Esc closes (React Dialog behavior); the editor's own keydown handler skips editable fields,
    // so this window listener is the one Esc path.
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if attrs_open.get_untracked().is_some() && ev.key() == "Escape" {
                crate::editor_ops::close_attributes();
            }
        });
        on_cleanup(move || esc.remove());
    }
    move || {
        let id = attrs_open.get()?;
        let _ = doc_tick.get(); // re-read fields on every doc change (undo/redo/drag)
        #[cfg(not(target_arch = "wasm32"))]
        let _ = &id;
        #[cfg(target_arch = "wasm32")]
        {
            match crate::editor_ops::read_attrs(&id) {
                Some(attrs) => Some(modal_view(attrs)),
                None => {
                    // Slot undone away while open → close (React's `slot &&` render guard).
                    crate::editor_ops::close_attributes();
                    None
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            None::<AnyView>
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn modal_view(attrs: crate::editor_ops::SlotAttrs) -> AnyView {
    let tab = RwSignal::new(1usize); // React useState('Identity')
    let id = StoredValue::new(attrs.id.clone());
    let attrs = StoredValue::new(attrs);
    let subtitle = {
        let a = attrs.get_value();
        let role = if a.role.is_empty() { "Slot".to_string() } else { a.role.clone() };
        format!("{role} · {}", a.id)
    };
    view! {
        <div
            class="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
            on:click=move |_| crate::editor_ops::close_attributes()
        ></div>
        <div class="glass fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
            <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                <div class="min-w-0">
                    <h2 class="text-headline-sm text-on-surface">"Attributes"</h2>
                    <p class="mt-1 text-label-md text-on-surface-variant">{subtitle}</p>
                </div>
                <button
                    type="button"
                    aria-label="Close"
                    on:click=move |_| crate::editor_ops::close_attributes()
                    class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                >
                    <crate::ui::MaterialIcon name="close" />
                </button>
            </div>
            <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                <div class="flex flex-col gap-4">
                    <div class="flex gap-1 rounded-lg bg-surface-container-lowest/50 p-1">
                        {TABS
                            .iter()
                            .enumerate()
                            .map(|(i, label)| {
                                view! {
                                    <button
                                        type="button"
                                        aria-label=*label
                                        on:click=move |_| tab.set(i)
                                        class=move || {
                                            if tab.get() == i {
                                                "flex-1 rounded-md px-2 py-1.5 text-label-md transition-colors bg-primary/20 text-primary"
                                            } else {
                                                "flex-1 rounded-md px-2 py-1.5 text-label-md transition-colors text-on-surface-variant hover:bg-white/5"
                                            }
                                        }
                                    >
                                        {*label}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    {move || match tab.get() {
                        0 => transform_tab(id, attrs).into_any(),
                        1 => identity_tab(id, attrs).into_any(),
                        2 => states_tab().into_any(),
                        _ => arsenal_stub().into_any(),
                    }}
                </div>
            </div>
        </div>
    }
    .into_any()
}

/* ─────────────────────────── field primitives (fields.tsx ports) ─────────────────────────── */

/// Mono numeric field committing on blur/Enter (one commit = one undo step). While focused it holds
/// the local draft; unfocused it mirrors the doc value (rounded), so a map drag updates it live.
#[cfg(target_arch = "wasm32")]
fn number_field(
    label: &'static str,
    value: f64,
    suffix: Option<&'static str>,
    on_commit: impl Fn(f64) + Copy + 'static,
) -> impl IntoView {
    let draft = RwSignal::new(String::new());
    let focused = RwSignal::new(false);
    let rounded = StoredValue::new(format!("{}", value.round()));
    let commit = move || {
        focused.set(false);
        if let Ok(n) = draft.get_untracked().parse::<f64>() {
            if n.is_finite() {
                on_commit(n);
            }
        }
    };
    view! {
        <label class="flex flex-col gap-1">
            <span class="text-label-sm uppercase tracking-wider text-outline">{label}</span>
            <div class="relative">
                <input
                    type="number"
                    prop:value=move || {
                        if focused.get() { draft.get() } else { rounded.get_value() }
                    }
                    on:focus=move |_| {
                        draft.set(rounded.get_value());
                        focused.set(true);
                    }
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:blur=move |_| commit()
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                        }
                    }
                    class=if suffix.is_some() {
                        format!("{CONTROL} font-mono pr-7")
                    } else {
                        format!("{CONTROL} font-mono")
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
        </label>
    }
}

/// Text field committing per input event — the React `TextField` semantics (one undo step per
/// keystroke is the oracle behavior).
#[cfg(target_arch = "wasm32")]
fn text_field(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    on_change: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1">
            <span class="text-label-sm uppercase tracking-wider text-outline">{label}</span>
            <input
                type="text"
                value=value
                placeholder=placeholder
                on:input=move |ev| on_change(event_target_value(&ev))
                class=CONTROL
            />
        </label>
    }
}

/* ─────────────────────────── tabs ─────────────────────────── */

#[cfg(target_arch = "wasm32")]
fn transform_tab(
    id: StoredValue<String>,
    attrs: StoredValue<crate::editor_ops::SlotAttrs>,
) -> impl IntoView {
    let a = attrs.get_value();
    view! {
        <div class="flex flex-col gap-4">
            <div class="grid grid-cols-3 gap-3">
                {number_field("X", a.x, None, move |x| {
                    crate::editor_ops::attrs_update_position(
                        &id.get_value(),
                        Some(x),
                        None,
                        None,
                        None,
                    )
                })}
                {number_field("Y", a.y, None, move |y| {
                    crate::editor_ops::attrs_update_position(
                        &id.get_value(),
                        None,
                        Some(y),
                        None,
                        None,
                    )
                })}
                {number_field("Z", a.z, None, move |z| {
                    crate::editor_ops::attrs_update_position(
                        &id.get_value(),
                        None,
                        None,
                        Some(z),
                        None,
                    )
                })}
            </div>
            {number_field("Rotation", a.rotation, Some("°"), move |r| {
                crate::editor_ops::attrs_update_position(&id.get_value(), None, None, None, Some(r))
            })}
            <label class="flex flex-col gap-1">
                <span class="text-label-sm uppercase tracking-wider text-outline">"Stance"</span>
                <select
                    prop:value=a.stance.clone()
                    on:change=move |ev| {
                        crate::editor_ops::attrs_update_slot(
                            &id.get_value(),
                            None,
                            None,
                            Some(event_target_value(&ev)),
                        )
                    }
                    class=CONTROL
                >
                    <option value="stand" class="bg-surface-container">"Standing"</option>
                    <option value="crouch" class="bg-surface-container">"Crouched"</option>
                    <option value="prone" class="bg-surface-container">"Prone"</option>
                </select>
            </label>
            <p class="text-label-sm normal-case text-outline">
                "Drag on the map or edit coordinates above. Z is manual until terrain elevation (DEM) ships."
            </p>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
fn identity_tab(
    id: StoredValue<String>,
    attrs: StoredValue<crate::editor_ops::SlotAttrs>,
) -> impl IntoView {
    let a = attrs.get_value();
    let squad = if a.squad.is_empty() { "—".to_string() } else { a.squad.clone() };
    view! {
        <div class="flex flex-col gap-4">
            {text_field("Role", a.role.clone(), "Rifleman", move |role| {
                crate::editor_ops::attrs_update_slot(&id.get_value(), Some(role), None, None)
            })}
            {text_field("Tag", a.tag.clone(), "MED · ENG · SL…", move |tag| {
                crate::editor_ops::attrs_update_slot(&id.get_value(), None, Some(tag), None)
            })}
            <label class="flex flex-col gap-1">
                <span class="text-label-sm uppercase tracking-wider text-outline">"Squad"</span>
                <div class="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant">
                    {squad}
                </div>
            </label>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
fn states_tab() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-3">
            <p class="text-label-sm normal-case text-outline">
                "Unit traits — wired to the compiler in a later phase."
            </p>
            <div class="flex items-center justify-between py-0.5">
                <span class="text-label-md text-on-surface-variant">"Medic (soon)"</span>
                <span class="text-label-sm text-outline">"—"</span>
            </div>
            <div class="flex items-center justify-between py-0.5">
                <span class="text-label-md text-on-surface-variant">"Engineer (soon)"</span>
                <span class="text-label-sm text-outline">"—"</span>
            </div>
        </div>
    }
}

/// A3 — the Arsenal Forge rides T-159.27; the tab is present but a stub.
#[cfg(target_arch = "wasm32")]
fn arsenal_stub() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-white/10 bg-white/5 px-6 py-16 text-center">
            <span class="material-symbols-outlined text-4xl text-on-surface-variant">
                "checkroom"
            </span>
            <p class="text-body-md text-on-surface-variant">
                "Arsenal — the Smart Forge lands in the next slice."
            </p>
        </div>
    }
}
