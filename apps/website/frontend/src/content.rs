//! Comms Broadcaster / Content Manager (/admin/content) — ported from pages/admin.tsx
//! `ContentManagerPage` + `ContentEditor`. `<AdminGate>` → a transparent `SplitPane`: a post list
//! (master) + the editor form (detail).
//!
//! T-267: Publish keeps the returned announcement id and re-Publish PATCHes that row (no duplicate
//! POSTs). Delete / Discord re-push hit the live CMS routes. SOP has no `announcement_tag` variant
//! (measured: `update|event|modpack_update|important` only) — mapped to closest tag `update`.
//! Markdown toolbar inserts real markers into the body (no mock success toasts). Hero image upload
//! needs multipart/`FormData` (web-sys features outside this file's owns) — honest error, not a
//! fake success toast.
#![allow(dead_code)]
use crate::split_pane::{ListDetailItem, SplitPane, SplitPaneEmpty};
use crate::ui::MaterialIcon;
use leptos::prelude::*;

#[derive(Clone, PartialEq)]
struct Doc {
    id: String,
    title: String,
    category: String,
    published: bool,
    date: String,
    body: String,
}

const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
const BADGE_WARNING: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow";

fn mock_docs() -> Vec<Doc> {
    vec![
        Doc {
            id: "d1".into(),
            title: "Operation Blue Storm Briefing".into(),
            category: "announcement".into(),
            published: true,
            date: "2026-06-18".into(),
            body: "All units, Operation Blue Storm kicks off Saturday at 1900Z. BLUFOR will stage at the southern airfield...\n\nReview your ORBAT assignments and ensure your modpack is current.".into(),
        },
        Doc {
            id: "d2".into(),
            title: "SOP: Armor Tactics".into(),
            category: "sop".into(),
            published: true,
            date: "2026-06-12".into(),
            body: "# Armor Doctrine\n\nNever advance armor without infantry support. Maintain hull-down positions where possible and...".into(),
        },
        Doc {
            id: "d3".into(),
            title: "Modpack v2.4.1 Changelog".into(),
            category: "modpack".into(),
            published: false,
            date: "2026-06-20".into(),
            body: "Draft notes for the upcoming modpack bump:\n- Added RHS Status Quo\n- Removed deprecated optics pack".into(),
        },
    ]
}

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

/// Server-minted announcement ids are UUIDs (36 chars with hyphens). Local mock / new-post ids
/// (`d1`, `new-…`) are not — those still POST on first Publish.
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
    let store = expect_context::<crate::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &store;
    let docs = RwSignal::new(mock_docs());
    let selected_id = RwSignal::new(Some("d1".to_string()));
    // The editor re-keys on selection: bump forces a rebuild seeded from the newly selected doc.
    let publish_busy = RwSignal::new(false);
    let delete_busy = RwSignal::new(false);

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
            };
            docs.update(|d| d.insert(0, doc));
            selected_id.set(Some(id));
        }
    };

    view! {
        <crate::ui::AdminGate>
            <div class="relative h-full w-full overflow-hidden">
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
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
                                let sel = selected_id.get();
                                docs.get()
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
                </div>
            </div>
        </crate::ui::AdminGate>
    }
}

fn editor(
    d: Doc,
    docs: RwSignal<Vec<Doc>>,
    selected_id: RwSignal<Option<String>>,
    publish_busy: RwSignal<bool>,
    delete_busy: RwSignal<bool>,
    store: crate::auth::AuthStore,
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
            crate::toast::use_toasts().success("Draft saved");
        }
    };

    let handle_publish = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::toast::use_toasts();
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
                "is_pinned": false,
                "push_to_discord": push,
                "status": "published",
            });
            leptos::task::spawn_local(async move {
                let result = if is_server_id(&id) {
                    // Edit existing row — never POST again (that duplicated announcements).
                    match crate::client::api_patch::<serde_json::Value>(
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
                                match crate::client::api_post_ok(
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
                    match crate::client::api_post::<serde_json::Value>(
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
                        toasts.error(crate::client::api_error_message(&e, "Publish failed"));
                    }
                }
                publish_busy.set(false);
            });
        }
    };

    let handle_delete = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::toast::use_toasts();
            if delete_busy.get_untracked() {
                return;
            }
            let id = doc_id.get_value();
            if is_server_id(&id) {
                delete_busy.set(true);
                leptos::task::spawn_local(async move {
                    match crate::client::api_delete(store, &announcement_id_path(&id)).await {
                        Ok(()) => {
                            docs.update(|list| list.retain(|d| d.id != id));
                            selected_id.set(None);
                            toasts.success("Announcement archived");
                        }
                        Err(e) => {
                            toasts.error(crate::client::api_error_message(&e, "Delete failed"));
                        }
                    }
                    delete_busy.set(false);
                });
            } else {
                // Local-only draft / mock row — drop from the list; nothing to hit on the API.
                docs.update(|list| list.retain(|d| d.id != id));
                selected_id.set(None);
                toasts.success("Draft discarded");
            }
        }
    };

    let handle_hero = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            // POST /cms/uploads is multipart (`file` field). Wiring it needs web-sys FormData/File
            // features (Cargo.toml — outside content.rs owns). Honest refusal, not a success toast.
            crate::toast::use_toasts()
                .error("Hero image upload unavailable — multipart client not wired in this slice");
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
        announcement_create_path, announcement_id_path, announcement_push_path, apply_md_tool,
        category_tag, is_server_id,
    };

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
    fn cms_paths_match_axum_routes() {
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
            "app.rs must register POST /cms/announcements"
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
            "app.rs must still register POST /cms/uploads (orphan until multipart client)"
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
            prod.contains("Hero image upload unavailable"),
            "hero button must error honestly, not toast success"
        );
        assert!(
            !prod.contains("Hero image upload coming soon"),
            "old stub success toast must be gone"
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
