//! The server control route: the configured servers, the one an administrator is working on, the
//! control that registers another, and the fleet's scenario registry.
//!
//! **Role:** builds the server registry and the fleet scenario registry, puts the screen behind the
//! administrator gate, and arranges the picker — with the control that opens the fleet scenario
//! sheet and the one that opens the registration sheet — beside the selected server's card.
//! **Position:** the `/admin/server` route, rendered inside the navigation frame.
//! **Signals & state:** owns the [`ServerRegistry`] — the server list, the selection and the
//! registration sheet — and the fleet scenario registry, which belongs to the whole fleet rather
//! than to one server.
//! **Invariants:** the list read is browser-only, so a native build renders the failure branch.
//! The card is rebuilt only when the selected server changes, never when a write changes a row:
//! each card builds its own console, deployments and credential state and reads its row from the
//! registry, so switching servers never shows one server's commands or deployments under another's
//! name, and editing a server keeps its card's state.
#![allow(dead_code)]

use super::fleet_scenarios::{scenario_sheet, ScenarioRegistry};
use super::server_cards::{server_detail, server_list};
use super::server_registry::{registration_sheet, ListRead, ServerRegistry};
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

/// The screen an administrator sees: the list read, and its three render states.
#[component]
fn ServerControlInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let toasts = crate::v2::core::ui::toast::use_toasts();
    let registry = ServerRegistry::new(store, toasts);
    registry.load();
    let scenarios = ScenarioRegistry::new(store, toasts);

    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                {move || match registry.read.get() {
                    ListRead::Loading => {
                        view! {
                            <p class="px-8 py-10 text-on-surface-variant">"Loading servers…"</p>
                        }
                            .into_any()
                    }
                    ListRead::Failed => {
                        view! { <p class="px-8 py-10 text-error">"Failed to load servers."</p> }
                            .into_any()
                    }
                    ListRead::Loaded => control_board(registry, scenarios).into_any(),
                }}
            </div>
            {scenario_sheet(scenarios)}
            {registration_sheet(registry)}
        </div>
    }
}

/// The picker and the selected server's card, side by side.
///
/// The card is keyed on the selected id alone — `shown` changes only when the selection does, or
/// when the selected server joins the list — so a write that changes a row updates the card in
/// place.
fn control_board(registry: ServerRegistry, scenarios: ScenarioRegistry) -> impl IntoView {
    let shown = Memo::new(move |_| {
        let id = registry.selected_id.get();
        registry
            .servers
            .with(|list| list.iter().any(|s| s.id == id))
            .then_some(id)
    });
    let no_servers = Memo::new(move |_| registry.servers.with(Vec::is_empty));

    view! {
        <crate::v2::core::ui::split_pane::SplitPane
            transparent=true
            master_width="17rem"
            master_header=master_header(registry, scenarios).into_any()
            master=view! {
                {move || registry.servers.with(|list| server_list(list, registry.selected_id))}
                {add_server_control(registry)}
            }
                .into_any()
            detail=view! {
                {move || match shown.get() {
                    Some(id) => server_detail(registry, id).into_any(),
                    None => no_server_shown(registry, no_servers.get()).into_any(),
                }}
            }
                .into_any()
        />
    }
}

/// The picker pane's heading: the word "Servers", how many there are, and the fleet scenario
/// sheet's control.
fn master_header(registry: ServerRegistry, scenarios: ScenarioRegistry) -> impl IntoView {
    view! {
        <div class="flex w-full items-center justify-between gap-2">
            <h1 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                "Servers"
                <span class="ml-2 font-mono text-code-md text-outline">
                    {move || registry.servers.with(Vec::len) as i64}
                </span>
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

/// The control at the foot of the picker that opens the registration sheet on a new server.
fn add_server_control(registry: ServerRegistry) -> impl IntoView {
    view! {
        <button
            type="button"
            data-testid="server-control-add"
            on:click=move |_| registry.open_to_register()
            class="mt-1 flex items-center justify-center gap-1.5 rounded-lg border border-dashed border-white/15 px-3 py-2.5 text-label-md text-on-surface-variant transition hover:bg-white/[0.03] hover:text-on-surface"
        >
            <MaterialIcon name="add" class="text-[16px]" />
            "Add server"
        </button>
    }
}

/// The detail pane when no card is shown: the first server still to be registered, or no
/// selection.
fn no_server_shown(registry: ServerRegistry, no_servers: bool) -> impl IntoView {
    if !no_servers {
        return view! { <p class="px-8 py-10 text-on-surface-variant">"No server selected."</p> }
            .into_any();
    }
    view! {
        <div class="space-y-3 px-8 py-10">
            <p class="text-on-surface-variant">"No servers configured."</p>
            <p class="max-w-prose text-sm text-on-surface-variant">
                "Add the game server to issue its machine credentials, send it fleet commands and deploy missions to it."
            </p>
            <button
                type="button"
                data-testid="server-control-add-first"
                on:click=move |_| registry.open_to_register()
                class="flex items-center gap-1.5 rounded-full bg-action px-4 py-2 text-label-md font-medium text-on-action transition hover:bg-action/90"
            >
                <MaterialIcon name="add" class="text-[16px]" />
                "Add server"
            </button>
        </div>
    }
        .into_any()
}
