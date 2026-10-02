//! The fleet scenario registry: which scenario header the fleet runs for each terrain.
//!
//! **Role:** declares the registry sheet and its wording, and holds the sheet's state with its
//! reads and its two changes: register or replace a terrain's scenario, and remove one.
//! **Position:** a side sheet over the server control screen, opened from the picker's heading —
//! the registry belongs to the whole fleet, not to one server.
//! **Signals & state:** [`ScenarioRegistry`] is one copyable handle created by the screen: whether
//! the sheet is open, the registry as read, the in-flight flag and the last refusal.
//! **Invariants:** a deployment of an artifact is refused for a terrain with no registered scenario,
//! so the registry is read whole every time the sheet opens and after every change. Removing a
//! terrain only stops offering it to new deployments. Every request is browser-only; a native build
//! keeps the registry idle.

mod scenario_sheet;
pub(super) mod scenario_wording;

pub(super) use scenario_sheet::scenario_sheet;

use crate::v2::core::api::dto::{FleetScenario, FleetScenarioUpdate};
use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::toast::Toasts;
use leptos::prelude::*;

/// The registry as read: not yet, in flight, refused with a reason, or answered.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum RegistryRead {
    Idle,
    Loading,
    Failed(String),
    Loaded(Vec<FleetScenario>),
}

/// Every signal the fleet scenario sheet runs on.
#[derive(Clone, Copy)]
pub(super) struct ScenarioRegistry {
    pub(super) store: AuthStore,
    pub(super) toasts: Toasts,
    pub(super) open: RwSignal<bool>,
    pub(super) registry: RwSignal<RegistryRead>,
    pub(super) busy: RwSignal<bool>,
    pub(super) refusal: RwSignal<Option<String>>,
}

impl ScenarioRegistry {
    /// A shut sheet.
    pub(super) fn new(store: AuthStore, toasts: Toasts) -> Self {
        Self {
            store,
            toasts,
            open: RwSignal::new(false),
            registry: RwSignal::new(RegistryRead::Idle),
            busy: RwSignal::new(false),
            refusal: RwSignal::new(None),
        }
    }

    /// Open the sheet and read the registry.
    pub(super) fn open_sheet(self) {
        self.open.set(true);
        self.refusal.set(None);
        self.reload();
    }

    /// Read the registry again.
    pub(super) fn reload(self) {
        let _ = self.registry.try_update(|registry| {
            if !matches!(registry, RegistryRead::Loaded(_)) {
                *registry = RegistryRead::Loading;
            }
        });
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            use crate::v2::core::api::endpoints::fleet_scenarios::load_fleet_scenarios;
            let read = load_fleet_scenarios(self.store).await;
            let _ = self.registry.try_set(match read {
                Ok(list) => RegistryRead::Loaded(list.items),
                Err(e) => RegistryRead::Failed(crate::v2::core::api::client::api_error_message(
                    &e,
                    "The fleet scenarios could not be read",
                )),
            });
        });
    }

    /// Register the terrain's scenario, or replace the one registered; `on_saved` runs once stored.
    pub(super) fn put(
        self,
        terrain_key: String,
        update: FleetScenarioUpdate,
        on_saved: impl FnOnce() + 'static,
    ) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            use crate::v2::core::api::endpoints::fleet_scenarios::put_fleet_scenario;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                match put_fleet_scenario(self.store, &terrain_key, &update).await {
                    Ok(stored) => {
                        self.toasts.success(format!(
                            "Terrain {} runs {}",
                            stored.terrain_key, stored.scenario_id
                        ));
                        on_saved();
                        self.reload();
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(
                            refused.message_or("The scenario could not be registered"),
                        ));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (terrain_key, update, on_saved);
    }

    /// Stop offering a terrain to new deployments.
    pub(super) fn remove(self, terrain_key: String) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            use crate::v2::core::api::endpoints::fleet_scenarios::delete_fleet_scenario;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                match delete_fleet_scenario(self.store, &terrain_key).await {
                    Ok(()) => {
                        self.toasts.success(format!(
                            "Terrain {terrain_key} is no longer offered to new deployments"
                        ));
                        self.reload();
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(
                            refused.message_or("The scenario could not be removed"),
                        ));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = terrain_key;
    }
}

#[cfg(test)]
#[path = "tests/fleet_scenarios.rs"]
mod tests;
