//! The command console's request controls: process control, the player list, a broadcast and a
//! kick.
//!
//! **Role:** the Start, Stop, Restart and List players controls — Stop and Restart asking to be
//! confirmed first — the broadcast form, and the kick form with its player and session pickers.
//! **Position:** the top of the fleet command section of the selected server's card.
//! **Signals & state:** owns the pending confirmation, the broadcast message and the kick's three
//! fields; sends through the [`CommandConsole`].
//! **Invariants:** only the six actions an operator may request are offered; the two a mission
//! deployment issues are not. A broadcast and a kick are checked as the backend checks them before
//! anything is sent. A kick names the Arma identity and the runtime session it is issued against:
//! the players offered are the ones the newest successful player listing reported, and the session
//! offered is the one that confirmed the server's newest confirmed deployment — both are offered,
//! never filled in behind the operator's back.

use super::command_wording::{
    latest_player_listing, listed_players, validated_broadcast, validated_kick,
};
use super::{CommandConsole, CommandHistory};
use crate::v2::core::api::dto::FleetCommandRequest;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// Shared styling for the console's fields.
const FIELD: &str = "w-full rounded-md border border-outline-variant/40 bg-surface px-3 py-1.5 text-sm text-on-surface outline-none focus:border-primary/60";

/// Shared styling for the console's plain buttons.
const BUTTON: &str = "flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/10 disabled:opacity-50";

/// The process-control action waiting on its confirmation.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Disruptive {
    Stop,
    Restart,
}

/// The request controls of one server's console.
pub(in super::super) fn command_requests(
    console: CommandConsole,
    server_name: String,
    suggested_session: Signal<Option<String>>,
) -> impl IntoView {
    let confirming = RwSignal::new(None::<Disruptive>);
    let server_name = StoredValue::new(server_name);
    let send = move |request: FleetCommandRequest| {
        confirming.set(None);
        console.request(request);
    };
    view! {
        <div class="space-y-4">
            <div class="flex flex-wrap items-center gap-2">
                <button type="button" class=BUTTON data-testid="fleet-command-start"
                    prop:disabled=move || console.busy.get()
                    on:click=move |_| send(FleetCommandRequest::start())>
                    <MaterialIcon name="play_arrow" class="text-[16px]" />
                    "Start"
                </button>
                <button type="button" class=BUTTON data-testid="fleet-command-stop"
                    prop:disabled=move || console.busy.get()
                    on:click=move |_| confirming.set(Some(Disruptive::Stop))>
                    <MaterialIcon name="stop" class="text-[16px]" />
                    "Stop"
                </button>
                <button type="button" class=BUTTON data-testid="fleet-command-restart"
                    prop:disabled=move || console.busy.get()
                    on:click=move |_| confirming.set(Some(Disruptive::Restart))>
                    <MaterialIcon name="restart_alt" class="text-[16px]" />
                    "Restart"
                </button>
                <button type="button" class=BUTTON data-testid="fleet-command-list-players"
                    prop:disabled=move || console.busy.get()
                    on:click=move |_| send(FleetCommandRequest::list_players())>
                    <MaterialIcon name="groups" class="text-[16px]" />
                    "List players"
                </button>
            </div>
            {move || {
                confirming
                    .get()
                    .map(|action| {
                        let (question, confirm, request) = match action {
                            Disruptive::Stop => (
                                format!("Stop {}? Every connected player is disconnected.", server_name.get_value()),
                                "Stop the server",
                                FleetCommandRequest::stop(),
                            ),
                            Disruptive::Restart => (
                                format!("Restart {}? Every connected player is disconnected until it is back.", server_name.get_value()),
                                "Restart the server",
                                FleetCommandRequest::restart(),
                            ),
                        };
                        view! {
                            <div class="flex flex-wrap items-center gap-3 rounded-lg border border-error-alert/30 bg-error-alert/10 px-3 py-2 text-sm">
                                <span class="text-on-surface">{question}</span>
                                <button type="button"
                                    class="rounded-full bg-error-alert/20 px-4 py-1.5 text-label-sm text-error-alert"
                                    on:click=move |_| send(request.clone())>
                                    {confirm}
                                </button>
                                <button type="button" class=BUTTON on:click=move |_| confirming.set(None)>
                                    "Keep it running"
                                </button>
                            </div>
                        }
                    })
            }}
            {broadcast_form(console)}
            {kick_form(console, suggested_session)}
            {move || {
                console
                    .refusal
                    .get()
                    .map(|why| view! { <p role="alert" class="text-sm text-error-alert">{why}</p> })
            }}
        </div>
    }
}

