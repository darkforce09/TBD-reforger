//! The command this console is following, and the server's command history.
//!
//! **Role:** the followed command's receipt — accepted, in flight, or how it ended, with an explicit
//! unknown outcome for an indeterminate one — and every receipt of the server with its state,
//! action, executor, who requested it and when, how many claims it took, its arguments, its
//! observed outcome and its failure reason, and a cancel control on a queued one.
//! **Position:** under the request controls in the fleet command section of the selected server's
//! card.
//! **Signals & state:** reads the [`CommandConsole`]; cancels through it.
//! **Invariants:** a receipt's state is shown as its own words, never promoted: a queued command
//! reads as queued, an indeterminate one as an unknown outcome. Only a queued command offers a
//! cancel control, because only a command no executor has claimed can be cancelled. The executor's
//! fencing token belongs to the claim and is not part of a receipt, so the history shows the
//! claim count instead.

use super::super::machine_credentials::executor_label;
use super::command_wording::{
    action_label, arguments_summary, in_flight, outcome_summary, receipt_outcome, state_label,
    state_tone, ReceiptOutcome,
};
use super::{CommandConsole, CommandHistory};
use crate::v2::core::api::dto::FleetCommandReceipt;
use crate::v2::core::ui::badge_class;
use crate::v2::core::utils::utc_timestamp::utc_label;
use leptos::prelude::*;

/// The followed command, then the history.
pub(in super::super) fn command_history(console: CommandConsole) -> impl IntoView {
    let me = StoredValue::new(console.store.user.get_untracked().map(|u| u.discord_id));
    view! {
        <div class="space-y-4">
            {move || console.followed.get().map(followed_panel)}
            <div class="flex items-center justify-between">
                <h4 class="font-mono text-label-sm tracking-widest text-outline uppercase">"History"</h4>
                <button type="button"
                    class="rounded-full border border-white/10 px-3 py-1 text-xs text-on-surface-variant hover:bg-white/5"
                    on:click=move |_| console.reload()>
                    "Refresh"
                </button>
            </div>
            {move || match console.history.get() {
                CommandHistory::Loaded(receipts) if receipts.is_empty() => {
                    view! { <p class="text-sm text-on-surface-variant">"No command has been requested for this server yet."</p> }.into_any()
                }
                CommandHistory::Loaded(receipts) => view! {
                    <ul class="space-y-2" data-testid="fleet-command-history">
                        {receipts
                            .into_iter()
                            .map(|receipt| history_row(console, receipt, me.get_value()))
                            .collect_view()}
                    </ul>
                }
                .into_any(),
                CommandHistory::Failed(why) => view! { <p class="text-sm text-error-alert">{why}</p> }.into_any(),
                CommandHistory::Loading | CommandHistory::Idle => {
                    view! { <p class="text-sm text-on-surface-variant">"Loading the command history…"</p> }.into_any()
                }
            }}
        </div>
    }
}

/// The command this console requested: still on its way, or how it ended.
fn followed_panel(receipt: FleetCommandReceipt) -> impl IntoView {
    let (tone, text) = match receipt_outcome(&receipt) {
        None => (
            "border-primary/30 bg-primary/10",
            format!(
                "{} is {} — following it until it finishes.",
                action_label(&receipt.action),
                state_label(&receipt.state).to_lowercase()
            ),
        ),
        Some(ReceiptOutcome::Succeeded(text)) => ("border-success/30 bg-success/10", text),
        Some(ReceiptOutcome::Failed(text)) => ("border-error-alert/30 bg-error-alert/10", text),
        Some(ReceiptOutcome::NotRun(text)) => ("border-white/10 bg-white/5", text),
        Some(ReceiptOutcome::Unknown(text)) => {
            ("border-tactical-yellow/40 bg-tactical-yellow/10", text)
        }
    };
    view! {
        <div role="status" data-testid="fleet-command-followed"
            class=format!("rounded-xl border px-4 py-3 text-sm text-on-surface {tone}")>
            <p class="font-mono text-label-sm tracking-widest text-outline uppercase">"Your command"</p>
            <p class="mt-1">{text}</p>
        </div>
    }
}

/// One receipt of the history.
fn history_row(
    console: CommandConsole,
    receipt: FleetCommandReceipt,
    me: Option<String>,
) -> impl IntoView {
    let requester = if me.as_deref() == Some(receipt.requested_by.as_str()) {
        "you".to_string()
    } else {
        receipt.requested_by.clone()
    };
    let queued = receipt.state == "queued";
    let command_id = StoredValue::new(receipt.id.clone());
    let stamps = [
        Some(format!(
            "Requested by {requester}, {}",
            utc_label(&receipt.requested_at)
        )),
        receipt
            .claimed_at
            .as_ref()
            .map(|at| format!("claimed {}", utc_label(at))),
        receipt
            .executing_at
            .as_ref()
            .map(|at| format!("executing {}", utc_label(at))),
        receipt
            .finished_at
            .as_ref()
            .map(|at| format!("finished {}", utc_label(at))),
        in_flight(&receipt.state).then(|| format!("expires {}", utc_label(&receipt.expires_at))),
    ];
    let stamps = stamps.into_iter().flatten().collect::<Vec<_>>().join(" · ");
    view! {
        <li class="rounded-xl border border-white/10 p-3 text-sm">
            <div class="flex flex-wrap items-center justify-between gap-2">
                <p class="flex flex-wrap items-center gap-2 text-on-surface">
                    <span class=badge_class(state_tone(&receipt.state))>{state_label(&receipt.state)}</span>
                    <span class="font-medium">{action_label(&receipt.action)}</span>
                    <span class="text-xs text-on-surface-variant">
                        {format!("{} · {} claim(s)", executor_label(&receipt.executor_kind), receipt.attempts)}
                    </span>
                </p>
                {queued
                    .then(|| {
                        view! {
                            <button type="button"
                                class="rounded-full border border-error-alert/30 px-3 py-1 text-xs text-error-alert hover:bg-error-alert/10 disabled:opacity-50"
                                prop:disabled=move || console.busy.get()
                                on:click=move |_| console.cancel(command_id.get_value())>
                                "Cancel"
                            </button>
                        }
                    })}
            </div>
            <p class="mt-1 text-xs text-on-surface-variant">{stamps}</p>
            <p class="mt-1 break-all font-mono text-code-md text-outline">
                {format!("Arguments — {}", arguments_summary(&receipt))}
            </p>
            {outcome_summary(&receipt)
                .map(|observed| {
                    view! { <p class="mt-1 break-all text-xs text-on-surface">{format!("Outcome — {observed}")}</p> }
                })}
            {receipt
                .failure_reason
                .clone()
                .map(|why| view! { <p class="mt-1 text-xs text-error-alert">{why}</p> })}
            {(receipt.state == "indeterminate")
                .then(|| {
                    view! {
                        <p class="mt-1 text-xs text-tactical-yellow">
                            "Outcome unknown: the executor stopped reporting after the command started. Nothing repeats it — inspect the server before issuing it again."
                        </p>
                    }
                })}
        </li>
    }
}
