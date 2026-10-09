//! The outliner row's affordance and its click share one answer: for every row kind, a row paints
//! as routable exactly when the registered router's click would select its subject.

use super::{row_router_subject, row_routes};
use crate::ui::inspector::validation_panel::{
    register_route_probe, register_select_by_id, route_select_by_subject_id,
};
use mission_creator_state::outliner_model::NodeKind;

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
            let click = row_router_subject(kind, id)
                .is_some_and(|subject| route_select_by_subject_id(&subject.into()));
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
