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
//! **T-266:** the first fetch used to discard `next_cursor`, so the trail silently stopped at the
//! default keyset page (~20). `Load more` now refetches `?before=<cursor>` and **appends** into the
//! same client-side filter set. Empty + `next_cursor: null` still shows "No audit logs." with no
//! Load-more control.
//!
//! The detail needs no second fetch — the row carries every field — so nothing here can serve one
//! entry's id under another entry's chrome. Items stay `serde_json::Value`; `severity` is pinned by
//! the `audit_severity` enum (`info` / `warn` / `crit`).
#![allow(dead_code)]
use crate::auth::AuthStore;
use crate::datefmt::log_stamp;
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

/// Keyset list path for `GET /admin/audit-logs`. First page has no query; continuation pages pass
/// the prior page's `next_cursor` as `?before=<id>` (newest-first, `id < before`).
#[must_use]
pub(crate) fn audit_logs_path(before: Option<i64>) -> String {
    match before {
        Some(id) => format!("/admin/audit-logs?before={id}"),
        None => "/admin/audit-logs".into(),
    }
}

/// Wire `next_cursor` is a JSON number (audit row id). Missing / null / non-number → stop paging.
#[must_use]
pub(crate) fn parse_next_cursor(cursor: &Option<Value>) -> Option<i64> {
    cursor.as_ref().and_then(Value::as_i64)
}

