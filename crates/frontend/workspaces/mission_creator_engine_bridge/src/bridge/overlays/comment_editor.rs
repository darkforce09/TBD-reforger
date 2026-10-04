//! Comment editor for editor overlays.
#[cfg(target_arch = "wasm32")]
use super::*;
#[cfg(target_arch = "wasm32")]
use crate::bridge::host_state::editor_context;
#[cfg(target_arch = "wasm32")]
use mission_editing_commands::hosted_commands as engine_ops;

/// Renders fields and actions for the selected map comment.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn CommentEditorOverlay(
    /// The key of the comment being edited; `None` closes the editor.
    open: RwSignal<Option<String>>,
    /// The document tick; a change re-reads the comment.
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    {
        let modal_id = frontend_ui::modal_stack::register(move || {
            open.try_get_untracked().flatten().is_some()
        });
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked().is_some()
                && ev.key() == "Escape"
                && frontend_ui::modal_stack::is_topmost_open(modal_id)
            {
                ev.prevent_default();
                editor_context::close_comment_editor();
            }
        });
        on_cleanup(move || {
            key.remove();
            frontend_ui::modal_stack::unregister(modal_id);
        });
    }

    move || {
        let id = open.get()?;
        let _ = doc_tick.get();
        let row = engine_ops::read_comment(id.as_str());
        let (title, tooltip, x, z) = match &row {
            Some(c) => (c.title.clone(), c.tooltip.clone(), c.x, c.z),
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
                            {
                                engine_ops::rename_comment(
                                    id_title.clone(),
                                    event_target_value(&ev),
                                );
                            }
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
                            {
                                engine_ops::set_comment_tooltip(
                                    id_tip.clone(),
                                    event_target_value(&ev),
                                );
                            }
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
                                if let Ok(v) = event_target_value(&ev).trim().parse::<f64>() {
                                    engine_ops::move_comment(id_x.clone(), v, z_for_x);
                                }
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
                                if let Ok(v) = event_target_value(&ev).trim().parse::<f64>() {
                                    engine_ops::move_comment(id_z.clone(), x_for_z, v);
                                }
                            }
                        />
                    </div>
                </div>
                <div class="flex items-center gap-2 pt-1">
                    <button
                        type="button"
                        class="rounded border border-border-subtle px-3 py-1.5 text-label-md text-on-surface hover:bg-primary/15"
                        on:click=move |_| {
                            if let Some(new_id) =
                                engine_ops::duplicate_comment(
                                    id_dup.as_str(),
                                    COMMENT_COPY_OFFSET_M,
                                    crate::bridge::host_state::active_folder::ensure_active_layer,
                                )
                            {
                                editor_context::open_comment_editor(new_id.into());
                            }
                        }
                    >
                        "Duplicate"
                    </button>
                    <button
                        type="button"
                        class="rounded border border-error/50 px-3 py-1.5 text-label-md text-error hover:bg-error/15"
                        on:click=move |_| {
                            {
                                engine_ops::delete_comment(id_del.clone());
                                editor_context::close_comment_editor();
                            }
                        }
                    >
                        "Delete"
                    </button>
                    <button
                        type="button"
                        class="ml-auto rounded bg-primary px-3 py-1.5 text-label-md text-on-primary"
                        on:click=move |_| {
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

#[cfg(target_arch = "wasm32")]
const COMMENT_COPY_OFFSET_M: f64 = 25.0;
