//! Comms Broadcaster / Content Manager (/admin/content) — ported from pages/admin.tsx
//! `ContentManagerPage` + `ContentEditor`. `<AdminGate>` → a transparent `SplitPane`: a post list
//! (master) + the editor form (detail).
//!
//! T-267: Publish keeps the returned announcement id and re-Publish PATCHes that row (no duplicate
//! POSTs). Delete / Discord re-push hit the live CMS routes. SOP has no `announcement_tag` variant
//! (measured: `update|event|modpack_update|important` only) — mapped to closest tag `update`.
//! Markdown toolbar inserts real markers into the body (no mock success toasts).
//!
//! T-446: Hero image upload POSTs multipart `file` to `/cms/uploads` via `api_upload_file`
//! (`web-sys` FormData/File), then stores the returned `url` on the draft `thumbnail_url`.
//!
//! T-447: master list boots from `GET /cms/announcements` (admin drafts+published), not
//! `mock_docs()`. Local New drafts still prepend into the RwSignal until first Publish.
//!
//! T-466: list LocalResource keeps `Result` (no `.ok()`). Failed GET surfaces an actionable
//! error + Retry in the master pane; `list_seeded` is set only on Ok so an error never looks
//! like a permanent empty catalog.
//!
//! T-470 / T-473: Class-R pins are order/window-sensitive — seed only inside the Ok arm (not
//! before match, not in Err); any `list_seeded.set(...)` form counts (whitespace-normalized).
//! Retry/error UI must sit in a reachable `if let Some(err) = list_error` branch (not behind
//! `.filter(|_| false)` or `if false { … }`).
#![allow(dead_code)]
use crate::core::dto::Paginated;
use crate::core::split_pane::{ListDetailItem, SplitPane, SplitPaneEmpty};
use crate::core::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::Value;

#[derive(Clone, PartialEq)]
struct Doc {
    id: String,
    title: String,
    category: String,
    published: bool,
    date: String,
    body: String,
    /// CMS `thumbnail_url` — set by hero upload (`POST /cms/uploads`) and hydrated from the list.
    thumbnail_url: String,
}

const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
const BADGE_WARNING: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow";

const CATEGORY_OPTIONS: &[(&str, &str)] = &[
    ("announcement", "Announcement"),
    ("sop", "SOP"),
    ("event", "Community Event"),
    ("modpack", "Modpack Update"),
    ("important", "Important"),
];
const MD_TOOLS: &[(&str, &str)] = &[
    ("format_bold", "Bold"),
    ("format_italic", "Italic"),
    ("link", "Link"),
    ("format_list_bulleted", "List"),
    ("image", "Image"),
];

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// `GET /api/v1/cms/announcements` — admin CMS list (drafts + published).
fn announcement_list_path() -> &'static str {
    "/cms/announcements?limit=100"
}

/// Create path — `POST /api/v1/cms/announcements`.
fn announcement_create_path() -> &'static str {
    "/cms/announcements"
}

/// Edit / archive path — `PATCH|DELETE /api/v1/cms/announcements/{id}`.
fn announcement_id_path(id: &str) -> String {
    format!("/cms/announcements/{id}")
}

/// Manual Discord (re)push — `POST /api/v1/cms/announcements/{id}/push-discord`.
fn announcement_push_path(id: &str) -> String {
    format!("/cms/announcements/{id}/push-discord")
}

/// Hero image multipart upload — `POST /api/v1/cms/uploads` (form field `"file"`).
fn cms_uploads_path() -> &'static str {
    "/cms/uploads"
}

/// Turn a `/uploads/…` response into an absolute http(s) URL so publish passes T-405
/// `validated_thumbnail` (relative paths are rejected). Already-absolute URLs pass through.
#[cfg(target_arch = "wasm32")]
fn absolute_cms_upload_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    let origin = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default();
    if origin.is_empty() {
        return trimmed.to_string();
    }
    if trimmed.starts_with('/') {
        format!("{origin}{trimmed}")
    } else {
        format!("{origin}/{trimmed}")
    }
}

/// Server-minted announcement ids are UUIDs (36 chars with hyphens). Local New-post ids
/// (`new-…`) are not — those still POST on first Publish.
fn is_server_id(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 36 && b[8] == b'-' && b[13] == b'-' && b[18] == b'-' && b[23] == b'-'
}

/// Map a doc category onto the announcement `tag` enum.
///
/// Measured (`apps/website/api/src/handlers/cms.rs` `valid_tag` /
/// `AnnouncementTag`): only `update|event|modpack_update|important`. There is **no** `sop` tag —
/// SOP posts as the closest tag `update` so Publish hits the live API (no fake local-only toast).
fn category_tag(category: &str) -> Option<&'static str> {
    match category {
        "announcement" | "sop" => Some("update"),
        "event" => Some("event"),
        "modpack" => Some("modpack_update"),
        "important" => Some("important"),
        _ => None,
    }
}

/// Inverse of [`category_tag`] for list hydrate. Wire `update` → `announcement` (SOP is
/// indistinguishable on the wire).
fn tag_category(tag: &str) -> String {
    match tag {
        "event" => "event".into(),
        "modpack_update" => "modpack".into(),
        "important" => "important".into(),
        _ => "announcement".into(),
    }
}

/// YYYY-MM-DD from an RFC3339 / Go-time ISO string (list meta column).
fn date_ymd(iso: &str) -> String {
    if iso.len() >= 10 {
        iso[..10].to_string()
    } else {
        String::new()
    }
}

/// Map one CMS announcement JSON row → editor `Doc`.
fn doc_from_announcement(v: &Value) -> Option<Doc> {
    let id = vstr(v, "id");
    if id.is_empty() {
        return None;
    }
    let status = vstr(v, "status");
    let published = status == "published";
    let published_at = vstr(v, "published_at");
    let created_at = vstr(v, "created_at");
    let updated_at = vstr(v, "updated_at");
    let date_src = if !published_at.is_empty() {
        published_at
    } else if !updated_at.is_empty() {
        updated_at
    } else {
        created_at
    };
    Some(Doc {
        id,
        title: vstr(v, "title"),
        category: tag_category(&vstr(v, "tag")),
        published,
        date: date_ymd(&date_src),
        body: vstr(v, "body"),
        thumbnail_url: vstr(v, "thumbnail_url"),
    })
}

