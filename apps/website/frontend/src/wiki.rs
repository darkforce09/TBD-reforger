//! SOPs & Manuals (/wiki) — ported from pages/doctrine.tsx `WikiPage`. `<AuthGate>` →
//! `GET /api/v1/wiki` → a `GlassSplit`: category-grouped manual index (master) + reading pane
//! (detail) rendering the active manual's Markdown. Admin edit mode PUTs `/wiki/{slug}`.
//!
//! List items ride `DataEnvelope<Value>` (dto.rs already pins the wiki golden that way — no
//! typed WikiPage DTO consumer yet).
#![allow(dead_code)]
use crate::dto::DataEnvelope;
use crate::nav::Role;
use crate::split_pane::{GlassSplit, ListDetailItem, SidebarSearch};
use leptos::prelude::*;
use serde_json::Value;

const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

fn vi64(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}

/// ISO timestamp → `YYYY-MM-DD` for the "Last updated" chip (no `datefmt` / js_sys — keeps
/// native unit tests compiling).
fn updated_day(iso: &str) -> String {
    let day = iso.get(..10).unwrap_or("");
    if day.len() == 10 && day.as_bytes().get(4) == Some(&b'-') {
        day.to_string()
    } else {
        "—".into()
    }
}

/// Resolve the active page from the route slug against the live list. Unknown/absent slug
/// falls back to the first row (API already orders by `nav_order ASC, title ASC`).
fn resolve_slug(pages: &[Value], slug: Option<&str>) -> Option<String> {
    if let Some(s) = slug {
        if pages.iter().any(|p| vstr(p, "slug") == s) {
            return Some(s.to_string());
        }
    }
    pages
        .first()
        .map(|p| vstr(p, "slug"))
        .filter(|s| !s.is_empty())
}

/// Distinct categories in first-seen order (which is nav_order order from the API list).
fn category_order(pages: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for p in pages {
        let c = vstr(p, "category");
        if !c.is_empty() && !out.iter().any(|x| x == &c) {
            out.push(c);
        }
    }
    out
}

/* ───────────────────────── Markdown renderer (ports renderInline + Markdown) ───────────────────────── */

/// Inline: `**bold**` → strong, `*italic*` → em, `` `code` `` → Mono, else plain text. Mirrors the
/// JS regex /(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/g: each token's inner content has no delimiter char.
fn render_inline(text: &str) -> Vec<AnyView> {
    let mut out: Vec<AnyView> = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let tok = if let Some(inner) = delim_token(rest, "**", '*') {
            Some(("b", inner.to_string(), 2 + inner.len() + 2))
        } else if rest.starts_with('*') {
            delim_token(rest, "*", '*').map(|inner| ("i", inner.to_string(), 1 + inner.len() + 1))
        } else if rest.starts_with('`') {
            delim_token(rest, "`", '`').map(|inner| ("c", inner.to_string(), 1 + inner.len() + 1))
        } else {
            None
        };
        match tok {
            Some((kind, inner, consumed)) => {
                if !plain.is_empty() {
                    out.push(view! { {plain.clone()} }.into_any());
                    plain.clear();
                }
                out.push(match kind {
                    "b" => view! { <strong class="font-semibold text-on-surface">{inner}</strong> }.into_any(),
                    "c" => view! { <code class="rounded bg-black/40 px-1.5 py-0.5 font-mono text-[0.85em] text-primary">{inner}</code> }.into_any(),
                    _ => view! { <em>{inner}</em> }.into_any(),
                });
                rest = &rest[consumed..];
            }
            None => {
                let ch = rest.chars().next().unwrap();
                plain.push(ch);
                rest = &rest[ch.len_utf8()..];
            }
        }
    }
    if !plain.is_empty() {
        out.push(view! { {plain} }.into_any());
    }
    out
}

/// If `s` opens with `open`, return the inner run up to the closing `open` — where the inner run
/// contains no `bad` char (the regex `[^delim]+`) and is non-empty. Else None.
fn delim_token<'a>(s: &'a str, open: &str, bad: char) -> Option<&'a str> {
    let after = s.strip_prefix(open)?;
    // inner = longest prefix with no `bad`
    let inner_end = after.find(bad).unwrap_or(after.len());
    if inner_end == 0 {
        return None;
    }
    // the char run must be immediately followed by the closing delimiter
    if after[inner_end..].starts_with(open) {
        Some(&after[..inner_end])
    } else {
        None
    }
}

