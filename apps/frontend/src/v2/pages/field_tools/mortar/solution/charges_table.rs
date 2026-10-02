//! One gun's charge table: every charge of the shell, the laid one highlighted.
//!
//! **Role:** renders a gun's [`GunFireSolution`] as a heading and one row per charge, worded by
//! the solve bridge's [`charge_row_text`] and [`gun_heading`].
//! **Position:** under each gun in the solution panel (`super`).
//! **Signals & state:** none; built from plain values.
//! **Invariants:** angles are in the weapon's mils and in degrees; a refused charge shows why and
//! no figures; the laid charge is the pinned charge, else the gun's recommendation.

use crate::v2::pages::field_tools::mortar::solve_bridge::{
    charge_row_text, gun_heading, laid_rings,
};
use leptos::prelude::*;
use map_engine::data::scenario::ballistics::battery::GunFireSolution;

/// The charge table of `gun` laying the mission's `charge_rings`.
pub(crate) fn charges_table(gun: &GunFireSolution, charge_rings: Option<u32>) -> impl IntoView {
    let laid = laid_rings(gun, charge_rings);
    let rows = gun
        .charges
        .iter()
        .map(|c| charge_row_text(c, laid))
        .collect::<Vec<_>>();
    view! {
        <section class="overflow-x-auto rounded-xl p-4 glass" data-mortar-gun=gun.label.clone()>
            <h2 class="text-sm font-semibold text-primary">{gun_heading(gun)}</h2>
            <table class="mt-2 w-full text-left text-xs">
                <thead>
                    <tr>
                        <th>"Charge"</th>
                        <th>"Elevation"</th>
                        <th>"Aim azimuth"</th>
                        <th>"Deflection corr."</th>
                        <th>"Range corr."</th>
                        <th>"TOF"</th>
                        <th>"Apex"</th>
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .into_iter()
                        .map(|r| {
                            view! {
                                <tr class=if r.laid { "font-semibold text-primary" } else { "" }>
                                    <td>{r.charge}</td>
                                    <td>{r.elevation}</td>
                                    <td>{r.aim_azimuth}</td>
                                    <td>{r.deflection}</td>
                                    <td>{r.range_correction}</td>
                                    <td>{r.time_of_flight}</td>
                                    <td>{r.apex}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </section>
    }
}
