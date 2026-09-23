//! The server control route: the configured servers, the one an administrator is working on, and
//! the fleet's scenario registry.
//!
//! **Role:** fetches the server list, puts it behind the administrator gate, and arranges the
//! picker — with the control that opens the fleet scenario sheet — beside the selected server's
//! card.
//! **Position:** the `/admin/server` route, rendered inside the navigation frame.
//! **Signals & state:** owns `selected_id` (which server the detail pane shows) and the fleet
//! scenario registry, which belongs to the whole fleet rather than to one server. The fetched list
//! lives in a `LocalResource` read inside a suspense boundary.
//! **Invariants:** the request future is not `Send`, so the fetch is a browser-only path and a
//! native build resolves to nothing and renders the failure branch. Each server card builds its own
//! console and deployments state, so switching servers never shows one server's commands or
//! deployments under another's name.
#![allow(dead_code)]

use super::fleet_scenarios::{scenario_sheet, ScenarioRegistry};
use super::server_cards::{pick_default_id, server_detail, server_list};
use crate::v2::core::api::dto::{DataEnvelope, ServerRowDto};
use crate::v2::core::ui::{AdminGate, MaterialIcon};
use leptos::prelude::*;

/// The server control screen, behind the administrator gate.
#[component]
pub fn ServerControlPage() -> impl IntoView {
    view! {
        <AdminGate>
            <ServerControlInner />
        </AdminGate>
    }
}

/// The screen an administrator sees: the fetch, and its three render states.
#[component]
fn ServerControlInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let servers = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<DataEnvelope<ServerRowDto>>(store, "/servers")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<ServerRowDto>>
        }
    });
    let scenarios = ScenarioRegistry::new(store, crate::v2::core::ui::toast::use_toasts());

    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                <Suspense fallback=move || {
                    view! {
                        <p class="px-8 py-10 text-on-surface-variant">"Loading servers…"</p>
                    }
                }>
                    {move || {
                        servers.get().map(|opt| match opt {
                            Some(env) => control_board(env.data, scenarios).into_any(),
                            None => {
                                view! {
                                    <p class="px-8 py-10 text-error">"Failed to load servers."</p>
                                }
                                    .into_any()
                            }
                        })
                    }}
                </Suspense>
            </div>
            {scenario_sheet(scenarios)}
        </div>
    }
}

/// The picker and the selected server's card, side by side.
///
/// The list is cloned once for each pane: the picker reads names and statuses, the card reads the
/// one row the picker selected.
fn control_board(list: Vec<ServerRowDto>, scenarios: ScenarioRegistry) -> impl IntoView {
    let selected_id = RwSignal::new(pick_default_id(&list).unwrap_or_default());
    let list_master = list.clone();
    let list_detail = list;

    view! {
        <crate::v2::core::ui::split_pane::SplitPane
            transparent=true
            master_width="17rem"
            master_header=master_header(list_master.len(), scenarios).into_any()
            master=view! {
                {move || {
                    server_list(&list_master, selected_id)
                }}
            }
                .into_any()
            detail=view! {
                {move || {
                    let id = selected_id.get();
                    let Some(s) = list_detail.iter().find(|s| s.id == id) else {
                        return view! {
                            <p class="px-8 py-10 text-on-surface-variant">
                                {if list_detail.is_empty() {
                                    "No servers configured."
                                } else {
                                    "No server selected."
                                }}
                            </p>
                        }
                            .into_any();
                    };
                    server_detail(s.clone()).into_any()
                }}
            }
                .into_any()
        />
    }
}

/// The picker pane's heading: the word "Servers", how many there are, and the fleet scenario
/// sheet's control.
fn master_header(count: usize, scenarios: ScenarioRegistry) -> impl IntoView {
    view! {
        <div class="flex w-full items-center justify-between gap-2">
            <h1 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                "Servers"
                <span class="ml-2 font-mono text-code-md text-outline">{count as i64}</span>
            </h1>
            <button
                type="button"
                data-testid="server-control-fleet-scenarios"
                title="Which scenario the fleet runs for each terrain"
                on:click=move |_| scenarios.open_sheet()
                class="flex items-center gap-1 rounded-full border border-white/10 px-3 py-1 text-xs text-on-surface transition hover:bg-white/5"
            >
                <MaterialIcon name="map" class="text-[14px]" />
                "Fleet scenarios"
            </button>
        </div>
    }
}
