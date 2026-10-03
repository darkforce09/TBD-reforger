//! The lead gun's impact spread, labelled as the interpretation it is.
//!
//! **Role:** words the [`ImpactDispersion`] of the solution: the range and deflection probable
//! errors, the 50 % ellipse and the shell's range-card standard dispersion.
//! **Position:** a card of the solution panel (`super`); the same ellipses are drawn on the map.
//! **Signals & state:** none; pure functions and a view over plain values.
//! **Invariants:** the card always carries [`DISPERSION_CAVEAT`]: the spread is derived from the
//! game's dispersion parameters and is not verified in-engine.

#[cfg(any(target_arch = "wasm32", test))]
use ballistics_solver::dispersion::ImpactDispersion;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The label every dispersion figure carries.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const DISPERSION_CAVEAT: &str = "Interpretation, not verified in-engine";

/// The dispersion lines, in the order the card shows them.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn dispersion_lines(dispersion: &ImpactDispersion) -> Vec<String> {
    vec![
        format!(
            "Range probable error {:.1} m · deflection probable error {:.1} m",
            dispersion.range_probable_error_m, dispersion.deflection_probable_error_m
        ),
        format!(
            "50 % ellipse {:.1} × {:.1} m, major axis {:.1}°",
            2.0 * dispersion.ellipse_semi_major_m,
            2.0 * dispersion.ellipse_semi_minor_m,
            dispersion.ellipse_orientation_deg
        ),
        format!(
            "Range-card standard dispersion {:.1} m",
            dispersion.standard_dispersion_m
        ),
    ]
}

/// The dispersion card; nothing when the lead gun has no solving charge.
#[cfg(target_arch = "wasm32")]
pub(crate) fn dispersion_card(dispersion: Option<ImpactDispersion>) -> impl IntoView {
    dispersion.map(|d| {
        view! {
            <section class="rounded-xl p-4 glass text-xs" data-mortar-dispersion="">
                <h2 class="text-sm font-semibold text-primary">"Dispersion (lead gun)"</h2>
                <p class="text-tactical-yellow" data-mortar-dispersion-caveat="">
                    {DISPERSION_CAVEAT}
                </p>
                <ul class="mt-1">
                    {dispersion_lines(&d).into_iter().map(|line| view! { <li>{line}</li> }).collect_view()}
                </ul>
            </section>
        }
    })
}
