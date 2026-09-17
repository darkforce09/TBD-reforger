//! Connections panel for editor overlays.
use super::*;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::editor_context;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::hosted_commands as engine_ops;

struct ConnRowView {
    kind: String,
    head: String,
    id: String,
    problems: Vec<String>,
}

/// Renders connection rows and controls for the selected entity.
#[component]
pub(crate) fn ConnectionsPanelOverlay(
    open: RwSignal<bool>,
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::v2::core::ui::modal_stack::register(move || {
            open.try_get_untracked().unwrap_or(false)
        });
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked()
                && ev.key() == "Escape"
                && crate::v2::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                ev.prevent_default();
                editor_context::close_connections_panel();
            }
        });
        on_cleanup(move || {
            key.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }

    move || {
        if !open.get() {
            return None;
        }
        let _ = doc_tick.get();
        #[cfg(target_arch = "wasm32")]
        let (rows, finding_count, armed_line) = {
            let list = engine_ops::connection_list();
            let findings = engine_ops::connection_findings();
            let mut by_row: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            for f in &findings {
                by_row
                    .entry(f.connection_id.clone())
                    .or_default()
                    .push(format!("{}: {}", f.code, f.detail));
            }
            let rows: Vec<ConnRowView> = list
                .into_iter()
                .map(|r| ConnRowView {
                    problems: by_row.get(&r.id).cloned().unwrap_or_default(),
                    head: format!("{} \u{2192} {}", r.from_label, r.to_label),
                    kind: r.kind,
                    id: r.id,
                })
                .collect();
            let armed_line = engine_ops::pending_connect()
                .map(|(kind, from)| format!("Connecting: {kind} from {from}"));
            (rows, findings.len(), armed_line)
        };
        #[cfg(not(target_arch = "wasm32"))]
        let (rows, finding_count, armed_line): (Vec<ConnRowView>, usize, Option<String>) =
            (Vec::new(), 0, None);

        let total = rows.len();
        let clean = finding_count == 0;
        let empty = total == 0;

        let row_views = rows
            .into_iter()
            .map(|r| {
                let bad = !r.problems.is_empty();
                let (problems, del_id, head) = (r.problems, r.id.clone(), r.head);
                view! {
                    <div class="flex flex-col gap-0.5 border-b border-outline-variant/20 py-1.5 last:border-b-0">
                        <div class="flex items-center gap-2">
                            <span
                                class="shrink-0 rounded px-1.5 py-0.5 font-code-sm text-code-sm"
                                class:bg-surface-dim=!bad
                                class:text-on-surface-variant=!bad
                                class:bg-error-container=bad
                                class:text-on-error-container=bad
                            >
                                {r.kind}
                            </span>
                            <span class="flex-1 truncate font-label-md text-label-md text-on-surface">
                                {head}
                            </span>
                            <span class="shrink-0 font-code-sm text-code-sm text-outline">
                                {r.id}
                            </span>
                            <button
                                type="button"
                                title="Delete this connection (CONN-DEL-001) — one Ctrl+Z restores it"
                                class="shrink-0 cursor-pointer rounded px-2 py-0.5 font-label-sm text-[11px] text-on-surface-variant hover:bg-error-container hover:text-on-error-container"
                                on:click=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        engine_ops::delete_connection(&del_id);
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let _ = &del_id;
                                }
                            >
                                "Delete"
                            </button>
                        </div>
                        {(!problems.is_empty())
                            .then(|| {
                                problems
                                    .into_iter()
                                    .map(|p| {
                                        view! {
                                            <div class="pl-2 font-code-sm text-code-sm text-error">
                                                {p}
                                            </div>
                                        }
                                    })
                                    .collect_view()
                            })}
                    </div>
                }
            })
            .collect_view();

        Some(view! {
            <div
                class="fixed inset-0 z-40 bg-scrim/40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    editor_context::close_connections_panel();
                }
            ></div>
            <div
                class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[80vh] w-[min(40rem,94vw)] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 rounded-xl border border-outline-variant/30 p-4 shadow-2xl outline-none"
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="flex items-center gap-2">
                    <span class="font-label-md text-label-md text-on-surface">"Connections"</span>
                    <span class="font-code-sm text-code-sm text-on-surface-variant">
                        {format!("{total} edge(s)")}
                    </span>
                    <button
                        type="button"
                        class="ml-auto cursor-pointer rounded px-2 py-1 font-label-sm text-[11px] text-on-surface-variant hover:bg-surface-dim"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            editor_context::close_connections_panel();
                        }
                    >
                        "Close"
                    </button>
                </div>
                {armed_line
                    .map(|line| {
                        view! {
                            <div class="flex items-center gap-2 rounded border border-primary/40 bg-surface-dim px-2 py-1">
                                <span class="flex-1 truncate font-code-sm text-code-sm text-on-surface">
                                    {line}
                                </span>
                                <button
                                    type="button"
                                    class="shrink-0 cursor-pointer rounded px-2 py-0.5 font-label-sm text-[11px] text-on-surface-variant hover:bg-surface-bright"
                                    on:click=move |_| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            engine_ops::cancel_connect();
                                            editor_context::open_connections_panel();
                                        }
                                    }
                                >
                                    "Cancel"
                                </button>
                            </div>
                        }
                    })}
                <div
                    class="rounded px-2 py-1 font-label-sm text-[11px]"
                    class:bg-surface-dim=clean
                    class:text-on-surface-variant=clean
                    class:bg-error-container=!clean
                    class:text-on-error-container=!clean
                >
                    {if clean {
                        "No problems found in the connection graph.".to_string()
                    } else {
                        format!(
                            "{finding_count} problem(s): dangling endpoints, self-links, duplicates or ownership cycles — see the rows below.",
                        )
                    }}
                </div>
                <div class="min-h-0 flex-1 overflow-y-auto">
                    {if empty {
                        view! {
                            <div class="py-6 text-center font-label-sm text-[11px] text-on-surface-variant">
                                "No connections yet. Right-click a unit → Connect → pick a relation, then left-click the target (or right-click it and choose Complete Connection)."
                            </div>
                        }
                            .into_any()
                    } else {
                        row_views.into_any()
                    }}
                </div>
            </div>
        })
    }
}