/// Append one fetched page onto the accumulated trail. Returns the new `next_cursor` for a further
/// Load-more (or `None` when the server reports the end of the keyset).
#[must_use]
pub(crate) fn merge_audit_page(lines: &mut Vec<Value>, page: CursorList<Value>) -> Option<i64> {
    lines.extend(page.data);
    parse_next_cursor(&page.next_cursor)
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
            crate::client::api_get::<CursorList<Value>>(store, &audit_logs_path(None))
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

fn board(store: AuthStore, page: CursorList<Value>) -> impl IntoView {
    // Accumulated trail grows via Load more; the filter box stays client-side over whatever is
    // already loaded (server `?q=` is unused here — same as T-232).
    let lines = RwSignal::new(page.data);
    let next_cursor = RwSignal::new(parse_next_cursor(&page.next_cursor));
    let loading_more = RwSignal::new(false);
    let load_more_error = RwSignal::new(false);
    let selected = RwSignal::new(None::<i64>);
    let query = RwSignal::new(String::new());

    let on_load_more = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(before) = next_cursor.get_untracked() else {
                return;
            };
            if loading_more.get_untracked() {
                return;
            }
            loading_more.set(true);
            load_more_error.set(false);
            let path = audit_logs_path(Some(before));
            leptos::task::spawn_local(async move {
                match crate::client::api_get::<CursorList<Value>>(store, &path).await {
                    Ok(page) => {
                        let mut rows = lines.get_untracked();
                        let cursor = merge_audit_page(&mut rows, page);
                        lines.set(rows);
                        next_cursor.set(cursor);
                    }
                    Err(_) => {
                        load_more_error.set(true);
                    }
                }
                loading_more.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, next_cursor, loading_more, load_more_error, lines);
        }
    };

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
            let rows_owned = lines.get();
            if rows_owned.is_empty() {
                return view! {
                    <p class="px-1 py-4 text-on-surface-variant">"No audit logs."</p>
                }
                    .into_any();
            }
            let rows: Vec<&Value> = rows_owned
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
        }}
    };

    let load_more = view! {
        {move || {
            if next_cursor.get().is_none() {
                return ().into_any();
            }
            view! {
                <div class="mt-3 flex flex-col items-start gap-2 px-1">
                    <button
                        type="button"
                        on:click=on_load_more
                        prop:disabled=move || loading_more.get()
                        class="rounded-lg border border-primary/40 bg-primary/10 px-3 py-1.5 font-mono text-xs tracking-widest text-primary uppercase transition hover:bg-primary/20 disabled:opacity-50"
                    >
                        {move || {
                            if loading_more.get() {
                                "Loading…"
                            } else {
                                "Load more"
                            }
                        }}
                    </button>
                    {move || {
                        load_more_error
                            .get()
                            .then(|| {
                                view! {
                                    <p class="font-mono text-xs text-error-alert">
                                        "Could not load the next page."
                                    </p>
                                }
                            })
                    }}
                </div>
            }
                .into_any()
        }}
    };

    let master = view! {
        <div class="font-mono text-code-md">
            {list} {load_more}
            <span class="ml-2 inline-block h-3 w-2 animate-pulse bg-primary align-middle"></span>
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
            let rows = lines.get();
            match rows.iter().find(|l| vid(l) == id) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn first_page_path_has_no_before() {
        assert_eq!(audit_logs_path(None), "/admin/audit-logs");
    }

    #[test]
    fn continuation_path_forwards_cursor_as_before() {
        assert_eq!(audit_logs_path(Some(42)), "/admin/audit-logs?before=42");
    }

    #[test]
    fn parse_next_cursor_reads_json_number() {
        assert_eq!(parse_next_cursor(&Some(json!(99))), Some(99));
        assert_eq!(parse_next_cursor(&None), None);
        assert_eq!(parse_next_cursor(&Some(Value::Null)), None);
        assert_eq!(parse_next_cursor(&Some(json!("99"))), None);
    }

    #[test]
    fn merge_appends_and_returns_cursor() {
        let mut lines = vec![json!({"id": 30, "action": "a"})];
        let page = CursorList {
            data: vec![
                json!({"id": 20, "action": "b"}),
                json!({"id": 10, "action": "c"}),
            ],
            next_cursor: Some(json!(10)),
        };
        let cursor = merge_audit_page(&mut lines, page);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1]["id"], 20);
        assert_eq!(cursor, Some(10));
        // The Load-more control must call this path — discarding next_cursor yields the first-page
        // URL and the trail silently truncates (the pre-T-266 defect).
        assert_eq!(audit_logs_path(cursor), "/admin/audit-logs?before=10");
        assert_ne!(audit_logs_path(cursor), audit_logs_path(None));
    }

    #[test]
    fn empty_page_with_null_cursor_stops() {
        let mut lines: Vec<Value> = Vec::new();
        let page = CursorList {
            data: vec![],
            next_cursor: None,
        };
        let cursor = merge_audit_page(&mut lines, page);
        assert!(lines.is_empty());
        assert_eq!(cursor, None);
        assert_eq!(audit_logs_path(cursor), "/admin/audit-logs");
    }

    /// Class-R perturbation: a page that *has* a continuation cursor must not be treated like a
    /// terminal page. Ignoring `next_cursor` (the old FE) makes `parse_next_cursor` look like
    /// `None` and the path collapses to the first page — this asserts the RED difference.
    #[test]
    fn ignoring_next_cursor_is_detectably_wrong() {
        let page = CursorList {
            data: vec![json!({"id": 20})],
            next_cursor: Some(json!(20)),
        };
        let forwarded = parse_next_cursor(&page.next_cursor);
        let discarded: Option<i64> = None; // pre-T-266: CursorList.next_cursor never read
        assert_eq!(forwarded, Some(20));
        assert_ne!(
            audit_logs_path(forwarded),
            audit_logs_path(discarded),
            "discarding next_cursor must not produce the same request path as forwarding it"
        );
    }

    /// Strip `//` / `/* */` so Class-R `contains` cannot green on a commented-out merge
    /// (T-457 / Wave 21–22 false-green class).
    fn strip_rust_comments(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let mut chars = src.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '/' {
                match chars.peek() {
                    Some('/') => {
                        chars.next();
                        while let Some(n) = chars.next() {
                            if n == '\n' {
                                out.push('\n');
                                break;
                            }
                        }
                        continue;
                    }
                    Some('*') => {
                        chars.next();
                        while let Some(n) = chars.next() {
                            if n == '*' && matches!(chars.peek(), Some('/')) {
                                chars.next();
                                break;
                            }
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            out.push(c);
        }
        out
    }

    fn collapse_ws(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// T-445 Class-R — helper unit tests alone do not pin the Load-more UI. A replace bug
    /// (`lines.set(page.data)` instead of `merge_audit_page`) keeps `merge_appends_*` green
    /// while truncating the trail. Bind to the live `on_load_more` Ok-arm (comment-stripped).
    #[test]
    fn on_load_more_appends_via_merge_audit_page() {
        const SRC: &str = include_str!("audit.rs");
        let production = SRC
            .split("mod tests {")
            .next()
            .expect("tests module marker");
        // Scope the pin to the Load-more closure so a dead string elsewhere cannot false-green.
        let load_more = production
            .split("let on_load_more = move |_|")
            .nth(1)
            .and_then(|rest| rest.split("let master_header =").next())
            .expect("on_load_more closure must sit before master_header");
        let code = collapse_ws(&strip_rust_comments(load_more));

        assert!(
            code.contains("audit_logs_path(Some(before))"),
            "on_load_more must request the continuation page via audit_logs_path(Some(before))"
        );
        assert!(
            code.contains("let cursor = merge_audit_page(&mut rows, page)"),
            "on_load_more Ok-arm must append via merge_audit_page (not replace the trail)"
        );
        assert!(
            code.contains("lines.set(rows)"),
            "on_load_more must write the merged rows back into the lines signal"
        );
        assert!(
            !code.contains("lines.set(page.data)"),
            "on_load_more must not replace the trail with page.data alone \
             (perturbation: lines.set(page.data) truncates prior pages)"
        );
    }
}
