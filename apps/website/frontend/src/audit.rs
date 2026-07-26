//! Audit Logs (/admin/audit) — ported from pages/admin.tsx `AuditLogsPage`. `<AdminGate>` →
//! `/admin/audit-logs` Resource → `QueryState` → a `SplitPane` (filter search + a mono log stream
//! master + a log-entry detail pane).
//!
//! **Empty-DB golden (unchanged):** with `{data:[], next_cursor:null}` the master still shows
//! "No audit logs." (+ the blinking cursor) and, with nothing selected, the detail still shows
//! `SplitPaneEmpty` — byte-exact-verified. Wiring the filter adds an event listener, not an
//! attribute, so that render is untouched.
//!
//! **T-232:** the populated half was never written — the non-empty branch was a literal
//! `().into_any()`, so 20 live rows from `/admin/audit-logs` rendered as a bare blinking cursor
//! over a permanently empty inspector. Now: one mono `[stamp] [LEVEL] action — message` line per
//! entry, selectable, with the entry detail (actor / target / severity / metadata) in the right
//! pane. The filter box above the stream, which had no `on:input` at all, now filters the loaded
//! page **client-side** through the shared `search_matches`: re-keying a Resource on every
//! keystroke is both the T-226 stale-value hazard and a request per character, and the cursor
//! endpoint serves a whole page at a time anyway.
//!
//! The detail needs no second fetch — the row carries every field — so nothing here can serve one
//! entry's id under another entry's chrome. Items stay `serde_json::Value`; `severity` is pinned by
//! the `audit_severity` enum (`info` / `warn` / `crit`).
#![allow(dead_code)]
use crate::dto::CursorList;
use crate::split_pane::{search_matches, SplitPane, SplitPaneEmpty};
use crate::ui::{badge_class, AdminGate, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// The log row's stable key. `audit_logs.id` is a bigint, so it is read as one and rendered as a
/// string only where it has to be.
fn vid(v: &Value) -> i64 {
    v.get("id").and_then(Value::as_i64).unwrap_or(-1)
}

/// `audit_severity` → the terminal level token. An unknown severity keeps its own wire value
/// upper-cased rather than being silently relabelled `INFO`.
fn level_label(severity: &str) -> String {
    match severity {
        "info" => "INFO".into(),
        "warn" => "WARN".into(),
        "crit" => "CRIT".into(),
        "" => "----".into(),
        other => other.to_uppercase(),
    }
}

/// Terminal foreground for the level token — the Aegis token, not a raw colour.
fn level_class(severity: &str) -> &'static str {
    match severity {
        "warn" => "shrink-0 text-tactical-yellow",
        "crit" => "shrink-0 font-bold text-error-alert",
        _ => "shrink-0 text-primary",
    }
}

fn severity_variant(severity: &str) -> &'static str {
    match severity {
        "warn" => "warning",
        "crit" => "error",
        "info" => "primary",
        _ => "neutral",
    }
}

/// `YYYY-MM-DD HH:MM:SS` in the browser's zone — the log-stream stamp. `datefmt`'s formatters are
/// the human-prose ones ("Sat Aug 1, 21:00 GMT+2"); a terminal wants fixed-width, so this is local
/// (the `event_manager.rs` precedent for a page-specific `js_sys::Date` helper). `datefmt.rs` is
/// not T-232's to extend.
fn log_stamp(iso: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return "--------- --:--:--".into();
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}

