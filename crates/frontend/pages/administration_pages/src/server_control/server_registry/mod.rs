//! The server registry: the configured game servers as read, which one is selected, and the writes
//! that register a server, change its registration, and take it out of service or back into it.
//!
//! **Role:** declares the registration sheet and its wording, and holds the server list the picker
//! and the card read, the selection, the sheet's state and the required-modpack choices, with the
//! list read and the four writes.
//! **Position:** created once by the server control screen; the picker, the selected server's card
//! and the registration sheet all read it.
//! **Signals & state:** [`ServerRegistry`] is one copyable handle: whether the list has been read,
//! the rows, the selected id, whether the sheet is open and which server it edits, the in-flight
//! flag, the last refusal and the modpack choices.
//! **Invariants:** the list is read once, and every accepted write changes only the one row it
//! concerns, so the rest of the screen — each card's command console, deployments and credential
//! sheet — keeps its state: a registration appends the answered row and selects it, a change or a
//! reactivation replaces the row with the answered one, and a deactivation, answered with no body,
//! marks the row inactive, the one field the backend changes. A refused write changes no row and
//! leaves its sentence in the sheet. Every request is browser-only, as is the panel itself.

mod registration_sheet;
pub(super) mod registration_wording;

#[cfg(target_arch = "wasm32")]
pub(super) use registration_sheet::registration_sheet;

#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::ServerRowDto;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::{ModpackDto, ServerChange, ServerRegistration};
#[cfg(target_arch = "wasm32")]
use frontend_session::AuthStore;
#[cfg(target_arch = "wasm32")]
use frontend_ui::toast::Toasts;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Whether the server list has been read: not yet, failed, or answered into the rows.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListRead {
    Loading,
    Failed,
    Loaded,
}

/// The required-modpack choices as read: not yet, in flight, refused with a reason, or answered.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, PartialEq)]
pub(super) enum ModpackChoices {
    Idle,
    Loading,
    Failed(String),
    Loaded(Vec<ModpackDto>),
}

/// Every signal the picker, the card and the registration sheet read.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(super) struct ServerRegistry {
    pub(super) store: AuthStore,
    pub(super) toasts: Toasts,
    pub(super) read: RwSignal<ListRead>,
    /// Every server, in the order the list was read, registrations appended.
    pub(super) servers: RwSignal<Vec<ServerRowDto>>,
    /// The server the card shows.
    pub(super) selected_id: RwSignal<String>,
    pub(super) sheet_open: RwSignal<bool>,
    /// The server the sheet edits; `None` while it registers a new one.
    pub(super) editing: RwSignal<Option<String>>,
    pub(super) busy: RwSignal<bool>,
    /// The sentence the last refused write came back with.
    pub(super) refusal: RwSignal<Option<String>>,
    pub(super) modpacks: RwSignal<ModpackChoices>,
}

/// The server the screen opens on: the first active one, else the first, else none.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn pick_default_id(servers: &[ServerRowDto]) -> Option<String> {
    servers
        .iter()
        .find(|s| s.is_active)
        .or_else(|| servers.first())
        .map(|s| s.id.to_string())
}

#[cfg(target_arch = "wasm32")]
impl ServerRegistry {
    /// An unread list and a shut sheet.
    pub(super) fn new(store: AuthStore, toasts: Toasts) -> Self {
        Self {
            store,
            toasts,
            read: RwSignal::new(ListRead::Loading),
            servers: RwSignal::new(Vec::new()),
            selected_id: RwSignal::new(String::new()),
            sheet_open: RwSignal::new(false),
            editing: RwSignal::new(None),
            busy: RwSignal::new(false),
            refusal: RwSignal::new(None),
            modpacks: RwSignal::new(ModpackChoices::Idle),
        }
    }

    /// Read the server list and select the server the screen opens on.
    pub(super) fn load(self) {
        leptos::task::spawn_local(async move {
            use frontend_transport::endpoints::server_registry::load_servers;
            match load_servers(self.store).await {
                Ok(list) => {
                    let _ = self
                        .selected_id
                        .try_set(pick_default_id(&list.data).unwrap_or_default());
                    let _ = self.servers.try_set(list.data);
                    let _ = self.read.try_set(ListRead::Loaded);
                }
                Err(_) => {
                    let _ = self.read.try_set(ListRead::Failed);
                }
            }
        });
    }

