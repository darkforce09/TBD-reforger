//! Overlays rendering for the top command strip.

use super::*;

macro_rules! overlays {
    ($title:ident, $can_undo:ident, $can_redo:ident, $save_semver:ident, $save_status:ident, $dirty:ident, $settings_open:ident, $doc_tick:ident, $obj_count:ident, $orbat_open:ident, $open_menu:ident, $export_open:ident, $save_open:ident, $save_notes:ident, $validation_open:ident, $hint_open:ident, $set_hint:ident, $close_transients:ident, $transient_closer_id:ident, $row_mirror:ident, $toasts:ident, $save_findings:ident, $last_flush:ident, $recency_tick:ident, $save_was_open:ident, $env:ident, $census:ident, $summary:ident, $validation_findings:ident, $export_gesture_ok:ident, $run_action:ident, $widget_is:ident, $snap_on:ident, $title_fallback:ident) => {
        view! {
            <crate::v2::apps::editor::ui::modals::help_modal::ControlsHint open=$hint_open />
            {move || {
                ($open_menu.get().is_some() || $export_open.get() || $validation_open.get())
                    .then(|| {
                        view! {
                            <div
                                class="fixed inset-0 z-40"
                                on:click=move |_| {
                                    $open_menu.set(None);
                                    $export_open.set(false);
                                    $validation_open.set(false);
                                }
                            ></div>
                        }
                    })
            }}
            {move || {
                $save_open
                    .get()
                    .then(|| {
                        let estimate = {
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::v2::apps::editor::bridge::host_state::editor_context::slots_json()
                                    .as_deref()
                                    .and_then(crate::v2::apps::editor::shell::mission_size::estimate_compiled_bytes)
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                None::<usize>
                            }
                        };
                        let obj = $obj_count.map_or(0, |o| o.get());
                        let size_line = StoredValue::new(match estimate {
                            Some(b) => {
                                format!(
                                    "~{} · {} objects",
                                    crate::v2::apps::editor::shell::mission_size::format_bytes(b),
                                    obj,
                                )
                            }
                            None => format!("{obj} objects"),
                        });
                        let big = estimate.is_some_and(|b| b > 200_000_000);
                        let version_ref = NodeRef::<leptos::html::Input>::new();
                        version_ref
                            .on_load(|el: web_sys::HtmlInputElement| {
                                let _ = el.focus();
                                el.select();
                            });
                        let dialog_ref = NodeRef::<leptos::html::Div>::new();
                        let trap_tab = move |ev: web_sys::KeyboardEvent| {
                            trap_tab_in_dialog(dialog_ref, &ev);
                        };
                        view! {
                            <Portal>
                            <div
                                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
                                on:click=move |_| $save_open.set(false)
                            ></div>
                            <div
                                node_ref=dialog_ref
                                on:keydown=trap_tab
                                class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
                                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                                    <div class="min-w-0">
                                        <h2 class="text-headline-sm text-on-surface">"Save Version"</h2>
                                        <p class="mt-1 text-label-md text-on-surface-variant">
                                            "Versions are immutable — pick a new semver."
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        aria-label="Close"
                                        on:click=move |_| $save_open.set(false)
                                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                                    >
                                        <MaterialIcon name="close" />
                                    </button>
                                </div>
                                <div class="flex flex-col gap-3 px-6 py-5">
                                    <label class="flex flex-col gap-1">
                                        <span class="text-label-sm uppercase tracking-wider text-outline">
                                            "Version"
                                        </span>
                                        <input
                                            type="text"
                                            aria-label="Version"
                                            node_ref=version_ref
                                            class="w-32 rounded border border-outline-variant/40 bg-surface-container px-2 py-1 font-mono text-xs text-on-surface"
                                            value=$save_semver.get_untracked()
                                            on:input=move |ev| $save_semver.set(event_target_value(&ev))
                                        />
                                    </label>
                                    <label class="flex flex-col gap-1">
                                        <span class="text-label-sm uppercase tracking-wider text-outline">
                                            "Notes"
                                        </span>
                                        <textarea
                                            aria-label="Editor notes"
                                            rows="2"
                                            class="w-full resize-none rounded border border-outline-variant/40 bg-surface-container px-2 py-1 text-xs text-on-surface"
                                            prop:value=move || $save_notes.get()
                                            on:input=move |ev| $save_notes.set(event_target_value(&ev))
                                        ></textarea>
                                    </label>
                                    <p class=if big {
                                        "font-mono text-xs text-tactical-yellow"
                                    } else {
                                        "font-mono text-xs text-on-surface-variant"
                                    }>{move || size_line.get_value()}</p>
                                    {move || {
                                        $save_status
                                            .get()
                                            .starts_with("Saving")
                                            .then(|| {
                                                view! {
                                                    <div class="h-1 w-full overflow-hidden rounded-full bg-surface-variant/40">
                                                        <div class="animate-mc-load-bar h-full w-1/4 rounded-full bg-primary"></div>
                                                    </div>
                                                }
                                            })
                                    }}
                                    <p class="min-h-4 font-mono text-xs text-on-surface-variant">
                                        {move || $save_status.get()}
                                    </p>
                                    {move || {
                                        let rows = $save_findings.get();
                                        (!rows.is_empty())
                                            .then(|| {
                                                view! {
                                                    <ul class="max-h-32 list-disc space-y-1 overflow-y-auto rounded border border-error/40 bg-error/5 py-1 pl-5 pr-2 font-mono text-[11px] leading-snug text-error">
                                                        {rows
                                                            .into_iter()
                                                            .map(|r| view! { <li>{r}</li> })
                                                            .collect_view()}
                                                    </ul>
                                                }
                                            })
                                    }}
                                    <button
                                        type="button"
                                        class="self-end rounded bg-primary px-4 py-1.5 text-xs font-medium text-on-primary"
                                        on:click=move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            crate::v2::apps::editor::shell::document_commands::save_now(
                                                $save_semver.get_untracked(),
                                                $save_notes.get_untracked(),
                                                $save_status,
                                                $save_findings,
                                            );
                                        }
                                    >
                                        "Save"
                                    </button>
                                </div>
                            </div>
                            </Portal>
                        }
                    })
            }}
        }
    };
}
/// Expose the overlays rendering fragment to the strip view.
pub(super) use overlays;
