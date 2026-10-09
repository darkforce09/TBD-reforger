//! The budget cases: level costs, decisions, reservations, peaks, the floor walk, the HUD tail,
//! measured growth, the default budget and the asset rows.

use super::*;

#[test]
fn decide_separates_shrink_from_never() {
    let mut led = Ledger::with_budget(1_000);
    assert_eq!(led.decide(1_000), Decision::Ok, "exactly the budget fits");
    assert_eq!(
        led.decide(1_001),
        Decision::Refuse,
        "bigger than the whole budget can never be made to fit, so the caller must not be \
             told to retry"
    );
    assert_eq!(led.reserve(Asset::Dem, 600), Decision::Ok);
    assert_eq!(
        led.decide(500),
        Decision::Degrade,
        "500 does not fit the 400 that is left but would fit an empty budget — that is the \
             `ask smaller` answer, and collapsing it into Refuse is what would leave the \
             satellite with no ladder to walk"
    );
    assert_eq!(led.decide(400), Decision::Ok, "the remainder still fits");
    assert_eq!(led.decide(0), Decision::Ok, "a zero request is free");
}

#[test]
fn reserve_records_only_on_ok() {
    let mut led = Ledger::with_budget(1_000);
    assert_eq!(led.reserve(Asset::Dem, 600), Decision::Ok);
    assert_eq!(led.held_total(), 600);
    assert_eq!(
        led.reserve(Asset::Satellite, 500),
        Decision::Degrade,
        "reserve and decide answer with the same arm"
    );
    assert_eq!(
        led.held_total(),
        600,
        "a degraded request must record NOTHING: bytes the loader never allocated would \
             refuse the next caller on the strength of a fiction"
    );
    assert_eq!(led.reserve(Asset::Satellite, 5_000), Decision::Refuse);
    assert_eq!(led.held_total(), 600);
    assert_eq!(led.entry(Asset::Satellite).held, 0);
    assert_eq!(led.entry(Asset::Satellite).peak, 0);
}

#[test]
fn peaks_are_per_asset_high_water_marks() {
    let mut led = Ledger::with_budget(MB_1024);
    assert_eq!(led.reserve(Asset::Dem, 71_911_548), Decision::Ok);
    assert_eq!(led.reserve(Asset::Dem, DEM_METERS), Decision::Ok);
    let peak = 71_911_548 + DEM_METERS;
    assert_eq!(
        led.entry(Asset::Dem).peak,
        peak,
        "the terrain lane's peak is the encoded body AND the raster it decodes into, alive \
             across the same decode call"
    );
    led.release(Asset::Dem, 71_911_548);
    assert_eq!(led.entry(Asset::Dem).held, DEM_METERS);
    assert_eq!(
        led.entry(Asset::Dem).peak,
        peak,
        "releasing must not lower the peak — the peak is the number the budget was sized for"
    );
    assert_eq!(led.total_peak(), peak);
    led.release(Asset::Dem, u64::MAX);
    assert_eq!(
        led.entry(Asset::Dem).held,
        0,
        "over-release saturates; wrapping to u64::MAX would install a budget nothing fits"
    );
}

#[test]
fn the_floor_rises_one_level_per_degrade() {
    let l = everon();
    let mut led = Ledger::with_budget(MB_1024);
    assert_eq!(led.reserve(Asset::Dem, DEM_METERS), Decision::Ok);

    let walk = floor_for_budget(&l, 0, &led);
    assert_eq!(
        walk.base, 1,
        "level 0 costs 1,026,523,730 B, which fits a 1024 MiB budget on its own but NOT \
             beside the 163,840,000 B DEM raster — so the floor must rise by exactly one"
    );
    assert_eq!(walk.raised(), 1);
    assert_eq!(
        walk.rejected,
        vec![(0, 1_026_523_730, Decision::Degrade)],
        "the rejected rung must name the level, its cost and WHY — a bare floor number \
             cannot tell an operator whether the budget or the GPU took his resolution"
    );
    assert_eq!(walk.bytes, 260_606_070);
    assert_eq!(
        led.decide(walk.bytes),
        Decision::Ok,
        "the level the walk lands on must actually be servable"
    );
}

#[test]
fn the_walk_never_reaches_past_the_ladder() {
    let l = everon();
    let led = Ledger::with_budget(1);
    let walk = floor_for_budget(&l, 0, &led);
    assert_eq!(
        walk.base,
        l.len() - 1,
        "when nothing fits, the coarsest level is loaded and the whole ladder is reported — \
             a blank basemap is strictly worse than a 1x1 one, and the caller warns either way"
    );
    assert_eq!(
        walk.raised(),
        13,
        "the floor moved 13 levels, even though 14 rungs were rejected — the coarsest was \
             rejected AND loaded. `raised()` must report the resolution that was lost, not the \
             number of questions asked"
    );
    assert_eq!(walk.rejected.len(), 14);
    assert_eq!(
        floor_for_budget(&[], 0, &led).base,
        0,
        "an empty ladder must not underflow"
    );
}

#[test]
fn the_floor_never_rises_above_the_gpu_limit_choice() {
    let l = everon();
    let led = Ledger::with_budget(MB_1024);
    let walk = floor_for_budget(&l, 3, &led);
    assert_eq!(
        walk.base, 3,
        "the budget may only take resolution AWAY from the level the GPU limit chose; it \
             must never hand back a level `pick_base_level_for_limit` already rejected"
    );
    assert!(walk.rejected.is_empty());
}

#[test]
fn growth_is_tracked_beside_the_declared_bytes_and_never_inside_them() {
    let mut led = Ledger::with_budget(MB_1024);
    led.add_growth(Asset::World, 40 * MIB);
    led.add_growth(Asset::World, 8 * MIB);
    assert_eq!(
        led.entry(Asset::World).growth,
        48 * MIB,
        "growth accumulates across passes — the residency drain runs up to twelve times"
    );
    assert_eq!(
        led.held_total(),
        0,
        "measured growth must never enter the budget arithmetic: an asset with both figures \
             would be counted twice and refuse loaders that would have fitted"
    );
    assert_eq!(led.hud_suffix(), "");
}