    /// One server's row as last read or written, tracked.
    pub(super) fn row(self, server_id: &str) -> Option<ServerRowDto> {
        self.servers
            .with(|list| list.iter().find(|s| s.id == server_id).cloned())
    }

    /// One server's row as last read or written, untracked.
    pub(super) fn row_untracked(self, server_id: &str) -> Option<ServerRowDto> {
        self.servers
            .with_untracked(|list| list.iter().find(|s| s.id == server_id).cloned())
    }

    /// Open the sheet on a server to register.
    pub(super) fn open_to_register(self) {
        self.editing.set(None);
        self.open_sheet();
    }

    /// Open the sheet on a registered server.
    pub(super) fn open_to_edit(self, server_id: String) {
        self.editing.set(Some(server_id));
        self.open_sheet();
    }

    /// Open the sheet with no refusal on it, and read the modpack choices again.
    fn open_sheet(self) {
        self.refusal.set(None);
        self.sheet_open.set(true);
        let _ = self.modpacks.try_update(|choices| {
            if !matches!(choices, ModpackChoices::Loaded(_)) {
                *choices = ModpackChoices::Loading;
            }
        });
        leptos::task::spawn_local(async move {
            use frontend_transport::endpoints::server_registry::load_required_modpack_choices;
            let read = load_required_modpack_choices(self.store).await;
            let _ = self.modpacks.try_set(match read {
                Ok(list) => ModpackChoices::Loaded(list.data),
                Err(e) => ModpackChoices::Failed(e.message_or("The modpacks could not be read")),
            });
        });
    }

    /// Register a server; its row joins the list and is selected, and the sheet closes.
    pub(super) fn register(self, registration: ServerRegistration) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        {
            use frontend_transport::endpoints::server_registry::register_server;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                match register_server(self.store, &registration).await {
                    Ok(row) => {
                        self.toasts.success(format!("Registered {}", row.name));
                        let id = row.id.to_string();
                        let _ = self
                            .servers
                            .try_update(|list| registration_wording::place_row(list, row));
                        let _ = self.selected_id.try_set(id);
                        let _ = self.sheet_open.try_set(false);
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(
                            refused.message_or("The server could not be registered"),
                        ));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
    }

    /// Change a server's registration; its row becomes the answered one, and the sheet closes.
    pub(super) fn save_change(self, server_id: String, change: ServerChange) {
        self.send_change(
            server_id,
            change,
            |name| format!("Saved {name}"),
            "The server's registration could not be changed",
        );
    }

    /// Put a deactivated server back into service.
    pub(super) fn reactivate(self, server_id: String) {
        self.send_change(
            server_id,
            registration_wording::reactivation(),
            |name| format!("Reactivated {name}"),
            "The server could not be reactivated",
        );
    }

    /// Send one change; `accepted` words the toast for the answered row's name, and `fallback` is
    /// the refusal shown when the backend sends no sentence.
    fn send_change(
        self,
        server_id: String,
        change: ServerChange,
        accepted: fn(&str) -> String,
        fallback: &'static str,
    ) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        {
            use frontend_transport::endpoints::server_registry::change_server;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                match change_server(self.store, &server_id.as_str().into(), &change).await {
                    Ok(row) => {
                        self.toasts.success(accepted(&row.name));
                        let _ = self
                            .servers
                            .try_update(|list| registration_wording::place_row(list, row));
                        let _ = self.sheet_open.try_set(false);
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(refused.message_or(fallback)));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
    }

    /// Take a server out of service; its row is marked inactive, and the sheet closes.
    pub(super) fn deactivate(self, server_id: String) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        {
            use frontend_transport::endpoints::server_registry::deactivate_server;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                match deactivate_server(self.store, &server_id.as_str().into()).await {
                    Ok(()) => {
                        let name = self.row_untracked(&server_id).map(|row| row.name);
                        self.toasts.success(format!(
                            "Deactivated {}",
                            name.unwrap_or_else(|| "the server".to_string())
                        ));
                        let _ = self.servers.try_update(|list| {
                            registration_wording::mark_deactivated(list, &server_id)
                        });
                        let _ = self.sheet_open.try_set(false);
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(
                            refused.message_or("The server could not be deactivated"),
                        ));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
    }
}

#[cfg(test)]
#[path = "tests/server_registry.rs"]
mod tests;
