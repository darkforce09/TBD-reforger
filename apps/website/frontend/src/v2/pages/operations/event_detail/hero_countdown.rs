//! The hub body: the operation hero, and the mission dossiers under it.
//!
//! **Role:** renders the shared body of an operation — the hero section with its name, T-minus
//! clock, local start time, briefing, voice-server chip and modpack link — and the list of
//! mission dossiers beneath it.
//! **Position:** the whole of the `/events/:id` route inside that route's chrome, and the detail
//! column of the `/events` schedule. Both callers own their own chrome and hand in the refetch
//! callback.
//! **Signals & state:** reads the session store from context and owns the modpack resource the
//! chip renders from.
//! **Invariants:** the operation briefing goes through the same trim-aware rule mission
//! briefings do, so a whitespace-only briefing reads as blank in both places. The modpack fetch
//! is a browser-only path and resolves to `None` in a native build, which simply omits the chip.
#![allow(dead_code)]

use super::mission_dossier::{briefing_text, mission_dossier};
use crate::v2::core::api::dto::{EventHub, ModpackDto};
use crate::v2::core::ui::MaterialIcon;
use crate::v2::core::utils::countdown::countdown_label;
use crate::v2::core::utils::datefmt::format_local_datetime;
use leptos::prelude::*;

// Choosing and making the modpack request is a browser-only path.
#[cfg(target_arch = "wasm32")]
use super::mission_dossier::{hub_modpack_fetch, HubModpackFetch};
#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::client::api_get;
#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::dto::DataEnvelope;

/// The shared hub body: the hero, then one dossier per mission.
///
/// `on_change` is run by every slotting mutation below, and is what reloads the operation so the
/// states derived from it stay live.
pub(crate) fn event_hub_view(ev: EventHub, on_change: Callback<()>) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let event_modpack_id = ev.modpack_id.clone();
    let modpack = LocalResource::new(move || {
        let event_modpack_id = event_modpack_id.clone();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                match hub_modpack_fetch(event_modpack_id.as_deref()) {
                    HubModpackFetch::ById(id) => {
                        match api_get::<DataEnvelope<ModpackDto>>(store, "/modpacks").await {
                            Ok(env) => env.data.into_iter().find(|mp| mp.modpack.id == id),
                            Err(_) => None,
                        }
                    }
                    HubModpackFetch::Current => {
                        crate::v2::core::api::client::api_get::<ModpackDto>(
                            store,
                            "/modpacks/current",
                        )
                        .await
                        .ok()
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, event_modpack_id);
                None::<ModpackDto>
            }
        }
    });
    let name = ev
        .name_override
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Untitled Operation".into());
    let countdown = countdown_label(&ev.start_time);
    let when = format_local_datetime(&ev.start_time);
    // The operation-level briefing is authorable and already shown on the schedule card; it
    // reads with the same trim/empty rule mission dossier briefings do.
    let operation_briefing = briefing_text(ev.briefing.as_deref());
    let missions = ev.missions;
    let has_missions = !missions.is_empty();
    view! {
        <section class="relative mb-8 overflow-hidden rounded-xl border border-outline-variant/30 bg-surface-container p-8">
            <div class="pointer-events-none absolute inset-0 bg-gradient-to-b from-primary/10 to-transparent"></div>
            <div class="relative flex flex-col gap-3">
                <span class="text-label-sm text-on-surface-variant uppercase">"Operation Hub"</span>
                <h1 class="text-headline-lg text-on-surface md:text-4xl">{name}</h1>
                <div class="font-mono text-headline-md tracking-widest text-primary">
                    "T-MINUS "
                    {countdown}
                </div>
                <p class="text-on-surface-variant">{when}</p>
                <div class="mt-1 max-w-prose">
                    <span class="mb-2 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                        "Briefing"
                    </span>
                    <p class="whitespace-pre-line text-sm leading-relaxed text-on-surface-variant">
                        {operation_briefing}
                    </p>
                </div>
                <div class="mt-2 flex flex-wrap gap-3 text-label-md">
                    <span class="flex items-center gap-2 rounded-lg border border-outline-variant/30 bg-surface-container-high px-3 py-2">
                        <MaterialIcon name="headset_mic" class="text-primary" />
                        " TS3: ts.tbdevent.eu"
                    </span>
                    {move || {
                        modpack
                            .get()
                            .flatten()
                            .map(|mp| {
                                // The pack DTO flattens the pack itself, and its workshop URL
                                // is an empty string rather than an absent field when unset.
                                let href = if mp.modpack.workshop_url.is_empty() {
                                    "#".to_string()
                                } else {
                                    mp.modpack.workshop_url.clone()
                                };
                                view! {
                                    <a
                                        href=href
                                        target="_blank"
                                        rel="noreferrer"
                                        class="flex items-center gap-2 rounded-lg border border-outline-variant/30 bg-surface-container-high px-3 py-2 hover:border-primary/40"
                                    >
                                        <MaterialIcon name="extension" class="text-primary" />
                                        " "
                                        {mp.modpack.name.clone()}
                                        " v"
                                        {mp.modpack.version.clone()}
                                    </a>
                                }
                            })
                    }}
                </div>
            </div>
        </section>

        <h2 class="mb-4 text-label-md text-on-surface-variant uppercase tracking-wide">
            "Mission Dossiers"
        </h2>
        {if has_missions {
            view! {
                <div class="flex flex-col gap-6">
                    {missions
                        .into_iter()
                        .enumerate()
                        .map(|(i, m)| mission_dossier(i + 1, m, on_change))
                        .collect_view()}
                </div>
            }
                .into_any()
        } else {
            view! {
                <p class="text-on-surface-variant">
                    "No missions have been added to this operation yet."
                </p>
            }
                .into_any()
        }}
    }
}
