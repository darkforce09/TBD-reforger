//! Comment row routing tests for the outliner.

// ═══════ T-784 — a comment row SELECTS, and its affordance is the router's own answer ════════════
//
// The gap this closes was TOTAL, not partial: the Outliner's comment row was `ROW_STATIC` with no
// click path to `ctx.selection` at all, the map glyph had no pick, the T-697 selection filter only
// NARROWS an existing selection (so it cannot introduce a comment that was never selected), and the
// document-search router had no comment arm, so comment hits rendered inert.
//
// Two pins here. The CORRESPONDENCE pin walks every `NodeKind` and asserts, in both directions,
// that the affordance `row_routes` paints is exactly what the click `route_select_by_subject_id`
// will find — with a non-vacuity assert proving the corpus contained BOTH selectable and inert
// rows, because an all-false corpus would green a `row_routes` that always answered `false`. It
// never names which kinds "should" be selectable: that stale-list shape is what made the dock-left
// pin green while it guarded a lie (wave-129 RV-1). The SHAPE pin holds the rendered row to the
// `eden_settings` a11y contract — button when live, non-focusable `aria-disabled` when not.
use super::{inert_row_reason, row_router_subject, row_routes};
use crate::v2::apps::editor::ui::inspector::validation_panel::{
    register_route_probe, register_select_by_id, route_select_by_subject_id,
};
use crate::v2::apps::editor::ui::outliner::outliner::NodeKind;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// A dense ordinal per `NodeKind`. **The compiler is the completeness check**: the match is
/// exhaustive, so a new variant cannot build until it is given an ordinal, and the coverage
/// assertion in the correspondence test then fails until [`every_node_kind`] actually contains
/// it. Rust has no derive that enumerates a foreign enum; a bare list with no ordinal check
/// would be precisely the stale kind list these pins exist to forbid.
fn kind_ordinal(k: NodeKind) -> usize {
    match k {
        NodeKind::Folder => 0,
        NodeKind::Unfiled => 1,
        NodeKind::Slot => 2,
        NodeKind::Faction => 3,
        NodeKind::Squad => 4,
        NodeKind::Comment => 5,
    }
}

/// One more than the largest ordinal `kind_ordinal` hands out.
const KINDS: usize = 6;

fn every_node_kind() -> [NodeKind; KINDS] {
    [
        NodeKind::Folder,
        NodeKind::Unfiled,
        NodeKind::Slot,
        NodeKind::Faction,
        NodeKind::Squad,
        NodeKind::Comment,
    ]
}

/// **The affordance and the click cannot disagree — over EVERY row kind, in BOTH directions.**
///
/// One resolver, registered as both seams, exactly as `mission_editor`'s mount wires them
/// (`Rc::clone` of a single closure). It resolves ids ending `-yes` and refuses the rest, so
/// every kind is exercised against a router that says yes AND a router that says no — the two
/// directions the invariant has actually been violated in: an affordance over a dead click
/// (T-754) and an inert row over a click that would have worked (wave-129 RV-1).
///
/// Perturbation RED: make `row_routes` return `true` for `NodeKind::Comment` without asking the
/// probe, or drop the `Comment` arm from `row_router_subject`.
#[test]
fn the_affordance_and_the_click_cannot_disagree_over_any_row_kind() {
    let resolve: std::rc::Rc<dyn Fn(&str) -> bool> =
        std::rc::Rc::new(|id: &str| id.ends_with("-yes"));
    {
        let p = std::rc::Rc::clone(&resolve);
        register_route_probe(std::rc::Rc::new(move |id: &str| p(id)));
    }
    {
        let p = std::rc::Rc::clone(&resolve);
        register_select_by_id(std::rc::Rc::new(move |id: &str| p(id)));
    }

    let mut covered = [false; KINDS];
    let mut saw_selectable = false;
    let mut saw_inert = false;
    for kind in every_node_kind() {
        covered[kind_ordinal(kind)] = true;
        for id in ["row-yes", "row-no"] {
            // What the VIEW paints.
            let affordance = row_routes(kind, id);
            // What the CLICK finds: this row's subject handed to the router itself.
            let click = row_router_subject(kind, id).is_some_and(route_select_by_subject_id);
            assert_eq!(
                affordance, click,
                "T-784: row kind {kind:?} with id {id:?} paints affordance={affordance} over a \
                 click that resolves {click} — a row is clickable IFF clicking it does \
                 something, and both ends of that must be the SAME resolver's answer"
            );
            saw_selectable |= affordance;
            saw_inert |= !affordance;
        }
    }
    assert!(
        covered.iter().all(|c| *c),
        "T-784: the corpus must cover every NodeKind ordinal — a kind this pin never saw is a \
         kind it never guarded"
    );
    // NON-VACUITY. Without this an all-inert corpus greens a `row_routes` that answers `false`
    // unconditionally, which is the defect (the comment row) restored under a passing test.
    assert!(
        saw_selectable,
        "T-784: VACUOUS — no row in the corpus was selectable, so the equality above proved \
         nothing about the affordance being painted"
    );
    assert!(
        saw_inert,
        "T-784: VACUOUS — no row in the corpus was inert, so the equality above proved nothing \
         about the affordance being WITHHELD"
    );
}

