use super::selectable_ids;

/// Two slots, one of them carrying T-701 `editorHidden` — the row `materialize()` drops and the
/// raw key map keeps.
fn slots() -> String {
    serde_json::json!({
        "n1": { "x": 1.0, "y": 2.0 },
        "n2": { "x": 3.0, "y": 4.0, "editorHidden": true },
    })
    .to_string()
}

/// One of every other by-id map a selection could be pruned against — including the two
/// (`zonesById`, `markersById`) that must NOT be admitted.
fn small() -> String {
    serde_json::json!({
        "vehiclesById": { "n3": { "position": { "x": 5.0, "y": 6.0 } } },
        "entitiesById": { "n4": { "position": { "x": 7.0, "y": 8.0 } } },
        "commentsById": { "cmt-1": { "title": "North", "position": { "x": 9.0, "z": 10.0 } } },
        "zonesById": { "z1": { "shape": { "circle": { "x": 0.0, "z": 0.0, "r": 3.0 } } } },
        "markersById": { "m1": { "position": { "x": 0.0, "z": 0.0 } } },
    })
    .to_string()
}

/// The prune exactly as `mission_history::prune_selection` performs it: `retain` over
/// [`selectable_ids`] of the POST-change document. Order-preserving, like the real `Vec`.
fn prune(sel: &[&str], slots_json: &str, small_maps_json: &str) -> Vec<String> {
    let live = selectable_ids(slots_json, small_maps_json);
    sel.iter()
        .filter(|id| live.contains(**id))
        .map(|id| (*id).to_string())
        .collect()
}

/// **DIRECTION ONE — what EXISTS survives.** The regression in one line: with the universe
/// sourced from the slot SoA this returned `["n1"]`, and the operator's comment, vehicle and
/// object left the selection on the next drag commit with nothing said.
#[test]
fn every_live_selectable_kind_survives_a_document_change() {
    assert_eq!(
        prune(&["n1", "n2", "n3", "n4", "cmt-1"], &slots(), &small()),
        vec!["n1", "n2", "n3", "n4", "cmt-1"],
        "wave 145 F-1: a slot, a HIDDEN slot, a vehicle, a placed object and a comment all \
         exist in this document — a prune that drops any of them is deleting the operator's \
         selection, not pruning it"
    );
}

/// **DIRECTION TWO — what is GONE still falls out.** This is the property the over-aggressive
/// prune was accidentally providing and that the widening must not spend: the universe is read
/// from the POST-change document, so a note removed by Delete / the panel / an undo is already
/// out of `commentsById` when this runs. Without it, a stale id reaches Delete — the wave-129
/// and wave-142 defect (a success report over an unchanged document; a stale id coexisting with
/// an edge selection so Delete removed the wrong object).
#[test]
fn an_id_the_document_no_longer_holds_still_falls_out() {
    // The post-change document: the comment and the placed object are gone, the vehicle stays.
    let after = serde_json::json!({
        "vehiclesById": { "n3": { "position": { "x": 5.0, "y": 6.0 } } },
        "entitiesById": {},
        "commentsById": {},
    })
    .to_string();
    assert_eq!(
        prune(
            &["n1", "n3", "n4", "cmt-1", "never-minted"],
            &slots(),
            &after
        ),
        vec!["n1", "n3"],
        "wave 145 F-1: a removed comment, a removed object and an id the document never held \
         must all leave the selection — the prune's whole reason for existing"
    );
    // The slot half is pruned by the same rule: a slot removed from `slots_json` is gone.
    assert_eq!(
        prune(&["n1", "n2"], "{\"n1\":{}}", &after),
        vec!["n1"],
        "wave 145 F-1: a deleted slot must still fall out — widening the universe is not \
         licence to keep a row the document dropped"
    );
}

/// **A HIDDEN slot is in the universe, because it still EXISTS.** `materialize()` drops slots on
/// a hidden layer (T-665) and slots carrying `editorHidden` (T-701) — that is a VIEW, and wave
/// 144 already established (`eden_dock_right::both_id_minters_prove_uniqueness_against_hidden_
/// slots_too`) that an id universe must not be built from it. Here the consequence of getting it
/// wrong is the other way round from the minters': an SoA-sourced prune deselects a slot for
/// being invisible, which makes `engine_ops::toggle_hidden` unable to toggle back and
/// `show_selection` unreachable — the hide runs `after_local_edit`, the prune removes the rows
/// it just hid, and the selection the Show verb needs is gone.
#[test]
fn a_hidden_slot_is_in_the_universe_because_hidden_is_not_gone() {
    let live = selectable_ids(&slots(), &small());
    assert!(
        live.contains("n2"),
        "wave 145 F-1: the slot half must come off the raw slots_json key map, hidden rows \
         included — hidden is a view state, and the prune asks about existence"
    );
}

/// **A zone and a marker are NOT in the universe.** A zone is selected in the Zones panel via
/// `eden_dock_right::route_select_zone` and never lands in `ctx.selection` (the router's `Zone`
/// arm is written around exactly that); a marker has no selection route at all. Admitting either
/// would widen the universe past the set being pruned, which costs the prune the only thing it
/// can say: "this id is gone".
#[test]
fn zones_and_markers_are_not_selectable_and_so_are_not_in_the_universe() {
    let live = selectable_ids(&slots(), &small());
    for id in ["z1", "m1"] {
        assert!(
            !live.contains(id),
            "wave 145 F-1: {id} has no route into the editor selection, so it must not widen \
             the universe the selection is pruned against"
        );
    }
}

/// Unparseable or absent maps yield an EMPTY universe rather than a panic — the prune runs on
/// every committed edit and on the IDB restore swap, and the restore is precisely where a
/// half-written document can appear. An empty universe prunes everything, which is the safe
/// direction (no stale id survives) and is what the SoA-sourced code did on the same input.
#[test]
fn a_document_that_does_not_parse_yields_an_empty_universe() {
    assert!(selectable_ids("not json", "not json").is_empty());
    assert!(selectable_ids("{}", "{}").is_empty());
    assert!(selectable_ids("[]", "null").is_empty());
}
