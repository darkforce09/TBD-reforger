//! One server's fleet command console: request a command, follow it to its outcome, read the history
//! and cancel what no executor has taken up.
//!
//! **Role:** declares the request controls, the followed-command panel with the history, and the
//! wording under them, and holds the console's state with its four operations — read the history,
//! request, follow and cancel.
//! **Position:** a section of the selected server's card on the server control screen.
//! **Signals & state:** [`CommandConsole`] is one copyable handle, created by the server card for
//! the server it shows. It holds the history as read, the command this console requested and is
//! following, the in-flight flag, the last refusal, and a follow generation that retires a
//! superseded or unmounted follow.
//! **Invariants:** a request's 202 is an acceptance only; the receipt is followed by reading
//! `GET /servers/:id/commands/:commandId` every two seconds until it reaches a terminal state, and
//! only then is its outcome announced — once, through [`command_wording::announce_receipt`]. A
//! follow stops when the console goes away, when a newer follow replaces it, or after five reads in
//! a row fail. Every request is browser-only; a native build keeps the history idle.

mod command_history;
mod command_requests;
pub(super) mod command_wording;

pub(super) use command_history::command_history;
pub(super) use command_requests::command_requests;

use crate::v2::core::api::dto::{FleetCommandReceipt, FleetCommandRequest};
use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::toast::Toasts;
use leptos::prelude::*;

/// How long the console waits between two reads of a followed command.
pub(super) const FOLLOW_INTERVAL_MS: u32 = 2_000;

/// How many reads of a followed command may fail in a row before the console stops following it.
pub(super) const FOLLOW_READ_FAILURES: u32 = 5;

/// The command history as read: not yet, in flight, refused with a reason, or answered.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum CommandHistory {
    Idle,
    Loading,
    Failed(String),
    Loaded(Vec<FleetCommandReceipt>),
}

/// Every signal the command console runs on.
#[derive(Clone, Copy)]
pub(super) struct CommandConsole {
    pub(super) store: AuthStore,
    pub(super) toasts: Toasts,
    pub(super) server_id: StoredValue<String>,
    pub(super) history: RwSignal<CommandHistory>,
    /// The command this console requested, as last read.
    pub(super) followed: RwSignal<Option<FleetCommandReceipt>>,
    pub(super) busy: RwSignal<bool>,
    /// What the last request or cancellation was refused with.
    pub(super) refusal: RwSignal<Option<String>>,
    follow_generation: StoredValue<u64>,
}

impl CommandConsole {
    /// The console of one server, reading its history at once.
    pub(super) fn new(store: AuthStore, toasts: Toasts, server_id: String) -> Self {
        let console = Self {
            store,
            toasts,
            server_id: StoredValue::new(server_id),
            history: RwSignal::new(CommandHistory::Idle),
            followed: RwSignal::new(None),
            busy: RwSignal::new(false),
            refusal: RwSignal::new(None),
            follow_generation: StoredValue::new(0),
        };
        console.reload();
        console
    }

    /// Read the history again. A history already on screen stays there until the new one lands.
    pub(super) fn reload(self) {
        let _ = self.history.try_update(|history| {
            if !matches!(history, CommandHistory::Loaded(_)) {
                *history = CommandHistory::Loading;
            }
        });
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            use crate::v2::core::api::endpoints::fleet_commands::load_fleet_commands;
            let Some(server) = self.server_id.try_get_value() else {
                return;
            };
            let read = load_fleet_commands(self.store, &server).await;
            let _ = self.history.try_set(match read {
                Ok(list) => CommandHistory::Loaded(list.items),
                Err(e) => CommandHistory::Failed(crate::v2::core::api::client::api_error_message(
                    &e,
                    "The command history could not be read",
                )),
            });
        });
    }

    /// Request one command, then follow its receipt to an outcome.
    pub(super) fn request(self, request: FleetCommandRequest) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            use self::command_wording::{accepted_line, command_refusal_sentence};
            use crate::v2::core::api::endpoints::fleet_commands::request_fleet_command;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                let Some(server) = self.server_id.try_get_value() else {
                    return;
                };
                match request_fleet_command(self.store, &server, &request).await {
                    Ok(receipt) => {
                        self.toasts.message(accepted_line(&receipt));
                        let id = receipt.id.clone();
                        let _ = self.followed.try_set(Some(receipt));
                        self.reload();
                        self.follow(id);
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(command_refusal_sentence(
                            &refused,
                            "The command could not be requested",
                        )));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = request;
    }

    /// Follow one command until it reaches a terminal state, then announce how it ended.
    #[cfg(target_arch = "wasm32")]
    pub(super) fn follow(self, command_id: String) {
        use self::command_wording::announce_receipt;
        use crate::v2::core::api::endpoints::fleet_commands::load_fleet_command;
        let generation = self.follow_generation.get_value().wrapping_add(1);
        self.follow_generation.set_value(generation);
        leptos::task::spawn_local(async move {
            let mut failures = 0;
            loop {
                gloo_timers::future::TimeoutFuture::new(FOLLOW_INTERVAL_MS).await;
                if self.follow_generation.try_get_value() != Some(generation) {
                    return;
                }
                let Some(server) = self.server_id.try_get_value() else {
                    return;
                };
                match load_fleet_command(self.store, &server, &command_id).await {
                    Ok(receipt) => {
                        failures = 0;
                        if self.followed.try_set(Some(receipt.clone())).is_some() {
                            return;
                        }
                        if announce_receipt(&receipt, &self.toasts) {
                            self.reload();
                            return;
                        }
                    }
                    Err((401, _)) => return,
                    Err(e) => {
                        failures += 1;
                        if failures >= FOLLOW_READ_FAILURES {
                            self.toasts.error(format!(
                                "Stopped following the command: {}",
                                crate::v2::core::api::client::api_error_message(
                                    &e,
                                    "its receipt could not be read"
                                )
                            ));
                            return;
                        }
                    }
                }
            }
        });
    }

    /// Cancel a command no executor has taken up.
    pub(super) fn cancel(self, command_id: String) {
        if self.busy.get_untracked() {
            return;
        }
        self.refusal.set(None);
        #[cfg(target_arch = "wasm32")]
        {
            use self::command_wording::{action_label, command_refusal_sentence};
            use crate::v2::core::api::endpoints::fleet_commands::cancel_fleet_command;
            self.busy.set(true);
            leptos::task::spawn_local(async move {
                let Some(server) = self.server_id.try_get_value() else {
                    return;
                };
                match cancel_fleet_command(self.store, &server, &command_id).await {
                    Ok(receipt) => {
                        let followed = self
                            .followed
                            .try_get_untracked()
                            .flatten()
                            .is_some_and(|f| f.id == receipt.id);
                        if followed {
                            // The cancellation is its outcome; the follow stands down.
                            if let Some(generation) = self.follow_generation.try_get_value() {
                                self.follow_generation.set_value(generation.wrapping_add(1));
                            }
                            let _ = self.followed.try_set(Some(receipt.clone()));
                        }
                        self.toasts.message(format!(
                            "{} cancelled — nothing ran",
                            action_label(&receipt.action)
                        ));
                        self.reload();
                    }
                    Err(refused) => {
                        let _ = self.refusal.try_set(Some(command_refusal_sentence(
                            &refused,
                            "The command could not be cancelled",
                        )));
                    }
                }
                let _ = self.busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = command_id;
    }
}

#[cfg(test)]
#[path = "tests/fleet_commands.rs"]
mod tests;
