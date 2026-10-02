//! Menu overlay.

use super::*;

/// Renders the menu overlay and handles keyboard dismissal.
#[component]
pub fn ContextMenuOverlay(menu: RwSignal<Option<MenuState>>) -> impl IntoView {
    let highlight = RwSignal::new(None::<usize>);
    let panel_ref = NodeRef::<leptos::html::Div>::new();
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::v2::core::ui::modal_stack::register(move || {
            menu.try_get_untracked().flatten().is_some()
        });
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            let Some(state) = menu.get_untracked() else {
                return;
            };
            if !crate::v2::core::ui::modal_stack::is_topmost_open(modal_id) {
                return;
            }
            match ev.key().as_str() {
                "Escape" => {
                    ev.prevent_default();
                    close();
                }
                "ArrowDown" => {
                    ev.prevent_default();
                    let entries = state.entries();
                    highlight.set(step_highlight(&entries, highlight.get_untracked(), 1));
                    if let Some(panel) = panel_ref.get_untracked() {
                        reveal_menu_row(&panel, highlight.get_untracked());
                    }
                }
                "ArrowUp" => {
                    ev.prevent_default();
                    let entries = state.entries();
                    highlight.set(step_highlight(&entries, highlight.get_untracked(), -1));
                    if let Some(panel) = panel_ref.get_untracked() {
                        reveal_menu_row(&panel, highlight.get_untracked());
                    }
                }
                "Enter" => {
                    ev.prevent_default();
                    let entries = state.entries();
                    if let Some(idx) = highlight.get_untracked() {
                        if let Some(entry) = entries.get(idx) {
                            if let Some(item) = entry.item {
                                if entry.enabled {
                                    if item.is_submenu_parent() {
                                        toggle_submenu(item);
                                    } else {
                                        dispatch(
                                            item,
                                            &state.target.target_ids,
                                            state.target.world,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        });
        let update_layout = move || {
            if let (Some(panel), Some(state)) = (
                panel_ref.try_get_untracked().flatten(),
                menu.try_get_untracked().flatten(),
            ) {
                if panel.is_connected() {
                    place_context_menu(&panel, &state);
                    reveal_menu_row(&panel, highlight.try_get_untracked().flatten());
                }
            }
        };
        Effect::new(move |_| {
            let _ = (menu.get(), panel_ref.get());
            update_layout();
        });
        let resize = window_event_listener(leptos::ev::resize, move |_| update_layout());
        on_cleanup(move || {
            key.remove();
            resize.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }
    Effect::new(move |_| {
        highlight.set(menu.get().and_then(|state| {
            let parent = state.open_submenu?;
            state.entries().iter().position(|e| e.item == Some(parent))
        }));
    });

    move || {
        let state = menu.get()?;
        let entries = state.entries();
        let target_ids = state.target.target_ids.clone();
        let world = state.target.world;
        let pos = format!("{MENU_BOUNDS}left:8px;top:8px;visibility:hidden");
        let open_submenu = state.open_submenu;
        let rows = entries
            .into_iter()
            .enumerate()
            .map(|(idx, e)| render_row(idx, e, target_ids.clone(), world, highlight, open_submenu))
            .collect_view();
        Some(view! {
            <div
                class="fixed inset-0 z-40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    close();
                }
                on:contextmenu=move |ev| ev.prevent_default()
            ></div>
            <div
                node_ref=panel_ref
                class="glass animate-dialog-in fixed z-50 overflow-y-auto overscroll-contain rounded-md border border-outline-variant/30 py-1 shadow-2xl outline-none"
                style=pos
                on:contextmenu=move |ev| ev.prevent_default()
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                {rows}
            </div>
        })
    }
}

fn render_row(
    idx: usize,
    entry: MenuEntry,
    target_ids: Vec<String>,
    world: Option<(f64, f64)>,
    highlight: RwSignal<Option<usize>>,
    open_submenu: Option<ContextItem>,
) -> AnyView {
    let Some(item) = entry.item else {
        return view! {
            <div class="my-1 h-px bg-outline-variant/25" aria-hidden="true"></div>
        }
        .into_any();
    };

    let label = entry.label;
    let shortcut = entry.shortcut;
    let submenu = entry.submenu;
    let enabled = entry.enabled;
    let child = entry.child;
    let note = entry.note.clone();
    let expanded = submenu && open_submenu == Some(item);
    let title = if enabled {
        String::new()
    } else if let Some(t) = entry.blocked {
        format!("Not available yet — {t}")
    } else {
        let reason = item.why().unwrap_or_default();
        debug_assert!(
            !reason.is_empty(),
            "F-37: disabled row {item:?} has neither a blocking ticket nor a `why()` reason — it \
             would render with no tooltip"
        );
        reason.to_string()
    };

    let is_hi = move || highlight.get() == Some(idx);
    let base = "flex w-full items-center gap-3 px-3 py-1 text-left text-label-md select-none";

    let on_click = {
        let target_ids = target_ids.clone();
        move |_ev: leptos::ev::MouseEvent| {
            if enabled {
                #[cfg(target_arch = "wasm32")]
                if item.is_submenu_parent() {
                    toggle_submenu(item);
                } else {
                    dispatch(item, &target_ids, world);
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (item, &target_ids, world);
            }
        }
    };

    view! {
        <button
            type="button"
            data-context-row=idx
            disabled=!enabled
            title=title
            class=base
            class:cursor-pointer=enabled
            class:text-on-surface=enabled
            class:text-on-surface-variant=move || !enabled
            class:opacity-40=!enabled
            class:bg-tactical-yellow=move || enabled && is_hi()
            class:text-background=move || enabled && is_hi()
            on:pointerenter=move |_| {
                if enabled {
                    highlight.set(Some(idx));
                }
            }
            class:pl-8=child
            on:click=on_click
        >
            <span class="flex-1 truncate">{label}</span>
            {note
                .map(|n| {
                    view! {
                        <span class="ml-2 shrink-0 truncate text-code-sm text-outline">{n}</span>
                    }
                })}
            {(!shortcut.is_empty())
                .then(|| {
                    view! {
                        <span class="ml-4 shrink-0 font-mono text-code-sm text-outline">
                            {shortcut}
                        </span>
                    }
                })}
            {submenu
                .then(|| {
                    view! {
                        <span class="shrink-0 text-outline">
                            {if expanded { "\u{25BC}" } else { "\u{25B6}" }}
                        </span>
                    }
                })}
        </button>
    }
    .into_any()
}
