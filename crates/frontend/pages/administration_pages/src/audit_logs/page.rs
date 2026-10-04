//! The audit route: the live stream and the paged history, merged into one trail.
//!
//! **Role:** puts the trail behind the administrator gate, opens the live audit stream, loads the
//! history when the stream says to, and hands the merged board to the panes below. Owns the list
//! paths and the cursor read the history loads use.
//! **Position:** the `/admin/audit` route, rendered inside the navigation frame.
//! **Signals & state:** owns the board, the stream state, the history state and the latest
//! rejected-event note, and the page-owned stream handle, which cleanup aborts.
//! **Invariants:** the stream connects first and the history loads after its `ready`, so no line
//! committed in between is missed; a `reset`, or a `ready` on a connection that could not resume,
//! empties the board and reloads the history. When the first connection fails before any `ready`,
//! the history loads anyway, and a later fresh `ready` reloads it. The history is keyset-paged by
//! id, so lines written while an operator reads cannot shift the window. Requests and the stream
//! run in the browser build only, as does the page.

#[cfg(target_arch = "wasm32")]
use super::live_merge::AuditBoard;
#[cfg(target_arch = "wasm32")]
use super::live_status::{HistoryLoad, history_load_start};
#[cfg(target_arch = "wasm32")]
use super::log_table::board_view;
#[cfg(target_arch = "wasm32")]
use frontend_session::AdminGate;
#[cfg(target_arch = "wasm32")]
use frontend_session::AuthStore;
#[cfg(target_arch = "wasm32")]
use frontend_transport::audit_stream::AuditStreamState;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(any(target_arch = "wasm32", test))]
use serde_json::Value;

/// The listing path: the first page plainly, later pages below the smallest id already loaded.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn audit_logs_path(before: Option<i64>) -> String {
    match before {
        Some(id) => format!("/admin/audit-logs?before={id}"),
        None => "/admin/audit-logs".into(),
    }
}

/// Whether the server reports a further page: the cursor is a number on the wire, and anything
/// else is treated as the end rather than guessed at.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn parse_next_cursor(cursor: &Option<Value>) -> Option<i64> {
    cursor.as_ref().and_then(Value::as_i64)
}

/// The audit screen, behind the administrator gate.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn AuditLogsPage() -> impl IntoView {
    view! {
        <AdminGate>
            <AuditLogsInner />
        </AdminGate>
    }
}

/// The screen an administrator sees: the live stream, the history and the trail they feed.
#[cfg(target_arch = "wasm32")]
#[component]
fn AuditLogsInner() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let board = RwSignal::new(AuditBoard::new());
    let stream = RwSignal::new(AuditStreamState::Connecting);
    let history = RwSignal::new(HistoryLoad::Waiting);
    let rejected = RwSignal::new(None::<String>);
    connect_live_feed(store, board, stream, history, rejected);
    board_view(store, board, stream, history, rejected)
}

/// Open the audit stream for this page, wire it into the board, and abort it on cleanup.
#[cfg(target_arch = "wasm32")]
fn connect_live_feed(
    store: AuthStore,
    board: RwSignal<AuditBoard>,
    stream: RwSignal<AuditStreamState>,
    history: RwSignal<HistoryLoad>,
    rejected: RwSignal<Option<String>>,
) {
    {
        use super::live_status::history_fallback_due;
        use frontend_transport::audit_stream::{
            AuditStreamCallbacks, AuditStreamHandle, open_audit_stream,
        };
        let callbacks = AuditStreamCallbacks {
            on_state: Box::new(move |state| {
                let _ = stream.try_set(state);
                let waiting = history.try_get_untracked();
                if waiting.is_some_and(|load| history_fallback_due(load, state)) {
                    reload_history(store, board, history);
                }
            }),
            on_ready: Box::new(move |_ready, history_required| {
                if history_required {
                    reload_history(store, board, history);
                }
            }),
            on_row: Box::new(move |entry| {
                let _ = board.try_update(|b| b.insert_live(entry));
            }),
            on_reset: Box::new(move |_reset| reload_history(store, board, history)),
            on_rejected: Box::new(move |text| {
                let _ = rejected.try_set(Some(text));
            }),
        };
        let handle: StoredValue<AuditStreamHandle, LocalStorage> =
            StoredValue::new_local(open_audit_stream(store, callbacks));
        on_cleanup(move || {
            let _ = handle.try_with_value(|live| live.abort());
        });
    }
}

/// Empty the board and load the newest history page under the board's new epoch.
///
/// A page that lands after a later restart belongs to the history that restart replaced, and the
/// board drops it; only the load of the current epoch settles the history state.
#[cfg(target_arch = "wasm32")]
pub(crate) fn reload_history(
    store: AuthStore,
    board: RwSignal<AuditBoard>,
    history: RwSignal<HistoryLoad>,
) {
    let Some(epoch) = board.try_update(AuditBoard::restart) else {
        return;
    };
    let _ = history.try_update(|load| *load = history_load_start(*load));
    leptos::task::spawn_local(async move {
        use frontend_api_dtos::CursorList;
        use frontend_api_dtos::administration::AuditLogEntry;
        let path = audit_logs_path(None);
        let answer =
            frontend_transport::client::api_get::<CursorList<AuditLogEntry>>(store, &path).await;
        let loaded = answer.is_ok();
        let current = match answer {
            Ok(page) => board.try_update(|b| b.merge_history(epoch, page)),
            Err(_) => board.try_with_untracked(|b| b.epoch() == epoch),
        };
        if current == Some(true) {
            let settled = if loaded {
                HistoryLoad::Loaded
            } else {
                HistoryLoad::Failed
            };
            let _ = history.try_set(settled);
        }
    });
}
