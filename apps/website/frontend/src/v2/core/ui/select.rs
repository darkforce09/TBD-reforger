//! The dropdown, with the platform's popup and none of its chrome.
//!
//! **Role:** the shared `<select>` primitive: a real select element with the browser's own arrow
//! replaced by an icon.
//! **Signals & state:** holds none of its own. The value is read from the caller's signal and
//! written straight to the DOM property, so the caller keeps ownership of it.
//! **Invariants:** the element stays a real select, so the popup is the platform's — which is the
//! correct behaviour on a phone and the accessible behaviour everywhere. The chevron is drawn over
//! it and takes no pointer events, so clicking it still opens the list.

use leptos::prelude::*;

use super::icons::MaterialIcon;
use super::page_header::cn;
use crate::v2::apps::editor::layout::{DISABLED_GLYPH, HOVER_FILL};

/// A dropdown.
///
/// Renders a wrapper span holding a `<select>` and the chevron drawn over it. The option table is
/// static data rather than markup repeated per call site, so an option set has one definition.
/// Placed inline wherever a choice belongs; the caller owns the width.
#[component]
#[allow(dead_code)]
pub fn Select(
    /// Accessible name, and the tooltip. Retained while disabled.
    label: &'static str,
    /// The wire value and its human label, rendered in order.
    options: &'static [(&'static str, &'static str)],
    /// The selected wire value. Written to the DOM property, so a value that
    /// is not in `options` shows as no selection rather than silently rewriting the document.
    #[prop(into)]
    value: Signal<String>,
    /// The newly selected wire value.
    /// Extra classes for the select element.
    /// Whether the control is inert.
    #[prop(into)]
    on_change: Callback<String>,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    view! {
    // The disabled dimming applies to the select element only, so the chevron is dimmed by a
    // sibling rule — without it the glyph stays fully lit beside a faded control.
           <span class="relative inline-flex items-center">
               <select
                   aria-label=label
                   title=label
                   disabled=disabled
                   class=cn(
                       &[
                           "peer appearance-none rounded border border-outline-variant/40 bg-surface-container py-0.5 pr-6 pl-1.5 text-xs text-on-surface outline-none focus-visible:outline-1 focus-visible:outline-primary/60",
                           HOVER_FILL,
                           DISABLED_GLYPH,
                           class,
                       ],
                   )
                   prop:value=move || value.get()
                   on:change=move |ev| on_change.run(event_target_value(&ev))
               >
                   {options
                       .iter()
                       .map(|(v, l)| {
                           view! {
                               <option class="bg-surface-container text-on-surface" value=*v>
                                   {*l}
                               </option>
                           }
                       })
                       .collect_view()}
               </select>
               <MaterialIcon
                   name="expand_more"
                   class="pointer-events-none absolute right-0.5 text-base leading-none text-on-surface-variant peer-disabled:opacity-30"
               />
           </span>
       }
}
