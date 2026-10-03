//! The battery summary: one line per gun with what it lays.
//!
//! **Role:** shows each gun's laid charge — charge, elevation, aim azimuth and time of flight —
//! in the table at the head of the solution panel, worded by the map engine's shared
//! [`map_engine::data::scenario::ballistics::solution_wording`].
//! **Position:** the first table of the solution panel (`super`), above the per-gun charge tables.
//! **Signals & state:** none; pure functions and a view over plain values.
//! **Invariants:** a gun lays the pinned charge when the operator chose one, else its
//! recommendation; a gun whose laid charge does not solve says so and shows no figures; angles
//! are in the weapon's mils and in degrees.

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(any(target_arch = "wasm32", test))]
use map_engine::data::scenario::ballistics::battery::GunFireSolution;
#[cfg(any(target_arch = "wasm32", test))]
use map_engine::data::scenario::ballistics::solution_wording::{
    battery_line_words, BatteryLineWords,
};

/// One gun's summary line: the map engine's shared [`BatteryLineWords`].
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) type BatteryRowText = BatteryLineWords;

/// The summary line of `gun` laying the mission's `charge_rings`, worded by
/// [`battery_line_words`].
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn battery_row_text(gun: &GunFireSolution, charge_rings: Option<u32>) -> BatteryRowText {
    battery_line_words(gun, charge_rings)
}

/// The battery summary table.
#[cfg(target_arch = "wasm32")]
pub(crate) fn battery_rows(guns: &[GunFireSolution], charge_rings: Option<u32>) -> impl IntoView {
    let rows: Vec<BatteryRowText> = guns
        .iter()
        .map(|gun| battery_row_text(gun, charge_rings))
        .collect();
    view! {
        <section class="overflow-x-auto rounded-xl p-4 glass" data-mortar-battery="">
            <h2 class="text-sm font-semibold text-primary">"Battery"</h2>
            <table class="mt-2 w-full text-left text-xs">
                <thead>
                    <tr>
                        <th>"Gun"</th>
                        <th>"Charge"</th>
                        <th>"Elevation"</th>
                        <th>"Aim azimuth"</th>
                        <th>"TOF"</th>
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .into_iter()
                        .map(|r| {
                            view! {
                                <tr>
                                    <td>{r.label}</td>
                                    <td>{r.charge}</td>
                                    <td>{r.elevation}</td>
                                    <td>{r.aim_azimuth}</td>
                                    <td>{r.time_of_flight}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </section>
    }
}
