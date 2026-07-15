//! Comms Broadcaster / Content Manager (/admin/content) — ported from pages/admin.tsx
//! `ContentManagerPage`. `<AdminGate>` → a transparent `SplitPane`: a post list (master) + a
//! `ContentEditor` form (detail). Fully client-MOCK-driven (`MOCK_DOCS`).
//!
//! **Gate scope (this slice):** the default render (first doc selected) — the post list + the editor
//! form (title, category select, Markdown toolbar, body textarea, publish footer with the Switch)
//! is byte-exact-verified. Editing + the mutations are behavior — a follow-up.
#![allow(dead_code)]
use crate::split_pane::{ListDetailItem, SplitPane};
use crate::ui::MaterialIcon;
use leptos::prelude::*;

struct Doc {
    id: &'static str,
    title: &'static str,
    category: &'static str,
    published: bool,
    date: &'static str,
    body: &'static str,
}

const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
const BADGE_WARNING: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow";

const DOCS: &[Doc] = &[
    Doc {
        id: "d1",
        title: "Operation Blue Storm Briefing",
        category: "announcement",
        published: true,
        date: "2026-06-18",
        body: "All units, Operation Blue Storm kicks off Saturday at 1900Z. BLUFOR will stage at the southern airfield...\n\nReview your ORBAT assignments and ensure your modpack is current.",
    },
    Doc {
        id: "d2",
        title: "SOP: Armor Tactics",
        category: "sop",
        published: true,
        date: "2026-06-12",
        body: "# Armor Doctrine\n\nNever advance armor without infantry support. Maintain hull-down positions where possible and...",
    },
    Doc {
        id: "d3",
        title: "Modpack v2.4.1 Changelog",
        category: "modpack",
        published: false,
        date: "2026-06-20",
        body: "Draft notes for the upcoming modpack bump:\n- Added RHS Status Quo\n- Removed deprecated optics pack",
    },
];

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

#[component]
pub fn ContentManagerPage() -> impl IntoView {
    let selected = &DOCS[0];
    view! {
        <crate::ui::AdminGate>
            <div class="relative h-full w-full overflow-hidden">
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                    <SplitPane
                        transparent=true
                        master_width="20rem"
                        master_header=master_header().into_any()
                        master=doc_list(selected.id).into_any()
                        detail=editor(selected).into_any()
                    />
                </div>
            </div>
        </crate::ui::AdminGate>
    }
}

fn master_header() -> impl IntoView {
    view! {
        <>
            <h1 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                "Comms Broadcaster"
            </h1>
            <button
                type="button"
                class="flex shrink-0 items-center gap-1.5 rounded-full border border-white/10 px-3 py-1.5 text-label-sm text-on-surface transition hover:bg-white/5"
            >
                <MaterialIcon name="add" class="text-[18px]" />
                "New"
            </button>
        </>
    }
    .into_any()
}

fn doc_list(selected_id: &'static str) -> impl IntoView {
    DOCS.iter()
        .map(move |d| {
            let (badge, label) = if d.published {
                (BADGE_SUCCESS, "Published")
            } else {
                (BADGE_WARNING, "Draft")
            };
            view! {
                <ListDetailItem
                    active=d.id == selected_id
                    meta=view! { {d.date} }.into_any()
                    title=view! { {d.title} }.into_any()
                    trailing=view! { <span class=badge>{label}</span> }.into_any()
                />
            }
        })
        .collect_view()
}

fn editor(d: &'static Doc) -> impl IntoView {
    view! {
        <div class="relative flex h-full min-w-0 flex-1 flex-col">
            <div class="flex items-start justify-between gap-4 p-8 pb-4">
                <input
                    type="text"
                    value=d.title
                    placeholder="Post Title"
                    class="min-w-0 flex-1 bg-transparent text-4xl font-bold text-on-surface outline-none placeholder:text-outline"
                />
                <div class="flex shrink-0 items-center gap-2">
                    <select class="rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface outline-none focus:border-primary/50">
                        {CATEGORY_OPTIONS
                            .iter()
                            .map(|(value, label)| {
                                view! { <option value=*value>{*label}</option> }
                            })
                            .collect_view()}
                    </select>
                    <button
                        type="button"
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
                        view! {
                            <button
                                type="button"
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
                placeholder="Start writing… Markdown supported."
                class="w-full flex-1 resize-none bg-transparent p-8 text-lg leading-relaxed text-on-surface outline-none placeholder:text-outline"
            >
                {d.body}
            </textarea>
            <div class="flex items-center justify-between gap-4 border-t border-white/10 bg-white/5 p-6 backdrop-blur-md">
                <label class="flex items-center gap-3" id="sw-label">
                    {switch(true)} <span class="text-label-md text-on-surface-variant">"Push to Discord"</span>
                </label>
                <div class="flex items-center gap-3">
                    <button
                        type="button"
                        class="rounded-full border border-white/10 px-6 py-3 text-label-md text-on-surface transition hover:bg-white/5"
                    >
                        "Save Draft"
                    </button>
                    <button
                        type="button"
                        class="rounded-full bg-action px-7 py-3 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
                    >
                        "Publish & Broadcast"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Base-UI Switch (checked) — reproduced from the oracle DOM: a `<span role="switch">` root (labelled
/// by the wrapping `<label id="sw-label">`) + a visually-hidden `<input type="checkbox" checked>`.
/// The ids (sw-label/sw-root/sw-input) are normalized positionally by dom.js, so concrete values in
/// document order match React's useId output. First base-ui primitive; reused by other toggles later.
fn switch(_checked: bool) -> impl IntoView {
    let root = "group relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-outline-variant/60 bg-surface-container-high p-0.5 outline-none transition-colors focus-visible:ring-2 focus-visible:ring-primary/50 data-[checked]:border-primary data-[checked]:bg-primary data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50";
    let thumb = "h-3.5 w-3.5 rounded-full bg-on-surface-variant shadow-sm transition-all data-[checked]:translate-x-4 data-[checked]:bg-on-primary";
    view! {
        <span
            id="sw-root"
            role="switch"
            aria-checked="true"
            aria-labelledby="sw-label"
            tabindex="0"
            data-checked=""
            class=root
        >
            <span class=thumb data-checked=""></span>
        </span>
        <input
            id="sw-input"
            type="checkbox"
            checked=true
            aria-hidden="true"
            tabindex="-1"
            style="clip-path: inset(50%); overflow: hidden; white-space: nowrap; border: 0px; padding: 0px; width: 1px; height: 1px; margin: -1px; position: fixed; top: 0px; left: 0px;"
        />
    }
}