fn render_markdown(source: &str) -> impl IntoView {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut blocks: Vec<AnyView> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            blocks.push(view! { <h2 class="mt-10 mb-3 border-b border-white/10 pb-2 text-xl font-bold tracking-tight text-white">{render_inline(rest)}</h2> }.into_any());
            i += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            blocks.push(view! { <h1 class="mb-4 text-2xl font-bold tracking-tight text-white">{render_inline(rest)}</h1> }.into_any());
            i += 1;
            continue;
        }
        if line.starts_with('>') {
            let mut quoted: Vec<String> = Vec::new();
            while i < lines.len() && lines[i].starts_with('>') {
                // strip /^>\s?/ — the '>' then an optional single whitespace
                let after = &lines[i][1..];
                let after = after.strip_prefix(' ').unwrap_or(after);
                quoted.push(after.to_string());
                i += 1;
            }
            blocks.push(callout(&quoted));
            continue;
        }
        if line.starts_with("- ") || line.starts_with("* ") {
            let mut items: Vec<String> = Vec::new();
            while i < lines.len() && (lines[i].starts_with("- ") || lines[i].starts_with("* ")) {
                items.push(lines[i][2..].to_string());
                i += 1;
            }
            blocks.push(view! {
                <ul class="mt-3 ml-1 space-y-2 text-body-md text-on-surface-variant">
                    {items.into_iter().map(|it| view! { <li>"• "{render_inline(&it)}</li> }).collect_view()}
                </ul>
            }.into_any());
            continue;
        }
        // paragraph
        let mut para: Vec<&str> = Vec::new();
        while i < lines.len()
            && !lines[i].trim().is_empty()
            && !lines[i].starts_with('#')
            && !lines[i].starts_with('>')
            && !lines[i].starts_with("- ")
            && !lines[i].starts_with("* ")
        {
            para.push(lines[i]);
            i += 1;
        }
        blocks.push(view! { <p class="mt-3 text-body-md leading-relaxed text-on-surface-variant">{render_inline(&para.join(" "))}</p> }.into_any());
    }
    blocks
}

/// A `> [!TYPE]` callout block. Ports the CALLOUT_TAGS + CALLOUT_STYLES mapping.
fn callout(quoted: &[String]) -> AnyView {
    // (variant box class, label class, default title) + optional explicit title
    let (mut box_cls, mut label_cls, mut default_title) =
        ("bg-primary/10 border-primary", "text-primary", "NOTE"); // info default
    let mut title: Option<String> = None;
    let mut body_lines: &[String] = quoted;
    if let Some(first) = quoted.first() {
        if let Some(tag) = parse_tag(first) {
            let mapped = match tag.0.to_uppercase().as_str() {
                "CRITICAL" | "CAUTION" => Some(("critical", None::<&str>)),
                "WARNING" => Some(("warning", None)),
                "TIP" => Some(("info", Some("PRO-TIP"))),
                "NOTE" | "INFO" => Some(("info", None)),
                _ => None,
            };
            if let Some((variant, tag_title)) = mapped {
                let styles = match variant {
                    "critical" => (
                        "bg-error/10 border-error",
                        "text-error-alert",
                        "CRITICAL RULE",
                    ),
                    "warning" => (
                        "bg-tactical-yellow/10 border-tactical-yellow",
                        "text-tactical-yellow",
                        "WARNING",
                    ),
                    _ => ("bg-primary/10 border-primary", "text-primary", "NOTE"),
                };
                box_cls = styles.0;
                label_cls = styles.1;
                default_title = tag_title.unwrap_or(styles.2);
                let explicit = tag.1.trim();
                title = if explicit.is_empty() {
                    None
                } else {
                    Some(explicit.to_string())
                };
                body_lines = &quoted[1..];
            }
        }
    }
    let shown_title = title.unwrap_or_else(|| default_title.to_string());
    let body = body_lines
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let outer = crate::ui::cn(&[
        "my-6 rounded-2xl border border-l-4 p-4 shadow-lg backdrop-blur-md",
        box_cls,
    ]);
    let label = crate::ui::cn(&[
        "mb-1 font-mono text-xs font-bold tracking-widest uppercase",
        label_cls,
    ]);
    view! {
        <div class=outer>
            <p class=label>{shown_title}</p>
            <div class="text-body-md leading-relaxed text-on-surface-variant">
                {render_inline(&body)}
            </div>
        </div>
    }
    .into_any()
}

