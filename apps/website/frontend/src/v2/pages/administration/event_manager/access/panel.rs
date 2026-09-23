//! The access panel: one operation's policies, groups, places and participant evidence in a sheet.
//!
//! **Role:** renders the sheet the calendar's day panel opens — the heading with the operation and
//! the access revision the panel is working against, the notice about the last change (what it
//! released and promoted, or why it was refused), the four section tabs, and the section on screen.
//! **Position:** a side sheet over the operations calendar.
//! **Signals & state:** reads the panel handle; the tabs write its section.
//! **Invariants:** nothing is editable until the access view has loaded, because every change must
//! name the revision it was read at. A failed read is shown as a failure with its reason, never as
//! an empty configuration.

use super::change_report::PanelNotice;
use super::groups::groups_section;
use super::participants_table::participants_table;
use super::policy_lists::policy_lists;
use super::quota_editor::quota_editor;
use super::state::{AccessPanel, AccessTab, Loadable};
use crate::v2::core::ui::{cn, MaterialIcon, Sheet};
use leptos::prelude::*;

/// The four tabs, with their labels.
const TABS: [(AccessTab, &str); 4] = [
    (AccessTab::Policies, "Policies"),
    (AccessTab::Groups, "Groups"),
    (AccessTab::Places, "Places"),
    (AccessTab::Participants, "Participants"),
];

/// The access sheet.
pub(in super::super) fn access_sheet(panel: AccessPanel) -> impl IntoView {
    view! {
        <Sheet open=panel.open bleed=true class="w-full max-w-none md:w-[56rem]">
            <div class="flex h-full flex-col">
                <header class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Access & Places"</h2>
                        <p class="mt-1 truncate text-label-md text-on-surface-variant">
                            {move || panel.event_name.get()}
                            {move || {
                                panel
                                    .access
                                    .with(|a| a.loaded().map(|view| view.access_revision))
                                    .map(|revision| format!(" · access revision {revision}"))
                            }}
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| panel.open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </header>
                {notice(panel)}
                <nav class="flex gap-1 border-b border-outline-variant/30 px-6 py-2">
                    {TABS
                        .into_iter()
                        .map(|(tab, label)| {
                            view! {
                                <button
                                    type="button"
                                    on:click=move |_| panel.tab.set(tab)
                                    class=move || {
                                        cn(
                                            &[
                                                "rounded-full px-4 py-1.5 text-sm transition",
                                                if panel.tab.get() == tab {
                                                    "bg-primary text-on-primary"
                                                } else {
                                                    "text-on-surface-variant hover:bg-white/5"
                                                },
                                            ],
                                        )
                                    }
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                </nav>
                <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">{body(panel)}</div>
            </div>
        </Sheet>
    }
}

/// Whether the access view can be edited yet.
#[derive(Clone, Debug, PartialEq)]
enum Readiness {
    Loading,
    Failed(String),
    Ready,
}

/// The section on screen, once the access view has loaded.
///
/// The readiness is a memo, so a newly answered view — which every change produces — rebuilds only
/// the section that reads it, not this switch.
fn body(panel: AccessPanel) -> impl IntoView {
    let readiness = Memo::new(move |_| {
        panel.access.with(|a| match a {
            Loadable::Loaded(_) => Readiness::Ready,
            Loadable::Failed(why) => Readiness::Failed(why.clone()),
            Loadable::Loading | Loadable::Idle => Readiness::Loading,
        })
    });
    move || match readiness.get() {
        Readiness::Failed(why) => {
            view! { <p class="text-sm text-error-alert">{why}</p> }.into_any()
        }
        Readiness::Loading => {
            view! { <p class="text-sm text-on-surface-variant">"Loading access settings…"</p> }
                .into_any()
        }
        Readiness::Ready => match panel.tab.get() {
            AccessTab::Policies => policy_lists(panel).into_any(),
            AccessTab::Groups => groups_section(panel).into_any(),
            AccessTab::Places => quota_editor(panel).into_any(),
            AccessTab::Participants => participants_table(panel).into_any(),
        },
    }
}

/// The notice about the last change: what it released and promoted, or why it was refused.
fn notice(panel: AccessPanel) -> impl IntoView {
    move || {
        panel.notice.get().map(|notice| match notice {
            PanelNotice::Refused(sentence) => view! {
                <div class="mx-6 mt-3 rounded-lg border border-error-alert/30 bg-error-alert/10 px-3 py-2 text-sm text-error-alert" role="alert">
                    {sentence}
                </div>
            }
            .into_any(),
            PanelNotice::Changed(report) => {
                let released = (!report.released.is_empty())
                    .then(|| format!("Released: {}", report.released.join("; ")));
                let promoted = (!report.promoted.is_empty())
                    .then(|| format!("Seated from the waiting list: {}", report.promoted.join("; ")));
                let quiet = released.is_none() && promoted.is_none();
                view! {
                    <div class="mx-6 mt-3 rounded-lg border border-success/30 bg-success/10 px-3 py-2 text-sm" role="status" data-testid="access-change-report">
                        <p class="text-success">{report.change}</p>
                        {released.map(|line| view! { <p class="mt-1 text-on-surface">{line}</p> })}
                        {promoted.map(|line| view! { <p class="mt-1 text-on-surface">{line}</p> })}
                        {quiet.then(|| view! { <p class="mt-1 text-on-surface-variant">"No reservation was released or promoted."</p> })}
                    </div>
                }
                .into_any()
            }
        })
    }
}
