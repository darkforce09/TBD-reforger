use super::{TAB_CELL_OFF, TAB_CELL_ON, TAB_CELL_VERB, TAB_COUNT, TAB_GROUP, TAB_STRIP};
use crate::v2::apps::editor::shell::layout::{tw_len_px, DOCK_PX, DOCK_R, STUB_PX};

/// The production half of this file — everything above the first test module, so a needle here
/// cannot satisfy itself (the T-759 hollow-pin trap).
fn production() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    ))
    .split("#[cfg(test)]")
    .next()
    .expect("the production half precedes the test modules")
}

/// The strip's rendered width, in CSS px, from the classes themselves.
fn strip_width_px() -> f64 {
    let cell = tw_len_px(TAB_CELL_ON, "size-").expect("a tab cell states its size");
    let verb = tw_len_px(TAB_CELL_VERB, "size-").expect("the verb cell states its size");
    let gap = tw_len_px(TAB_GROUP, "gap-").expect("the groups state their gap");
    let outer = tw_len_px(TAB_STRIP, "gap-").expect("the strip states its gap");
    let tabs = TAB_COUNT as f64;
    // Tab group: N cells + (N−1) gaps. Right group: the verb + the 24 px collapse chevron
    // (STUB_PX — its hit box must match the collapsed stub exactly, so it is not ours to shrink)
    // + one gap. Then the gap between the two groups.
    let left = tabs * cell + (tabs - 1.0) * gap;
    let right = verb + STUB_PX + gap;
    left + right + outer
}

/// The whole strip fits inside the equalised dock, gutters included.
#[test]
fn the_tab_strip_fits_the_dock() {
    let pad = tw_len_px(DOCK_R, "p-").expect("the dock states its padding");
    let budget = DOCK_PX - 2.0 * pad;
    let width = strip_width_px();
    assert!(
        width <= budget,
        "T-637/T-632: the tab strip renders {width} px inside a {budget} px dock — the trailing \
         cell clips at the panel edge, which is exactly the defect this ticket absorbed"
    );
    // …and it is not fitting by a hair, which would clip the moment a glyph gained a border.
    assert!(
        budget - width >= 8.0,
        "T-637: only {} px of slack — that is a clipping bug waiting for the next cell",
        budget - width
    );
    // The budget is computed from the SELECTED cell, so the two states must be the same box.
    // A selected tab that were wider would shuffle every cell right of it on each tab change —
    // and would make this arithmetic a lower bound rather than the width.
    assert_eq!(
        tw_len_px(TAB_CELL_ON, "size-"),
        tw_len_px(TAB_CELL_OFF, "size-"),
        "T-637: selecting a tab must not resize its cell — the strip would jitter, and the \
         width budget would be measuring the wrong state"
    );
}

/// **PERTURB / FAIL / RESTORE on the real failure mode.** The pre-T-637 strip labelled its cells
/// with uppercase `text-label-sm` words. Even at a conservative 7 px per character plus the
/// `px-1.5` gutters, those seven labels plus `Manage` blow the budget — which is why the trailing
/// tab clipped. The budget check must REJECT that layout, or it is asserting nothing.
#[test]
fn the_budget_rejects_the_word_labelled_strip_it_replaced() {
    let pad = tw_len_px(DOCK_R, "p-").expect("the dock states its padding");
    let budget = DOCK_PX - 2.0 * pad;
    // The labels the strip used to render, verbatim.
    let words = [
        "Factions",
        "Vehicles",
        "Zones",
        "Compositions",
        "Triggers",
        "Favourites",
        "Markers",
        "Manage",
    ];
    // 12 px uppercase at ~7 px/char, plus `px-1.5` (12 px of gutter per cell) — deliberately
    // conservative; the real advance width of uppercase 12 px is wider.
    let word_strip: f64 = words
        .iter()
        .map(|w| w.chars().count() as f64 * 7.0 + 12.0)
        .sum::<f64>()
        + STUB_PX;
    assert!(
        word_strip > budget,
        "PERTURB: the word-labelled strip must NOT fit ({word_strip} px in {budget} px) — if it \
         did, the budget check would be passing everything"
    );
    // RESTORE: the shipped glyph strip does fit, and by a wide margin.
    assert!(
        strip_width_px() < word_strip,
        "RESTORE: glyphs must actually be narrower than the words they replaced"
    );
}

/// The count the budget is computed from is the count the view renders, and every cell keeps its
/// word where a human (or a screen reader, or a gate selector) can still reach it. A glyph strip
/// whose cells were anonymous would trade a clipping bug for an unusable one.
#[test]
fn every_glyph_tab_keeps_its_name() {
    let src = production();
    assert_eq!(
        src.matches("tab_btn(").count(),
        TAB_COUNT,
        "T-637: TAB_COUNT ({TAB_COUNT}) must be the number of tabs the strip actually renders — \
         the width budget is computed from it"
    );
    for label in [
        "Factions",
        "Vehicles",
        "Zones",
        "Compositions",
        "Triggers",
        "Favourites",
        "Markers",
    ] {
        assert!(
            src.contains(&format!("{:?}", label)),
            "T-637: `{label}` must survive as the cell's title/aria-label — the word moved off \
             the glyph, it did not vanish"
        );
    }
    // The label reaches BOTH the tooltip and the accessible name, from the one `label` argument.
    assert!(
        src.contains("title=label") && src.contains("aria-label=label"),
        "T-637: one label, two consumers — a tooltip a pointer finds and a name a screen reader \
         (and every `[aria-label]` gate selector) resolves"
    );
    // Every tab index the strip renders has a glyph of its own: two tabs sharing one is a strip
    // you cannot read.
    let mut glyphs: Vec<&str> = (0..TAB_COUNT).map(super::tab_icon).collect();
    glyphs.sort_unstable();
    let n = glyphs.len();
    glyphs.dedup();
    assert_eq!(
        glyphs.len(),
        n,
        "T-637: each tab needs a DISTINCT glyph — with the words gone, the glyph is the only \
         thing telling two tabs apart"
    );
}
