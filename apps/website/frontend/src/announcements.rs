//! Announcements (/announcements) — ported from pages/operations.tsx `AnnouncementsPage`.
//! `<AuthGate>` → `/announcements` Resource → `QueryState` → a topo-map/frosted-glass encasing
//! around a transparent `SplitPane` (Comms Link master list + reading detail pane).
//!
//! **Empty-DB golden (unchanged):** with `Paginated` empty the master still shows
//! "No announcements yet." and, with nothing selected, the detail still shows `SplitPaneEmpty` —
//! byte-exact-verified.
//!
//! **T-232:** the populated half was never written — the non-empty master branch was a literal
//! `().into_any()`, so a real `/announcements` payload (4 rows against the live API) rendered an
//! empty aside next to a permanently empty reading pane. Both halves now exist: pinned-first
//! `ListDetailItem` rows drive a `selected` id, and the detail pane is the reading view
//! (tag + PINNED chip, headline, byline, body prose).
//!
//! **T-239 body contract — plain text, one escape at render:** `body` is authored markdown-ish
//! plain text. CMS stores it **without** ammonia. This page renders paragraphs as Leptos **text**
//! nodes (`{p.to_string()}`), which HTML-escape once. Do **not** switch this to `inner_html`
//! without a real HTML sanitizer on the write path — that is the only coherent alternative to
//! the text contract, and inventing both escapes is what produced live `a &lt; b` on screen.
//!
//! Selection is deliberately **not** auto-advanced to the first row (unlike `events.rs`, whose
//! surface spec asks for it): this is the Apple-Mail port, where the resting state is "nothing
//! opened yet", and it keeps the empty-DB golden's `SplitPaneEmpty` as the honest zero-selection
//! render rather than a special case of it. **T-353** adds `/announcements/:id` so the dashboard
//! intel feed (and in-list clicks) can deep-link a post — URL `id` drives selection; bare
//! `/announcements` stays unselected.
//!
//! The detail needs **no second fetch** — the list payload already carries `body` — so the T-226
//! stale-Resource hazard (a `Resource` serving its previous value under a new row's chrome) cannot
//! arise here. The row items stay `serde_json::Value`; the fields read are pinned by the enum
//! vocabulary in `announcement_tag` (`update` / `event` / `modpack_update` / `important`).
#![allow(dead_code)]
use crate::core::datefmt::{format_local_datetime, format_short_date};
use crate::core::dto::Paginated;
use crate::core::split_pane::{ListDetailItem, SplitPane, SplitPaneEmpty};
use crate::core::ui::{badge_class, AuthGate, MaterialIcon};
use crate::core::url_guard;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde_json::Value;

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}
fn vbool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

/// `announcement_tag` → its `badge_class` variant. An unknown tag (one added server-side before
/// this table learns it) degrades to the neutral chip rather than vanishing.
fn tag_variant(tag: &str) -> &'static str {
    match tag {
        "modpack_update" => "primary",
        "event" => "tertiary",
        "important" => "error",
        _ => "neutral",
    }
}

/// `modpack_update` → `MODPACK UPDATE`. An absent tag reads `NOTICE`.
fn tag_label(tag: &str) -> String {
    if tag.is_empty() {
        return "NOTICE".into();
    }
    tag.replace('_', " ").to_uppercase()
}

/// The row's preview line: the backend's `snippet` when it wrote one, else the body's opening
/// paragraph. `ListDetailItem` line-clamps to two lines, so no truncation is done here.
fn preview_text(p: &Value) -> String {
    let s = vstr(p, "snippet");
    if !s.is_empty() {
        return s;
    }
    let body = vstr(p, "body");
    body.split("\n\n").next().unwrap_or_default().to_string()
}