/// Insert a real markdown snippet for a toolbar tool (no toast — the body change is the feedback).
fn apply_md_tool(body: &str, tool: &str) -> String {
    let snippet = match tool {
        "Bold" => "**bold**",
        "Italic" => "*italic*",
        "Link" => "[text](https://)",
        "List" => "\n- item",
        "Image" => "![alt](https://)",
        _ => return body.to_string(),
    };
    if body.is_empty() {
        snippet.to_string()
    } else if body.ends_with('\n') || tool == "List" {
        format!("{body}{snippet}")
    } else {
        format!("{body} {snippet}")
    }
}

/// `new Date().toISOString().slice(0, 10)` (frozen-clock parity in gates).
#[cfg(target_arch = "wasm32")]
fn today_iso() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .map(|s| s[..10.min(s.len())].to_string())
        .unwrap_or_default()
}

#[component]
pub fn ContentManagerPage() -> impl IntoView {
    let store = expect_context::<crate::core::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &store;
    // Mutable working set: seeded once from the CMS list Resource, then New/Publish/Delete mutate.
    let docs = RwSignal::new(Vec::<Doc>::new());
    let selected_id = RwSignal::new(None::<String>);
    let list_seeded = RwSignal::new(false);
    let list_error = RwSignal::new(None::<String>);
    let publish_busy = RwSignal::new(false);
    let delete_busy = RwSignal::new(false);

    // Keep Result — `.ok()` would collapse Err into None and the hydrate Effect would treat it
    // like a successful empty page (T-466).
    let list_res = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::core::client::api_get::<Paginated<Value>>(store, announcement_list_path()).await
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            Err::<Paginated<Value>, crate::core::client::ApiErr>((
                0,
                Some("CMS list unavailable off wasm".into()),
            ))
        }
    });

    // One-shot hydrate — do not overwrite local New drafts / publish edits on later reads.
    // Success-only: `list_seeded` stays false on Err so Retry can re-fetch and re-enter this Effect.
    Effect::new(move |_| {
        if list_seeded.get() {
            return;
        }
        let Some(result) = list_res.get() else {
            return;
        };
        match result {
            Ok(page) => {
                list_error.set(None);
                list_seeded.set(true);
                let mapped: Vec<Doc> = page.data.iter().filter_map(doc_from_announcement).collect();
                if selected_id.get_untracked().is_none() {
                    selected_id.set(mapped.first().map(|d| d.id.clone()));
                }
                docs.set(mapped);
            }
            Err(e) => {
                list_error.set(Some(crate::core::client::api_error_message(
                    &e,
                    "Failed to load announcements",
                )));
            }
        }
    });

    let retry_list = move |_| {
        list_error.set(None);
        list_res.refetch();
    };

    let new_post = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let id = format!("new-{}", js_sys::Date::now() as u64);
            let doc = Doc {
                id: id.clone(),
                title: "Untitled Post".into(),
                category: "announcement".into(),
                published: false,
                date: today_iso(),
                body: String::new(),
                thumbnail_url: String::new(),
            };
            docs.update(|d| d.insert(0, doc));
            selected_id.set(Some(id));
        }
    };

    view! {
        <crate::core::ui::AdminGate>
            <div class="relative h-full w-full overflow-hidden">
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                    <Suspense fallback=move || {
                        view! {
                            <p class="p-6 text-on-surface-variant">"Loading…"</p>
                        }
                    }>
                        {move || {
                            // Touch the resource so Suspense waits for the first fetch.
                            let _ = list_res.get();
                            view! {
                                <SplitPane
                                    transparent=true
                                    master_width="20rem"
                                    master_header=view! {
                                        <>
                                            <h1 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                                                "Comms Broadcaster"
                                            </h1>
                                            <button
                                                type="button"
                                                on:click=new_post
                                                class="flex shrink-0 items-center gap-1.5 rounded-full border border-white/10 px-3 py-1.5 text-label-sm text-on-surface transition hover:bg-white/5"
                                            >
                                                <MaterialIcon name="add" class="text-[18px]" />
                                                "New"
                                            </button>
                                        </>
                                    }
                                        .into_any()
                                    master=view! {
                                        {move || {
                                            if let Some(err) = list_error.get() {
                                                return view! {
                                                    <div class="flex flex-col gap-3 px-1 py-4">
                                                        <p
                                                            class="text-label-md text-error"
                                                            data-testid="content-list-error"
                                                        >
                                                            {err}
                                                        </p>
                                                        <button
                                                            type="button"
                                                            data-testid="content-list-retry"
                                                            on:click=retry_list
                                                            class="self-start rounded-full border border-white/10 px-3 py-1.5 text-label-sm text-on-surface transition hover:bg-white/5"
                                                        >
                                                            "Retry"
                                                        </button>
                                                    </div>
                                                }
                                                    .into_any();
                                            }
                                            let sel = selected_id.get();
                                            let rows = docs.get();
                                            if rows.is_empty() {
                                                return view! {
                                                    <p class="px-1 py-4 text-label-md text-on-surface-variant">
                                                        "No announcements yet."
                                                    </p>
                                                }
                                                    .into_any();
                                            }
                                            rows
                                                .into_iter()
                                                .map(|d| {
                                                    let (badge, label) = if d.published {
                                                        (BADGE_SUCCESS, "Published")
                                                    } else {
                                                        (BADGE_WARNING, "Draft")
                                                    };
                                                    let active = sel.as_deref() == Some(d.id.as_str());
                                                    let id_click = d.id.clone();
                                                    let title = if d.title.is_empty() {
                                                        "Untitled Post".to_string()
                                                    } else {
                                                        d.title.clone()
                                                    };
                                                    view! {
                                                        <ListDetailItem
                                                            active=active
                                                            on_click=Callback::new(move |()| {
                                                                selected_id.set(Some(id_click.clone()))
                                                            })
                                                            meta=view! { {d.date.clone()} }.into_any()
                                                            title=view! { {title} }.into_any()
                                                            trailing=view! { <span class=badge>{label}</span> }
                                                                .into_any()
                                                        />
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }}
                                    }
                                        .into_any()
                                    detail=view! {
                                        {move || {
                                            let sel = selected_id.get();
                                            let doc = docs
                                                .get()
                                                .into_iter()
                                                .find(|d| Some(&d.id) == sel.as_ref());
                                            match doc {
                                                Some(d) => {
                                                    editor(
                                                        d,
                                                        docs,
                                                        selected_id,
                                                        publish_busy,
                                                        delete_busy,
                                                        store,
                                                    )
                                                        .into_any()
                                                }
                                                None => {
                                                    view! {
                                                        <SplitPaneEmpty
                                                            icon=view! {
                                                                <MaterialIcon name="edit_note" class="text-4xl" />
                                                            }
                                                                .into_any()
                                                            message="Select a post or create a new one."
                                                        />
                                                    }
                                                        .into_any()
                                                }
                                            }
                                        }}
                                    }
                                        .into_any()
                                />
                            }
                                .into_any()
                        }}
                    </Suspense>
                </div>
            </div>
        </crate::core::ui::AdminGate>
    }
}

fn editor(
    d: Doc,
    docs: RwSignal<Vec<Doc>>,
    selected_id: RwSignal<Option<String>>,
    publish_busy: RwSignal<bool>,
    delete_busy: RwSignal<bool>,
    store: crate::core::auth::AuthStore,
) -> impl IntoView {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, publish_busy, delete_busy, docs, selected_id);
    let doc_id = StoredValue::new(d.id.clone());
    let was_published = StoredValue::new(d.published);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&doc_id, &was_published);
    let title = RwSignal::new(d.title.clone());
    let body = RwSignal::new(d.body.clone());
    let category = RwSignal::new(d.category.clone());
    let push_discord = RwSignal::new(true);
    let thumbnail_url = RwSignal::new(d.thumbnail_url.clone());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &thumbnail_url;

    // Write the edited fields back into the local list; optionally retarget the list id when the
    // server mints a UUID on first Publish.
    #[cfg(target_arch = "wasm32")]
    let apply = move |published: bool, new_id: Option<String>| {
        let t = title.get_untracked().trim().to_string();
        let old = doc_id.get_value();
        docs.update(|list| {
            if let Some(doc) = list.iter_mut().find(|x| x.id == old) {
                if let Some(ref nid) = new_id {
                    doc.id = nid.clone();
                }
                doc.title = if t.is_empty() {
                    "Untitled Post".into()
                } else {
                    t.clone()
                };
                doc.body = body.get_untracked();
                doc.category = category.get_untracked();
                doc.published = published;
                doc.date = today_iso();
                doc.thumbnail_url = thumbnail_url.get_untracked();
            }
        });
        if let Some(nid) = new_id {
            doc_id.set_value(nid.clone());
            selected_id.set(Some(nid));
        }
        if published {
            was_published.set_value(true);
        }
    };

    let save_draft = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            apply(false, None);
            crate::core::toast::use_toasts().success("Draft saved");
        }
    };

    let handle_publish = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::core::toast::use_toasts();
            let t = title.get_untracked().trim().to_string();
            let b = body.get_untracked().trim().to_string();
            if t.is_empty() || b.is_empty() {
                toasts.error("Title and body are required");
                return;
            }
            let Some(tag) = category_tag(&category.get_untracked()) else {
                toasts.error("Unknown category — cannot publish");
                return;
            };
            if publish_busy.get_untracked() {
                return;
            }
            publish_busy.set(true);
            let push = push_discord.get_untracked();
            let id = doc_id.get_value();
            let already_published = was_published.get_value();
            let payload = serde_json::json!({
                "title": t,
                "body": b,
                "tag": tag,
                "thumbnail_url": thumbnail_url.get_untracked(),
                "is_pinned": false,
                "push_to_discord": push,
                "status": "published",
            });
            leptos::task::spawn_local(async move {
                let result = if is_server_id(&id) {
                    // Edit existing row — never POST again (that duplicated announcements).
                    match crate::core::client::api_patch::<serde_json::Value>(
                        store,
                        &announcement_id_path(&id),
                        payload,
                    )
                    .await
                    {
                        Ok(_) => {
                            // PATCH only auto-pushes on first publish / never-pushed. Re-push an
                            // already-published row through the dedicated route.
                            if push && already_published {
                                match crate::core::client::api_post_ok(
                                    store,
                                    &announcement_push_path(&id),
                                    serde_json::json!({}),
                                )
                                .await
                                {
                                    Ok(()) => Ok(None),
                                    Err(e) => Err(e),
                                }
                            } else {
                                Ok(None)
                            }
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    match crate::core::client::api_post::<serde_json::Value>(
                        store,
                        announcement_create_path(),
                        payload,
                    )
                    .await
                    {
                        Ok(created) => {
                            let sid = created
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            if sid.is_empty() || !is_server_id(&sid) {
                                Err((0u16, Some("publish returned no id".into())))
                            } else {
                                Ok(Some(sid))
                            }
                        }
                        Err(e) => Err(e),
                    }
                };
                match result {
                    Ok(new_id) => {
                        apply(true, new_id);
                        toasts.success(if push {
                            "Published & broadcast to Discord"
                        } else {
                            "Published"
                        });
                    }
                    Err(e) => {
                        toasts.error(crate::core::client::api_error_message(&e, "Publish failed"));
                    }
                }
                publish_busy.set(false);
            });
        }
    };

    let handle_delete = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::core::toast::use_toasts();
            if delete_busy.get_untracked() {
                return;
            }
            let id = doc_id.get_value();
            if is_server_id(&id) {
                delete_busy.set(true);
                leptos::task::spawn_local(async move {
                    match crate::core::client::api_delete(store, &announcement_id_path(&id)).await {
                        Ok(()) => {
                            docs.update(|list| list.retain(|d| d.id != id));
                            selected_id.set(None);
                            toasts.success("Announcement archived");
                        }
                        Err(e) => {
                            toasts
                                .error(crate::core::client::api_error_message(&e, "Delete failed"));
                        }
                    }
                    delete_busy.set(false);
                });
            } else {
                // Local-only New draft — drop from the list; nothing to hit on the API.
                docs.update(|list| list.retain(|d| d.id != id));
                selected_id.set(None);
                toasts.success("Draft discarded");
            }
        }
    };

    let handle_hero = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::closure::Closure;
            use wasm_bindgen::JsCast;

            let toasts = crate::core::toast::use_toasts();
            let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                toasts.error("Hero image upload failed — no document");
                return;
            };
            let Ok(input) = document
                .create_element("input")
                .map_err(|_| ())
                .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().map_err(|_| ()))
            else {
                toasts.error("Hero image upload failed — could not open file picker");
                return;
            };
            input.set_type("file");
            input.set_accept("image/jpeg,image/png,image/webp,.jpg,.jpeg,.png,.webp");

            let input_for_cb = input.clone();
            let on_change = Closure::once(move |_ev: web_sys::Event| {
                let Some(file) = input_for_cb.files().and_then(|list| list.item(0)) else {
                    return;
                };
                leptos::task::spawn_local(async move {
                    match crate::core::client::api_upload_file::<serde_json::Value>(
                        store,
                        cms_uploads_path(),
                        file,
                    )
                    .await
                    {
                        Ok(resp) => {
                            let raw = resp
                                .get("url")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if raw.is_empty() {
                                toasts.error("Upload returned no url");
                                return;
                            }
                            let url = absolute_cms_upload_url(&raw);
                            thumbnail_url.set(url.clone());
                            let id = doc_id.get_value();
                            docs.update(|list| {
                                if let Some(doc) = list.iter_mut().find(|x| x.id == id) {
                                    doc.thumbnail_url = url;
                                }
                            });
                            toasts.success("Hero image uploaded");
                        }
                        Err(e) => {
                            toasts.error(crate::core::client::api_error_message(
                                &e,
                                "Hero upload failed",
                            ));
                        }
                    }
                });
            });
            let _ = input
                .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
            // Keep the one-shot listener alive past this stack frame (picker is fire-and-forget).
            on_change.forget();
            input.click();
        }
    };

    view! {
        <div class="relative flex h-full min-w-0 flex-1 flex-col">
            <div class="flex items-start justify-between gap-4 p-8 pb-4">
                <input
                    type="text"
                    prop:value=move || title.get()
                    on:input=move |ev| title.set(event_target_value(&ev))
                    placeholder="Post Title"
                    class="min-w-0 flex-1 bg-transparent text-4xl font-bold text-on-surface outline-none placeholder:text-outline"
                />
                <div class="flex shrink-0 items-center gap-2">
                    <select
                        prop:value=move || category.get()
                        on:change=move |ev| category.set(event_target_value(&ev))
                        class="rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface outline-none focus:border-primary/50"
                    >
                        {CATEGORY_OPTIONS
                            .iter()
                            .map(|(value, label)| {
                                view! { <option value=*value>{*label}</option> }
                            })
                            .collect_view()}
                    </select>
                    <button
                        type="button"
                        data-testid="content-hero-image"
                        on:click=handle_hero
                        class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                    >
                        <MaterialIcon name="image" class="text-[18px]" />
                        "Add Hero Image"
                    </button>
                </div>
            </div>
            <div class="sticky top-0 z-10 mx-8 flex items-center gap-1 rounded-xl border border-white/10 bg-surface-container/60 p-1 backdrop-blur-md">
                {MD_TOOLS
                    .iter()
                    .map(|(icon, label)| {
                        let tool = (*label).to_string();
                        view! {
                            <button
                                type="button"
                                data-testid=format!("content-md-{}", label.to_lowercase())
                                on:click=move |_| {
                                    body.update(|b| *b = apply_md_tool(b, &tool));
                                }
                                aria-label=*label
                                title=*label
                                class="flex size-9 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-white/10 hover:text-on-surface"
                            >
                                <MaterialIcon name=*icon class="text-[20px]" />
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <textarea
                prop:value=move || body.get()
                on:input=move |ev| body.set(event_target_value(&ev))
                placeholder="Start writing… Markdown supported."
                class="w-full flex-1 resize-none bg-transparent p-8 text-lg leading-relaxed text-on-surface outline-none placeholder:text-outline"
            >
                {d.body.clone()}
            </textarea>
            <div class="flex items-center justify-between gap-4 border-t border-white/10 bg-white/5 p-6 backdrop-blur-md">
                <label class="flex items-center gap-3" id="sw-label">
                    {switch(push_discord)}
                    <span class="text-label-md text-on-surface-variant">"Push to Discord"</span>
                </label>
                <div class="flex items-center gap-3">
                    <button
                        type="button"
                        data-testid="content-delete"
                        on:click=handle_delete
                        prop:disabled=move || delete_busy.get()
                        class="rounded-full border border-error-alert/40 px-6 py-3 text-label-md text-error-alert transition hover:bg-error-alert/10 disabled:opacity-50"
                    >
                        "Delete"
                    </button>
                    <button
                        type="button"
                        on:click=save_draft
                        class="rounded-full border border-white/10 px-6 py-3 text-label-md text-on-surface transition hover:bg-white/5"
                    >
                        "Save Draft"
                    </button>
                    <button
                        type="button"
                        data-testid="content-publish"
                        on:click=handle_publish
                        prop:disabled=move || publish_busy.get()
                        class="rounded-full bg-action px-7 py-3 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
                    >
                        "Publish & Broadcast"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Base-UI Switch — reproduced from the oracle DOM: a `<span role="switch">` root (labelled by the
/// wrapping `<label id="sw-label">`) + a visually-hidden checkbox. T-159.25 makes it live: click /
/// toggle flips the signal, mirrored into aria-checked + the data-checked styling attributes.
fn switch(checked: RwSignal<bool>) -> impl IntoView {
    let root = "group relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-outline-variant/60 bg-surface-container-high p-0.5 outline-none transition-colors focus-visible:ring-2 focus-visible:ring-primary/50 data-[checked]:border-primary data-[checked]:bg-primary data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50";
    let thumb = "h-3.5 w-3.5 rounded-full bg-on-surface-variant shadow-sm transition-all data-[checked]:translate-x-4 data-[checked]:bg-on-primary";
    view! {
        <span
            id="sw-root"
            role="switch"
            aria-checked=move || if checked.get() { "true" } else { "false" }
            aria-labelledby="sw-label"
            tabindex="0"
            attr:data-checked=move || checked.get().then_some("")
            class=root
            on:click=move |_| checked.update(|v| *v = !*v)
        >
            <span class=thumb attr:data-checked=move || checked.get().then_some("")></span>
        </span>
        <input
            id="sw-input"
            type="checkbox"
            prop:checked=move || checked.get()
            aria-hidden="true"
            tabindex="-1"
            style="clip-path: inset(50%); overflow: hidden; white-space: nowrap; border: 0px; padding: 0px; width: 1px; height: 1px; margin: -1px; position: fixed; top: 0px; left: 0px;"
        />
    }
}

#[cfg(test)]
mod tests {
    use super::{
        announcement_create_path, announcement_id_path, announcement_list_path,
        announcement_push_path, apply_md_tool, category_tag, date_ymd, doc_from_announcement,
        is_server_id, tag_category,
    };
    use serde_json::json;

    /// T-267 Class-R — every UI category (incl. SOP) must resolve to a live `announcement_tag`.
    #[test]
    fn category_tag_covers_all_ui_categories_including_sop() {
        assert_eq!(category_tag("announcement"), Some("update"));
        assert_eq!(
            category_tag("sop"),
            Some("update"),
            "SOP has no enum variant — closest tag is update (perturbation: return None for sop)"
        );
        assert_eq!(category_tag("event"), Some("event"));
        assert_eq!(category_tag("modpack"), Some("modpack_update"));
        assert_eq!(category_tag("important"), Some("important"));
        assert_eq!(category_tag("nope"), None);
    }

    #[test]
    fn server_id_detects_uuid_not_local_mock() {
        assert!(is_server_id("44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"));
        assert!(!is_server_id("d1"));
        assert!(!is_server_id("new-1710000000000"));
        assert!(!is_server_id(""));
    }

    #[test]
    fn tag_category_round_trips_live_tags() {
        assert_eq!(tag_category("update"), "announcement");
        assert_eq!(tag_category("event"), "event");
        assert_eq!(tag_category("modpack_update"), "modpack");
        assert_eq!(tag_category("important"), "important");
    }

    #[test]
    fn doc_from_announcement_maps_status_and_dates() {
        let row = json!({
            "id": "44fa4c17-5bd5-4c6b-b02d-4ccd52af6910",
            "title": "Live row",
            "body": "hello",
            "tag": "modpack_update",
            "status": "published",
            "published_at": "2026-07-27T12:00:00Z",
            "created_at": "2026-07-01T00:00:00Z",
            "updated_at": "2026-07-27T12:00:00Z",
        });
        let doc = doc_from_announcement(&row).expect("row maps");
        assert_eq!(doc.id, "44fa4c17-5bd5-4c6b-b02d-4ccd52af6910");
        assert_eq!(doc.category, "modpack");
        assert!(doc.published);
        assert_eq!(doc.date, "2026-07-27");
        assert_eq!(date_ymd("2026-06-18T00:00:00Z"), "2026-06-18");
    }

    #[test]
    fn cms_paths_match_axum_routes() {
        assert_eq!(announcement_list_path(), "/cms/announcements?limit=100");
        assert_eq!(announcement_create_path(), "/cms/announcements");
        assert_eq!(
            announcement_id_path("44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"),
            "/cms/announcements/44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"
        );
        assert_eq!(
            announcement_push_path("44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"),
            "/cms/announcements/44fa4c17-5bd5-4c6b-b02d-4ccd52af6910/push-discord"
        );
        const APP_RS: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../api/src/app.rs"));
        assert!(
            APP_RS.contains(r#""/cms/announcements""#),
            "app.rs must register /cms/announcements"
        );
        // T-447 Class-R — GET must share the POST route (was POST-only → 405 on list).
        assert!(
            APP_RS.contains("get(handlers::cms::list_cms_announcements).post(handlers::cms::create_announcement)"),
            "app.rs must register GET+POST on /cms/announcements (perturbation: post-only)"
        );
        assert!(
            APP_RS.contains(r#""/cms/announcements/{id}""#),
            "app.rs must register PATCH|DELETE /cms/announcements/{{id}}"
        );
        assert!(
            APP_RS.contains(r#""/cms/announcements/{id}/push-discord""#),
            "app.rs must register POST …/push-discord"
        );
        assert!(
            APP_RS.contains(r#""/cms/uploads""#),
            "app.rs must still register POST /cms/uploads"
        );
        assert_eq!(super::cms_uploads_path(), "/cms/uploads");
    }

    /// T-447 / T-465 Class-R — boot must LocalResource GET the CMS list **and** the hydrate
    /// Effect must apply mapped page data into `docs` (not ignore `opt` / hardcode).
    ///
    /// RED perturbation (Wave 25 verifier B2): keep LocalResource/api_get + `doc_from_announcement`
    /// but Effect does `docs.set(hardcoded)` / never reads `opt` → FAIL (missing map + set needles).
    ///
    /// # Cure 2 (scrub-then-grep), and why not cure 1 (T-601)
    ///
    /// What is pinned here is a **wiring seam**, not a value: a `LocalResource` inside a Leptos
    /// component and an `Effect` that reads its `Option` and writes a signal. There is no function
    /// to call — the seam only exists once a reactive runtime has mounted the component — so cure 1
    /// would have to stand up a fake Leptos and then assert against the fake. The mapping half that
    /// *does* have a runtime signature, `doc_from_announcement`, is already covered by its own
    /// value tests below; this pin covers only "the boot path reaches it", which is source shape.
    ///
    /// T-601 replaces the raw `SRC.split("#[cfg(test)]")` with the shared scrubber: comments and
    /// string literals blanked, `#[cfg(<false>)]` items and constant-false blocks removed. Before
    /// that, every positive needle here would have been satisfied by a commented-out line — and
    /// `announcement_list_path` in particular is discussed in prose a few lines above its own
    /// definition.
    #[test]
    fn content_boots_from_cms_list_not_mock_docs() {
        use crate::arsenal::class_r_scrub::live_code;
        const SRC: &str = include_str!("content.rs");
        let prod = live_code(SRC);
        assert!(
            !prod.contains("fn mock_docs()"),
            "mock_docs must be gone from production (perturbation: restore mock seed helper)"
        );
        assert!(
            !prod.contains("RwSignal::new(mock_docs())"),
            "must not boot the master list from mock_docs alone"
        );
        assert!(
            prod.contains("announcement_list_path"),
            "list path helper must exist"
        );
        assert!(
            prod.contains("LocalResource::new")
                && prod.contains("api_get::<Paginated<Value>>(store, announcement_list_path())"),
            "boot must LocalResource api_get the CMS list (perturbation: drop Resource)"
        );
        // B2 — Effect must map `page.data` and write it into `docs`. Needles assembled so
        // this test's source / a free `doc_from_announcement` mention cannot false-green.
        let map_page = format!(
            "{}{}",
            "page.data.iter().filter_map(", "doc_from_announcement)"
        );
        let set_docs = format!("{}{}", "docs.set(", "mapped)");
        assert!(
            prod.contains(&map_page),
            "hydrate Effect must `{map_page}` (perturbation: ignore opt / skip mapping)"
        );
        assert!(
            prod.contains(&set_docs),
            "hydrate Effect must `{set_docs}` (perturbation: hardcoded docs.set / drop apply)"
        );
    }

    /// **T-601 — calibration for the boot pin above.**
    ///
    /// Every wrapper in the handed-down battery is applied to the two needles the pin cannot do
    /// without, and each must stop satisfying it. Without this, a future edit that weakened the
    /// scrubber would leave the pin green over a CMS list the SPA never fetches — which is the
    /// exact bug T-447 shipped to fix.
    #[test]
    fn the_boot_pin_rejects_every_dead_code_wrapper() {
        use crate::arsenal::class_r_scrub::live_code;
        let needle = "LocalResource::new";
        let attacks: [(&str, String); 12] = [
            (
                "if true == false",
                format!("if true == false {{ {needle}(f); }}"),
            ),
            (
                "loop { break; … }",
                format!("loop {{ break; {needle}(f); }}"),
            ),
            (
                "#[cfg(any())]",
                format!("#[cfg(any())] fn d() {{ {needle}(f); }}"),
            ),
            ("while false", format!("while false {{ {needle}(f); }}")),
            ("if !true", format!("if !true {{ {needle}(f); }}")),
            ("if 1 > 2", format!("if 1 > 2 {{ {needle}(f); }}")),
            (
                "if std::hint::black_box(false)",
                format!("if std::hint::black_box(false) {{ {needle}(f); }}"),
            ),
            (
                "const C: bool = false; if C",
                format!("const C: bool = false;\nfn d() {{ if C {{ {needle}(f); }} }}"),
            ),
            (
                "return; above",
                format!("fn d() {{ return; {needle}(f); }}"),
            ),
            (
                "#[cfg(any())] mod shadow",
                format!("#[cfg(any())] mod shadow {{ fn d() {{ {needle}(f); }} }}"),
            ),
            (
                "match guard",
                format!("match () {{ _ if false => {{ {needle}(f); }} _ => {{}} }}"),
            ),
            ("comment", format!("// {needle}(f)")),
        ];
        for (label, body) in attacks {
            let forged = format!("fn content_page() {{\n    {body}\n}}\n#[cfg(test)]\n");
            assert!(
                !live_code(&forged).contains(needle),
                "{label}: the boot needle survived scrubbing — this pin would report a live CMS \
                 fetch over code the build never runs"
            );
        }
        // Two attacks the handed-down list does not contain (see arsenal.rs for the full write-up).
        assert!(
            !live_code(&format!(
                "const NEVER: bool = 1 > 2;\nfn d() {{ if NEVER {{ {needle}(f); }} }}\n#[cfg(test)]\n"
            ))
            .contains(needle),
            "A2: a constant folded through a comparison never spells `false` — grepping for \
             `= false` would have shipped this hole"
        );
        assert!(
            !live_code(&format!(
                "#[cfg(all(any(), unix))] fn d() {{ {needle}(f); }}\n#[cfg(test)]\n"
            ))
            .contains(needle),
            "a composite never-true cfg is not the literal `#[cfg(any())]`"
        );
        let live = format!("fn content_page() {{\n    {needle}(f);\n}}\n#[cfg(test)]\n");
        assert!(live_code(&live).contains(needle));
    }

    /// Strip all whitespace so `set( true )` and `set(true)` compare equal.
    fn strip_ws(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    /// T-475 — strip TOML `#` comments and `//` line comments (string-aware) so a commented
    /// feature line (`# "FormData",`) cannot satisfy Cargo feature pins.
    fn strip_toml_comments(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let mut chars = src.chars().peekable();
        let mut in_string = false;
        while let Some(c) = chars.next() {
            if in_string {
                out.push(c);
                if c == '\\' {
                    if let Some(n) = chars.next() {
                        out.push(n);
                    }
                    continue;
                }
                if c == '"' {
                    in_string = false;
                }
                continue;
            }
            if c == '"' {
                in_string = true;
                out.push(c);
                continue;
            }
            if c == '/' && matches!(chars.peek(), Some('/')) {
                chars.next();
                while let Some(n) = chars.next() {
                    if n == '\n' {
                        out.push('\n');
                        break;
                    }
                }
                continue;
            }
            if c == '#' {
                while let Some(n) = chars.next() {
                    if n == '\n' {
                        out.push('\n');
                        break;
                    }
                }
                continue;
            }
            out.push(c);
        }
        out
    }

    /// Strip `//` and `/* */` so comment-only needles cannot satisfy live-call pins.
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

    /// Count every `list_seeded.set(` call — any arg form, whitespace-insensitive.
    fn count_list_seeded_sets(s: &str) -> usize {
        strip_ws(s).matches("list_seeded.set(").count()
    }

    /// True-ish seed args after whitespace strip: `true`, `(true)`, `true as bool`, `{ true }`.
    fn has_true_list_seed(s: &str) -> bool {
        let t = strip_ws(s);
        t.contains("list_seeded.set(true)")
            || t.contains("list_seeded.set((true))")
            || t.contains("list_seeded.set(trueasbool)")
            || t.contains("list_seeded.set({true})")
    }

    /// T-466 / T-470 / T-473 Class-R — list fetch must keep `Result` (no `.ok()`), seed **only**
    /// inside the Ok arm (order/window pin; all `list_seeded.set(...)` forms count), and surface
    /// a **reachable** error + Retry.
    ///
    /// Wave 26/27 adversarial RED (exact-string / filter-only pins were false-green):
    /// 1. Early `list_seeded.set( true )` (spaced) while Ok keeps `set(true)` → must FAIL
    /// 2. Err `set( true )` / `set((true))` / `set(true as bool)` / `set({ true })` /
    ///    `let seed=true; set(seed)` → must FAIL
    /// 3. Wrap Retry UI in `if false { … }` (no `.filter`) → must FAIL
    #[test]
    fn content_list_error_does_not_seed_as_empty_success() {
        const SRC: &str = include_str!("content.rs");
        let prod = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("content.rs must have a #[cfg(test)] module");

        // List Resource must not collapse Err→None (perturbation: restore `.await.ok()`).
        let list_get = "api_get::<Paginated<Value>>(store, announcement_list_path())";
        let list_region = prod
            .split("LocalResource::new")
            .nth(1)
            .and_then(|s| s.split("Effect::new").next())
            .expect("LocalResource block before Effect");
        assert!(
            list_region.contains(list_get),
            "list LocalResource must call {list_get}"
        );
        assert!(
            !list_region.contains(".ok()"),
            "list LocalResource must not .ok() the GET (perturbation: Err→None empty success)"
        );

        // Hydrate Effect window (one-shot seed + Err surface) — before retry_list.
        let effect = prod
            .split("Effect::new")
            .nth(1)
            .and_then(|s| s.split("let retry_list").next())
            .expect("hydrate Effect must sit before retry_list");

        // (1) Order pin — any list_seeded.set(...) before Ok(page) must FAIL (incl. spaced).
        let before_ok = effect
            .split("Ok(page)")
            .next()
            .expect("hydrate Effect must match on Ok(page)");
        assert_eq!(
            count_list_seeded_sets(before_ok),
            0,
            "list_seeded.set(...) must not run before Ok(page) \
             (perturbation: spaced/early seed before match / outside Ok)"
        );

        // Ok arm window: Ok(page) … Err(e) — exactly one true-ish seed.
        let ok_arm = effect
            .split("Ok(page)")
            .nth(1)
            .and_then(|s| s.split("Err(e)").next())
            .expect("Ok(page) arm must precede Err(e)");
        assert!(
            has_true_list_seed(ok_arm),
            "Ok arm must set list_seeded to true (perturbation: drop success seed)"
        );
        assert_eq!(
            count_list_seeded_sets(ok_arm),
            1,
            "Ok arm must call list_seeded.set exactly once"
        );
        let set_docs = format!("{}{}", "docs.set(", "mapped)");
        assert!(
            ok_arm.contains(&set_docs),
            "Ok hydrate must `{set_docs}` inside the Ok arm"
        );

        // (2) Err arm — any list_seeded.set form must FAIL (spaced / (true) / as bool / {{}} / seed).
        let err_arm = effect
            .split("Err(e)")
            .nth(1)
            .expect("hydrate Effect must have Err(e) arm");
        assert_eq!(
            count_list_seeded_sets(err_arm),
            0,
            "Err arm must not call list_seeded.set(...) \
             (perturbation: spaced/indirect seed in Err)"
        );
        assert!(
            err_arm.contains("Failed to load announcements") && err_arm.contains("list_error.set"),
            "Err arm must write list_error with the actionable message"
        );

        // Exactly one set in the whole hydrate Effect — and it is inside Ok (above).
        assert_eq!(
            count_list_seeded_sets(effect),
            1,
            "hydrate Effect must call list_seeded.set exactly once (inside Ok only)"
        );
        assert!(
            !strip_ws(prod).contains(&strip_ws(
                "list_seeded.set(true);\n        let Some(page) = opt else"
            )),
            "must not seed then early-return on None (perturbation: restore seed-on-None)"
        );

        // (3) Reachable error UI — dead `.filter(|_| false)` OR `if false { … }` must FAIL.
        let master = prod
            .split("master=view!")
            .nth(1)
            .and_then(|s| s.split("detail=view!").next())
            .expect("master pane must sit before detail");
        let master_ws = strip_ws(master);
        assert!(
            !master_ws.contains("filter(|_|false)"),
            "master pane must not gate UI behind .filter(|_| false) \
             (perturbation: unreachable error UI)"
        );
        assert!(
            !master_ws.contains("iffalse"),
            "master pane must not wrap UI in `if false` / `if false {{ … }}` \
             (perturbation: unreachable Retry without .filter)"
        );
        let err_bind = "if let Some(err) = list_error.get()";
        let err_start = master
            .find(err_bind)
            .unwrap_or_else(|| panic!("master must bind list_error via `{err_bind}`"));
        let empty_i = master
            .find("\"No announcements yet.\"")
            .expect("empty success copy must remain for true zero-row Ok responses");
        assert!(
            err_start < empty_i,
            "reachable list_error if-let must precede empty success copy"
        );
        let err_window = &master[err_start..empty_i];
        assert!(
            err_window.contains("content-list-error")
                && err_window.contains("content-list-retry")
                && err_window.contains("\"Retry\"")
                && err_window.contains("on:click=retry_list"),
            "Retry/error UI must sit inside the reachable list_error if-let branch \
             (perturbation: needles present but unreachable)"
        );
        assert!(
            !strip_ws(err_window).contains("iffalse"),
            "list_error if-let body must not nest `if false` around Retry"
        );
        assert!(
            prod.contains("list_res.refetch()"),
            "Retry handler must refetch the list Resource"
        );
    }

    /// Source guards — go RED if Publish discards the id again, SOP fakes success, or MD mocks.
    /// Production-only slice of the file so assert strings cannot self-satisfy `include_str!`.
    #[test]
    fn publish_edit_delete_push_are_wired_no_fake_toasts() {
        const SRC: &str = include_str!("content.rs");
        let prod = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("content.rs must have a #[cfg(test)] module");
        assert!(
            prod.contains("api_patch::<serde_json::Value>"),
            "re-Publish of a server id must PATCH (perturbation: remove api_patch)"
        );
        assert!(
            prod.contains("api_delete(store, &announcement_id_path"),
            "Delete must call api_delete on /cms/announcements/{{id}}"
        );
        assert!(
            prod.contains("announcement_push_path") && prod.contains("api_post_ok"),
            "re-push must hit …/push-discord via api_post_ok"
        );
        assert!(
            prod.contains(".get(\"id\")"),
            "POST create must read the returned id (perturbation: discard Ok(_) body)"
        );
        assert!(
            prod.contains("is_server_id"),
            "Publish must branch POST vs PATCH on server id"
        );
        assert!(
            !prod.contains("success(\"SOP published\")"),
            "SOP must not toast local-only success (perturbation: restore fake SOP toast)"
        );
        assert!(
            prod.contains("\"announcement\" | \"sop\""),
            "SOP must share the update tag arm with announcement"
        );
        assert!(
            !prod.contains("(mock)"),
            "markdown tools must not toast mock success"
        );
        assert!(
            prod.contains("apply_md_tool"),
            "markdown toolbar must mutate body via apply_md_tool"
        );
        assert!(
            !prod.contains("Hero image upload unavailable"),
            "hero stub toast must be gone (perturbation: restore honest-refusal stub)"
        );
        assert!(
            !prod.contains("Hero image upload coming soon"),
            "old stub success toast must be gone"
        );
        assert!(
            prod.contains("api_upload_file")
                && prod.contains("cms_uploads_path()")
                && prod.contains("thumbnail_url.set"),
            "hero button must POST multipart via api_upload_file and set draft thumbnail_url \
             (perturbation: restore stub toast / drop upload call)"
        );
        assert!(
            prod.contains("\"thumbnail_url\": thumbnail_url.get_untracked()"),
            "Publish payload must carry thumbnail_url (perturbation: omit from json!)"
        );
    }

    /// T-446 / T-475 Class-R — multipart hero path is wired end-to-end in owns files (Cargo
    /// features + client helper + content handler). A regression that drops FormData/File or the
    /// helper FAILS even if content.rs still looks "honest".
    ///
    /// T-475: Cargo feature pins run on comment-stripped TOML (commenting `# "FormData",` must
    /// FAIL). Hero Ok path must live-call `absolute_cms_upload_url(&raw)` (not comment-only) so
    /// Publish keeps T-405 absolute http(s) thumbnails.
    #[test]
    fn hero_multipart_upload_is_wired_not_stubbed() {
        const CARGO: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        const CLIENT: &str = include_str!("core/client.rs");
        const SRC: &str = include_str!("content.rs");
        let prod = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("content.rs must have a #[cfg(test)] module");
        let prod_code = strip_rust_comments(prod);
        let cargo_live = strip_toml_comments(CARGO);
        let client_prod = CLIENT
            .split("#[cfg(test)]")
            .next()
            .expect("client.rs must have a #[cfg(test)] module");

        assert!(
            cargo_live.contains("\"FormData\"") && cargo_live.contains("\"File\""),
            "Cargo.toml must enable web-sys FormData + File (perturbation: comment-out / drop features)"
        );
        assert!(
            cargo_live.contains("\"FileList\""),
            "Cargo.toml must enable FileList for HtmlInputElement::files() \
             (perturbation: comment-out / drop FileList)"
        );
        assert!(
            client_prod.contains("pub async fn api_upload_file<")
                && client_prod.contains("FormData::new()")
                && client_prod.contains("append_with_blob_and_filename(\"file\""),
            "client must expose api_upload_file with FormData field \"file\" \
             (perturbation: delete helper / rename field)"
        );
        assert!(
            !prod.contains("Hero image upload unavailable — multipart client not wired"),
            "content must not keep the T-267 honest-refusal stub"
        );
        assert!(
            prod.contains("api_upload_file::<serde_json::Value>")
                && prod.contains("success(\"Hero image uploaded\")"),
            "handle_hero must upload and toast success on Ok"
        );
        assert!(
            prod.contains("api_error_message") && prod.contains("Hero upload failed"),
            "handle_hero must toast the real API error on Err"
        );
        // T-475 hole 2 — absolute URL for T-405 validated_thumbnail (non-comment).
        // Definition needle (`fn absolute_cms_upload_url`) ≠ call needle (`…(&raw)`): a bare
        // `absolute_cms_upload_url(` would false-green on the fn signature alone.
        assert!(
            prod_code.contains("fn absolute_cms_upload_url"),
            "prod must define absolute_cms_upload_url (perturbation: comment-only / delete helper)"
        );
        assert!(
            prod_code.contains("absolute_cms_upload_url(&raw)"),
            "hero Ok path must live-call absolute_cms_upload_url(&raw) \
             (perturbation: let url = raw; / comment-only call)"
        );
    }

    #[test]
    fn apply_md_tool_inserts_real_markers() {
        assert_eq!(apply_md_tool("", "Bold"), "**bold**");
        assert_eq!(apply_md_tool("hi", "Italic"), "hi *italic*");
        assert!(apply_md_tool("x", "Link").contains("](https://)"));
        assert!(apply_md_tool("x", "List").contains("- item"));
        assert!(apply_md_tool("", "Image").starts_with("![alt]"));
    }

    /// Perturbation oracle: discarding next_cursor-style — if we mapped SOP to None again, Publish
    /// would take the fake-success branch. This pins the RED difference.
    #[test]
    fn sop_none_mapping_is_detectably_wrong() {
        let live = category_tag("sop");
        let discarded: Option<&str> = None; // pre-T-267: category_tag("sop") → None → local toast
        assert_eq!(live, Some("update"));
        assert_ne!(
            live, discarded,
            "mapping SOP to None reintroduces the fake local-only publish path"
        );
    }
}
