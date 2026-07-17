//! Comms Broadcaster / Content Manager (/admin/content) — ported from pages/admin.tsx
//! `ContentManagerPage` + `ContentEditor`. `<AdminGate>` → a transparent `SplitPane`: a post list
//! (master) + the editor form (detail).
//!
//! T-159.25: fully interactive. The docs list is LOCAL state seeded from the same MOCK_DOCS the
//! React page uses (a docs API doesn't exist yet — announcement publish is the one live mutation):
//! New post / select / Save Draft mutate the local list; **Publish & Broadcast** maps the category
//! onto the announcement `tag` and POSTs `/cms/announcements` (usePublishAnnouncement port; SOP has
//! no tag → local-only publish, matching React). The Push-to-Discord Switch is live.
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

/// Map a doc category onto the announcement `tag` enum (SOP has no equivalent).
fn category_tag(category: &str) -> Option<&'static str> {
    match category {
        "announcement" => Some("update"),
        "event" => Some("event"),
        "modpack" => Some("modpack_update"),
        "important" => Some("important"),
        _ => None,
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
                                        editor(d, docs, publish_busy, store).into_any()
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
    publish_busy: RwSignal<bool>,
    store: crate::auth::AuthStore,
) -> impl IntoView {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, publish_busy, docs);
    let doc_id = StoredValue::new(d.id.clone());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = doc_id;
    let title = RwSignal::new(d.title.clone());
    let body = RwSignal::new(d.body.clone());
    let category = RwSignal::new(d.category.clone());
    let push_discord = RwSignal::new(true);

    // `current(status)` + `onChange(saveDoc)` — write the edited fields back into the local list.
    #[cfg(target_arch = "wasm32")]
    let apply = move |published: bool| {
        let t = title.get_untracked().trim().to_string();
        docs.update(|list| {
            if let Some(doc) = list.iter_mut().find(|x| x.id == doc_id.get_value()) {
                doc.title = if t.is_empty() { "Untitled Post".into() } else { t.clone() };
                doc.body = body.get_untracked();
                doc.category = category.get_untracked();
                doc.published = published;
                doc.date = today_iso();
            }
        });
    };

    let save_draft = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            apply(false);
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
                // SOPs have no announcement equivalent — publish locally only.
                apply(true);
                toasts.success("SOP published");
                return;
            };
            if publish_busy.get_untracked() {
                return;
            }
            publish_busy.set(true);
            let push = push_discord.get_untracked();
            let payload = serde_json::json!({
                "title": t,
                "body": b,
                "tag": tag,
                "is_pinned": false,
                "push_to_discord": push,
                "status": "published",
            });
            leptos::task::spawn_local(async move {
                match crate::client::api_post::<serde_json::Value>(
                    store,
                    "/cms/announcements",
                    payload,
                )
                .await
                {
                    Ok(_) => {
                        apply(true);
                        toasts.success(if push {
                            "Published & broadcast to Discord"
                        } else {
                            "Published"
                        });
                    }
                    Err(_) => toasts.error("Publish failed"),
                }
                publish_busy.set(false);
            });
        }
    };
    let stub = move |msg: &'static str| {
        move |_| {
            #[cfg(target_arch = "wasm32")]
            crate::toast::use_toasts().success(msg);
            #[cfg(not(target_arch = "wasm32"))]
            let _ = msg;
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
                        on:click=stub("Hero image upload coming soon")
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
                        let msg = StoredValue::new(format!("{label} (mock)"));
                        view! {
                            <button
                                type="button"
                                on:click=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    crate::toast::use_toasts().success(msg.get_value());
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let _ = msg;
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
                        on:click=save_draft
                        class="rounded-full border border-white/10 px-6 py-3 text-label-md text-on-surface transition hover:bg-white/5"
                    >
                        "Save Draft"
                    </button>
                    <button
                        type="button"
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
