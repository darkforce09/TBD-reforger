//! The read half of the modpack detail pane: the manifest and the actions around it.
//!
//! **Role:** renders a modpack's heading and totals, the table of addons it includes, the launch
//! and workshop links, and — for an administrator — the read/edit switch and the buttons that
//! make the pack current or delete it.
//! **Position:** the detail pane of the modpacks split view whenever the mode is reading.
//! **Signals & state:** owns the `set_busy` and `del_busy` flags for its two requests, writes the
//! page's `mode` through the switch, reads the `AuthStore` from context, refetches the pack list
//! resource after a write and reports every outcome through `toasts`.
//! **Invariants:** the make-current button is absent on the pack that is already current. Both
//! requests run on `wasm32` only; natively each button clears its busy flag and does nothing.

use super::mode_toggle::{read_edit_toggle, MpMode};
use super::pack_edit::{format_bytes, PackEdit};
use crate::v2::core::api::dto::{DataEnvelope, ModpackDto};
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use serde_json::json;

/// The read view of one modpack.
///
/// `p` is the pack, `mode` the read/edit switch, `is_admin` whether the administrative actions
/// are shown at all, `packs_res` the list resource refetched after a write, and `toasts` where
/// the outcome of each write is reported.
pub(super) fn dossier(
    p: &ModpackDto,
    mode: RwSignal<MpMode>,
    is_admin: bool,
    packs_res: LocalResource<Option<DataEnvelope<ModpackDto>>>,
    toasts: crate::v2::core::ui::toast::Toasts,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let data = PackEdit::from_dto(p);
    let mod_count = data.mods.len() as i64;
    let pack_id = p.modpack.id.clone();
    let workshop_url = p.modpack.workshop_url.clone();
    let version = p.modpack.version.clone();
    let size = p.modpack.total_size_bytes;
    let is_current = p.modpack.is_current;
    let set_busy = RwSignal::new(false);
    let del_busy = RwSignal::new(false);

    view! {
        <div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
            <header class="flex items-start justify-between gap-4">
                <div>
                    <h2 class="text-4xl font-bold tracking-tight text-on-surface">{data.name.clone()}</h2>
                    <div class="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 font-mono text-sm text-on-surface-variant">
                        <span>"v"{version.clone()}</span>
                        <span>
                            <span class="text-on-surface">{format_bytes(size)}</span>
                            " total"
                        </span>
                        <span>
                            <span class="text-on-surface">{mod_count}</span>
                            " mods included"
                        </span>
                    </div>
                </div>
                {is_admin.then(|| read_edit_toggle(mode))}
            </header>
            <ul class="mt-8">
                {data
                    .mods
                    .into_iter()
                    .map(|m| {
                        let wid = m.workshop_id.clone();
                        view! {
                            <li class="flex items-center gap-4 rounded-xl border-b border-white/5 px-4 py-5 transition hover:bg-white/[0.02]">
                                <div class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-white/5 text-on-surface-variant">
                                    <MaterialIcon name="extension" />
                                </div>
                                <div class="flex min-w-0 flex-1 flex-col">
                                    <span class="font-medium text-on-surface">{m.name.clone()}</span>
                                    {(!wid.is_empty()).then(|| {
                                        view! {
                                            <span class="font-mono text-[11px] text-on-surface-variant/70">
                                                {wid}
                                            </span>
                                        }
                                    })}
                                </div>
                                {m.required.then(|| {
                                    view! {
                                        <span class="rounded-md border border-tactical-yellow/20 bg-tactical-yellow/10 px-2.5 py-1 font-mono text-xs tracking-wider text-tactical-yellow">
                                            "[ REQUIRED ]"
                                        </span>
                                    }
                                })}
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <div class="mt-10 space-y-3 pt-2">
                <button
                    type="button"
                    class="w-full rounded-full bg-action py-5 text-lg font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                    on:click=move |_| toasts.message("Launch requires the Reforger client")
                >
                    "[ Launch Game & Auto-Download ]"
                </button>
                {(!workshop_url.is_empty()).then(|| {
                    let url = workshop_url.clone();
                    view! {
                        <a
                            href=url
                            target="_blank"
                            rel="noreferrer"
                            class="mt-4 block text-center text-sm text-on-surface-variant transition hover:text-on-surface"
                        >
                            "View collection in Reforger Workshop ↗"
                        </a>
                    }
                })}
                {is_admin.then(|| {
                    let pack_id_set = pack_id.clone();
                    let pack_id_del = pack_id.clone();
                    view! {
                        <div class="flex flex-wrap gap-2 pt-4">
                            {(!is_current).then(|| {
                                let pack_id_set = pack_id_set.clone();
                                view! {
                                    <button
                                        type="button"
                                        disabled=move || set_busy.get()
                                        class="rounded-full border border-success/30 bg-success/10 px-4 py-2 font-mono text-xs tracking-wider text-success uppercase transition hover:bg-success/20 disabled:opacity-50"
                                        on:click=move |_| {
                                            if set_busy.get_untracked() {
                                                return;
                                            }
                                            set_busy.set(true);
                                            let path = format!("/modpacks/{pack_id_set}/set-current");
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                leptos::task::spawn_local(async move {
                                                    match crate::v2::core::api::client::api_post_ok(
                                                        store,
                                                        &path,
                                                        json!({}),
                                                    )
                                                    .await
                                                    {
                                                        Ok(()) => {
                                                            toasts.success("Set as current modpack");
                                                            packs_res.refetch();
                                                        }
                                                        Err(e) => {
                                                            toasts.error(
                                                                crate::v2::core::api::client::api_error_message(
                                                                    &e,
                                                                    "Failed to set current",
                                                                ),
                                                            );
                                                        }
                                                    }
                                                    set_busy.set(false);
                                                });
                                            }
                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                let _ = (store, path, packs_res, toasts);
                                                set_busy.set(false);
                                            }
                                        }
                                    >
                                        {move || {
                                            if set_busy.get() {
                                                "Setting…"
                                            } else {
                                                "Set current"
                                            }
                                        }}
                                    </button>
                                }
                            })}
                            <button
                                type="button"
                                disabled=move || del_busy.get()
                                class="rounded-full border border-error-alert/30 px-4 py-2 font-mono text-xs tracking-wider text-error-alert uppercase transition hover:bg-error-alert/10 disabled:opacity-50"
                                on:click=move |_| {
                                    if del_busy.get_untracked() {
                                        return;
                                    }
                                    del_busy.set(true);
                                    let path = format!("/modpacks/{pack_id_del}");
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        leptos::task::spawn_local(async move {
                                            match crate::v2::core::api::client::api_delete(store, &path).await {
                                                Ok(()) => {
                                                    toasts.success("Modpack deleted");
                                                    packs_res.refetch();
                                                }
                                                Err(e) => {
                                                    toasts.error(crate::v2::core::api::client::api_error_message(
                                                        &e,
                                                        "Failed to delete modpack",
                                                    ));
                                                }
                                            }
                                            del_busy.set(false);
                                        });
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        let _ = (store, path, packs_res, toasts);
                                        del_busy.set(false);
                                    }
                                }
                            >
                                {move || if del_busy.get() { "Deleting…" } else { "Delete" }}
                            </button>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}
