//! The edit half of the modpack detail pane: the form that writes a modpack back.
//!
//! **Role:** renders the editable pack fields, the reorderable addon rows with their required
//! flag and workshop id, the row that appends a new addon, and the save and cancel buttons.
//! **Position:** the detail pane of the modpacks split view whenever an administrator has
//! switched the mode to editing.
//! **Signals & state:** owns one signal per editable field plus the `mods` vector, the two
//! new-addon inputs, and the `save_busy` and `save_err` flags; writes the page's `mode` back to
//! reading after a successful save, reads the `AuthStore` from context, refetches the pack list
//! resource and reports success through `toasts`.
//! **Invariants:** the form seeds from the pack once, so an in-progress edit survives a refetch
//! of the list behind it. An empty name falls back to the pack's stored name and an empty
//! version to `0.0.0`, so neither can be blanked by accident. The save runs on `wasm32` only;
//! natively the button clears its busy flag and does nothing.

use super::mode_toggle::{read_edit_toggle, MpMode};
use super::pack_edit::{ModEdit, PackEdit};
use crate::v2::core::api::dto::{DataEnvelope, ModpackDto};
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// The edit form for one modpack.
///
/// `p` is the pack the form seeds from, `mode` the read/edit switch it returns to reading on
/// save, `packs_res` the list resource refetched once the save lands, and `toasts` where the
/// saved name is reported.
pub(super) fn editor(
    p: &ModpackDto,
    mode: RwSignal<MpMode>,
    packs_res: LocalResource<Option<DataEnvelope<ModpackDto>>>,
    toasts: crate::v2::core::ui::toast::Toasts,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let initial = PackEdit::from_dto(p);
    let name = RwSignal::new(initial.name.clone());
    let version = RwSignal::new(initial.version.clone());
    let workshop_url = RwSignal::new(initial.workshop_url.clone());
    let total_size = RwSignal::new(initial.total_size_bytes);
    let is_current = RwSignal::new(initial.is_current);
    let mods = RwSignal::new(initial.mods);
    let new_mod = RwSignal::new(String::new());
    let new_workshop = RwSignal::new(String::new());
    let pack_id = p.modpack.id.clone();
    let fallback_name = p.modpack.name.clone();
    let save_busy = RwSignal::new(false);
    let save_err = RwSignal::new(None::<String>);

    let add_mod = move || {
        let trimmed = new_mod.get_untracked().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let wid = new_workshop.get_untracked().trim().to_string();
        mods.update(|m| {
            m.push(ModEdit {
                name: trimmed,
                required: false,
                workshop_id: wid,
                mod_guid: String::new(),
                version: String::new(),
            })
        });
        new_mod.set(String::new());
        new_workshop.set(String::new());
    };

    view! {
        <div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
            <header class="flex items-start justify-between gap-4">
                <div class="flex-1 space-y-3">
                    <div>
                        <label class="mb-1 block font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                            "Modpack name"
                        </label>
                        <input
                            prop:value=initial.name.clone()
                            on:input=move |ev| name.set(event_target_value(&ev))
                            class="w-full rounded-xl border border-white/10 bg-black/30 px-4 py-3 text-2xl font-bold tracking-tight text-on-surface focus:border-primary/50 focus:outline-none"
                        />
                    </div>
                    <div class="flex flex-wrap gap-3">
                        <div class="min-w-[8rem] flex-1">
                            <label class="mb-1 block font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                                "Version"
                            </label>
                            <input
                                prop:value=initial.version.clone()
                                on:input=move |ev| version.set(event_target_value(&ev))
                                class="w-full rounded-xl border border-white/10 bg-black/30 px-3 py-2 font-mono text-sm text-on-surface focus:border-primary/50 focus:outline-none"
                            />
                        </div>
                        <div class="min-w-[12rem] flex-[2]">
                            <label class="mb-1 block font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                                "Workshop URL"
                            </label>
                            <input
                                prop:value=initial.workshop_url.clone()
                                on:input=move |ev| workshop_url.set(event_target_value(&ev))
                                class="w-full rounded-xl border border-white/10 bg-black/30 px-3 py-2 text-sm text-on-surface focus:border-primary/50 focus:outline-none"
                            />
                        </div>
                    </div>
                    <label class="flex items-center gap-2 font-mono text-xs text-on-surface-variant">
                        <input
                            type="checkbox"
                            prop:checked=initial.is_current
                            on:change=move |ev| {
                                is_current.set(event_target_checked(&ev));
                            }
                        />
                        "Mark as current modpack"
                    </label>
                </div>
                {read_edit_toggle(mode)}
            </header>
            <ul class="mt-8">
                {move || {
                    let list = mods.get();
                    if list.is_empty() {
                        return view! {
                            <li class="px-4 py-6 text-center text-sm text-on-surface-variant">
                                "No mods yet — add one below."
                            </li>
                        }
                            .into_any();
                    }
                    list.into_iter()
                        .enumerate()
                        .map(|(i, m)| {
                            let req_class = if m.required {
                                "rounded-md border px-2.5 py-1 font-mono text-xs tracking-wider transition border-tactical-yellow/20 bg-tactical-yellow/10 text-tactical-yellow"
                            } else {
                                "rounded-md border px-2.5 py-1 font-mono text-xs tracking-wider transition border-white/10 text-on-surface-variant hover:bg-white/5"
                            };
                            let remove_label = format!("Remove {}", m.name);
                            let wid = m.workshop_id.clone();
                            view! {
                                <li class="flex flex-col gap-2 rounded-xl border-b border-white/5 px-4 py-4">
                                    <div class="flex items-center gap-3">
                                        <MaterialIcon
                                            name="drag_indicator"
                                            class="text-on-surface-variant/50"
                                        />
                                        <span class="flex-1 font-medium text-on-surface">{m.name.clone()}</span>
                                        <button
                                            type="button"
                                            class=req_class
                                            on:click=move |_| {
                                                mods.update(|list| {
                                                    if let Some(entry) = list.get_mut(i) {
                                                        entry.required = !entry.required;
                                                    }
                                                })
                                            }
                                        >
                                            "[ REQUIRED ]"
                                        </button>
                                        <button
                                            type="button"
                                            aria-label=remove_label
                                            class="flex size-8 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
                                            on:click=move |_| {
                                                mods.update(|list| {
                                                    list.remove(i);
                                                })
                                            }
                                        >
                                            <MaterialIcon name="close" />
                                        </button>
                                    </div>
                                    <input
                                        prop:value=wid
                                        placeholder="Workshop id (game.mods[].modId)"
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev);
                                            mods.update(|list| {
                                                if let Some(entry) = list.get_mut(i) {
                                                    entry.workshop_id = v;
                                                }
                                            })
                                        }
                                        class="ml-8 rounded-lg border border-white/10 bg-black/20 px-3 py-1.5 font-mono text-xs text-on-surface placeholder:text-on-surface-variant/50 focus:border-primary/50 focus:outline-none"
                                    />
                                </li>
                            }
                                .into_any()
                        })
                        .collect_view()
                        .into_any()
                }}
            </ul>
            <div class="mt-4 flex flex-col gap-2 sm:flex-row">
                <input
                    prop:value=move || new_mod.get()
                    on:input=move |ev| new_mod.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            add_mod();
                        }
                    }
                    placeholder="Add a mod (e.g. ACE Reforged)…"
                    class="flex-1 rounded-xl border border-white/10 bg-black/30 px-4 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 focus:border-primary/50 focus:outline-none"
                />
                <input
                    prop:value=move || new_workshop.get()
                    on:input=move |ev| new_workshop.set(event_target_value(&ev))
                    placeholder="Workshop id"
                    class="w-full rounded-xl border border-white/10 bg-black/30 px-4 py-3 font-mono text-sm text-on-surface placeholder:text-on-surface-variant/60 focus:border-primary/50 focus:outline-none sm:w-44"
                />
                <button
                    type="button"
                    on:click=move |_| add_mod()
                    class="flex items-center gap-1.5 rounded-xl border border-white/10 px-4 text-sm font-medium text-on-surface transition hover:bg-white/5"
                >
                    <MaterialIcon name="add" class="text-base" />
                    "Add"
                </button>
            </div>
            <div class="mt-10 flex flex-col gap-3 pt-2">
                {move || {
                    save_err.get().map(|m| {
                        view! {
                            <p class="font-mono text-sm text-error-alert">{m}</p>
                        }
                    })
                }}
                <div class="flex gap-3">
                    <button
                        type="button"
                        disabled=move || save_busy.get()
                        on:click=move |_| {
                            if save_busy.get_untracked() {
                                return;
                            }
                            let n = name.get_untracked().trim().to_string();
                            let final_name = if n.is_empty() {
                                fallback_name.clone()
                            } else {
                                n
                            };
                            let mut ver = version.get_untracked().trim().to_string();
                            if ver.is_empty() {
                                ver = "0.0.0".into();
                            }
                            let edit = PackEdit {
                                name: final_name.clone(),
                                version: ver,
                                total_size_bytes: total_size.get_untracked(),
                                workshop_url: workshop_url.get_untracked(),
                                is_current: is_current.get_untracked(),
                                mods: mods.get_untracked(),
                            };
                            let body = edit.to_put_body();
                            let path = format!("/modpacks/{pack_id}");
                            save_busy.set(true);
                            save_err.set(None);
                            #[cfg(target_arch = "wasm32")]
                            {
                                leptos::task::spawn_local(async move {
                                    match crate::v2::core::api::client::api_put::<ModpackDto>(store, &path, body)
                                        .await
                                    {
                                        Ok(saved) => {
                                            toasts.success(format!(
                                                "Saved \"{}\"",
                                                saved.modpack.name
                                            ));
                                            mode.set(MpMode::Read);
                                            packs_res.refetch();
                                        }
                                        Err(e) => {
                                            save_err.set(Some(crate::v2::core::api::client::api_error_message(
                                                &e,
                                                "Failed to save modpack",
                                            )));
                                        }
                                    }
                                    save_busy.set(false);
                                });
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let _ = (store, path, body, packs_res, toasts, mode);
                                save_busy.set(false);
                            }
                        }
                        class="flex-1 rounded-full bg-action py-4 text-lg font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
                    >
                        {move || if save_busy.get() { "Saving…" } else { "Save Changes" }}
                    </button>
                    <button
                        type="button"
                        on:click=move |_| mode.set(MpMode::Read)
                        class="rounded-full border border-white/10 px-8 text-base font-medium text-on-surface-variant transition hover:bg-white/5 hover:text-on-surface"
                    >
                        "Cancel"
                    </button>
                </div>
            </div>
        </div>
    }
}
