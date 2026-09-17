//! Places body rendering for the left dock.

use super::*;

macro_rules! places_body {
    ($nodes:ident, $selected:ident, $active_layer:ident, $collapsed:ident, $layer_query:ident, $layer_nodes:ident, $layers_filtered_empty:ident, $doc_hits:ident, $sel_facets:ident, $tab:ident, $query:ident, $places:ident, $places_armed:ident, $bookmarks:ident, $renaming:ident, $rename_draft:ident, $rename_abandon:ident, $adding:ident, $add_draft:ident, $commit_bookmarks:ident, $arm_places:ident, $tab_btn:ident, $fly_row:ident, $hits_body:ident, $facets_row:ident, $places_body:ident, $full:ident, $stub:ident) => {
move || {
        view! {
            <div class="mt-1 flex min-h-0 flex-1 flex-col gap-2">
                <input
                    type="text"
                    data-testid="dock-left-places-filter"
                    aria-label="Filter bookmarks and locations"
                    placeholder="Filter places…"
                    prop:value=move || $query.get()
                    class="w-full rounded border border-outline-variant/30 bg-black/20 px-1.5 py-1 text-label-sm text-on-surface placeholder:text-outline"
                    on:input=move |ev| $query.set(event_target_value(&ev))
                />
                <section class="flex min-h-0 flex-col">
                    <h3 class="px-1 text-label-sm font-semibold uppercase tracking-wide text-outline">
                        "Bookmarks"
                    </h3>
                    {move || {
                        if !$adding.get() {
                            return None;
                        }
                        let add_ref = NodeRef::<leptos::html::Input>::new();
                        add_ref.on_load(|el: web_sys::HtmlInputElement| {
                            let _ = el.focus();
                            el.select();
                        });
                        Some(view! {
                            <input
                                type="text"
                                node_ref=add_ref
                                data-testid="dock-left-bookmark-name"
                                aria-label="Bookmark name"
                                value=$add_draft.get_untracked()
                                class="mx-1 my-0.5 rounded border border-primary/60 bg-black/30 px-1.5 py-1 text-label-sm text-on-surface"
                                on:input=move |ev| $add_draft.set(event_target_value(&ev))
                                on:blur=move |_| $adding.set(false)
                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                    match ev.key().as_str() {
                                        "Enter" => {
                                            ev.prevent_default();
                                            let name = $add_draft.get_untracked();
                                            if let Some((x, y, zoom)) = live_camera() {
                                                let mut next = $bookmarks.get_untracked();
                                                if next.add(&name, x, y, zoom) {
                                                    $commit_bookmarks(next);
                                                }
                                            }
                                            $adding.set(false);
                                        }
                                        "Escape" => {
                                            ev.stop_propagation();
                                            $adding.set(false);
                                        }
                                        _ => {}
                                    }
                                }
                            />
                        })
                    }}
                    <div class="max-h-40 overflow-y-auto" data-testid="dock-left-bookmark-list">
                        {move || {
                            let q = $query.get();
                            let editing = $renaming.get();
                            let rows = $bookmarks.with(|b| filter_bookmarks(b, &q));
                            if rows.is_empty() {
                                return view! {
                                    <p class="px-1 py-1 text-label-sm text-outline">
                                        "No bookmarks yet — frame a view and use the bookmark button."
                                    </p>
                                }
                                    .into_any();
                            }
                            rows.into_iter()
                                .map(|bm| {
                                    let key = bm.name.clone();
                                    let is_editing = editing.as_ref() == Some(&key);
                                    if is_editing {
                                        let rename_ref = NodeRef::<leptos::html::Input>::new();
                                        rename_ref
                                            .on_load(|el: web_sys::HtmlInputElement| {
                                                let _ = el.focus();
                                                el.select();
                                            });
                                        return view! {
                                            <input
                                                type="text"
                                                node_ref=rename_ref
                                                data-testid="dock-left-bookmark-rename"
                                                aria-label="Rename bookmark"
                                                value=$rename_draft.get_untracked()
                                                class="mx-1 my-0.5 w-[calc(100%-0.5rem)] rounded border border-primary/60 bg-black/30 px-1.5 py-1 text-label-sm text-on-surface"
                                                on:input=move |ev| $rename_draft.set(event_target_value(&ev))
                                                on:blur=move |_| {
                                                    if $rename_abandon.get_untracked() {
                                                        $rename_abandon.set(false);
                                                        $renaming.set(None);
                                                        return;
                                                    }
                                                    if let Some(from) = $renaming.get_untracked() {
                                                        let to = $rename_draft.get_untracked();
                                                        let mut next = $bookmarks.get_untracked();
                                                        if next.rename(&from, &to) {
                                                            $commit_bookmarks(next);
                                                        }
                                                        $renaming.set(None);
                                                    }
                                                }
                                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                    match ev.key().as_str() {
                                                        "Enter" => {
                                                            ev.prevent_default();
                                                            if let Some(from) = $renaming.get_untracked()
                                                            {
                                                                let to = $rename_draft.get_untracked();
                                                                let mut next = $bookmarks.get_untracked();
                                                                if next.rename(&from, &to) {
                                                                    $commit_bookmarks(next);
                                                                }
                                                            }
                                                            $renaming.set(None);
                                                        }
                                                        "Escape" => {
                                                            ev.stop_propagation();
                                                            $rename_abandon.set(true);
                                                            $renaming.set(None);
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            />
                                        }
                                            .into_any();
                                    }
                                    let rename_key = key.clone();
                                    let remove_key = key.clone();
                                    let actions = view! {
                                        <span class="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                                            <button
                                                type="button"
                                                aria-label="Rename bookmark"
                                                title="Rename"
                                                class="flex size-5 cursor-pointer items-center justify-center rounded text-outline hover:bg-white/10 hover:text-on-surface"
                                                on:click=move |ev: web_sys::MouseEvent| {
                                                    ev.stop_propagation();
                                                    $rename_abandon.set(false);
                                                    $rename_draft.set(rename_key.clone());
                                                    $renaming.set(Some(rename_key.clone()));
                                                }
                                            >
                                                <MaterialIcon name="edit" class="block text-sm" />
                                            </button>
                                            <button
                                                type="button"
                                                aria-label="Remove bookmark"
                                                title="Remove"
                                                class="flex size-5 cursor-pointer items-center justify-center rounded text-outline hover:bg-white/10 hover:text-error"
                                                on:click=move |ev: web_sys::MouseEvent| {
                                                    ev.stop_propagation();
                                                    let mut next = $bookmarks.get_untracked();
                                                    next.remove(&remove_key);
                                                    $commit_bookmarks(next);
                                                }
                                            >
                                                <MaterialIcon name="close" class="block text-sm" />
                                            </button>
                                        </span>
                                    }
                                        .into_any();
                                    $fly_row(
                                            key,
                                            format!(
                                                "Fly to {} ({:.0} m, {:.0} m, zoom {:.1})",
                                                bm.name,
                                                bm.x,
                                                bm.y,
                                                bm.zoom,
                                            ),
                                            bm.x,
                                            bm.y,
                                            Some(bm.zoom),
                                            actions,
                                        )
                                        .into_any()
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>
                </section>
                <section class="flex min-h-0 flex-1 flex-col">
                    <h3 class="px-1 text-label-sm font-semibold uppercase tracking-wide text-outline">
                        "Locations"
                    </h3>
                    <div
                        class="min-h-0 flex-1 overflow-y-auto"
                        data-testid="dock-left-location-list"
                    >
                        {move || {
                            let q = $query.get();
                            let rows = $places.with(|p| filter_places(p, &q));
                            if rows.is_empty() {
                                return view! {
                                    <p class="px-1 py-1 text-label-sm text-outline">
                                        {move || {
                                            if $places.with(Vec::is_empty) {
                                                "No named locations for this terrain."
                                            } else {
                                                "No location matches that filter."
                                            }
                                        }}
                                    </p>
                                }
                                    .into_any();
                            }
                            rows.into_iter()
                                .map(|p| {
                                    let kind = p.kind.clone();
                                    let badge = view! {
                                        <span class="shrink-0 text-label-sm lowercase text-outline">{kind}</span>
                                    }
                                        .into_any();
                                    $fly_row(
                                            p.name.clone(),
                                            format!("Fly to {} ({:.0} m, {:.0} m)", p.name, p.x, p.y),
                                            p.x,
                                            p.y,
                                            None,
                                            badge,
                                        )
                                        .into_any()
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>
                </section>
            </div>
        }
    }
    };
}
/// Expose the places body view fragment.
pub(super) use places_body;
