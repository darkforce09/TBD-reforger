//! Comment editor for editor overlays.
use super::*;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::editor_context;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::outliner::outliner;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::hosted_commands as engine_ops;

/// Renders fields and actions for the selected map comment.
#[component]
pub(crate) fn CommentEditorOverlay(
    open: RwSignal<Option<String>>,
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::v2::core::ui::modal_stack::register(move || {
            open.try_get_untracked().flatten().is_some()
        });
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked().is_some()
                && ev.key() == "Escape"
                && crate::v2::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                ev.prevent_default();
                editor_context::close_comment_editor();
            }
        });
        on_cleanup(move || {
            key.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }

    move || {
        let id = open.get()?;
        let _ = doc_tick.get();
        #[cfg(target_arch = "wasm32")]
        let row = engine_ops::read_comment(&id);
        #[cfg(not(target_arch = "wasm32"))]
        let row: Option<()> = None;
        let (title, tooltip, x, z) = match &row {
            #[cfg(target_arch = "wasm32")]
            Some(c) => (c.title.clone(), c.tooltip.clone(), c.x, c.z),
            #[cfg(not(target_arch = "wasm32"))]
            Some(()) => (String::new(), String::new(), 0.0, 0.0),
            None => return None,
        };
        let (id_title, id_tip, id_x, id_z, id_dup, id_del) = (
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
        );
        let (x_for_z, z_for_x) = (x, z);
        Some(view! {
            <div
                class="fixed inset-0 z-40 bg-scrim/40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    editor_context::close_comment_editor();
                }
            ></div>
            <div
                class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex w-[min(28rem,92vw)] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 rounded-xl border border-outline-variant/30 p-4 shadow-2xl outline-none"
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="flex items-center gap-2">
                    <span class="font-label-md text-label-md text-on-surface">"Comment"</span>
                    <span class="ml-auto font-code-sm text-code-sm text-on-surface-variant">
                        {id.clone()}
                    </span>
                </div>
                <div class="space-y-1">
                    <label class="font-label-sm text-[11px] text-on-surface-variant">"Title"</label>
                    <input
                        type="text"
                        class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-label-md text-label-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                        prop:value=title
                        on:change=move |ev| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                engine_ops::rename_comment(
                                    id_title.clone(),
                                    event_target_value(&ev),
                                );
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = (&id_title, &ev);
                        }
                    />
                </div>
                <div class="space-y-1">
                    <label class="font-label-sm text-[11px] text-on-surface-variant">"Tooltip"</label>
                    <textarea
                        rows="5"
                        class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-label-md text-label-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                        prop:value=tooltip
                        on:change=move |ev| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                engine_ops::set_comment_tooltip(
                                    id_tip.clone(),
                                    event_target_value(&ev),
                                );
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = (&id_tip, &ev);
                        }
                    ></textarea>
                </div>
                <div class="flex gap-2">
                    <div class="flex-1 space-y-1">
                        <label class="font-label-sm text-[11px] text-on-surface-variant">"X (m)"</label>
                        <input
                            type="number"
                            class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-code-md text-code-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                            prop:value=x
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                if let Ok(v) = event_target_value(&ev).trim().parse::<f64>() {
                                    engine_ops::move_comment(id_x.clone(), v, z_for_x);
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = (&id_x, &ev, z_for_x);
                            }
                        />
                    </div>
                    <div class="flex-1 space-y-1">
                        <label class="font-label-sm text-[11px] text-on-surface-variant">"Z (m)"</label>
                        <input
                            type="number"
                            class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-code-md text-code-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                            prop:value=z
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                if let Ok(v) = event_target_value(&ev).trim().parse::<f64>() {
                                    engine_ops::move_comment(id_z.clone(), x_for_z, v);
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = (&id_z, &ev, x_for_z);
                            }
                        />
                    </div>
                </div>
                <div class="flex items-center gap-2 pt-1">
                    <button
                        type="button"
                        class="rounded border border-border-subtle px-3 py-1.5 text-label-md text-on-surface hover:bg-primary/15"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            if let Some(new_id) =
                                engine_ops::duplicate_comment(
                                    &id_dup,
                                    COMMENT_COPY_OFFSET_M,
                                    outliner::ensure_active_layer,
                                )
                            {
                                editor_context::open_comment_editor(new_id);
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_dup;
                        }
                    >
                        "Duplicate"
                    </button>
                    <button
                        type="button"
                        class="rounded border border-error/50 px-3 py-1.5 text-label-md text-error hover:bg-error/15"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                engine_ops::delete_comment(id_del.clone());
                                editor_context::close_comment_editor();
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_del;
                        }
                    >
                        "Delete"
                    </button>
                    <button
                        type="button"
                        class="ml-auto rounded bg-primary px-3 py-1.5 text-label-md text-on-primary"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            editor_context::close_comment_editor();
                        }
                    >
                        "Close"
                    </button>
                </div>
            </div>
        })
    }
}

const COMMENT_COPY_OFFSET_M: f64 = 25.0;
