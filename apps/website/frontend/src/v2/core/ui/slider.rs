//! The range slider, painted rather than tinted.
//!
//! **Role:** the shared `<input type="range">` primitive, styled end to end so it stops looking
//! like browser furniture inside a custom interface.
//! **Signals & state:** holds none of its own. The value is read from the caller's signal and
//! written straight to the DOM property, so the caller keeps ownership of it.
//! **Invariants:** uncontrolled by construction. A slider is dragged, and a drag emits values tens
//! of times a second; a control that owned a signal and re-rendered from it would put a full render
//! on every one of those. So the browser owns the drag, the value is a one-line property write, and
//! the only handler is the settle event — there is no per-keystroke callback and no internal state.
//!
//! Styling is the parts, not a tint. Setting an accent colour only recolours the browser's own
//! widget; the track geometry and the handle shape stay its. Painting the track and thumb
//! pseudo-elements directly is what actually takes the control off browser chrome.

use leptos::prelude::*;

use super::page_header::cn;
use crate::v2::apps::editor::layout::{DISABLED_GLYPH, HOVER_FILL};

/// The element box. Transparent on purpose: the visible rail is the track pseudo-element,
/// which leaves the box itself free to carry the hover fill without painting a second rail.
const SLIDER_BOX: &str = "h-5 cursor-pointer appearance-none rounded bg-transparent px-1 outline-none focus-visible:outline-1 focus-visible:outline-primary/60";

/// Rail and handle on WebKit and Blink.
///
/// The negative top margin is not decoration: the thumb is laid out against the *top* of the runnable
/// track, so a twelve-pixel handle on a four-pixel rail needs four pixels back to sit on the rail's
/// centre line.
const SLIDER_WEBKIT: &str = "[&::-webkit-slider-runnable-track]:h-1 [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-surface-container-highest [&::-webkit-slider-thumb]:-mt-1 [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary";

/// Rail and handle on Gecko, which centres its own thumb and so needs no offset.
///
/// Clearing the border is load-bearing — the default handle carries one, and it reads as a light halo
/// on a dark surface.
const SLIDER_MOZ: &str = "[&::-moz-range-track]:h-1 [&::-moz-range-track]:rounded-full [&::-moz-range-track]:bg-surface-container-highest [&::-moz-range-thumb]:size-3 [&::-moz-range-thumb]:appearance-none [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-primary";

/// A range slider.
///
/// Renders one `<input type="range">` carrying the painted track and handle. `value` is read
/// reactively and written to the DOM property; `on_change` fires on the settle, never per pixel. A
/// caller that needs live feedback during a drag should keep its own preview rather than ask for a
/// callback the work downstream would throw away. Values are whole numbers because a stepped range
/// emits whole numbers, and one the control could not have produced is dropped rather than guessed at.
/// Placed inline wherever a numeric scrub belongs; the caller owns the width.
#[component]
#[allow(dead_code)]
pub fn Slider(
    /// Accessible name, and the tooltip. Deliberately not conditional on
    /// being enabled: a control that cannot act must still explain itself.
    label: &'static str,
    min: i32,
    max: i32,
    /// Where the handle sits. Read reactively, written straight to the DOM property.
    #[prop(default = 1)]
    step: i32,
    #[prop(into)] value: Signal<i32>,
    /// The settled value. Fires on the native change event, never on input.
    #[prop(into)]
    on_change: Callback<i32>,
    /// Extra classes. The caller owns the width; this owns the paint.
    #[prop(optional)]
    class: &'static str,
    /// Whether the control is inert.
    #[prop(optional)]
    disabled: bool,
) -> impl IntoView {
    view! {
        <input
            type="range"
            min=min
            max=max
            step=step
            aria-label=label
            title=label
            disabled=disabled
            class=cn(&[SLIDER_BOX, SLIDER_WEBKIT, SLIDER_MOZ, HOVER_FILL, DISABLED_GLYPH, class])
            prop:value=move || value.get().to_string()
            on:change=move |ev| {
                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                    on_change.run(v);
                }
            }
        />
    }
}
