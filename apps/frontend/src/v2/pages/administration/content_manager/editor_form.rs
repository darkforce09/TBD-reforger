//! The editor form: writing a post, and the four things that can be done with it.
//!
//! **Role:** the title, category, body and hero fields, the markdown toolbar, the publish switch,
//! and the save, publish, delete and re-push actions behind them.
//! **Position:** the detail pane of the content route, beside the post list.
//! **Signals & state:** every field is a signal seeded from the post being edited. Saving writes
//! the edited fields straight back into the page's working set, so the list agrees with the form
//! without waiting for a refetch. `publish_busy` and `delete_busy` keep a second click from sending
//! a second request.
//! **Invariants:** publishing a post that has never been saved creates it and **keeps the
//! identifier the server mints**, so publishing again updates that row instead of creating a
//! second one. The markdown toolbar writes real markers into the body; the change to the text is
//! the feedback, not a notification. Deleting removes the post from the working set and clears the
//! selection, so the form cannot stay open over a post that is gone.
#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use super::doc::{
    announcement_create_path, announcement_id_path, announcement_push_path, category_tag,
    is_server_id,
};
use super::doc::{Doc, CATEGORY_OPTIONS};
use super::hero_upload::pick_and_upload_hero;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// The markdown controls the toolbar offers, as an icon paired with its name.
pub(super) const MD_TOOLS: &[(&str, &str)] = &[
    ("format_bold", "Bold"),
    ("format_italic", "Italic"),
    ("link", "Link"),
    ("format_list_bulleted", "List"),
    ("image", "Image"),
];

/// The body with one markdown marker inserted for the named control.
///
/// The inserted text is the feedback, so nothing else is reported. A list item starts on its own
/// line; everything else is separated by a space unless the body already ends in a newline.
pub(super) fn apply_md_tool(body: &str, tool: &str) -> String {
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

/// Today's date, as the day part of an instant.
#[cfg(target_arch = "wasm32")]
pub(super) fn today_iso() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .map(|s| s[..10.min(s.len())].to_string())
        .unwrap_or_default()
}

/// The form for one post, with its four actions.
pub(super) fn editor(
    d: Doc,
    docs: RwSignal<Vec<Doc>>,
    selected_id: RwSignal<Option<String>>,
    publish_busy: RwSignal<bool>,
    delete_busy: RwSignal<bool>,
    store: crate::v2::core::auth::AuthStore,
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

    // Write the edited fields back into the working set, retargeting the row's identifier when the
    // server mints one on the first publish.
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
            crate::v2::core::ui::toast::use_toasts().success("Draft saved");
        }
    };

    let handle_publish = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
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
                    // An existing row is updated, never created again: posting twice would
                    // leave two of the same post.
                    match crate::v2::core::api::client::api_patch::<serde_json::Value>(
                        store,
                        &announcement_id_path(&id),
                        payload,
                    )
                    .await
                    {
                        Ok(_) => {
                            // An update only pushes to chat on a first publish, so an already
                            // published row is pushed again through its own route.
                            if push && already_published {
                                match crate::v2::core::api::client::api_post_ok(
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
                    match crate::v2::core::api::client::api_post::<serde_json::Value>(
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
                        toasts.error(crate::v2::core::api::client::api_error_message(
                            &e,
                            "Publish failed",
                        ));
                    }
                }
                publish_busy.set(false);
            });
        }
    };

    let handle_delete = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
            if delete_busy.get_untracked() {
                return;
            }
            let id = doc_id.get_value();
            if is_server_id(&id) {
                delete_busy.set(true);
                leptos::task::spawn_local(async move {
                    match crate::v2::core::api::client::api_delete(
                        store,
                        &announcement_id_path(&id),
                    )
                    .await
                    {
                        Ok(()) => {
                            docs.update(|list| list.retain(|d| d.id != id));
                            selected_id.set(None);
                            toasts.success("Announcement archived");
                        }
                        Err(e) => {
                            toasts.error(crate::v2::core::api::client::api_error_message(
                                &e,
                                "Delete failed",
                            ));
                        }
                    }
                    delete_busy.set(false);
                });
            } else {
                // A draft that has never been saved only exists here, so it is dropped from the
                // working set and no request is made.
                docs.update(|list| list.retain(|d| d.id != id));
                selected_id.set(None);
                toasts.success("Draft discarded");
            }
        }
    };

    let handle_hero = move |_| pick_and_upload_hero(store, doc_id, docs, thumbnail_url);

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

/// The publish switch: a two-state control the form's publish action reads.
pub(super) fn switch(checked: RwSignal<bool>) -> impl IntoView {
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
