//! Dense row geometry tests for the outliner.

/// T-637 — **THE PITCH IS ONE NUMBER, STATED ONCE.**
///
/// `ROW_H` is not decoration: the windowed renderer reserves off-screen rows with two spacer divs
/// sized `n × ROW_H`, so if `ROW_H` and the row class disagree the scroll height is a lie and a fast
/// scroll lands on the wrong row — by `n × Δ`, which grows with the tree. Before this ticket the two
/// were connected only by a comment, AND they already disagreed: `ROW_ACTIVE` carries a `border-t`
/// that `ROW` does not, so under `box-sizing: border-box` a selected row rendered a pixel taller than
/// the 24 the spacers reserved.
///
/// Both halves are fixed here. Every row recipe states `h-4` explicitly (so the border lives inside
/// the box and no recipe can be taller than another), and `ROW_H` is that class read back through the
/// Tailwind spacing scale rather than re-typed as a literal.
use super::{
    PALETTE_LEAF, ROW, ROW_ACTIVE, ROW_BADGE, ROW_FACTION, ROW_GEOM, ROW_H, ROW_STATIC, ROW_UNFILED,
};
use crate::v2::apps::editor::shell::layout::tw_len_px;

/// Every recipe a tree row can wear is [`ROW_GEOM`] plus a paint, and the windowing constant is
/// that geometry's stated height — not a number that merely happens to match it today.
#[test]
fn every_row_recipe_states_the_one_height_and_row_h_reads_it_back() {
    let geom_h = tw_len_px(ROW_GEOM, "h-").expect("ROW_GEOM must state an explicit `h-*`");
    assert!(
        (geom_h - ROW_H).abs() < f64::EPSILON,
        "T-637: ROW_H ({ROW_H}) must BE the height the shared row class renders ({geom_h}) — \
         the virtual spacers reserve n × ROW_H, so a mismatch is a scroll position that drifts \
         further wrong the longer the tree gets"
    );
    for (name, recipe) in [
        ("ROW", ROW),
        ("ROW_ACTIVE", ROW_ACTIVE),
        ("PALETTE_LEAF", PALETTE_LEAF),
        ("ROW_STATIC", ROW_STATIC),
        ("ROW_UNFILED", ROW_UNFILED),
        ("ROW_FACTION", ROW_FACTION),
    ] {
        let h = tw_len_px(recipe, "h-")
            .unwrap_or_else(|| panic!("T-637: `{name}` must state the row height explicitly"));
        assert!(
            (h - ROW_H).abs() < f64::EPSILON,
            "T-637: `{name}` renders at {h} px in a tree windowed at {ROW_H} px"
        );
        assert!(
            recipe.starts_with(ROW_GEOM),
            "T-637: `{name}` must be built from ROW_GEOM — the pitch lived in seven separate \
             string literals before this ticket, which is how ROW_ACTIVE drifted"
        );
        // No vertical padding may creep back: it would add to `h-4` under border-box only if the
        // content overflowed, but stating both is how the two definitions start disagreeing again.
        assert!(
            !recipe.contains(" py-") && !recipe.contains(" p-"),
            "T-637: `{name}` states its height; a `py-*`/`p-*` beside it is a second opinion"
        );
    }
}

/// **THE DENSITY, as a number.** 24 px was the complaint — a title, one row and ~900 px of void
/// in a 240 px dock. Eden's outliner runs at ~15.8 px and that is how it fits a mission's worth
/// of structure in the same width. This pins the pitch change against the historical 420 px
/// budget (17 → 26 rows). T-769 made the live scroller measured/`h-full`; the arithmetic here is
/// still the densification magnitude, not a claim about the live container height.
#[test]
fn the_tree_is_dense_enough_to_be_eden_shaped() {
    assert!(
        ROW_H <= 16.0,
        "T-637: {ROW_H} px per row is not a dense tree — Eden's is ~15.8"
    );
    // Historical reference height from the pre-T-769 fixed budget — densification only.
    const DENSITY_REFERENCE_H: f64 = 420.0;
    let rows_now = (DENSITY_REFERENCE_H / ROW_H).floor();
    let rows_before = (DENSITY_REFERENCE_H / 24.0).floor();
    assert!(
        rows_now >= rows_before + 8.0,
        "T-637: the densification must buy real rows — {rows_now} visible where the pre-ticket \
         24 px pitch gave {rows_before}"
    );
}

/// The in-row SL badge must fit INSIDE the row rather than setting its height. `ui::badge_class`
/// (the page-level pill) is `px-2 py-0.5` + a border ⇒ 22 px, which burst a 16 px row open; that
/// is why the tree carries its own.
#[test]
fn the_leader_badge_fits_inside_the_row() {
    let badge_h = tw_len_px(ROW_BADGE, "h-").expect("the row badge must state a height");
    assert!(
        badge_h < ROW_H,
        "T-637: the SL badge is {badge_h} px inside a {ROW_H} px row — a badge that sets the \
         row height is what made the ORBAT tree ragged"
    );
    assert!(
        ROW_BADGE.contains("leading-none"),
        "T-637: without `leading-none` the badge's line box (16 px) exceeds its own `h-3` box"
    );
    assert!(
        ROW_BADGE.contains("shrink-0"),
        "T-637: the badge must not be squeezed by a long label in a 240 px dock"
    );
}

/// T-769 — the windowed scroller fills the flex tree region (`h-full`) and sizes its window from
/// the measured `clientHeight`. A fixed `height:420px` coming back is the defect this pin guards.
#[test]
fn the_windowed_scroller_is_measured_h_full_not_a_fixed_budget() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source};
    let raw = crate::v2::apps::editor::ui::outliner::tree::TREE_PRODUCTION_SOURCE;
    let code = live_code(raw);
    let source = live_source(raw);
    assert!(
        code.contains("client_height"),
        "T-769: windowing must read the live scroller's clientHeight"
    );
    assert!(
        source.contains("h-full min-h-0 overflow-y-auto"),
        "T-769: the windowed scroller must be h-full inside the flex-1 tree region"
    );
    assert!(
        source.contains("outliner-window-scroller"),
        "T-769: the smoke measures this scroller by data-testid"
    );
    // Assembled so this pin's own prose cannot satisfy the negative check.
    let fixed = format!("height:{}px", 420);
    assert!(
        !source.contains(&fixed),
        "T-769: a fixed 420 px windowed budget must not return"
    );
    assert!(
        !code.contains("CONTAINER_H:") && !code.contains("CONTAINER_H}"),
        "T-769: CONTAINER_H must not drive windowing anymore"
    );
}

/// Every glyph in a row is `text-sm`, whose default line box is 20 px — 4 px taller than the row
/// it sits in. `leading-none` collapses that line box to the glyph, which is what lets a 16 px row
/// hold a 16 px icon cell without the icons setting the height. A single icon that forgets it
/// re-inflates every row it appears in, so this is checked over the whole file rather than per
/// call site.
#[test]
fn no_row_glyph_carries_an_uncollapsed_line_box() {
    let src = crate::v2::apps::editor::ui::outliner::tree::TREE_PRODUCTION_SOURCE;
    let production = src
        .split("#[cfg(test)]")
        .next()
        .expect("the production half precedes the test modules");
    // Needles assembled so this test's own source cannot satisfy or false-fail them.
    let loose = format!("{}{}", "text-sm", "\"");
    assert!(
        !production.contains(&loose),
        "T-637: a row glyph ends its class list at `text-sm` — its 20 px line box would set the \
         height of the 16 px row it sits in. Add `leading-none`."
    );
}
