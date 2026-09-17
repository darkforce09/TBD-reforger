//! Tool row rendering for the top command strip.

use super::*;

macro_rules! tool_row {
    ($title:ident, $can_undo:ident, $can_redo:ident, $save_semver:ident, $save_status:ident, $dirty:ident, $settings_open:ident, $doc_tick:ident, $obj_count:ident, $orbat_open:ident, $open_menu:ident, $export_open:ident, $save_open:ident, $save_notes:ident, $validation_open:ident, $hint_open:ident, $set_hint:ident, $close_transients:ident, $transient_closer_id:ident, $row_mirror:ident, $toasts:ident, $save_findings:ident, $last_flush:ident, $recency_tick:ident, $save_was_open:ident, $env:ident, $census:ident, $summary:ident, $validation_findings:ident, $export_gesture_ok:ident, $run_action:ident, $widget_is:ident, $snap_on:ident, $title_fallback:ident) => {
        view! {
            <div class=ROW_TOOLS>
            <button
                type="button"
                aria-label="History"
                title="Version history (soon)"
                class=cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])
                disabled=true
            >
                <MaterialIcon name="history" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Undo"
                title="Undo (Ctrl+Z)"
                class=cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])
                disabled=move || !$can_undo.get()
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::v2::apps::editor::bridge::document_host::history::undo();
                    }
                }
            >
                <MaterialIcon name="undo" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Redo"
                title="Redo (Ctrl+Shift+Z)"
                class=cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])
                disabled=move || !$can_redo.get()
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::v2::apps::editor::bridge::document_host::history::redo();
                    }
                }
            >
                <MaterialIcon name="redo" class="block text-base leading-none" />
            </button>
            <span class=DIVIDER></span>
            <button
                type="button"
                aria-label="No widget"
                title="No widget (1)"
                class=move || {
                    if $widget_is(1) {
                        cn(&[BTN_ICON, HOVER_FILL, TOGGLED_PLATE])
                    } else {
                        cn(&[BTN_ICON, HOVER_FILL])
                    }
                }
                on:click=move |_| $run_action(MenuAction::SetWidget(1))
            >
                <MaterialIcon name="block" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Translate widget"
                title="Translate widget (2)"
                class=move || {
                    if $widget_is(2) {
                        cn(&[BTN_ICON, HOVER_FILL, TOGGLED_PLATE])
                    } else {
                        cn(&[BTN_ICON, HOVER_FILL])
                    }
                }
                on:click=move |_| $run_action(MenuAction::SetWidget(2))
            >
                <MaterialIcon name="open_with" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Rotate widget"
                title="Rotate widget (3)"
                class=move || {
                    if $widget_is(3) {
                        cn(&[BTN_ICON, HOVER_FILL, TOGGLED_PLATE])
                    } else {
                        cn(&[BTN_ICON, HOVER_FILL])
                    }
                }
                on:click=move |_| $run_action(MenuAction::SetWidget(3))
            >
                <MaterialIcon name="rotate_right" class="block text-base leading-none" />
            </button>
            <span class=DIVIDER></span>
            <button
                type="button"
                aria-label="Toggle snap grid"
                title="Toggle snap grid (G)"
                class=move || {
                    if $snap_on() {
                        cn(&[BTN_ICON, HOVER_FILL, TOGGLED_PLATE])
                    } else {
                        cn(&[BTN_ICON, HOVER_FILL])
                    }
                }
                on:click=move |_| $run_action(MenuAction::ToggleSnap)
            >
                <MaterialIcon name="grid_on" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Decrease snap step"
                title="Decrease snap step ([)"
                class=cn(&[BTN_ICON, HOVER_FILL])
                on:click=move |_| $run_action(MenuAction::SnapStep(-1))
            >
                <MaterialIcon name="remove" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Increase snap step"
                title="Increase snap step (])"
                class=cn(&[BTN_ICON, HOVER_FILL])
                on:click=move |_| $run_action(MenuAction::SnapStep(1))
            >
                <MaterialIcon name="add" class="block text-base leading-none" />
            </button>
            <span class=DIVIDER></span>
            <div class="flex shrink-0 items-center gap-2">
                <Slider
                    label="Time of day"
                    min=0
                    max=1439
                    class="w-28"
                    value=Signal::derive(move || {
                        hhmm_to_minutes(&$env.get().time).unwrap_or(360) as i32
                    })
                    on_change=Callback::new(move |mins: i32| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let hhmm = minutes_to_hhmm(mins.clamp(0, 1439) as u32);
                            author_env("time", hhmm.as_str().into());
                            $row_mirror.set_time(&hhmm);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = mins;
                    })
                />
                <span class="font-mono text-xs tabular-nums text-on-surface-variant">
                    {move || $env.get().time}
                </span>
                <Select
                    label="Weather"
                    options=WEATHER_OPTIONS
                    value=Signal::derive(move || $env.get().weather)
                    on_change=Callback::new(move |w: String| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            author_env("weather", w.as_str().into());
                            $row_mirror.set_weather(&w);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = w;
                    })
                />
                <button
                    type="button"
                    aria-label="Mission settings"
                    title="Mission Settings — the rest of the environment"
                    class=cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])
                    disabled=$settings_open.is_none()
                    on:click=move |_| {
                        if let Some(s) = $settings_open {
                            $close_transients();
                            s.set(true);
                        }
                    }
                >
                    <MaterialIcon name="settings" class="block text-base leading-none" />
                </button>
            </div>
            <div class="min-w-4 flex-1"></div>
            <div class="relative shrink-0">
                <button
                    type="button"
                    aria-label="Validation issues"
                    aria-haspopup="menu"
                    aria-expanded=move || $validation_open.get()
                    title="Mission validation — click for the findings"
                    data-validation-chip
                    data-issue-total=move || {
                        crate::v2::apps::editor::ui::inspector::validation_panel::Rollup::of(&$validation_findings.get()).total()
                    }
                    class=move || {
                        if $validation_open.get() {
                            cn(&[VALIDATION_CHIP, TOGGLED_PLATE])
                        } else {
                            cn(&[VALIDATION_CHIP, HOVER_FILL])
                        }
                    }
                    on:click=move |_| {
                        $open_menu.set(None);
                        $export_open.set(false);
                        $validation_open.update(|o| *o = !*o);
                    }
                >
                    <MaterialIcon name="rule" class="block text-sm leading-none" />
                    <span class=move || {
                        let r = crate::v2::apps::editor::ui::inspector::validation_panel::Rollup::of(&$validation_findings.get());
                        let accent = if r.has_blocking() {
                            "text-error-alert"
                        } else if r.total() > 0 {
                            "text-tactical-yellow"
                        } else {
                            "text-on-surface-variant"
                        };
                        cn(&["text-xs font-medium tabular-nums", accent])
                    }>
                        {move || {
                            let r = crate::v2::apps::editor::ui::inspector::validation_panel::Rollup::of(&$validation_findings.get());
                            if r.is_empty() { "No issues".to_string() } else { r.chip_text() }
                        }}
                    </span>
                    <MaterialIcon
                        name="expand_more"
                        class="inline-block align-middle text-sm leading-none"
                    />
                </button>
                {move || {
                    $validation_open
                        .get()
                        .then(|| {
                            view! {
                                <div class=cn(&[MENU_PANEL, "right-0 w-80"])>
                                    {crate::v2::apps::editor::ui::inspector::validation_panel::findings_dropdown(
                                        $validation_findings.get(),
                                    )}
                                </div>
                            }
                        })
                }}
            </div>
            <span class="min-w-24 shrink-0 font-mono text-xs text-on-surface-variant">
                {move || $save_status.get()}
            </span>
            <button
                type="button"
                title="Save an immutable version of this mission"
                class=ACTION_PRIMARY
                on:click=move |_| {
                    $close_transients();
                    $save_open.set(true);
                }
            >
                "Save Version"
            </button>
            <div class="relative shrink-0">
                <button
                    type="button"
                    aria-label="Export"
                    aria-haspopup="menu"
                    title="Download this mission — pick a format"
                    class=move || {
                        if $export_open.get() {
                            cn(&[ACTION_SECONDARY, TOGGLED_PLATE])
                        } else {
                            cn(&[ACTION_SECONDARY, HOVER_FILL])
                        }
                    }
                    on:click=move |_| {
                        $open_menu.set(None);
                        $export_open.update(|o| *o = !*o);
                    }
                >
                    "Export"
                    <MaterialIcon
                        name="expand_more"
                        class="ml-0.5 inline-block align-middle text-sm leading-none"
                    />
                </button>
                {move || {
                    $export_open
                        .get()
                        .then(|| {
                            view! {
                                <div class=cn(&[MENU_PANEL, "right-0 w-72"])>
                                    <button
                                        type="button"
                                        title="The editor superset envelope — re-imports here; the mod cannot load it"
                                        class=cn(&[MENU_ROW, HOVER_FILL])
                                        on:click=move |ev| {
                                            if $export_gesture_ok(&ev) {
                                                $run_action(MenuAction::Export);
                                            }
                                        }
                                    >
                                        <span class=MENU_GUTTER></span>
                                        <span>"Export JSON"</span>
                                    </button>
                                    <button
                                        type="button"
                                        title="The compiled mission document the game server receives"
                                        class=cn(&[MENU_ROW, HOVER_FILL])
                                        on:click=move |ev| {
                                            if $export_gesture_ok(&ev) {
                                                $run_action(MenuAction::ExportCompiled);
                                            }
                                        }
                                    >
                                        <span class=MENU_GUTTER></span>
                                        <span>"Export Compiled"</span>
                                    </button>
                                </div>
                            }
                        })
                }}
            </div>
            </div>
        }
    };
}
/// Expose the tool row rendering fragment to the strip view.
pub(super) use tool_row;
