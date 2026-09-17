//! Right dock compositions behavior.

use super::*;

/// the Compositions panel. `editing` holds the composition id currently in inline-edit
/// (rename/recategorize), or `None`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn compositions_panel(
    doc_tick: RwSignal<u64>,
    editing: RwSignal<Option<String>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};

    let save_open = RwSignal::new(false);
    let save_title = RwSignal::new(String::new());
    let save_category = RwSignal::new(String::new());
    let input_class = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60";

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Compositions"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    engine_ops::composition_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Reusable multi-entity stamps. Select entities and Save; click a saved row to arm, then click the map to place."
        </p>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "A save takes the whole selection: placed entities keep the elevation they were saved at, and any comment in the selection is captured and stays editor-only — comments never reach the compiled mission."
        </p>

        {move || {
            let _ = doc_tick.get();
            let Some(armed) = armed_placement::armed_composition_id() else {
                return ().into_any();
            };
            let label = engine_ops::composition_rows()
                .into_iter()
                .find(|r| r.id == armed)
                .map(|r| {
                    if r.title.trim().is_empty() {
                        "Untitled".to_string()
                    } else {
                        r.title
                    }
                })
                .unwrap_or(armed);
            view! {
                <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                    <p class="text-label-sm normal-case text-on-surface">
                        {format!("Placing \u{201c}{label}\u{201d}")}
                    </p>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">
                        "Click the map to stamp it at the cursor. Esc or right-click to cancel."
                    </p>
                </div>
            }
                .into_any()
        }}

        {move || {
            let _ = doc_tick.get();
            if website_map_engine::editing::host::selection_len() == 0 {
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "Select one or more placed entities to save a composition."
                    </p>
                }
                    .into_any();
            }
            if save_open.get() {
                view! {
                    <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                        <label class="block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                            "Title"
                        </label>
                        <input
                            type="text"
                            aria-label="Composition title"
                            placeholder="Fireteam + Technical"
                            class=input_class
                            prop:value=move || save_title.get()
                            on:input=move |ev| save_title.set(event_target_value(&ev))
                        />
                        <label class="mt-2 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                            "Category"
                        </label>
                        <input
                            type="text"
                            aria-label="Composition category"
                            placeholder="Infantry"
                            class=input_class
                            prop:value=move || save_category.get()
                            on:input=move |ev| save_category.set(event_target_value(&ev))
                        />
                        <div class="mt-2 flex gap-1.5">
                            <button
                                type="button"
                                class="flex-1 rounded-md bg-primary/25 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-primary/40"
                                on:click=move |_| {
                                    let title = save_title.get_untracked();
                                    let title = if title.trim().is_empty() {
                                        "Untitled".to_string()
                                    } else {
                                        title
                                    };
                                    let category = save_category.get_untracked();
                                    let category = if category.trim().is_empty() {
                                        "Uncategorized".to_string()
                                    } else {
                                        category
                                    };
                                    let author = use_context::<crate::v2::core::auth::AuthStore>()
                                        .and_then(|s| s.user.get_untracked().map(|u| u.username))
                                        .filter(|u| !u.is_empty())
                                        .unwrap_or_else(|| "You".to_string());
                                    let _ = engine_ops::save_composition(title, category, author);
                                    save_open.set(false);
                                    save_title.set(String::new());
                                    save_category.set(String::new());
                                    doc_tick.update(|n| *n = n.wrapping_add(1));
                                }
                            >
                                "Save"
                            </button>
                            <button
                                type="button"
                                class="rounded-md px-2 py-1.5 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10"
                                on:click=move |_| save_open.set(false)
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                }
                    .into_any()
            } else {
                view! {
                    <button
                        type="button"
                        class="mt-3 w-full rounded-md border border-primary/40 px-2 py-1.5 text-label-sm text-primary transition-colors hover:bg-primary/15"
                        on:click=move |_| save_open.set(true)
                    >
                        {move || format!("Save composition… ({} selected)", website_map_engine::editing::host::selection_len())}
                    </button>
                }
                    .into_any()
            }
        }}

        {move || {
            let _ = doc_tick.get();
            let rows = engine_ops::composition_rows();
            if rows.is_empty() {
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "No saved compositions yet."
                    </p>
                }
                    .into_any();
            }
            let mut groups: Vec<(String, Vec<engine_ops::CompositionRow>)> = Vec::new();
            for r in rows {
                match groups.last_mut() {
                    Some((cat, list)) if *cat == r.category => list.push(r),
                    _ => groups.push((r.category.clone(), vec![r])),
                }
            }
            view! {
                <div class="mt-3 flex flex-col gap-2" role="list" aria-label="Saved compositions">
                    {groups
                        .into_iter()
                        .map(|(category, list)| {
                            let heading = if category.is_empty() {
                                "Uncategorized".to_string()
                            } else {
                                category
                            };
                            view! {
                                <div>
                                    <h4 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                                        {heading}
                                    </h4>
                                    <ul class="mt-1 flex flex-col gap-0.5" role="list">
                                        {list
                                            .into_iter()
                                            .map(|c| composition_row_view(c, doc_tick, editing, ROW, ROW_ACTIVE))
                                            .collect_view()}
                                    </ul>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            }
                .into_any()
        }}
    }
    .into_any()
}

/// one saved-composition row: press to ARM the place, hover actions to inline-edit / delete.
/// When `editing == this id`, the row swaps to inline title + category inputs (the idiom).
#[cfg(target_arch = "wasm32")]
fn composition_row_view(
    c: engine_ops::CompositionRow,
    doc_tick: RwSignal<u64>,
    editing: RwSignal<Option<String>>,
    row: &'static str,
    row_active: &'static str,
) -> AnyView {
    use crate::v2::apps::editor::bridge::host_state::editor_context;

    let _ = row_active;
    let id = c.id.clone();
    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let is_editing = {
        let id = id.clone();
        move || editing.get().as_deref() == Some(id.as_str())
    };

    let edit_title = RwSignal::new(c.title.clone());
    let edit_category = RwSignal::new(c.category.clone());
    let edit_author = RwSignal::new(c.author.clone());
    let input_class = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface outline-none focus:border-primary/60";

    let title = c.title.clone();
    let title = if title.trim().is_empty() {
        "Untitled".to_string()
    } else {
        title
    };
    let author = c.author.clone();
    let count = c.entity_count;

    let arm_id = id.clone();
    let edit_open_id = id.clone();
    let (del_id, save_id) = (id.clone(), id.clone());

    view! {
        <li>
            {move || {
                if is_editing() {
                    let (save_id, edit_title, edit_category, edit_author) = (
                        save_id.clone(),
                        edit_title,
                        edit_category,
                        edit_author,
                    );
                    view! {
                        <div class="rounded-md border border-primary/40 bg-primary/10 p-2">
                            <input
                                type="text"
                                aria-label="Composition title"
                                class=input_class
                                prop:value=move || edit_title.get()
                                on:input=move |ev| edit_title.set(event_target_value(&ev))
                            />
                            <input
                                type="text"
                                aria-label="Composition category"
                                class=format!("{input_class} mt-1")
                                prop:value=move || edit_category.get()
                                on:input=move |ev| edit_category.set(event_target_value(&ev))
                            />
                            <input
                                type="text"
                                aria-label="Composition author"
                                class=format!("{input_class} mt-1")
                                prop:value=move || edit_author.get()
                                on:input=move |ev| edit_author.set(event_target_value(&ev))
                            />
                            <div class="mt-1.5 flex gap-1.5">
                                <button
                                    type="button"
                                    class="flex-1 rounded-md bg-primary/25 px-2 py-1 text-label-sm text-on-surface transition-colors hover:bg-primary/40"
                                    on:click=move |_| {
                                        let t = edit_title.get_untracked();
                                        let t = if t.trim().is_empty() { "Untitled".to_string() } else { t };
                                        engine_ops::rename_composition(save_id.clone(), t);
                                        engine_ops::recategorize_composition(
                                            save_id.clone(),
                                            edit_category.get_untracked(),
                                        );
                                        engine_ops::set_composition_author(
                                            save_id.clone(),
                                            edit_author.get_untracked(),
                                        );
                                        editing.set(None);
                                        bump();
                                    }
                                >
                                    "Save"
                                </button>
                                <button
                                    type="button"
                                    class="rounded-md px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10"
                                    on:click=move |_| editing.set(None)
                                >
                                    "Cancel"
                                </button>
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    let (arm_id, edit_open_id, del_id) =
                        (arm_id.clone(), edit_open_id.clone(), del_id.clone());
                    let title = title.clone();
                    let author = author.clone();
                    view! {
                        <div class="group relative flex items-center gap-1">
                            <button
                                type="button"
                                title="Click to arm, then click the map to place"
                                class=format!("{row} flex-1")
                                on:pointerdown=move |_| {
                                    armed_placement::begin_place_composition(arm_id.clone());
                                }
                            >
                                <MaterialIcon name="dashboard_customize" class="block text-sm" />
                                <span class="flex min-w-0 flex-col">
                                    <span class="truncate">{title}</span>
                                    <span class="truncate text-[10px] text-outline">
                                        {format!("by {author} · {count} item{}", if count == 1 { "" } else { "s" })}
                                    </span>
                                </span>
                            </button>
                            <button
                                type="button"
                                aria-label="Edit composition"
                                title="Rename / recategorize"
                                class="shrink-0 rounded-md p-1 text-on-surface-variant opacity-0 transition-opacity hover:bg-white/10 group-hover:opacity-100"
                                on:click=move |_| {
                                    editing.set(Some(edit_open_id.clone()));
                                }
                            >
                                <MaterialIcon name="edit" class="block text-sm" />
                            </button>
                            <button
                                type="button"
                                aria-label="Delete composition"
                                title="Delete"
                                class="shrink-0 rounded-md p-1 text-error opacity-0 transition-opacity hover:bg-error/15 group-hover:opacity-100"
                                on:click=move |_| {
                                    if engine_ops::delete_composition(&del_id) {
                                        editor_context::cancel_armed_composition(&del_id);
                                    }
                                    bump();
                                }
                            >
                                <MaterialIcon name="delete" class="block text-sm" />
                            </button>
                        </div>
                    }
                        .into_any()
                }
            }}
        </li>
    }
    .into_any()
}

/// Native shell: no document, so no compositions. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn compositions_panel(
    doc_tick: RwSignal<u64>,
    editing: RwSignal<Option<String>>,
) -> AnyView {
    let _ = (doc_tick, editing);
    ().into_any()
}