/// The refusal is the PROBE's, never a kind ban, and never a fallback when no probe is
/// registered: no probe means no router to click into, and `false` is the honest answer.
#[test]
fn no_probe_means_no_affordance_and_no_fallback() {
    register_route_probe(std::rc::Rc::new(|_: &str| false));
    assert!(
        !row_routes(NodeKind::Comment, "cmt-1"),
        "T-784: a refusing probe must leave the comment row inert"
    );
    let src = live_code(include_str!("../../tree.rs"));
    let routes = only_body(&src, "pub(crate) fn row_routes(");
    assert!(
        routes.contains(&format!("subject_id{}", "_routes")),
        "T-784: clickability must BE subject_id_routes — the shape follows that boolean, it \
         does not replace it"
    );
    // NEGATIVE, and scoped to the two functions that make the decision (a negative over the
    // whole file would be green by construction — `single_row` is one giant kind match).
    for marker in [
        "pub(crate) fn row_routes(",
        "pub(crate) fn row_router_subject",
    ] {
        let body = only_body(&src, marker);
        assert!(
            !body.contains(&format!("route{}", "_target")),
            "T-784: {marker} must not re-ask mission_editor::route_target directly — the \
             registered probe is the one answer, and a second reader of the resolution is how \
             the affordance and the click drift apart"
        );
    }
}

/// **The rendered shape follows that boolean.** Routable ⇒ a real `<button>` (a keyboard user
/// can activate the selection); refused ⇒ a non-focusable element carrying `aria-disabled` and
/// [`inert_row_reason`] — never a tab-stop button that does nothing (the wave-115 MINOR-6 shape,
/// as `eden_settings` fixed it).
///
/// Literals kept (`live_source`): the claim is about the tags and attributes that ship.
#[test]
fn the_comment_row_branches_on_the_router_and_is_never_a_dead_button() {
    let lit = live_source(include_str!("../../tree.rs"));
    let arm = only_body(&lit, &format!("fn comment{}", "_row("));
    assert!(
        arm.contains(&format!("row{}", "_routes(")),
        "T-784: the comment arm must branch on row_routes — the one boolean that owns \
         clickability"
    );
    assert!(
        arm.contains("<button") && arm.contains("</button>"),
        "T-784: a routable comment row must be a real button"
    );
    assert_eq!(
        arm.matches("<button").count(),
        1,
        "T-784: exactly one <button> in the comment arm — an inert `<button aria-disabled>` is \
         still a tab stop, which is the shape this rejects"
    );
    assert!(
        arm.contains("aria-disabled") && arm.contains(&format!("inert_row{}", "_reason(")),
        "T-784: the inert branch must be a non-focusable element that says WHY"
    );
    let router = ["route_select", "_by_subject_id("].concat();
    assert!(
        arm.contains(&router),
        "T-784: the click must be the shipped router — the same resolution row_routes asked, \
         not a second selection path"
    );
    // The reason must name the refusal it actually got, never a kind ban: dock-left's old
    // "resolves slots and vehicles only" became a lie the moment the router grew an arm.
    let reason = inert_row_reason().to_lowercase();
    assert!(
        reason.contains("not selectable") || reason.contains("resolves nothing"),
        "T-784: the inert reason must name the router's refusal, got {reason:?}"
    );
    for banned in ["slot", "vehicle", "comment"] {
        assert!(
            !reason.contains(banned),
            "T-784: the inert reason must not name a KIND ({banned:?}) — that sentence goes \
             stale the next time the router grows an arm"
        );
    }
}