/// Match `^\[!([A-Za-z-]+)\]\s*(.*)$` → (tag, rest).
fn parse_tag(line: &str) -> Option<(&str, &str)> {
    let inner = line.strip_prefix("[!")?;
    let close = inner.find(']')?;
    let tag = &inner[..close];
    if tag.is_empty() || !tag.chars().all(|c| c.is_ascii_alphabetic() || c == '-') {
        return None;
    }
    let rest = inner[close + 1..].trim_start();
    Some((tag, rest))
}

/* ───────────────────────────────── page ───────────────────────────────── */

#[derive(Clone, Copy, PartialEq, Eq)]
enum WikiMode {
    Read,
    Edit,
}

#[component]
pub fn WikiPage() -> impl IntoView {
    view! {
        <crate::ui::AuthGate>
            <WikiInner />
        </crate::ui::AuthGate>
    }
}

#[component]
fn WikiInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let pages = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<DataEnvelope<Value>>(store, "/wiki")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<Value>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                pages
                    .get()
                    .map(|opt| match opt {
                        Some(env) => wiki_board(env.data, pages).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load wiki."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn wiki_board(
    page_list: Vec<Value>,
    pages_res: LocalResource<Option<DataEnvelope<Value>>>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let is_admin = store.has_min_role(Role::Admin);
    let params = leptos_router::hooks::use_params_map();
    let search = RwSignal::new(String::new());
    let mode = RwSignal::new(WikiMode::Read);
    // Session-local draft body keyed by slug; Save clears the draft after a successful PUT.
    let drafts = RwSignal::new(std::collections::HashMap::<String, String>::new());
    let save_busy = RwSignal::new(false);
    let save_err = RwSignal::new(None::<String>);

    let page_list_for_sel = page_list.clone();
    let selected =
        Memo::new(move |_| resolve_slug(&page_list_for_sel, params.read().get("slug").as_deref()));

    Effect::new(move |prev: Option<Option<String>>| {
        let id = selected.get();
        if prev.as_ref().is_some_and(|p| p != &id) {
            mode.set(WikiMode::Read);
            save_err.set(None);
        }
        id
    });

    let page_list_master = page_list.clone();
    let page_list_detail = page_list;

    view! {
        <GlassSplit
            master_width="17rem"
            master_header=master_header(search).into_any()
            master=view! {
                {move || {
                    manual_index(
                        selected.get(),
                        &search.get(),
                        &page_list_master,
                    )
                }}
            }
                .into_any()
            detail=view! {
                {move || {
                    let slug = selected.get();
                    let page = slug.as_ref().and_then(|s| {
                        page_list_detail.iter().find(|p| vstr(p, "slug") == *s).cloned()
                    });
                    match page {
                        Some(p) => article(
                            p,
                            mode,
                            drafts,
                            save_busy,
                            save_err,
                            is_admin,
                            store,
                            pages_res,
                        )
                        .into_any(),
                        None => view! {
                            <section class="flex h-full items-center justify-center p-8">
                                <p class="font-mono text-sm text-on-surface-variant">
                                    {if page_list_detail.is_empty() {
                                        "No manuals yet."
                                    } else {
                                        "Select a manual."
                                    }}
                                </p>
                            </section>
                        }
                        .into_any(),
                    }
                }}
            }
                .into_any()
        />
    }
}

fn master_header(search: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <p class="font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
                "SOPs & Manuals"
            </p>
            <SidebarSearch placeholder="Search manuals..." bind=search />
        </div>
    }
}

