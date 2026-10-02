//! All settings dialog for the mission settings interface.

use super::*;

/// Renders the read-only list of authored settings.
#[component]
pub(super) fn AllSettingsDialog(open: RwSignal<bool>, doc_tick: RwSignal<u64>) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::v2::core::ui::modal_stack::register(move || {
            open.try_get_untracked().unwrap_or(false)
        });
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked()
                && ev.key() == "Escape"
                && crate::v2::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                open.set(false);
            }
        });
        on_cleanup(move || {
            esc.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }
    let only_diffs = RwSignal::new(false);
    move || {
        if !open.get() {
            return None;
        }
        let _ = doc_tick.get(); // re-aggregate on undo/redo while open
        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-3xl -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"All Settings"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            {ALL_SETTINGS_NOTE}
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </div>
                <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                    {render_all_settings_body(only_diffs)}
                </div>
            </div>
        })
    }
}

/// Renders and optionally filters the authored settings rows.
pub(super) fn render_all_settings_body(only_diffs: RwSignal<bool>) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = only_diffs;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let hint = "text-label-sm normal-case text-outline";
        let root = document_root().unwrap_or(serde_json::Value::Null);
        let all = aggregate_settings(&root);
        let total = all.len();
        let filtered = only_diffs.get();
        let rows: Vec<SettingRow> = if filtered {
            all.into_iter()
                .filter(SettingRow::survives_diff_filter)
                .collect()
        } else {
            all
        };
        let shown = rows.len();
        let toasts = crate::v2::core::ui::toast::use_toasts();

        let toggle_class = if filtered {
            format!(
                "rounded-md px-2.5 py-1.5 text-label-md {} {}",
                crate::v2::apps::editor::shell::layout::TOGGLED_PLATE,
                crate::v2::apps::editor::shell::layout::HOVER_FILL
            )
        } else {
            format!(
                "rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface-variant {}",
                crate::v2::apps::editor::shell::layout::HOVER_FILL
            )
        };

        let body = if rows.is_empty() {
            let empty = if total == 0 {
                "This mission authors no settings yet."
            } else {
                "Every authored setting is at the default its schema declares."
            };
            view! { <p class=hint data-all-settings-empty>{empty}</p> }.into_any()
        } else {
            rows.into_iter()
                .map(|r| {
                    let clickable = owner_is_routable(&r.owner);
                    setting_row_view(r, toasts, clickable)
                })
                .collect::<Vec<_>>()
                .into_any()
        };

        view! {
            <div class="flex flex-col gap-3" data-all-settings>
                <div class="flex items-center justify-between gap-3">
                    <span class=hint>
                        {format!("{shown} of {total} authored settings")}
                    </span>
                    <button
                        type="button"
                        class=toggle_class
                        aria-pressed=filtered.to_string()
                        data-all-settings-filter=filtered.to_string()
                        title="Hide every setting that is provably at the default its schema declares. Rows whose schema declares no default stay, because nothing can prove those are unchanged."
                        on:click=move |_| only_diffs.update(|v| *v = !*v)
                    >
                        "Changed from default"
                    </button>
                </div>
                <div class="flex flex-col gap-1">{body}</div>
            </div>
        }
        .into_any()
    }
}

#[must_use]
/// Explains why a setting row has no selection action.
pub fn inert_settings_row_reason(owner: &SettingOwner) -> String {
    match owner {
        SettingOwner::Mission => "Mission-owned settings name no entity — they are not click                                   targets. Change the value where it is authored."
            .to_string(),
        SettingOwner::Entity { .. } => "Found, but not selectable from here: the editor's                                         click-to-select router resolves no selection for this                                         owner right now, so a click would do nothing. Open it                                         from its own panel."
            .to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
/// Renders one selectable or inert settings row.
pub(super) fn setting_row_view(
    row: SettingRow,
    toasts: crate::v2::core::ui::toast::Toasts,
    clickable: bool,
) -> AnyView {
    let state = row.diff_state();
    let subject = row.owner.subject_id().map(ToString::to_string);
    let selectable = clickable;
    let inert_reason = inert_settings_row_reason(&row.owner);
    let owner_label = row.owner.label();
    let click_id = subject.clone().unwrap_or_default();
    let click_owner = owner_label.clone();
    let key_label = crate::v2::apps::editor::ui::inspector::zones_panel::humanize_key(&row.key);
    let value_text = fmt_setting_value(&row.value);
    let default_text = fmt_setting_default(&row.default);
    let source = match &row.default {
        SettingDefault::Schema { pointer, .. } | SettingDefault::Declared { pointer } => {
            pointer.clone()
        }
        SettingDefault::NotInSchema => String::new(),
    };
    let (badge, badge_class) = match state {
        DiffState::Differs => ("changed", "bg-tactical-yellow/20 text-tactical-yellow"),
        DiffState::Matches => ("default", "bg-primary/15 text-on-surface-variant"),
        DiffState::Unknown => ("no default", "bg-surface-variant/40 text-outline"),
    };
    let cursor = row_cursor_class(selectable);
    let row_class = format!(
        "grid w-full grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)_minmax(0,0.9fr)_minmax(0,0.9fr)_auto] items-baseline gap-3 rounded px-2 py-1.5 text-left outline-none transition-colors {cursor}",
    );
    let key_attr = row.key.clone();
    let owner_id_attr = subject.clone().unwrap_or_default();
    let diff_attr = format!("{state:?}").to_lowercase();
    let source_attr = source.clone();
    let cells = view! {
        <span class="min-w-0 truncate text-label-md text-on-surface" title=row.key.clone()>
            {key_label}
        </span>
        <span class="min-w-0 truncate text-label-sm text-on-surface-variant">
            {owner_label}
        </span>
        <span class="min-w-0 truncate font-mono text-code-md text-on-surface">
            {value_text}
        </span>
        <span class="min-w-0 truncate font-mono text-code-md text-outline">
            {default_text}
        </span>
        <span class=format!("shrink-0 rounded px-1.5 py-0.5 text-label-sm {badge_class}")>
            {badge}
        </span>
    };
    if selectable {
        view! {
            <button
                type="button"
                class=row_class
                data-all-settings-row=key_attr
                data-owner-id=owner_id_attr
                data-diff=diff_attr
                data-default-source=source_attr
                data-selectable="true"
                title=source
                on:click=move |_| {
                    if !crate::v2::apps::editor::ui::inspector::validation_panel::route_select_by_subject_id(&click_id) {
                        toasts.message(format!("{click_owner} — {OWNER_UNRESOLVED_NOTE}"));
                    }
                }
            >
                {cells}
            </button>
        }
        .into_any()
    } else {
        view! {
            <div
                class=row_class
                data-all-settings-row=key_attr
                data-owner-id=owner_id_attr
                data-diff=diff_attr
                data-default-source=source_attr
                data-selectable="false"
                aria-disabled="true"
                title=inert_reason
            >
                {cells}
            </div>
        }
        .into_any()
    }
}
