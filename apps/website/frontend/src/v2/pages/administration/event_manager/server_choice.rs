//! The operation's game server: the choice both forms offer, and the value each form sends.
//!
//! **Role:** renders the "Game server" choice the schedule and edit forms share — no server, or one
//! of the servers as read, a deactivated one marked — and words the `server_id` a new operation is
//! created with and the one an edit sends.
//! **Position:** read by the schedule and edit dialogs of the operations calendar.
//! **Signals & state:** writes the form's chosen server id, empty for none; reads the [`Manager`]'s
//! server list, which is read while either form is open.
//! **Invariants:** the server an operation is scheduled on is the one whose game runtime may read
//! its roster and whose mission deployments may bind its seats. An edit sends `server_id` only when
//! the choice differs from the operation's, and `null` only to clear a server it had. While the
//! list is unread the choice cannot change, so a list that failed to load never clears an
//! operation's server.
//!
//! [`Manager`]: super::state::Manager

use crate::v2::core::api::dto::ServerRowDto;
use leptos::prelude::*;
use serde_json::Value;

/// The servers both forms offer: none wanted while both are shut, the list, or a failed read.
#[derive(Clone, PartialEq)]
pub(super) enum ServerChoices {
    Idle,
    Failed,
    Loaded(Vec<ServerRowDto>),
}

/// A server as the choice names it: its name, marked when it is deactivated.
pub(super) fn server_choice_label(server: &ServerRowDto) -> String {
    if server.is_active {
        server.name.clone()
    } else {
        format!("{} (deactivated)", server.name)
    }
}

/// The `server_id` a new operation is created with: the chosen id, or none.
pub(super) fn chosen_server_id(chosen: &str) -> Option<String> {
    let id = chosen.trim();
    (!id.is_empty()).then(|| id.to_string())
}

/// The `server_id` an edit sends: nothing when the choice is the operation's server, `null` to
/// clear the server it had, else the chosen id.
pub(super) fn server_id_change(current: Option<&str>, chosen: &str) -> Option<Value> {
    let chosen = chosen_server_id(chosen);
    if chosen.as_deref() == current {
        return None;
    }
    Some(chosen.map_or(Value::Null, Value::String))
}

/// The "Game server" choice of one form, writing `chosen`.
///
/// The option chosen when the list arrives is marked `selected`, because the select's value is
/// written before its options exist.
pub(super) fn server_picker(
    servers: LocalResource<ServerChoices>,
    chosen: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="mt-6">
            <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                "Game server"
            </p>
            {move || match servers.get() {
                Some(ServerChoices::Loaded(list)) => {
                    let current = chosen.get_untracked();
                    view! {
                        <select
                            aria-label="Game server"
                            data-testid="operation-game-server"
                            prop:value=move || chosen.get()
                            on:change=move |ev| chosen.set(event_target_value(&ev))
                            class="w-full rounded-full border border-white/10 bg-white/5 px-5 py-3 text-sm text-on-surface outline-none focus:border-primary/50"
                        >
                            <option value="" selected=current.is_empty()>
                                "No game server"
                            </option>
                            {list
                                .iter()
                                .map(|server| {
                                    let selected = server.id == current;
                                    view! {
                                        <option value=server.id.clone() selected=selected>
                                            {server_choice_label(server)}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    }
                        .into_any()
                }
                Some(ServerChoices::Failed) => {
                    view! {
                        <p class="px-1 text-xs text-error-alert">
                            "The game servers could not be read; the operation's server stays as it is."
                        </p>
                    }
                        .into_any()
                }
                Some(ServerChoices::Idle) | None => {
                    view! {
                        <p class="px-1 text-xs text-on-surface-variant">"Reading the game servers…"</p>
                    }
                        .into_any()
                }
            }}
            <p class="mt-2 px-1 text-xs text-on-surface-variant/70">
                "Its game runtime reads this operation's roster, and its mission deployments may bind the operation's seats."
            </p>
        </div>
    }
}

#[cfg(test)]
#[path = "tests/server_choice.rs"]
mod tests;
