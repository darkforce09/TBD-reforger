//! The combat history table: one row per mission the caller has flown.
//!
//! **Role:** renders the service-record table — date, operation, role played, outcome and the
//! after-action replay link — and owns the outcome labels and the decision about which stored
//! replay strings may become an anchor.
//! **Position:** the combat-history section of the right-hand column.
//! **Signals & state:** none — the table is built from the values handed to it.
//! **Invariants:** the replay cell emits an `href` only for an `http(s)` URL. A stored
//! `javascript:` or `data:` value takes the same inert em dash an absent replay does, because a
//! scheme like that is not a quote breakout an escape could neutralise — the attribute's
//! *content* runs, so the only safe move at the sink is not to emit it.
#![allow(dead_code)]

use super::page::vstr;
use super::table_head::ServiceHead;
use crate::v2::core::auth::url_guard;
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::datefmt::format_short_date;
use leptos::prelude::*;
use serde_json::Value;

/// The service-record label a `mission_outcome` is shown as.
fn outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "success" => "Mission Success",
        "failure" => "Failed",
        "aborted" => "Aborted",
        "pending" => "Pending",
        _ => "Unknown",
    }
}

/// The badge variant a `mission_outcome` is shown in.
fn outcome_variant(outcome: &str) -> &'static str {
    match outcome {
        "success" => "success",
        "failure" => "error",
        "aborted" => "warning",
        _ => "neutral",
    }
}

/// The service-record table.
///
/// A real `<table>` rather than a grid of divs: it is tabular data, and the wide personnel roster
/// is the platform's precedent for one.
pub(super) fn service_record(history: Vec<Value>) -> impl IntoView {
    view! {
        <div class="custom-scrollbar overflow-x-auto rounded-xl border border-white/10">
            <table class="w-full min-w-[40rem] border-collapse text-left text-sm">
                <thead>
                    <tr class="border-b border-white/10 bg-surface-container-lowest/40">
                        <ServiceHead label="Date" />
                        <ServiceHead label="Operation" />
                        <ServiceHead label="Role Played" />
                        <ServiceHead label="Outcome" />
                        <ServiceHead label="AAR" />
                    </tr>
                </thead>
                <tbody>
                    {history
                        .into_iter()
                        .map(|h| {
                            let date = vstr(&h, "date");
                            let operation = vstr(&h, "operation");
                            let operation = if operation.is_empty() {
                                "—".to_string()
                            } else {
                                operation
                            };
                            let role = vstr(&h, "role");
                            let role = if role.is_empty() { "—".to_string() } else { role };
                            let outcome = vstr(&h, "outcome");
                            let replay = vstr(&h, "aar_replay_url");
                            view! {
                                <tr class="border-b border-white/5 transition last:border-b-0 hover:bg-white/[0.02]">
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&date)}
                                    </td>
                                    <td class="px-4 py-3 font-medium text-on-surface">{operation}</td>
                                    <td class="px-4 py-3 font-mono text-xs text-on-surface-variant">
                                        {role}
                                    </td>
                                    <td class="px-4 py-3">
                                        <span class=badge_class(
                                            outcome_variant(&outcome),
                                        )>{outcome_label(&outcome)}</span>
                                    </td>
                                    <td class="px-4 py-3">
                                        {match replay_href(&replay) {
                                            None => {
                                                view! {
                                                    <span class="font-mono text-xs text-outline">"—"</span>
                                                }
                                                    .into_any()
                                            }
                                            Some(href) => {
                                                view! {
                                                    <a
                                                        href=href
                                                        target="_blank"
                                                        rel="noreferrer"
                                                        class="inline-flex items-center gap-1 font-mono text-xs tracking-wider text-primary uppercase transition hover:underline"
                                                    >
                                                        <MaterialIcon name="play_circle" class="text-base" />
                                                        "View Replay"
                                                    </a>
                                                }
                                                    .into_any()
                                            }
                                        }}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

/// The replay cell's one decision: is this stored string safe to put in an `href`, or does the
/// row get the inert em dash?
///
/// An empty string carries no scheme, so it answers `false` here and takes the same em dash it
/// would anyway. This deliberately duplicates the API's write-boundary check: that guard governs
/// values written after it shipped, and this one governs every value that reaches the table
/// whatever door it came in by — an older row, an operator's `psql`, a writer added later. They
/// fail independently, which is the point of guarding an output.
///
/// Lifted out of the view so it can be tested: the crate renders in a browser and cannot render
/// to a string natively, so a returned `Option` is the largest testable unit this cell has.
pub(super) fn replay_href(replay: &str) -> Option<&str> {
    url_guard::is_http_url(replay).then_some(replay)
}
