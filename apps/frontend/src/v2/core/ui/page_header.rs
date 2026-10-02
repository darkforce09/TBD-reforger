//! The standard page title block, and the class-string join every primitive uses.
//!
//! **Role:** renders a page's heading and optional strapline, and joins class fragments.
//! **Position:** the first element inside a page's content area.
//! **Signals & state:** none.
//! **Invariants:** the join drops empty fragments and does nothing else — it does not resolve
//! conflicting utility classes, so a caller must not pass two fragments that fight over the same
//! property.

use leptos::prelude::*;

/// Join class fragments into one attribute string, dropping the empty ones.
///
/// Deliberately not a conflict resolver: two fragments setting the same property both survive, and
/// the later one wins by stylesheet order rather than by anything decided here.
pub fn cn(classes: &[&str]) -> String {
    classes
        .iter()
        .filter(|c| !c.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A page heading with an optional strapline.
///
/// Renders a `<header>` holding an `<h1>` and, when `subtitle` is non-empty, a paragraph beneath it.
/// Sits at the top of a page's content column.
#[component]
pub fn PageHeader(title: &'static str, #[prop(optional)] subtitle: &'static str) -> impl IntoView {
    view! {
        <header class="mb-8">
            <h1 class="mb-2 text-3xl font-bold text-on-surface">{title}</h1>
            {(!subtitle.is_empty())
                .then(|| view! { <p class="max-w-3xl text-on-surface-variant">{subtitle}</p> })}
        </header>
    }
}
