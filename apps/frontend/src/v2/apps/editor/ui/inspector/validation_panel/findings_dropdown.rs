//! Validation panel findings dropdown.

use super::*;

/// Renders the toolbar findings dropdown.
#[must_use]
pub fn findings_dropdown(rows: Vec<PanelFinding>) -> AnyView {
    let empty = rows.is_empty();
    view! {
        <div class="flex flex-col" data-validation-dropdown>
            {if empty {
                view! {
                    <div
                        class="flex items-center gap-2 px-3 py-2.5 text-label-md text-on-surface-variant opacity-80"
                        data-validation-empty
                    >
                        <crate::v2::core::ui::MaterialIcon name="check_circle" />
                        <span>"No issues"</span>
                    </div>
                }
                    .into_any()
            } else {
                view! {
                    <div class="flex max-h-[22rem] flex-col overflow-y-auto">
                        {group_list_view(rows)}
                    </div>
                }
                    .into_any()
            }} {legend_view()}
        </div>
    }
    .into_any()
}

fn group_list_view(rows: Vec<PanelFinding>) -> AnyView {
    let groups = group_by_rule(&rows);
    view! {
        <div class="flex flex-col py-1" data-validation-list>
            {groups
                .into_iter()
                .map(rule_group_view)
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

fn rule_group_view(group: RuleGroup) -> AnyView {
    let count = group.count();
    let sev = severity_tag(group.severity);
    let dot = severity_dot_class(group.severity);
    let rule_id = group.rule_id.clone();
    let rule_id_attr = group.rule_id.clone();
    view! {
        <div class="px-1 pb-1" data-validation-group=rule_id_attr data-severity=sev>
            <div class="flex items-center gap-1.5 px-2 pt-1.5 pb-0.5">
                <span class=format!("inline-block size-2 rounded-full {dot}")></span>
                <span class="text-label-sm font-medium tracking-wide text-on-surface-variant">
                    {rule_id}
                </span>
                <span class="ml-auto text-label-sm tabular-nums text-outline">{count}</span>
            </div>
            {group
                .findings
                .into_iter()
                .map(finding_row_view)
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

/// Explains why a finding row cannot select an entity.
#[must_use]
pub(super) fn inert_finding_row_reason(f: &PanelFinding) -> String {
    match f.subject_id.as_deref() {
        None | Some("") => {
            "This finding names no selectable subject — there is nothing for click-to-select to              pin."
                .to_string()
        }
        Some(_) => {
            "Found, but not selectable from here: the editor's click-to-select router resolves no              selection for this subject right now, so a click would do nothing."
                .to_string()
        }
    }
}

fn finding_row_view(f: PanelFinding) -> AnyView {
    let selectable = finding_is_routable(&f);
    let click_id = f.subject_id.clone().unwrap_or_default();
    let subject_id_attr = f.subject_id.clone().unwrap_or_default();
    let subject_attr = f.subject.clone();
    let message = f.message.clone();
    let subject_body = f.subject.clone();
    let cursor = row_cursor_class(selectable);
    let inert_reason = inert_finding_row_reason(&f);
    let row_class = format!(
        "flex w-full flex-col gap-0.5 rounded px-2 py-1 text-left outline-none transition-colors {cursor}",
    );
    let cells = view! {
        <span class="text-label-md leading-snug text-on-surface">{message}</span>
        <span class="text-label-sm text-outline">{subject_body}</span>
    };
    if selectable {
        view! {
            <button
                type="button"
                class=row_class
                data-validation-finding=subject_attr
                data-subject-id=subject_id_attr
                data-selectable="true"
                on:click=move |_| {
                    select_finding_subject(&click_id);
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
                data-validation-finding=subject_attr
                data-subject-id=subject_id_attr
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

fn legend_view() -> AnyView {
    view! {
        <div
            class="flex flex-col gap-1 border-t border-outline-variant/20 px-3 py-2"
            data-validation-legend
        >
            {SEVERITY_LADDER
                .iter()
                .map(|rung| {
                    let dot = severity_dot_class(rung.severity);
                    view! {
                        <div class="flex items-start gap-1.5">
                            <span class=format!(
                                "mt-1 inline-block size-2 shrink-0 rounded-full {dot}",
                            )></span>
                            <span class="text-label-sm text-on-surface-variant">
                                <span class="font-medium text-on-surface">{rung.label}</span>
                                " — "
                                {rung.meaning}
                            </span>
                        </div>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

/// Chooses the pointer class for a routable finding row.
#[must_use]
pub(super) fn row_cursor_class(clickable: bool) -> &'static str {
    if clickable {
        "cursor-pointer hover:bg-primary/10"
    } else {
        "cursor-default"
    }
}

#[must_use]
fn severity_dot_class(s: Severity) -> &'static str {
    match s {
        Severity::Error => "bg-error",
        Severity::Warning => "bg-tactical-yellow",
        Severity::Info => "bg-primary",
    }
}

fn select_finding_subject(subject_id: &str) {
    route_select_by_subject_id(subject_id);
}