/// Split an announcement body into non-empty paragraphs (blank-line separated). Pure so the
/// T-239 text-contract pin can assert bare `<` / `&` survive into the strings Leptos will
/// text-escape once — never pre-escaped as `&lt;` / `&amp;`.
fn body_paragraph_texts(body: &str) -> Vec<String> {
    body.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

/// Announcement bodies are authored as markdown-ish plain text (T-239). Full markdown rendering
/// lives in `wiki.rs`'s private `render_markdown` — this reader splits on blank lines and lets
/// `whitespace-pre-line` keep single newlines, the same treatment `event_hub.rs` gives a briefing.
/// Inline `**bold**` / backticks therefore show as written. Each paragraph is a **text** node.
fn body_paragraphs(body: &str) -> impl IntoView + use<> {
    body_paragraph_texts(body)
        .into_iter()
        .map(|p| {
            view! {
                <p class="whitespace-pre-line text-sm leading-relaxed text-on-surface-variant">
                    {p}
                </p>
            }
        })
        .collect_view()
}

#[component]
pub fn AnnouncementsPage() -> impl IntoView {
    view! {
        <AuthGate>
            <AnnouncementsInner />
        </AuthGate>
    }
}

#[component]
fn AnnouncementsInner() -> impl IntoView {
    let store = expect_context::<crate::core::auth::AuthStore>();
    let posts = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::core::client::api_get::<Paginated<Value>>(store, "/announcements")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<Value>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                posts
                    .get()
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

fn board(posts: Vec<Value>) -> impl IntoView {
    // Pinned-first, then the server's order preserved (`sort_by_key` is stable) — the React
    // `sort((a,b) => Number(b.is_pinned) - Number(a.is_pinned))`.
    let mut posts = posts;
    posts.sort_by_key(|p| !vbool(p, "is_pinned"));
    // The rows and the reader both read the same fetched payload, so it is stored once and read by
    // both closures. Nothing here refetches, so nothing here can go stale.
    let posts = StoredValue::new(posts);
    // T-353 — URL `id` is the selection SoT (no auto-select on bare `/announcements`).
    let params = use_params_map();
    let selected = Memo::new(move |_| {
        params
            .read()
            .get("id")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    });
    let navigate = use_navigate();

    let master = view! {
        {move || {
            // Read selection so list highlight tracks `/announcements/:id` navigation.
            let sel = selected.get();
            posts
                .with_value(|posts| {
                    if posts.is_empty() {
                        return view! {
                            <p class="px-1 py-4 text-label-md text-on-surface-variant">
                                "No announcements yet."
                            </p>
                        }
                            .into_any();
                    }
                    posts
                        .iter()
                        .map(|p| {
                            let id = vstr(p, "id");
                            let click_id = id.clone();
                            let navigate = navigate.clone();
                            let pinned = vbool(p, "is_pinned");
                            let tag = vstr(p, "tag");
                            let title = vstr(p, "title");
                            let title = if title.is_empty() {
                                "Untitled Post".to_string()
                            } else {
                                title
                            };
                            let date = format_short_date(&vstr(p, "published_at"));
                            let preview = preview_text(p);
                            let is_active = sel.as_deref() == Some(id.as_str());
                            view! {
                                <ListDetailItem
                                    active=is_active
                                    meta=view! { {date} }.into_any()
                                    dot_class=if pinned { "bg-tactical-yellow" } else { "" }
                                    title=view! { {title} }.into_any()
                                    trailing=view! {
                                        <span class=badge_class(tag_variant(&tag))>
                                            {tag_label(&tag)}
                                        </span>
                                    }
                                        .into_any()
                                    preview=view! { {preview} }.into_any()
                                    on_click=Callback::new(move |()| {
                                        navigate(
                                            &format!("/announcements/{click_id}"),
                                            Default::default(),
                                        );
                                    })
                                />
                            }
                        })
                        .collect_view()
                        .into_any()
                })
        }}
    }
    .into_any();

    let detail = view! {
        {move || {
            let Some(id) = selected.get() else {
                // Nothing opened yet — including the empty-DB golden, where there is nothing to
                // open. Same render either way.
                return view! {
                    <SplitPaneEmpty
                        icon=view! { <MaterialIcon name="campaign" class="text-4xl" /> }.into_any()
                        message="Select a broadcast to read."
                    />
                }
                    .into_any();
            };
            posts
                .with_value(|posts| {
                    match posts.iter().find(|p| vstr(p, "id") == id) {
                        Some(p) => reader(p).into_any(),
                        // Deep link to an id that is not in the current feed.
                        None => {
                            view! {
                                <SplitPaneEmpty
                                    icon=view! { <MaterialIcon name="campaign" class="text-4xl" /> }
                                        .into_any()
                                    message="That broadcast is no longer in the feed."
                                />
                            }
                                .into_any()
                        }
                    }
                })
        }}
    }
    .into_any();

    let master_header = view! {
        <>
            <h2 class="text-headline-sm tracking-wide text-on-surface uppercase">"Comms Link"</h2>
            <MaterialIcon name="filter_list" class="text-outline" />
        </>
    }
    .into_any();

    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                <SplitPane transparent=true master_header=master_header master=master detail=detail />
            </div>
        </div>
    }
}

/// The reading pane for one selected broadcast — the `AnnouncementDetail` half of the Apple-Mail
/// port. Optional fields (`thumbnail_url`, `discord_message_id`) are omitted by the backend when
/// empty, so each is gated on being non-empty rather than rendered as a blank slot.
fn reader(p: &Value) -> impl IntoView + use<> {
    let tag = vstr(p, "tag");
    let pinned = vbool(p, "is_pinned");
    let title = vstr(p, "title");
    let title = if title.is_empty() {
        "Untitled Post".to_string()
    } else {
        title
    };
    let published = format_local_datetime(&vstr(p, "published_at"));
    let author = vstr(p, "author_id");
    let thumb = vstr(p, "thumbnail_url");
    let pushed = vbool(p, "pushed_to_discord");
    let body = vstr(p, "body");
    view! {
        <article class="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-10">
            <header class="flex flex-col gap-3 border-b border-outline-variant/30 pb-6">
                <div class="flex flex-wrap items-center gap-2">
                    <span class=badge_class(tag_variant(&tag))>{tag_label(&tag)}</span>
                    {pinned
                        .then(|| {
                            view! { <span class=badge_class("warning")>"Pinned"</span> }
                        })}
                    {pushed
                        .then(|| {
                            view! {
                                <span class="inline-flex items-center gap-1 font-mono text-xs text-on-surface-variant">
                                    <MaterialIcon name="forum" class="text-sm" />
                                    "Pushed to Discord"
                                </span>
                            }
                        })}
                </div>
                <h1 class="text-headline-md tracking-tight text-on-surface">{title}</h1>
                <div class="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-xs text-on-surface-variant">
                    <span class="inline-flex items-center gap-1">
                        <MaterialIcon name="account_circle" class="text-sm" />
                        {if author.is_empty() { "Command".to_string() } else { author }}
                    </span>
                    <span>{published}</span>
                </div>
            </header>
            {thumbnail_img_src(&thumb)
                .map(|src| {
                    view! {
                        <img
                            src=src.to_string()
                            alt=""
                            class="max-h-72 w-full rounded-xl border border-white/10 object-cover"
                        />
                    }
                })}
            <div class="flex flex-col gap-4">{body_paragraphs(&body)}</div>
        </article>
    }
}

/// Announcement detail thumbnail `src`. **T-413** — writer guarded by T-405; sink still checks.
fn thumbnail_img_src(url: &str) -> Option<&str> {
    url_guard::is_http_url(url).then_some(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// T-239: the strings fed to Leptos text nodes must still contain bare `<` / `&`.
    /// RED: pretreat with HTML-escaping before split — this fails on `&lt;`.
    #[test]
    fn body_paragraphs_preserve_bare_angle_brackets() {
        let authored = "Damage: a < b & c > d\n\nSecond paragraph.";
        let paras = body_paragraph_texts(authored);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0], "Damage: a < b & c > d");
        assert!(!paras[0].contains("&lt;"));
        assert!(!paras[0].contains("&amp;"));
    }

    #[test]
    fn preview_prefers_snippet_but_falls_back_to_body_without_entities() {
        let with_snip = json!({"snippet": "teaser < ok", "body": "ignored"});
        assert_eq!(preview_text(&with_snip), "teaser < ok");

        let from_body = json!({"snippet": "", "body": "a < b\n\nmore"});
        assert_eq!(preview_text(&from_body), "a < b");
        assert!(!preview_text(&from_body).contains("&lt;"));
    }

    include!("../../shared/is_http_url_cases.rs");

    #[test]
    fn announcement_thumbnail_emits_src_only_for_http_urls() {
        let mut wrong = Vec::new();
        for (input, should_img) in IS_HTTP_URL_CASES {
            match (thumbnail_img_src(input), should_img) {
                (Some(_), false) => wrong.push(format!("  RENDERED AN IMG FOR {input:?}")),
                (None, true) => wrong.push(format!("  refused a legitimate thumb {input:?}")),
                _ => {}
            }
        }
        assert!(
            wrong.is_empty(),
            "announcement thumbnail sink wrong on {} of {} cases:\n{}",
            wrong.len(),
            IS_HTTP_URL_CASES.len(),
            wrong.join("\n")
        );
    }
}
