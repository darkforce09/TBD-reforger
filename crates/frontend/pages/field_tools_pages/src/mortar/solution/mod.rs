//! The solution panel: what the last Calculate produced.
//!
//! **Role:** shows the input problems, or the solved fire mission: the battery summary, the lead
//! gun's crest clearance, fuze setting and dispersion, and one charge table per gun.
//! **Position:** below Calculate on `/tools/mortar`; reads the page's outcome signal. The figures
//! are `fire_mission_planning`'s [`FireMissionSolution`] as solved on this device.
//! **Signals & state:** reads the outcome signal; holds nothing.
//! **Invariants:** every figure comes from the one solution, so the summary, the cards and the
//! tables always describe the same solve; the dispersion is labelled as an interpretation.
//!
//! [`FireMissionSolution`]: fire_mission_planning::fire_mission::FireMissionSolution

pub(crate) mod battery_rows;
pub(crate) mod charges_table;
pub(crate) mod crest_warning;
pub(crate) mod dispersion_card;
pub(crate) mod fuze_card;

#[cfg(target_arch = "wasm32")]
use super::solve_bridge::SolvedMission;
#[cfg(target_arch = "wasm32")]
use battery_rows::battery_rows;
#[cfg(target_arch = "wasm32")]
use charges_table::charges_table;
#[cfg(target_arch = "wasm32")]
use crest_warning::crest_warning;
#[cfg(target_arch = "wasm32")]
use dispersion_card::dispersion_card;
#[cfg(target_arch = "wasm32")]
use fuze_card::fuze_card;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// What the last Calculate produced: the solved mission, or every input problem.
#[cfg(target_arch = "wasm32")]
pub type SolveOutcome = Result<SolvedMission, Vec<String>>;

/// The solution panel over the page's `outcome`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn solution_panel(outcome: RwSignal<Option<SolveOutcome>>) -> impl IntoView {
    move || match outcome.get() {
        None => view! {
            <p class="text-sm text-on-surface-variant">
                "Enter the target and the guns, then calculate."
            </p>
        }
        .into_any(),
        Some(Err(problems)) => view! {
            <ul class="list-disc pl-5 text-sm text-error" data-mortar-problems="">
                {problems.into_iter().map(|p| view! { <li>{p}</li> }).collect_view()}
            </ul>
        }
        .into_any(),
        Some(Ok(mission)) => {
            let charge_rings = mission.inputs.charge_rings;
            let solution = mission.solution;
            view! {
                <div class="flex flex-col gap-4" data-mortar-solution="">
                    {battery_rows(&solution.guns, charge_rings)}
                    {crest_warning(solution.crest)}
                    {fuze_card(solution.fuze)}
                    {dispersion_card(solution.dispersion)}
                    {solution
                        .guns
                        .iter()
                        .map(|gun| charges_table(gun, charge_rings))
                        .collect_view()}
                </div>
            }
            .into_any()
        }
    }
}

#[cfg(test)]
#[path = "../tests/solution.rs"]
mod tests;
