//! Menu row rendering for the top command strip.

use super::*;

macro_rules! menu_row {
    ($title:ident, $can_undo:ident, $can_redo:ident, $save_semver:ident, $save_status:ident, $dirty:ident, $settings_open:ident, $doc_tick:ident, $obj_count:ident, $orbat_open:ident, $open_menu:ident, $export_open:ident, $save_open:ident, $save_notes:ident, $validation_open:ident, $hint_open:ident, $set_hint:ident, $close_transients:ident, $transient_closer_id:ident, $row_mirror:ident, $toasts:ident, $save_findings:ident, $last_flush:ident, $recency_tick:ident, $save_was_open:ident, $env:ident, $census:ident, $summary:ident, $validation_findings:ident, $export_gesture_ok:ident, $run_action:ident, $widget_is:ident, $snap_on:ident, $title_fallback:ident) => {
        view! {
            <div class=ROW_MENUS>
            <div class="flex shrink-0 items-center">
                {move || {
                    if let Some(t) = $doc_tick {
                        t.track();
                    }
                    #[cfg(target_arch = "wasm32")]
                    let doc_title = {
                        let t = crate::v2::apps::editor::bridge::host_state::editor_context::read_title();
                        if t.is_empty() { $title_fallback.get_value() } else { t }
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let doc_title = $title_fallback.get_value();
                    view! {
                        <input
                            type="text"
                            aria-label="Mission title"
                            class="w-36 sm:w-48 max-w-[16rem] truncate rounded border border-transparent bg-transparent px-1.5 py-0 text-label-md font-semibold text-on-surface outline-none transition-colors focus:border-outline-variant/40 focus:bg-surface-container"
                            prop:value=doc_title
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let v = event_target_value(&ev);
                                    if !v.trim().is_empty() {
                                        crate::v2::apps::editor::bridge::host_state::editor_context::set_title(v.trim());
                                    }
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = &ev;
                            }
                        />
                    }
                }}
                {$dirty
                    .map(|d| {
                        view! {
                            <span
                                class=move || if d.get() { "ml-1.5 text-primary" } else { "hidden" }
                                title="Unsaved changes"
                                aria-label="Unsaved changes"
                            >
                                "•"
                            </span>
                        }
                    })}
            </div>
            <span class=DIVIDER></span>
            <div class="flex shrink-0 items-center">
                {MENUS
                    .iter()
                    .enumerate()
                    .map(|(i, (name, items))| {
                        view! {
                            <div class="relative">
                                <button
                                    type="button"
                                    class=move || {
                                        if $open_menu.get() == Some(i) {
                                            cn(&["rounded px-2 py-0.5 text-label-sm", TOGGLED_PLATE])
                                        } else {
                                            cn(&[
                                                "rounded px-2 py-0.5 text-label-sm text-on-surface-variant",
                                                HOVER_FILL,
                                            ])
                                        }
                                    }
                                    on:click=move |_| {
                                        $export_open.set(false);
                                        $open_menu
                                            .update(|m| {
                                                *m = if *m == Some(i) { None } else { Some(i) };
                                            });
                                    }
                                >
                                    {*name}
                                </button>
                                {move || {
                                    ($open_menu.get() == Some(i))
                                        .then(|| {
                                            view! {
                                                <div class=cn(&[MENU_PANEL, "left-0 w-64"])>
                                                    {items
                                                        .iter()
                                                        .map(|it| {
                                                            let label = it.label;
                                                            let chord = arrange_chord_for_label(label);
                                                            match it.action {
                                                                Some(a) => {
                                                                    let disabled = move || match a {
                                                                        MenuAction::Undo => !$can_undo.get(),
                                                                        MenuAction::Redo => !$can_redo.get(),
                                                                        MenuAction::Pattern(_)
                                                                        | MenuAction::Align(_)
                                                                        | MenuAction::Space(_)
                                                                        | MenuAction::Orient(_) => {
                                                                            selection_count() == 0
                                                                        }
                                                                        _ => false,
                                                                    };
                                                                    let title = move || {
                                                                        if !disabled() {
                                                                            ""
                                                                        } else {
                                                                            match a {
                                                                                MenuAction::Pattern(_)
                                                                                | MenuAction::Align(_)
                                                                                | MenuAction::Space(_)
                                                                                | MenuAction::Orient(_) => {
                                                                                    "Select entities first"
                                                                                }
                                                                                _ => "Nothing to do yet",
                                                                            }
                                                                        }
                                                                    };
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            title=title
                                                                            class=cn(
                                                                                &[MENU_ROW, HOVER_FILL, DISABLED_GLYPH],
                                                                            )
                                                                            disabled=disabled
                                                                            on:click=move |ev| {
                                                                                if matches!(
                                                                                    a,
                                                                                    MenuAction::Export
                                                                                        | MenuAction::ExportCompiled
                                                                                ) && !$export_gesture_ok(&ev)
                                                                                {
                                                                                    return;
                                                                                }
                                                                                $run_action(a);
                                                                            }
                                                                        >
                                                                            <span class=MENU_GUTTER>
                                                                                {move || {
                                                                                    (matches!(a, MenuAction::ControlsHint)
                                                                                        && $hint_open.get())
                                                                                        .then(|| {
                                                                                            view! {
                                                                                                <MaterialIcon
                                                                                                    name="check"
                                                                                                    class="block text-base leading-none"
                                                                                                />
                                                                                            }
                                                                                        })
                                                                                }}
                                                                            </span>
                                                                            <span>{label}</span>
                                                                            {(!chord.is_empty())
                                                                                .then(|| {
                                                                                    view! {
                                                                                        <span class=MENU_CHORD>{chord}</span>
                                                                                    }
                                                                                })}
                                                                        </button>
                                                                    }
                                                                        .into_any()
                                                                }
                                                                None => {
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            disabled=true
                                                                            title="Not available yet"
                                                                            class=cn(&[MENU_ROW, DISABLED_GLYPH])
                                                                        >
                                                                            <span class=MENU_GUTTER></span>
                                                                            <span>{label}</span>
                                                                        </button>
                                                                    }
                                                                        .into_any()
                                                                }
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                        })
                                }}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            <button
                type="button"
                aria-label="ORBAT Manager"
                title="Open the ORBAT Manager"
                class=cn(
                    &[
                        "shrink-0 rounded px-2 py-0.5 text-label-sm text-on-surface-variant",
                        HOVER_FILL,
                        DISABLED_GLYPH,
                    ],
                )
                disabled=$orbat_open.is_none()
                on:click=move |_| {
                    if let Some(o) = $orbat_open {
                        $close_transients();
                        o.set(true);
                    }
                }
            >
                "ORBAT Manager"
            </button>
            <div class="flex min-w-0 flex-1 items-center justify-end">
                {move || {
                    $recency_tick.track();
                    $last_flush
                        .get()
                        .map(|ts| {
                            #[cfg(target_arch = "wasm32")]
                            let elapsed = js_sys::Date::now() - ts;
                            #[cfg(not(target_arch = "wasm32"))]
                            let elapsed = {
                                let _ = ts;
                                0.0
                            };
                            let label = format_draft_recency(elapsed);
                            let aria = label.clone();
                            view! {
                                <span
                                    class="ml-2 shrink-0 whitespace-nowrap text-label-sm text-on-surface-variant"
                                    title=DRAFT_CHIP_TOOLTIP
                                    aria-label=aria
                                    data-draft-chip
                                >
                                    {label}
                                </span>
                            }
                        })
                }}
            </div>
            <div class="hidden">
                <div
                    class="flex items-center gap-1.5 font-mono text-[11px] leading-none tabular-nums text-on-surface-variant"
                    title="Per-side slot census (WEST · EAST · IND · TOTAL)"
                    data-slot-$census
                >
                    <span title="WEST (BLUFOR) slots">
                        "WEST "
                        <span class="text-on-surface">{move || $census.get().west}</span>
                    </span>
                    <span class="text-outline">"·"</span>
                    <span title="EAST (OPFOR) slots">
                        "EAST "
                        <span class="text-on-surface">{move || $census.get().east}</span>
                    </span>
                    <span class="text-outline">"·"</span>
                    <span title="IND (INDFOR) slots">
                        "IND "
                        <span class="text-on-surface">{move || $census.get().ind}</span>
                    </span>
                    {move || {
                        let u = $census.get().unassigned;
                        (u > 0)
                            .then(|| {
                                view! {
                                    <span class="text-outline">"·"</span>
                                    <span class="text-tactical-yellow" title="Slots with no side">
                                        "UNA "
                                        <span>{u}</span>
                                    </span>
                                }
                            })
                    }}
                    <span class="text-outline">"·"</span>
                    <span title="Total placed slots">
                        "TOTAL "
                        <span class="text-on-surface">{move || $census.get().total}</span>
                    </span>
                </div>
                <span class=DIVIDER></span>
                <div
                    class="max-w-[22rem] truncate font-mono text-[10px] leading-none text-outline"
                    title=move || $summary.get()
                    data-mission-$summary
                >
                    {move || $summary.get()}
                </div>
            </div>
            </div>
        }
    };
}
/// Expose the menu row rendering fragment to the strip view.
pub(super) use menu_row;
