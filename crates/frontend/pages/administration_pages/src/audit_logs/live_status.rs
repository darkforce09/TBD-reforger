//! The live status of the audit route: where the history stands, what the badge says, and when the
//! history loads without waiting for the stream.
//!
//! **Role:** the history load state, the badge label and colour for a stream state and a history
//! state, the note shown when the stream is offline, and the badge itself with the live row count.
//! **Position:** read by the audit route's stream callbacks and rendered in the master header next
//! to the filter box.
//! **Signals & state:** the badge reads the route's stream state, history state and board signals;
//! the rest is pure.
//! **Invariants:** a history reload in flight outranks the connection state on the badge, because
//! the trail is incomplete until it lands; an offline stream outranks both, because nothing will
//! arrive. The history waits for the stream's `ready`, except when the first connection fails
//! before one: then it loads at once so the trail never hangs on a stream that is down.

#[cfg(target_arch = "wasm32")]
use super::live_merge::AuditBoard;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_transport::audit_stream::{AuditStreamState, OfflineReason};
#[cfg(target_arch = "wasm32")]
use frontend_ui::badge_class;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Where the history half of the board stands.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryLoad {
    /// Nothing requested yet: the route waits for the stream's `ready`.
    Waiting,
    /// The first history page is in flight.
    Loading,
    /// A reload after a reset or a fresh `ready` is in flight; the board was emptied for it.
    Reloading,
    /// The latest requested page arrived.
    Loaded,
    /// The latest requested page failed.
    Failed,
}

/// The state a history (re)load enters: the first load is `Loading`, any later one `Reloading`.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn history_load_start(current: HistoryLoad) -> HistoryLoad {
    match current {
        HistoryLoad::Waiting | HistoryLoad::Loading => HistoryLoad::Loading,
        HistoryLoad::Reloading | HistoryLoad::Loaded | HistoryLoad::Failed => {
            HistoryLoad::Reloading
        }
    }
}

/// Whether the history must load now without the stream: nothing was requested and the stream
/// has dropped or stopped before its first `ready`.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn history_fallback_due(history: HistoryLoad, stream: AuditStreamState) -> bool {
    history == HistoryLoad::Waiting
        && matches!(
            stream,
            AuditStreamState::Reconnecting | AuditStreamState::Offline(_)
        )
}

/// The badge's label and its colour variant.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn status_badge(
    stream: AuditStreamState,
    history: HistoryLoad,
) -> (&'static str, &'static str) {
    match (stream, history) {
        (AuditStreamState::Offline(_), _) => ("Offline", "error"),
        (_, HistoryLoad::Reloading) => ("Reloading", "warning"),
        (AuditStreamState::Live, _) => ("Live", "success"),
        (AuditStreamState::Reconnecting, _) => ("Reconnecting", "warning"),
        (AuditStreamState::Connecting, _) => ("Connecting", "neutral"),
    }
}

/// Why the stream is offline, in words, or nothing while it is not.
#[cfg(target_arch = "wasm32")]
pub(crate) fn offline_note(stream: AuditStreamState) -> Option<&'static str> {
    match stream {
        AuditStreamState::Offline(OfflineReason::SignedOut) => {
            Some("The session ended; sign in again for live updates.")
        }
        AuditStreamState::Offline(OfflineReason::Forbidden) => {
            Some("Live updates need the admin role.")
        }
        _ => None,
    }
}

/// How many live lines the board holds, in words.
#[cfg(target_arch = "wasm32")]
pub(crate) fn live_count_label(count: usize) -> String {
    match count {
        1 => "1 live row".into(),
        n => format!("{n} live rows"),
    }
}

/// The status badge, the live row count and, when offline, the reason.
#[cfg(target_arch = "wasm32")]
pub(crate) fn live_status(
    stream: RwSignal<AuditStreamState>,
    history: RwSignal<HistoryLoad>,
    board: RwSignal<AuditBoard>,
    rejected: RwSignal<Option<String>>,
) -> AnyView {
    view! {
        <div class="flex shrink-0 flex-col items-end gap-1">
            <div class="flex items-center gap-2">
                <span
                    class="font-mono text-xs text-on-surface-variant"
                    data-testid="audit-live-count"
                >
                    {move || live_count_label(board.with(AuditBoard::live_count))}
                </span>
                <span
                    class=move || badge_class(status_badge(stream.get(), history.get()).1)
                    data-testid="audit-live-status"
                >
                    {move || status_badge(stream.get(), history.get()).0}
                </span>
            </div>
            {move || {
                offline_note(stream.get())
                    .map(|note| view! { <p class="font-mono text-xs text-error-alert">{note}</p> })
            }}
            {move || {
                rejected
                    .get()
                    .map(|text| view! { <p class="font-mono text-xs text-tactical-yellow">{text}</p> })
            }}
        </div>
    }
    .into_any()
}

#[cfg(test)]
#[path = "tests/live_status.rs"]
mod tests;
