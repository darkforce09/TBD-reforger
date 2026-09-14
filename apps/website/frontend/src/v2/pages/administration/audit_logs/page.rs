//! The audit route: the trail of administrative actions, newest first.
//!
//! **Role:** fetches the first page of the trail, puts it behind the administrator gate, and hands
//! it to the pane below. Owns the keyset paging helpers the load control uses.
//! **Position:** the `/admin/audit` route, rendered inside the navigation frame.
//! **Signals & state:** none of its own; the trail's state belongs to the pane it renders.
//! **Invariants:** the trail is keyset-paged, not offset-paged: the next page is asked for by the
//! last id already seen, so entries written while an operator is reading cannot shift the window
//! and hide a row. Entries are read as free-form values, because an audit record's fields differ
//! by action; only the severity is a fixed vocabulary. The request future is not `Send`, so a
//! native build resolves it to nothing and renders the failure branch.
#![allow(dead_code)]

use super::log_table::board;
use crate::v2::core::api::dto::CursorList;
use crate::v2::core::ui::AdminGate;
use leptos::prelude::*;
use serde_json::Value;

/// The string at `k`, or an empty string when the key is absent or is not a string.
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// An entry's stable key. The identifier is an integer on the wire and is kept as one.
pub(super) fn vid(v: &Value) -> i64 {
    v.get("id").and_then(Value::as_i64).unwrap_or(-1)
}

/// The listing path: the first page plainly, later pages by the last id already seen.
pub(crate) fn audit_logs_path(before: Option<i64>) -> String {
    match before {
        Some(id) => format!("/admin/audit-logs?before={id}"),
        None => "/admin/audit-logs".into(),
    }
}

/// The next page's starting id, or nothing when the server reports the end of the trail.
///
/// The cursor is a number on the wire; anything else is treated as the end rather than guessed at.
pub(crate) fn parse_next_cursor(cursor: &Option<Value>) -> Option<i64> {
    cursor.as_ref().and_then(Value::as_i64)
}

/// Append one fetched page onto the accumulated trail, returning where the next one starts.
pub(crate) fn merge_audit_page(lines: &mut Vec<Value>, page: CursorList<Value>) -> Option<i64> {
    lines.extend(page.data);
    parse_next_cursor(&page.next_cursor)
}

/// The audit screen, behind the administrator gate.
#[component]
pub fn AuditLogsPage() -> impl IntoView {
    view! {
        <AdminGate>
            <AuditLogsInner />
        </AdminGate>
    }
}

/// The screen an administrator sees: the first fetch, and its three render states.
#[component]
fn AuditLogsInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let logs = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<CursorList<Value>>(
                store,
                &audit_logs_path(None),
            )
            .await
            .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<CursorList<Value>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                logs.get()
                    .map(|opt| match opt {
                        Some(page) => board(store, page).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}
