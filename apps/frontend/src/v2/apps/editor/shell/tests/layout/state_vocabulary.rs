//! Layout t668 state vocabulary tests.

//! T-668 — the ONE state vocabulary, pinned. Each rule's recipe const is its own source of truth for
//! the whole chrome, so a drift that would re-introduce two state languages fails HERE, once, rather
//! than being caught (or missed) file-by-file. Pure `const`s in the production half, natively
//! compiled — a `cargo test` reads them directly.

use super::{DISABLED_GLYPH, DISABLED_KEEPS_TOOLTIP, HOVER_FILL, MENU_GUTTER, TOGGLED_PLATE};

/// Rule (1) — HOVER is a solid fill and NOTHING ELSE. It fills on hover (`hover:bg-white/10`),
/// eases (`transition-colors`), and must NOT carry a border token — a border is rule (2)'s cue,
/// and mixing it in is exactly the confusion this ticket removes.
#[test]
fn hover_fill_is_a_solid_fill_no_border() {
    assert!(
        HOVER_FILL.contains("hover:bg-white/10"),
        "HOVER = solid fill (the Aegis reading of Eden's amber-hover)"
    );
    assert!(
        HOVER_FILL.contains("transition-colors"),
        "the hover fill must ease in, not snap"
    );
    assert!(
        !HOVER_FILL.contains("border"),
        "HOVER must carry no border — a border is the TOGGLED cue; sharing it is the bug"
    );
}

/// Rule (2) — TOGGLED-ON is a lighter plate PLUS a 1px dark top border, and the two together are
/// what make it distinct from a hover fill BY CONSTRUCTION. The plate is the Aegis primary tint;
/// the `border-t border-background/…` is the dark top lip a hovered control never grows.
#[test]
fn toggled_plate_is_plate_plus_dark_top_border() {
    assert!(
        TOGGLED_PLATE.contains("bg-primary/20") && TOGGLED_PLATE.contains("text-primary"),
        "TOGGLED = the lighter Aegis primary plate"
    );
    assert!(
        TOGGLED_PLATE.contains("border-t") && TOGGLED_PLATE.contains("border-background"),
        "TOGGLED = plate + a 1px dark TOP border (distinct-by-construction from hover)"
    );
}

/// Rules (1) and (2) are distinct BY CONSTRUCTION — the whole point. The toggled plate carries a
/// `border-t` the hover fill does not, and the hover fill carries a `hover:` fill the toggled
/// plate does not, so no control can ever render in a state where the two are indistinguishable.
#[test]
fn hover_and_toggled_can_never_be_confused() {
    assert!(
        TOGGLED_PLATE.contains("border-t") && !HOVER_FILL.contains("border-t"),
        "only the toggled plate has the top border"
    );
    assert!(
        HOVER_FILL.contains("hover:bg-") && !TOGGLED_PLATE.contains("hover:bg-"),
        "only the hover recipe fills on hover; the toggled plate is a persistent fill"
    );
    assert_ne!(
        HOVER_FILL, TOGGLED_PLATE,
        "the two states must not be the same string (the bg-white/10-as-active defect)"
    );
}

/// Rule (3) — DISABLED dims the glyph and cancels the hover fill, so a dimmed control does not
/// still light up under the pointer. The tooltip half is a pattern, not a class — its name is
/// pinned so the chrome files' tooltip-retention pins have a shared referent.
#[test]
fn disabled_glyph_dims_and_cancels_hover() {
    assert!(
        DISABLED_GLYPH.contains("disabled:opacity-30"),
        "DISABLED = dimmed glyph"
    );
    assert!(
        DISABLED_GLYPH.contains("disabled:hover:bg-transparent"),
        "a disabled control must not still fill on hover (cancels HOVER_FILL)"
    );
    assert!(
        DISABLED_KEEPS_TOOLTIP.contains("tooltip"),
        "rule 3's tooltip-retention pattern must be named for the per-file pins to cite"
    );
}

/// Convention — the menu checkmark gutter is a fixed-width, always-present cell (Eden's jumping
/// indent is the bug NOT to copy). `shrink-0` keeps it from collapsing when a row is tight, and
/// `size-4` matches the tree chevron cell so a menu and a tree read at the same indent.
#[test]
fn menu_gutter_is_a_fixed_always_present_cell() {
    assert!(
        MENU_GUTTER.contains("size-4") && MENU_GUTTER.contains("shrink-0"),
        "the gutter is a fixed-width cell that never collapses (no jumping indent)"
    );
}

/// FIRE THE RULE ONCE (perturb / fail / restore) on rule (2), the load-bearing one. The property
/// under test is "toggled-on is distinguishable from hover by a border". PERTURB: a would-be
/// toggled recipe that is just the hover fill (the `bg-white/10`-as-active defect, stated as a
/// value) has NO border, so the distinguishing check FAILS on it — proving the check has teeth.
/// RESTORE: the real `TOGGLED_PLATE` carries the border and passes. A check that passed for both
/// would be asserting nothing.
#[test]
fn toggled_distinct_from_hover_rule_fires() {
    // The real recipe is distinguishable from a hover fill — it has the top border.
    let distinguishable = |toggled: &str| toggled.contains("border-t");
    assert!(
        distinguishable(TOGGLED_PLATE),
        "RESTORE: the real toggled plate carries the distinguishing top border"
    );
    // PERTURB: the defect this ticket removes — "toggled" rendered as the neutral hover fill.
    let defect_toggled = "bg-white/10";
    assert!(
        !distinguishable(defect_toggled),
        "PERTURB: a toggled state that is merely the hover fill has no border — the check must \
             REJECT it, or it is asserting nothing (this is the bg-white/10-as-active bug)"
    );
    // And the defect value is not what we ship.
    assert_ne!(
        TOGGLED_PLATE, defect_toggled,
        "the toggled recipe must not be the bare hover fill"
    );
}
