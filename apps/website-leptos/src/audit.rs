//! Audit Logs (/admin/audit) — ported from pages/admin.tsx `AuditLogsPage`. `<AdminGate>` →
//! `/admin/audit-logs` Resource → `QueryState` → a `SplitPane` (filter search + a mono log stream
//! master + a log-entry detail pane).
//!
//! **Gate scope (this slice):** the empty-DB `/admin/audit-logs` golden ({data:[], next_cursor:null})
//! → master shows "No audit logs." (+ the blinking cursor) and, with nothing selected, the detail
//! shows `SplitPaneEmpty`. Byte-exact-verified. The populated log rows + entry detail are
//! content-golden gated; the log item type stays `serde_json::Value` until then.
#![allow(dead_code)]
use crate::dto::CursorList;
use crate::split_pane::{SplitPane, SplitPaneEmpty};
use crate::ui::{AdminGate, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn AuditLogsPage() -> impl IntoView {
    view! {
        <AdminGate>
            <AuditLogsInner />
        </AdminGate>
    }
}

#[component]
fn AuditLogsInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let logs = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<CursorList<Value>>(store, "/admin/audit-logs")
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
                        Some(page) => board(page.data).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn board(lines: Vec<Value>) -> impl IntoView {
    let master_header = view! {
        <input
            type="search"
            placeholder="Filter by admin, action, or keyword..."
            value=""
            class="w-full rounded-lg border border-outline-variant/40 bg-surface-container px-3 py-1.5 font-mono text-code-md outline-none focus:border-primary/60"
        />
    }
    .into_any();
    // Populated log rows are content-golden gated (empty golden → empty state).
    let list = if lines.is_empty() {
        view! { <p class="px-1 py-4 text-on-surface-variant">"No audit logs."</p> }.into_any()
    } else {
        ().into_any()
    };
    let master = view! {
        <div class="font-mono text-code-md">
            {list} <span class="ml-2 inline-block h-3 w-2 animate-pulse bg-primary align-middle"></span>
        </div>
    }
    .into_any();
    let detail = view! {
        <SplitPaneEmpty
            icon=view! { <MaterialIcon name="terminal" class="text-4xl" /> }.into_any()
            message="Select a log entry to inspect."
        />
    }
    .into_any();

    view! {
        <SplitPane
            master_width="60%"
            master_header=master_header
            master=master
            detail=detail
        />
    }
}