/// Everything the filter box matches on, joined into one haystack: the stamp, the level, the
/// action, the actor and the message — i.e. exactly what the operator can see on the line.
fn haystack(l: &Value) -> String {
    let sev = vstr(l, "severity");
    format!(
        "{} {} {} {} {} {}",
        log_stamp(&vstr(l, "created_at")),
        level_label(&sev),
        vstr(l, "action"),
        vstr(l, "actor_name"),
        vstr(l, "message"),
        vstr(l, "target_type"),
    )
}

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
    // The stream and the inspector read the same fetched page; nothing here refetches.
    let lines = StoredValue::new(lines);
    let selected = RwSignal::new(None::<i64>);
    let query = RwSignal::new(String::new());

    let master_header = view! {
        <input
            type="search"
            placeholder="Filter by admin, action, or keyword..."
            value=""
            on:input=move |ev| query.set(event_target_value(&ev))
            class="w-full rounded-lg border border-outline-variant/40 bg-surface-container px-3 py-1.5 font-mono text-code-md outline-none focus:border-primary/60"
        />
    }
    .into_any();

    let list = view! {
        {move || {
            let q = query.get();
            lines
                .with_value(|lines| {
                    if lines.is_empty() {
                        return view! {
                            <p class="px-1 py-4 text-on-surface-variant">"No audit logs."</p>
                        }
                            .into_any();
                    }
                    let rows: Vec<&Value> = lines
                        .iter()
                        .filter(|l| search_matches(&q, &haystack(l)))
                        .collect();
                    if rows.is_empty() {
                        // The page HAS entries; this query matched none of them. Saying "No audit
                        // logs." here would read as an empty trail rather than an empty filter.
                        return view! {
                            <p class="px-1 py-4 text-on-surface-variant">
                                "No entries match this filter."
                            </p>
                        }
                            .into_any();
                    }
                    rows.into_iter()
                        .map(|l| {
                            let id = vid(l);
                            let sev = vstr(l, "severity");
                            let stamp = log_stamp(&vstr(l, "created_at"));
                            let level = level_label(&sev);
                            let lvl_class = level_class(&sev);
                            let action = vstr(l, "action");
                            let message = vstr(l, "message");
                            view! {
                                <button
                                    type="button"
                                    on:click=move |_| selected.set(Some(id))
                                    class=move || {
                                        crate::ui::cn(
                                            &[
                                                "flex w-full items-start gap-2 rounded px-2 py-1 text-left transition",
                                                if selected.get() == Some(id) {
                                                    "bg-primary/15 text-on-surface shadow-[inset_2px_0_0_0_#adc6ff]"
                                                } else {
                                                    "text-on-surface-variant hover:bg-white/[0.04] hover:text-on-surface"
                                                },
                                            ],
                                        )
                                    }
                                >
                                    <span class="shrink-0 text-outline">{stamp}</span>
                                    <span class=lvl_class>"["{level}"]"</span>
                                    <span class="shrink-0 text-tertiary">{action}</span>
                                    <span class="min-w-0 flex-1 truncate">{message}</span>
                                </button>
                            }
                        })
                        .collect_view()
                        .into_any()
                })
        }}
    };

    let master = view! {
        <div class="font-mono text-code-md">
            {list} <span class="ml-2 inline-block h-3 w-2 animate-pulse bg-primary align-middle"></span>
        </div>
    }
    .into_any();

    let detail = view! {
        {move || {
            let Some(id) = selected.get() else {
                return view! {
                    <SplitPaneEmpty
                        icon=view! { <MaterialIcon name="terminal" class="text-4xl" /> }.into_any()
                        message="Select a log entry to inspect."
                    />
                }
                    .into_any();
            };
            lines
                .with_value(|lines| {
                    match lines.iter().find(|l| vid(l) == id) {
                        Some(l) => entry(l).into_any(),
                        // A filter can hide the selected row but never delete it; this only fires
                        // if the page is ever replaced under a live selection.
                        None => {
                            view! {
                                <SplitPaneEmpty
                                    icon=view! { <MaterialIcon name="terminal" class="text-4xl" /> }
                                        .into_any()
                                    message="That entry is no longer in this page of the trail."
                                />
                            }
                                .into_any()
                        }
                    }
                })
        }}
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

/// One expanded audit entry. Everything but `id` / `action` / `created_at` / `message` /
/// `severity` is optional on the wire (a `server.fps_drop` has no actor; an `auth.login` has no
/// target), so each optional row is gated on being present rather than rendered as an empty field.
fn entry(l: &Value) -> impl IntoView + use<> {
    let sev = vstr(l, "severity");
    let action = vstr(l, "action");
    let message = vstr(l, "message");
    let stamp = log_stamp(&vstr(l, "created_at"));
    let actor_name = vstr(l, "actor_name");
    let actor_id = vstr(l, "actor_id");
    let target_type = vstr(l, "target_type");
    let target_id = vstr(l, "target_id");
    let id = vid(l);
    // `metadata` is free-form jsonb. Pretty-printed as JSON rather than guessed at per action —
    // the vocabulary differs for every action and the operator reading an audit trail wants the
    // raw record, not a paraphrase.
    let metadata = l
        .get("metadata")
        .filter(|m| !m.is_null())
        .and_then(|m| serde_json::to_string_pretty(m).ok());
    view! {
        <div class="flex flex-col gap-6 px-8 py-8">
            <header class="flex flex-col gap-3 border-b border-outline-variant/30 pb-5">
                <div class="flex flex-wrap items-center gap-2">
                    <span class=badge_class(severity_variant(&sev))>{level_label(&sev)}</span>
                    <span class="font-mono text-code-md text-tertiary">{action}</span>
                </div>
                <p class="text-body-md leading-relaxed text-on-surface">{message}</p>
                <span class="font-mono text-xs text-outline">{stamp}</span>
            </header>
            <dl class="flex flex-col gap-3 font-mono text-code-md">
                <EntryField label="Entry" value=id.to_string() />
                {(!actor_name.is_empty() || !actor_id.is_empty())
                    .then(|| {
                        let who = if actor_name.is_empty() {
                            actor_id.clone()
                        } else {
                            format!("{actor_name} ({actor_id})")
                        };
                        view! { <EntryField label="Actor" value=who /> }
                    })}
                {(!target_type.is_empty())
                    .then(|| view! { <EntryField label="Target type" value=target_type.clone() /> })}
                {(!target_id.is_empty())
                    .then(|| view! { <EntryField label="Target id" value=target_id.clone() /> })}
            </dl>
            {metadata
                .map(|m| {
                    view! {
                        <section class="flex flex-col gap-2">
                            <h3 class="font-mono text-xs tracking-widest text-on-surface-variant uppercase">
                                "Metadata"
                            </h3>
                            <pre class="custom-scrollbar overflow-x-auto rounded-lg border border-white/10 bg-black/30 p-4 font-mono text-code-md text-on-surface-variant">
                                {m}
                            </pre>
                        </section>
                    }
                })}
        </div>
    }
}

#[component]
fn EntryField(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="flex items-baseline gap-3">
            <dt class="w-28 shrink-0 text-xs tracking-widest text-on-surface-variant uppercase">
                {label}
            </dt>
            <dd class="min-w-0 break-all text-on-surface">{value}</dd>
        </div>
    }
}
