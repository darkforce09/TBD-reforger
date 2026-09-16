//! Role: Domain regression cases.
//! Position: `streaming/memory/budget/t938_6` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

#[test]
fn the_everon_ladder_costs_what_the_audit_says() {
    let l = everon();
    assert_eq!(
        l[0].rgba(),
        655_360_000,
        "the audit's headline figure: satellite L0 is 655 MB of RGBA on its own"
    );
    assert_eq!(
        satellite_resident_bytes(&l, 0),
        1_026_523_730,
        "a base-0 load holds the WHOLE tail at once — every level is decoded before the \
             engine borrow is taken — so the peak is 873,813,260 B of RGBA plus 152,710,470 B of \
             bodies, not just the largest level"
    );
    assert_eq!(
        satellite_resident_bytes(&l, 1),
        260_606_070,
        "one level down is 218,453,260 + 42,152,810"
    );
    assert_eq!(satellite_resident_bytes(&l, 13), 4 + 38, "the 1x1 tail");
    assert_eq!(
        satellite_resident_bytes(&l, 99),
        0,
        "a base past the ladder costs nothing rather than panicking"
    );
}

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
fn a_tighter_budget_walks_further_down_the_ladder() {
    let l = everon();
    let led = Ledger::with_budget(MB_512);
    let walk = floor_for_budget(&l, 0, &led);
    assert_eq!(
        walk.base, 1,
        "512 MiB is smaller than level 0's 978.97 MiB outright — Refuse, not Degrade — and \
             level 1's 248.53 MiB fits"
    );
    assert_eq!(walk.rejected.len(), 1);
    assert_eq!(walk.rejected[0].2, Decision::Refuse);

    let led = Ledger::with_budget(64 * MIB);
    let walk = floor_for_budget(&l, 0, &led);
    assert_eq!(
        walk.base, 2,
        "62.85 MiB from level 2 down is the first rung under a 64 MiB budget"
    );
    assert_eq!(walk.raised(), 2);
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
fn the_hud_shows_reserved_against_budget_and_the_floor() {
    let mut led = Ledger::with_budget(MB_1024);
    assert_eq!(
        led.hud_suffix(),
        "",
        "nothing held and no floor chosen is a dead cell, not a readout"
    );
    assert_eq!(led.reserve(Asset::Dem, DEM_METERS), Decision::Ok);
    assert_eq!(led.hud_suffix(), " · mem 156/1024MB");
    led.set_satellite_floor(1, 1);
    assert_eq!(
        led.hud_suffix(),
        " · mem 156/1024MB · sat L1 (+1)",
        "the raise count is on the HUD because `sat L1` alone cannot be told apart from a \
             GPU that only ever offered L1"
    );
    led.set_satellite_floor(0, 0);
    assert_eq!(led.hud_suffix(), " · mem 156/1024MB · sat L0");
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

#[test]
fn the_default_budget_is_the_one_the_spec_names() {
    assert_eq!(DEFAULT_BUDGET_MB, 1536);
    assert_eq!(configured_budget_bytes(), 1536 * MIB);
    assert_eq!(
        heap_bytes(),
        0,
        "there is no wasm linear memory on the host"
    );
}

#[test]
fn every_asset_indexes_its_own_row() {
    for (i, a) in Asset::ALL.iter().enumerate() {
        assert_eq!(a.index(), i);
    }
    let mut led = Ledger::with_budget(MB_1024);
    for a in Asset::ALL {
        assert_eq!(led.reserve(a, 1), Decision::Ok);
    }
    for a in Asset::ALL {
        assert_eq!(led.entry(a).held, 1, "{} must have its own row", a.name());
    }
    assert_eq!(led.held_total(), 7);
}
