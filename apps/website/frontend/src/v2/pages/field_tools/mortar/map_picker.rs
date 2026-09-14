//! The coordinate entry fields and the terrain preview they drive.
//!
//! **Role:** owns the shared form-control class, the numeric field the four coordinates are typed
//! into, and the preview box — its grid backdrop, the gun-target line, and the two markers.
//! **Position:** the coordinate half of the inputs card, and the backdrop layer of the map panel
//! the saved-fires and firing-solution cards float over.
//! **Signals & state:** reads and writes the page's `fp_x`, `fp_y`, `tgt_x` and `tgt_y` signals;
//! holds no state of its own.
//! **Invariants:** every marker position comes from
//! [`preview_pos`](super::grid::preview_pos), so the preview moves whenever a coordinate does.
//! The SVG shares the markers' 0–100 percentage frame, which is what makes the line land on their
//! centres whatever shape the box takes.

use super::grid::preview_pos;
use leptos::prelude::*;

/// The class every text, number and select control on the page wears.
pub(super) const INPUT_CLASS: &str =
    "mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm";

/// One labelled number field bound to a coordinate signal. A value that does not parse reads as
/// `0.0` rather than leaving the signal at its previous number, so what is typed is what is sent.
fn num_input(label: &'static str, sig: RwSignal<f64>) -> impl IntoView {
    view! {
        <label class="text-sm">
            {label}
            <input
                type="number"
                // The attribute carries the value at rest ("1000" etc.) so the served markup
                // reads as the field looks; prop:value stays the live binding.
                value=move || sig.get().to_string()
                prop:value=move || sig.get().to_string()
                on:input=move |ev| sig.set(event_target_value(&ev).parse().unwrap_or(0.0))
                class=INPUT_CLASS
            />
        </label>
    }
}

/// The four coordinate fields — fire position then target, x then y — as siblings of the two
/// pickers inside the inputs card's grid.
pub(super) fn coordinate_inputs(
    fp_x: RwSignal<f64>,
    fp_y: RwSignal<f64>,
    tgt_x: RwSignal<f64>,
    tgt_y: RwSignal<f64>,
) -> impl IntoView {
    view! {
        {num_input("FP X", fp_x)} {num_input("FP Y", fp_y)} {num_input("TGT X", tgt_x)}
        {num_input("TGT Y", tgt_y)}
    }
}

/// The preview box's own layers: the grid backdrop, the dashed gun-target line, and the fire
/// position and target markers.
///
/// Returned as a fragment rather than a container so the page can lay the saved-fires and
/// firing-solution cards over the same positioned box.
pub(super) fn terrain_preview(
    fp_x: RwSignal<f64>,
    fp_y: RwSignal<f64>,
    tgt_x: RwSignal<f64>,
    tgt_y: RwSignal<f64>,
) -> impl IntoView {
    view! {
        <div
            class="absolute inset-0 opacity-30"
            style="background-image: linear-gradient(rgba(59, 130, 246, 0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(59, 130, 246, 0.08) 1px, transparent 1px); background-size: 40px 40px;"
        ></div>
        // Gun-target line. `preserveAspectRatio=none` puts the SVG on the same 0–100
        // percentage frame as the two markers below, so the line lands on their
        // centres at any box shape; `non-scaling-stroke` keeps it 1px anyway.
        <svg
            class="pointer-events-none absolute inset-0 h-full w-full"
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
        >
            <line
                x1=move || preview_pos((fp_x.get(), fp_y.get()), (tgt_x.get(), tgt_y.get())).0.0
                y1=move || preview_pos((fp_x.get(), fp_y.get()), (tgt_x.get(), tgt_y.get())).0.1
                x2=move || preview_pos((fp_x.get(), fp_y.get()), (tgt_x.get(), tgt_y.get())).1.0
                y2=move || preview_pos((fp_x.get(), fp_y.get()), (tgt_x.get(), tgt_y.get())).1.1
                stroke="currentColor"
                stroke-width="1"
                stroke-dasharray="3 3"
                vector-effect="non-scaling-stroke"
                class="text-tertiary/60"
            />
        </svg>
        <div
            class="absolute h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-success bg-success/30"
            style=move || {
                let (fp, _) = preview_pos(
                    (fp_x.get(), fp_y.get()),
                    (tgt_x.get(), tgt_y.get()),
                );
                format!("left:{}%;top:{}%", fp.0, fp.1)
            }
            title="Fire Position"
        ></div>
        <div
            class="absolute h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-error bg-error/30"
            style=move || {
                let (_, tgt) = preview_pos(
                    (fp_x.get(), fp_y.get()),
                    (tgt_x.get(), tgt_y.get()),
                );
                format!("left:{}%;top:{}%", tgt.0, tgt.1)
            }
            title="Target"
        ></div>
    }
}
