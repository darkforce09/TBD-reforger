//! The filter field: a real search input, with its own clear button.
//!
//! **Role:** the shared `<input type="search">` primitive, with the leading glyph and the clear
//! affordance drawn rather than left to the browser.
//! **Signals & state:** holds none of its own. The value is read from the caller's signal and
//! written straight to the DOM property, so the caller keeps ownership of it.
//! **Invariants:** fires on every keystroke rather than on the settle. A filter that narrowed only
//! once the field lost focus would not be a filter. The clear button routes through the same
//! callback as typing does, so clearing is not a separate event a caller could forget to handle.
//! The input keeps its search type — that is what tells assistive technology and the browser's own
//! autofill what it is — while the two engine-drawn decorations are switched off, because the
//! clear affordance is ours and must behave the same everywhere.

use leptos::prelude::*;

use super::icons::MaterialIcon;
use super::page_header::cn;
use crate::v2::apps::editor::shell::layout::{DISABLED_GLYPH, HOVER_FILL};

/// The field's own box.
///
/// The left and right padding are the gutters the leading glyph and the clear button sit in, so text
/// can never slide under either.
const SEARCH_BOX: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 py-1.5 pl-7 pr-7 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60";

/// The two engine-drawn search decorations, switched off.
const SEARCH_UA_PARTS: &str = "[&::-webkit-search-cancel-button]:appearance-none [&::-webkit-search-decoration]:appearance-none";

/// A search field.
///
/// Renders a wrapper span holding the leading glyph, the input, and — only while there is something to
/// clear — a clear button. Placed inline above whatever it filters; the caller owns the width.
#[component]
#[allow(dead_code)]
pub fn SearchBox(
    /// Accessible name, and the tooltip. Also the stem of the clear button's
    /// own label, so a page with two filters does not present two buttons both called "Clear".
    label: &'static str,
    /// Visible hint. Empty by default rather than a generic prompt — a
    /// field that says what it filters is the whole point of the prop.
    #[prop(optional)]
    placeholder: &'static str,
    /// The live query. Owned by the caller, read reactively.
    #[prop(into)]
    value: Signal<String>,
    /// Every keystroke, and the clear button's empty string.
    #[prop(into)]
    on_input: Callback<String>,
    /// Extra classes for the wrapper. The caller owns the width.
    #[prop(optional)]
    class: &'static str,
    /// Whether the field is inert.
    /// Test hook. Omitted entirely when empty, so a selector looking for one
    /// cannot match an unlabelled field.
    #[prop(optional)]
    disabled: bool,
    #[prop(optional)] test_id: &'static str,
) -> impl IntoView {
    view! {
        <span class=cn(&["relative flex items-center", class])>
            <MaterialIcon
                name="search"
                class="pointer-events-none absolute left-1.5 text-base leading-none text-outline"
            />
            <input
                type="search"
                aria-label=label
                title=label
                placeholder=placeholder
                disabled=disabled
                data-testid=(!test_id.is_empty()).then_some(test_id)
                class=cn(&[SEARCH_BOX, SEARCH_UA_PARTS, HOVER_FILL, DISABLED_GLYPH])
                prop:value=move || value.get()
                on:input=move |ev| on_input.run(event_target_value(&ev))
            />
            <Show when=move || !disabled && !value.get().is_empty()>
                <button
                    type="button"
                    aria-label=format!("Clear {label}")
                    title="Clear"
                    class=cn(&["absolute right-1 rounded-sm p-0.5 text-outline", HOVER_FILL])
                    on:click=move |_| on_input.run(String::new())
                >
                    <MaterialIcon name="close" class="text-base leading-none" />
                </button>
            </Show>
        </span>
    }
}
