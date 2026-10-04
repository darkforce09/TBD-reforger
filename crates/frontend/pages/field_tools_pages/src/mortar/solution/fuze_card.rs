//! The time-fuze setting for a burst above the target.
//!
//! **Role:** words the solution's [`FireMissionFuze`]: the fuze time or why there is none, the
//! shell's fuze window, and the lay for the burst point with the charge it is fired on.
//! **Position:** a card of the solution panel (`super`), shown only for a time-fuzed shell given a
//! burst height.
//! **Signals & state:** none; pure functions and a view over plain values.
//! **Invariants:** a refused fuze shows no time and names its cause over every charge the
//! solver considered: the burst point reached only outside the fuze window, or out of reach at
//! every charge with the solver's precise cause at the lowest one (above the apex, beyond
//! maximum range, inside the minimum range, no convergence, beyond the shell's lifetime,
//! invalid input); the burst lay names the charge it is fired on (which may be stronger than
//! the ground-impact charge) and is in the weapon's mils and in degrees.

#[cfg(any(target_arch = "wasm32", test))]
use crate::mortar::solve_bridge::mils_and_degrees;
#[cfg(any(target_arch = "wasm32", test))]
use fire_mission_planning::fire_mission::FireMissionFuze;
#[cfg(any(target_arch = "wasm32", test))]
use fire_mission_planning::fuze::FuzeRefusal;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The fuze lines, in the order the card shows them.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn fuze_lines(fuze: &FireMissionFuze) -> Vec<String> {
    let setting = match (fuze.time_s, fuze.refusal) {
        (Some(time), _) => format!(
            "Fuze {time:.1} s for a burst {:.0} m above the target",
            fuze.burst_height_m
        ),
        (None, Some(FuzeRefusal::OutsideFuzeWindow)) => {
            "No fuze time: the burst falls outside the fuze window".to_string()
        }
        (None, Some(refusal)) => format!(
            "No fuze time: no charge reaches the burst point ({} at the lowest charge)",
            unreached_burst_point_cause(refusal)
        ),
        (None, None) => "No fuze time".to_string(),
    };
    let mut lines = vec![
        setting,
        format!(
            "Fuze window {:.1}–{:.1} s, default {:.1} s",
            fuze.min_s, fuze.max_s, fuze.default_s
        ),
    ];
    if let Some(aim) = fuze.burst_aim {
        lines.push(format!(
            "Burst lay: charge {} · elevation {} · aim azimuth {}",
            aim.rings,
            mils_and_degrees(aim.elevation_mils, aim.elevation_deg),
            mils_and_degrees(aim.aim_azimuth_mils, aim.aim_azimuth_deg)
        ));
    }
    lines
}

/// Why no charge reaches the burst point, as the fuze line words it.
#[cfg(any(target_arch = "wasm32", test))]
fn unreached_burst_point_cause(refusal: FuzeRefusal) -> &'static str {
    match refusal {
        FuzeRefusal::AboveApex => "above the apex",
        FuzeRefusal::BeyondRange => "beyond maximum range",
        FuzeRefusal::InsideMinimumRange => "inside the minimum range the elevation limit sets",
        FuzeRefusal::DidNotConverge => "the elevation search does not converge",
        FuzeRefusal::TimeToLiveExceeded => "the shell's lifetime ends first",
        FuzeRefusal::InvalidInput => "an input is invalid",
        FuzeRefusal::OutsideFuzeWindow => "outside the fuze window",
    }
}

/// The fuze card; nothing without a fuze setting.
#[cfg(target_arch = "wasm32")]
pub(crate) fn fuze_card(fuze: Option<FireMissionFuze>) -> impl IntoView {
    fuze.map(|f| {
        view! {
            <section class="rounded-xl p-4 glass text-xs" data-mortar-fuze="">
                <h2 class="text-sm font-semibold text-primary">"Time fuze (lead gun)"</h2>
                <ul class="mt-1">
                    {fuze_lines(&f).into_iter().map(|line| view! { <li>{line}</li> }).collect_view()}
                </ul>
            </section>
        }
    })
}
