//! The Edit Armory dialog: the write half of the mission armory.
//!
//! **Role:** the form that edits the armory rows for one faction at a time and sends the whole
//! armory back, plus the failure wording when the server refuses it.
//! **Position:** a dialog over the `/missions/:id` route, opened from the page header; never
//! inside the dossier body, which is shared with the library's slide-over and stays read-only.
//! **Signals & state:** every signal it reads and writes belongs to the [`ArmoryEditor`] handle it
//! is given. Reads the session store and the toast queue from context.
//! **Invariants:** the write is wholesale — it replaces the mission's entire armory — so the
//! dialog always holds every faction's rows, not just the one on screen, and the refusal guard
//! runs before anything is sent. The faction is picked, never typed.

use super::armory_editor::{draft_problem, key_label, key_storable, ArmoryEditor};
use super::armory_editor::{parse_qty, DraftRow, KeySource};
// The request body and its type are built only inside the browser-only save closure.
#[cfg(target_arch = "wasm32")]
use super::armory_editor::armory_body;
use crate::v2::core::ui::{cn, Dialog, MaterialIcon};
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use serde_json::Value;

/// The Edit Armory dialog.
///
/// A dialog rather than a section inside the dossier body for two reasons: the body is shared
/// with the library's slide-over, so a form there would be a form inside an overlay that another
/// overlay then stacks on top of; and a half-typed row must not sit in the same column as
/// published content and read as if it were part of it.
pub(super) fn armory_dialog(ed: ArmoryEditor) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let on_save = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let id = ed.mission_id.get_untracked();
            if id.is_empty() || ed.busy.get_untracked() {
                return;
            }
            let rows = ed.rows.get_untracked();
            // The button is already disabled on this condition, and it is re-checked here because
            // the disabled attribute is the browser's promise, not the handler's.
            if let Some(problem) = draft_problem(&rows) {
                crate::v2::core::ui::toast::use_toasts().error(problem);
                return;
            }
            let body = armory_body(&rows);
            let count = rows.len();
            ed.busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            leptos::task::spawn_local(async move {
                let path = format!("/missions/{id}/armory");
                match crate::v2::core::api::client::api_put::<Value>(store, &path, body).await {
                    Ok(_) => {
                        toasts.success(if count == 0 {
                            "Armory cleared".to_string()
                        } else {
                            format!("Armory saved \u{2014} {count} items")
                        });
                        ed.open.set(false);
                        ed.saved.update(|n| *n = n.wrapping_add(1));
                    }
                    // The refusal names the offending item and field; showing it verbatim is more
                    // use than anything this page could invent, and it is the only channel through
                    // which a guard this editor does not yet mirror can reach the author.
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not save the armory",
                    )),
                }
                ed.busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (ed, store);
        }
    };
    // Rows filed under the faction currently on screen, paired with their index in the full draft:
    // removal has to address the real vector, not the filtered view.
    let visible = move || {
        let f = ed.faction.get();
        ed.rows
            .get()
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.faction == f)
            .collect::<Vec<_>>()
    };
    let active_key = move || {
        let f = ed.faction.get();
        ed.keys.get().into_iter().find(|k| k.key == f)
    };
    let can_add = move || {
        !ed.new_name.get().trim().is_empty()
            && parse_qty(&ed.new_qty.get()).is_some()
            && active_key().is_some_and(|k| key_storable(&k.key))
    };
    let add_row = move |_| {
        let faction = ed.faction.get_untracked();
        if faction.is_empty() || !can_add() {
            return;
        }
        ed.rows.update(|rows| {
            rows.push(DraftRow {
                faction,
                item_name: ed.new_name.get_untracked(),
                category: ed.new_category.get_untracked(),
                quantity: ed.new_qty.get_untracked(),
            })
        });
        ed.new_name.set(String::new());
        ed.new_category.set(String::new());
        ed.new_qty.set(String::new());
    };
    const PILL: &str = "rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:ring-1 focus:ring-primary/50";
    view! {
        <Dialog
            open=ed.open
            title="Edit Armory"
            description="Saving replaces this mission's entire armory. Faction keys are taken from the mission's ORBAT — the Event Hub matches them byte-for-byte, so they are chosen here, never typed."
            class="max-w-2xl"
        >
            {move || {
                let keys = ed.keys.get();
                if keys.is_empty() {
                    // No order of battle and no stored rows: there is no key that would join, and
                    // a text box here would only manufacture one that does not. Say what is
                    // missing instead.
                    return view! {
                        <div class="rounded-xl border border-tactical-yellow/20 bg-tactical-yellow/5 p-4 text-label-md text-on-surface-variant">
                            <p class="mb-2 text-on-surface">"This mission has no ORBAT factions yet."</p>
                            <p>
                                "The armory is keyed by faction, and the Event Hub builds its faction list from the ORBAT slots a mission materialises when it is attached to an operation. Author the ORBAT in the Mission Creator and save a version first — otherwise every armory row would be filed under a key that matches nothing."
                            </p>
                        </div>
                    }
                        .into_any();
                }
                let tabs = keys
                    .iter()
                    .map(|k| {
                        let key = k.key.clone();
                        let key_active = k.key.clone();
                        let label = key_label(&k.key);
                        let unstorable = !key_storable(&k.key);
                        view! {
                            <button
                                type="button"
                                on:click=move |_| ed.faction.set(key.clone())
                                class=move || {
                                    cn(
                                        &[
                                            "rounded-full px-4 py-2 text-sm font-medium transition",
                                            if ed.faction.get() == key_active {
                                                "bg-white/10 text-on-surface"
                                            } else {
                                                "text-on-surface-variant hover:text-on-surface"
                                            },
                                            if unstorable { "line-through decoration-error/70" } else { "" },
                                        ],
                                    )
                                }
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="inline-flex flex-wrap rounded-full bg-white/5 p-1">{tabs}</div>
                    // Why the selected key is, or is not, usable. The author has to be told which
                    // of the two it is rather than shown a refusal after the fact.
                    {move || {
                        active_key()
                            .map(|k| {
                                if !key_storable(&k.key) {
                                    view! {
                                        <p class="mt-3 rounded-lg border border-error/20 bg-error-container/10 p-3 text-label-md text-error">
                                            "The endpoint refuses this faction key: it is blank or whitespace-padded. It is not trimmed here on purpose — the ORBAT side of the join stores its value verbatim too, so trimming one side would break a pair that agrees today. Fix the faction key in the Mission Creator, re-attach the mission, then author its armory."
                                        </p>
                                    }
                                        .into_any()
                                } else if k.source == KeySource::StoredOnly {
                                    view! {
                                        <p class="mt-3 rounded-lg border border-tactical-yellow/20 bg-tactical-yellow/5 p-3 text-label-md text-on-surface-variant">
                                            "This key is on stored armory rows but is in no ORBAT faction of the current version, so its items render here and on nothing else. Kept so saving does not delete them."
                                        </p>
                                    }
                                        .into_any()
                                } else {
                                    ().into_any()
                                }
                            })
                    }}

                    <div class="mt-5 space-y-2">
                        {move || {
                            let rows = visible();
                            if rows.is_empty() {
                                return view! {
                                    <p class="px-1 text-sm text-on-surface-variant/70">
                                        "No items for this faction yet."
                                    </p>
                                }
                                    .into_any();
                            }
                            rows.into_iter()
                                .map(|(i, r)| {
                                    let name = r.item_name.clone();
                                    let aria = format!("Remove {}", r.item_name);
                                    let category = r.category.clone();
                                    let qty = parse_qty(&r.quantity)
                                        .flatten()
                                        .map(|q| format!("x{q}"))
                                        .unwrap_or_else(|| "\u{221e}".to_string());
                                    view! {
                                        <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
                                            <MaterialIcon name="inventory_2" class="text-on-surface-variant" />
                                            <span class="flex-1 truncate text-sm text-on-surface">{name}</span>
                                            {(!category.is_empty())
                                                .then(|| {
                                                    view! {
                                                        <span class="shrink-0 font-mono text-xs text-on-surface-variant/70">
                                                            {category}
                                                        </span>
                                                    }
                                                })}
                                            <span class="w-12 shrink-0 text-right font-mono text-xs text-tactical-yellow">
                                                {qty}
                                            </span>
                                            <button
                                                type="button"
                                                on:click=move |_| {
                                                    ed.rows
                                                        .update(|rows| {
                                                            if i < rows.len() {
                                                                rows.remove(i);
                                                            }
                                                        })
                                                }
                                                aria-label=aria
                                                class="flex size-7 shrink-0 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
                                            >
                                                <MaterialIcon name="close" class="text-base" />
                                            </button>
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>

                    // A row is composed here and appended whole, so no keystroke ever re-renders
                    // the list above it.
                    <div class="mt-4 flex flex-wrap items-center gap-2">
                        <input
                            prop:value=move || ed.new_name.get()
                            on:input=move |ev| ed.new_name.set(event_target_value(&ev))
                            placeholder="Item (e.g. M4A1)"
                            class=cn(&["min-w-0 flex-1", PILL])
                        />
                        <input
                            prop:value=move || ed.new_category.get()
                            on:input=move |ev| ed.new_category.set(event_target_value(&ev))
                            placeholder="Category"
                            class=cn(&["w-32", PILL])
                        />
                        <input
                            inputmode="numeric"
                            prop:value=move || ed.new_qty.get()
                            on:input=move |ev| ed.new_qty.set(event_target_value(&ev))
                            placeholder="Qty"
                            class=cn(&["w-20 font-mono", PILL])
                        />
                        <button
                            type="button"
                            on:click=add_row
                            prop:disabled=move || !can_add()
                            class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-sm text-on-surface transition hover:bg-white/5 disabled:opacity-40"
                        >
                            <MaterialIcon name="add" class="text-base" />
                            "Add"
                        </button>
                    </div>
                    <p class="mt-2 px-1 text-label-md text-on-surface-variant/70">
                        "Blank quantity = unlimited (\u{221e})."
                    </p>
                }
                    .into_any()
            }}

            <div class="mt-6 border-t border-outline-variant/30 pt-4">
                <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                    {move || {
                        let rows = ed.rows.get();
                        let mut factions: Vec<&String> = Vec::new();
                        for r in &rows {
                            if !factions.contains(&&r.faction) {
                                factions.push(&r.faction);
                            }
                        }
                        format!(
                            "{} item{} across {} faction{}",
                            rows.len(),
                            if rows.len() == 1 { "" } else { "s" },
                            factions.len(),
                            if factions.len() == 1 { "" } else { "s" },
                        )
                    }}
                </p>
                {move || {
                    draft_problem(&ed.rows.get())
                        .map(|p| {
                            view! {
                                <p class="mb-3 rounded-lg border border-error/20 bg-error-container/10 p-3 text-label-md text-error">
                                    {p}
                                </p>
                            }
                        })
                }}
                <button
                    type="button"
                    on:click=on_save
                    prop:disabled=move || {
                        ed.busy.get() || draft_problem(&ed.rows.get()).is_some()
                    }
                    class="w-full rounded-full bg-action py-4 text-base font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
                >
                    {move || {
                        if ed.busy.get() {
                            "Saving\u{2026}".to_string()
                        } else if ed.rows.get().is_empty() {
                            "Clear Armory".to_string()
                        } else {
                            "Save Armory".to_string()
                        }
                    }}
                </button>
            </div>
        </Dialog>
    }
}