/// The broadcast form: one message, shown to everyone in the running game.
fn broadcast_form(console: CommandConsole) -> impl IntoView {
    let message = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let send = move |_| match validated_broadcast(&message.get_untracked()) {
        Ok(text) => {
            problem.set(None);
            message.set(String::new());
            console.request(FleetCommandRequest::broadcast(&text));
        }
        Err(why) => problem.set(Some(why)),
    };
    view! {
        <div class="rounded-xl border border-white/10 p-3">
            <p class="mb-2 text-label-sm font-medium text-on-surface">"Broadcast to everyone in the game"</p>
            <div class="flex gap-2">
                <input aria-label="Broadcast message" placeholder="Message, at most 256 bytes"
                    prop:value=move || message.get()
                    on:input=move |ev| message.set(event_target_value(&ev))
                    class=FIELD />
                <button type="button" class=BUTTON data-testid="fleet-command-broadcast"
                    prop:disabled=move || console.busy.get() || message.with(|m| m.trim().is_empty())
                    on:click=send>
                    <MaterialIcon name="campaign" class="text-[16px]" />
                    "Broadcast"
                </button>
            </div>
            <p class="mt-1 flex justify-between text-xs">
                <span class="text-error-alert">{move || problem.get()}</span>
                <span class="font-mono text-outline">{move || format!("{} / 256 bytes", message.with(|m| m.trim().len()))}</span>
            </p>
        </div>
    }
}

/// The kick form: the player's Arma identity, the runtime session, and an optional reason.
fn kick_form(console: CommandConsole, suggested_session: Signal<Option<String>>) -> impl IntoView {
    let arma_id = RwSignal::new(String::new());
    let session = RwSignal::new(String::new());
    let reason = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let players = move || match console.history.get() {
        CommandHistory::Loaded(receipts) => latest_player_listing(&receipts)
            .map(listed_players)
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let send = move |_| match validated_kick(
        &arma_id.get_untracked(),
        &session.get_untracked(),
        &reason.get_untracked(),
    ) {
        Ok((identity, runtime_session, why)) => {
            problem.set(None);
            console.request(FleetCommandRequest::kick(
                &identity,
                &runtime_session,
                why.as_deref(),
            ));
        }
        Err(why) => problem.set(Some(why)),
    };
    view! {
        <div class="space-y-2 rounded-xl border border-white/10 p-3">
            <p class="text-label-sm font-medium text-on-surface">"Kick a player"</p>
            <div class="grid gap-2 md:grid-cols-2">
                <input aria-label="Player's Arma identity" placeholder="Arma identity (UID)"
                    prop:value=move || arma_id.get()
                    on:input=move |ev| arma_id.set(event_target_value(&ev))
                    class=FIELD />
                <input aria-label="Runtime session" placeholder="Runtime session id — the server's open session"
                    prop:value=move || session.get()
                    on:input=move |ev| session.set(event_target_value(&ev))
                    class=FIELD />
            </div>
            <div class="flex flex-wrap gap-1.5">
                {move || {
                    players()
                        .into_iter()
                        .map(|(identity, name)| {
                            let pick = identity.clone();
                            let label = if name.is_empty() { identity.clone() } else { name };
                            view! {
                                <button type="button" title=identity
                                    class="rounded-full border border-white/10 px-2.5 py-0.5 text-xs text-on-surface-variant hover:bg-white/5"
                                    on:click=move |_| arma_id.set(pick.clone())>
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()
                }}
                {move || {
                    suggested_session
                        .get()
                        .map(|id| {
                            let pick = id.clone();
                            view! {
                                <button type="button"
                                    class="rounded-full border border-primary/30 px-2.5 py-0.5 text-xs text-primary hover:bg-primary/10"
                                    on:click=move |_| session.set(pick.clone())>
                                    {format!("Use the session that confirmed the latest deployment ({id})")}
                                </button>
                            }
                        })
                }}
            </div>
            <div class="flex gap-2">
                <input aria-label="Kick reason" placeholder="Reason shown to the player (optional)"
                    prop:value=move || reason.get()
                    on:input=move |ev| reason.set(event_target_value(&ev))
                    class=FIELD />
                <button type="button" class=BUTTON data-testid="fleet-command-kick"
                    prop:disabled=move || console.busy.get()
                    on:click=send>
                    <MaterialIcon name="person_remove" class="text-[16px]" />
                    "Kick"
                </button>
            </div>
            <p class="text-xs text-error-alert">{move || problem.get()}</p>
        </div>
    }
}