fn manual_index(active_slug: Option<String>, query: &str, pages: &[Value]) -> impl IntoView {
    let query = query.to_string();
    let active = active_slug.unwrap_or_default();
    category_order(pages)
        .into_iter()
        .filter_map(move |category| {
            let rows: Vec<Value> = pages
                .iter()
                .filter(|p| vstr(p, "category") == category)
                .filter(|p| {
                    crate::split_pane::search_matches(
                        &query,
                        &format!("{} {}", vstr(p, "title"), vstr(p, "category")),
                    )
                })
                .cloned()
                .collect();
            if rows.is_empty() {
                return None;
            }
            let cat_label = category.clone();
            Some(view! {
                <div class="mb-3">
                    <p class="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                        {cat_label}
                    </p>
                    <div class="mt-1 flex flex-col gap-1">
                        {rows
                            .into_iter()
                            .map(|m| {
                                let id = vstr(&m, "slug");
                                let title = vstr(&m, "title");
                                let navigate = leptos_router::hooks::use_navigate();
                                let active_row = id == active;
                                view! {
                                    <ListDetailItem
                                        active=active_row
                                        title=view! { {title} }.into_any()
                                        on_click=Callback::new(move |()| {
                                            navigate(&format!("/wiki/{id}"), Default::default());
                                        })
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            })
        })
        .collect_view()
}

fn article(
    page: Value,
    mode: RwSignal<WikiMode>,
    drafts: RwSignal<std::collections::HashMap<String, String>>,
    save_busy: RwSignal<bool>,
    save_err: RwSignal<Option<String>>,
    is_admin: bool,
    store: crate::auth::AuthStore,
    pages_res: LocalResource<Option<DataEnvelope<Value>>>,
) -> impl IntoView {
    let slug = vstr(&page, "slug");
    let title = vstr(&page, "title");
    let category = vstr(&page, "category");
    let icon = vstr(&page, "icon");
    let nav_order = vi64(&page, "nav_order");
    let body_md = vstr(&page, "body_md");
    let updated = updated_day(&vstr(&page, "updated_at"));
    let slug_for_draft = slug.clone();
    let body_for_read = body_md.clone();

    view! {
        <section class="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
            <header class="flex shrink-0 items-start justify-between gap-4 border-b border-white/10 px-8 pt-8 pb-5 md:px-12">
                <div class="min-w-0">
                    <div class="mb-3 flex items-center gap-2">
                        <span class=BADGE_NEUTRAL>
                            <span class="material-symbols-outlined text-[14px]">"schedule"</span>
                            "Last updated "
                            {updated}
                        </span>
                        <span class="font-mono text-xs tracking-widest text-outline uppercase">
                            {category.clone()}
                        </span>
                    </div>
                    <h1 class="text-4xl font-bold tracking-tight text-white">{title.clone()}</h1>
                </div>
                {if is_admin {
                    view! {
                        <div class="flex shrink-0 flex-col items-end gap-2">
                            {read_edit_toggle(mode)}
                            {move || {
                                if mode.get() == WikiMode::Edit {
                                    let slug_put = slug.clone();
                                    let title_put = title.clone();
                                    let category_put = category.clone();
                                    let icon_put = icon.clone();
                                    let body_fallback = body_md.clone();
                                    view! {
                                        <button
                                            type="button"
                                            disabled=move || save_busy.get()
                                            class="rounded-full border border-primary/40 bg-primary/15 px-4 py-1.5 font-mono text-xs tracking-widest text-primary uppercase hover:bg-primary/25 disabled:opacity-50"
                                            on:click=move |_| {
                                                if save_busy.get_untracked() {
                                                    return;
                                                }
                                                save_busy.set(true);
                                                save_err.set(None);
                                                let draft = drafts
                                                    .with_untracked(|d| d.get(&slug_put).cloned())
                                                    .unwrap_or_else(|| body_fallback.clone());
                                                let body = serde_json::json!({
                                                    "category": category_put.clone(),
                                                    "title": title_put.clone(),
                                                    "icon": icon_put.clone(),
                                                    "body_md": draft,
                                                    "nav_order": nav_order,
                                                });
                                                let path = format!("/wiki/{slug_put}");
                                                let slug_clear = slug_put.clone();
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    leptos::task::spawn_local(async move {
                                                        match crate::client::api_put::<Value>(
                                                            store, &path, body,
                                                        )
                                                        .await
                                                        {
                                                            Ok(_) => {
                                                                drafts.update(|d| {
                                                                    d.remove(&slug_clear);
                                                                });
                                                                mode.set(WikiMode::Read);
                                                                pages_res.refetch();
                                                            }
                                                            Err(e) => {
                                                                save_err.set(Some(
                                                                    crate::client::api_error_message(
                                                                        &e,
                                                                        "Failed to save wiki page",
                                                                    ),
                                                                ));
                                                            }
                                                        }
                                                        save_busy.set(false);
                                                    });
                                                }
                                                #[cfg(not(target_arch = "wasm32"))]
                                                {
                                                    let _ = (store, path, body, pages_res, slug_clear);
                                                    save_busy.set(false);
                                                }
                                            }
                                        >
                                            {move || {
                                                if save_busy.get() { "Saving…" } else { "Save" }
                                            }}
                                        </button>
                                        {move || {
                                            save_err
                                                .get()
                                                .map(|m| {
                                                    view! {
                                                        <p class="max-w-xs text-right font-mono text-[11px] text-error-alert">
                                                            {m}
                                                        </p>
                                                    }
                                                })
                                        }}
                                    }
                                        .into_any()
                                } else {
                                    ().into_any()
                                }
                            }}
                        </div>
                    }
                        .into_any()
                } else {
                    ().into_any()
                }}
            </header>
            {move || {
                if mode.get() == WikiMode::Edit {
                    let initial = drafts
                        .with_untracked(|e| e.get(&slug_for_draft).cloned())
                        .unwrap_or_else(|| body_for_read.clone());
                    let key = slug_for_draft.clone();
                    view! {
                        <textarea
                            prop:value=initial
                            spellcheck="false"
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                drafts.update(|e| {
                                    e.insert(key.clone(), v);
                                });
                            }
                            class="h-full w-full flex-1 resize-none border-none bg-transparent p-8 font-mono text-sm leading-relaxed text-on-surface-variant outline-none focus:ring-0 md:p-12"
                        ></textarea>
                    }
                        .into_any()
                } else {
                    let source = drafts
                        .with(|e| e.get(&slug_for_draft).cloned())
                        .unwrap_or_else(|| body_for_read.clone());
                    view! {
                        <article class="custom-scrollbar flex-1 overflow-y-auto p-8 md:p-12">
                            <div class="max-w-3xl">{render_markdown(&source)}</div>
                        </article>
                    }
                        .into_any()
                }
            }}
        </section>
    }
}

fn read_edit_toggle(mode: RwSignal<WikiMode>) -> impl IntoView {
    let btn = |m: WikiMode, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || {
                    if mode.get() == m {
                        "rounded-full px-3 py-1 font-medium transition-all bg-surface-glass text-on-surface shadow-md"
                    } else {
                        "rounded-full px-3 py-1 font-medium transition-all text-on-surface-variant hover:text-on-surface"
                    }
                }
                on:click=move |_| mode.set(m)
            >
                {label}
            </button>
        }
    };
    view! {
        <div class="inline-flex shrink-0 gap-1 rounded-full border border-white/5 bg-black/20 p-1 font-mono text-xs">
            {btn(WikiMode::Read, "[ READ ]")}
            {btn(WikiMode::Edit, "[ EDIT ]")}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{category_order, resolve_slug, updated_day};
    use serde_json::json;

    #[test]
    fn slug_resolution_falls_back_to_first() {
        let pages = vec![
            json!({"slug": "field-manual", "category": "Doctrine", "title": "Field Manual"}),
            json!({"slug": "radio-procedure", "category": "Doctrine", "title": "Radio"}),
        ];
        assert_eq!(resolve_slug(&pages, None).as_deref(), Some("field-manual"));
        assert_eq!(
            resolve_slug(&pages, Some("radio-procedure")).as_deref(),
            Some("radio-procedure")
        );
        assert_eq!(
            resolve_slug(&pages, Some("nope")).as_deref(),
            Some("field-manual")
        );
    }

    #[test]
    fn categories_preserve_first_seen_order() {
        let pages = vec![
            json!({"slug": "a", "category": "Doctrine"}),
            json!({"slug": "b", "category": "Administration"}),
            json!({"slug": "c", "category": "Doctrine"}),
        ];
        assert_eq!(
            category_order(&pages),
            vec!["Doctrine".to_string(), "Administration".to_string()]
        );
    }

    #[test]
    fn updated_day_takes_iso_prefix() {
        assert_eq!(updated_day("2026-07-14T10:12:00Z"), "2026-07-14");
        assert_eq!(updated_day(""), "—");
    }
}
