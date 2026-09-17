/// T-939.2 — batch faction / squad reassign: a faction selector and an editable squad picker that
/// act on the whole selection in one undo group.
///
/// Pinned two ways, on purpose:
///
///   * the **decision** (the engine's `plan_reassign`, [`super::faction_label`]) is pure and
///     native, so
///     these tests CALL it — the `axis_chip_class` / `nudge_step` precedent. The refusal reasons are
///     user-visible copy, and a sentence only a source pin ever reads is a sentence nobody has
///     proved the modal can produce;
///   * the **wiring** is a `view!` tree over `web_sys` nodes that `cargo test` cannot instantiate,
///     so it is pinned against the SCRUBBED live source of the two files that carry it — this one
///     and the engine's `editing/hosted_commands/squad_reassignment.rs`.
///
/// The doc-level behaviour — the emptied source squad keeping its row, its attached vehicles and
/// its place in `faction.squadIds`, and the derived side key following the move — is native and
/// lives with the primitive it exercises, in `store.rs`'s own test module
/// (`move_slot_to_squad_keep_source_keeps_the_emptied_squad_its_vehicles_and_its_position`,
/// `keep_source_move_carries_the_derived_side_key_across_factions`, and the additive-proof
/// `the_default_move_slot_to_squad_still_garbage_collects_an_emptied_source`).
use super::faction_label;
use crate::v2::apps::editor::ui::outliner::outliner::{FactionRow, SquadRow};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
use website_map_engine::data::store::operations::reassign::plan_reassign;

const REASSIGN_RS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../map-engine/src/editing/hosted_commands/squad_reassignment.rs"
));

/// Two factions, three squads: Alpha and Charlie under BLUFOR, Bravo under OPFOR. Bravo is the
/// cross-faction pick; `faction-EMPTY` is the faction with nowhere to put anyone.
fn rows() -> (Vec<FactionRow>, Vec<SquadRow>) {
    let squad = |id: &str, name: &str, faction: &str| SquadRow {
        id: id.to_string(),
        name: name.to_string(),
        faction_id: faction.to_string(),
        slot_ids: vec!["s1".to_string()],
        leader_slot_id: String::new(),
        vehicle_ids: Vec::new(),
    };
    (
        vec![
            FactionRow {
                id: "faction-BLUFOR".to_string(),
                key: "BLUFOR".to_string(),
                name: "US Army".to_string(),
                squad_ids: vec!["sq-a".to_string(), "sq-c".to_string()],
            },
            FactionRow {
                id: "faction-OPFOR".to_string(),
                key: "OPFOR".to_string(),
                name: "Soviet Army".to_string(),
                squad_ids: vec!["sq-b".to_string()],
            },
            FactionRow {
                id: "faction-EMPTY".to_string(),
                key: "INDFOR".to_string(),
                name: "Militia".to_string(),
                squad_ids: Vec::new(),
            },
        ],
        vec![
            squad("sq-a", "Alpha", "faction-BLUFOR"),
            squad("sq-b", "Bravo", "faction-OPFOR"),
            squad("sq-c", "Charlie", "faction-BLUFOR"),
        ],
    )
}

/* ─────────────────────────── the decision, called ─────────────────────────── */

/// REQUIREMENT 4 — a squad of another faction is refused with a NAMED reason. Both faction
/// names and the squad name must be in the sentence: an operator looking at a dialog that says
/// "US Army" needs to be told the squad they picked is the Soviets', not merely that something
/// was wrong.
#[test]
fn a_squad_of_another_faction_is_refused_and_the_reason_names_squad_and_both_factions() {
    let (factions, squads) = rows();
    let why = plan_reassign(&factions, &squads, "faction-BLUFOR", "sq-b")
        .expect_err("a squad under OPFOR must be refused for a BLUFOR pick");
    for needle in ["Bravo", "Soviet Army", "OPFOR", "US Army", "BLUFOR"] {
        assert!(
            why.contains(needle),
            "T-939.2: the refusal must name {needle}; got: {why}"
        );
    }
    // And it must be a refusal, not a silent redirect: no destination comes back.
    assert!(plan_reassign(&factions, &squads, "faction-BLUFOR", "sq-b").is_err());
}

/// The same-faction pick is NOT refused — the guard above must be about the faction, not about
/// naming a squad at all.
#[test]
fn a_squad_of_the_picked_faction_resolves_to_itself() {
    let (factions, squads) = rows();
    assert_eq!(
        plan_reassign(&factions, &squads, "faction-BLUFOR", "sq-c"),
        Ok("sq-c".to_string())
    );
}

/// The FACTION selector's own commit: an empty squad id means "this faction, its first squad",
/// in `faction.squadIds` order — so choosing a faction moves the whole selection under it
/// without a second gesture, and lands where the Outliner shows that faction's first squad.
#[test]
fn picking_a_faction_alone_resolves_to_that_factions_first_squad_in_doc_order() {
    let (mut factions, squads) = rows();
    assert_eq!(
        plan_reassign(&factions, &squads, "faction-BLUFOR", ""),
        Ok("sq-a".to_string())
    );
    // Doc ORDER, not id order and not insertion order into `squads`: reversing the faction's
    // own `squadIds` must change the answer.
    factions[0].squad_ids = vec!["sq-c".to_string(), "sq-a".to_string()];
    assert_eq!(
        plan_reassign(&factions, &squads, "faction-BLUFOR", ""),
        Ok("sq-c".to_string())
    );
    // A dangling id in `squadIds` is skipped rather than returned as a destination.
    factions[0].squad_ids = vec!["sq-deleted".to_string(), "sq-a".to_string()];
    assert_eq!(
        plan_reassign(&factions, &squads, "faction-BLUFOR", ""),
        Ok("sq-a".to_string())
    );
}

/// A faction with no squads is a reachable pick (the selector lists every faction), so it must
/// refuse by name and say what to do — not move the selection to some other faction's squad.
#[test]
fn a_faction_with_no_squads_refuses_by_name_and_says_what_to_do() {
    let (factions, squads) = rows();
    let why = plan_reassign(&factions, &squads, "faction-EMPTY", "")
        .expect_err("a faction with no squads has no destination");
    assert!(
        why.contains("Militia") && why.contains("INDFOR"),
        "T-939.2: the refusal must name the faction; got: {why}"
    );
    assert!(
        why.to_lowercase().contains("orbat"),
        "T-939.2: the refusal must point at where squads are made; got: {why}"
    );
}

/// A destination that vanished under the open modal (undo, a peer, the Outliner) refuses rather
/// than moving the selection somewhere nobody chose.
#[test]
fn a_destination_that_no_longer_exists_refuses_rather_than_guessing() {
    let (factions, squads) = rows();
    assert!(plan_reassign(&factions, &squads, "faction-GONE", "").is_err());
    assert!(plan_reassign(&factions, &squads, "faction-BLUFOR", "sq-gone").is_err());
}

/// `faction_label` names both halves when both exist, and never renders empty: a faction row
/// missing its name still says something the operator can pick.
#[test]
fn faction_label_names_the_faction_and_its_side_key() {
    let f = |id: &str, key: &str, name: &str| FactionRow {
        id: id.to_string(),
        key: key.to_string(),
        name: name.to_string(),
        squad_ids: Vec::new(),
    };
    assert_eq!(
        faction_label(&f("f1", "BLUFOR", "US Army")),
        "US Army (BLUFOR)"
    );
    assert_eq!(faction_label(&f("f1", "BLUFOR", "")), "BLUFOR");
    assert_eq!(faction_label(&f("f1", "", "US Army")), "US Army");
    assert_eq!(faction_label(&f("f1", "BLUFOR", "BLUFOR")), "BLUFOR");
    assert_eq!(faction_label(&f("f1", "", "")), "f1");
}

/* ─────────────────────────── the wiring, pinned ─────────────────────────── */

/// THE DEFECT (RED before this slice): the Identity tab offered NO faction control and the
/// Squad entry was an inert read-only div, so a faction/squad move was unreachable from this
/// modal at any selection size.
///
/// Pinned on `identity_tab`'s own body rather than the whole file because every other faction
/// mention in `attributes_modal.rs` belongs to `type_picker` (it edits `assetId`, not faction)
/// and would green this test on code that cannot move a single slot.
#[test]
fn the_identity_tab_offers_a_faction_control_and_an_editable_squad_control() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "fn identity_tab(");
    assert!(
            body.contains("reassign_picker("),
            "T-939.2: the Identity tab must render the faction/squad reassign controls; body was:\n{body}"
        );
    // The read-only div was the defect. Its exact shape must not be what renders the squad.
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body_src = only_body(&src, "fn identity_tab(");
    assert!(
        !body_src.contains("{} entities"),
        "T-939.2: the squad entry must no longer be the inert '{{n}} entities' text"
    );
    // Both controls are real form controls with accessible names, not styled divs.
    let picker = only_body(&src, "fn reassign_picker(");
    assert!(
        picker.contains("aria-label=\"Faction\"") && picker.contains("aria-label=\"Squad\""),
        "T-939.2: both controls must be labelled selects"
    );
}

/// The controls act on the WHOLE selection and land as ONE undo group: the picker commits
/// through `reassign_slots` over `targets` (the multi-edit id set that `attrs_multi_ids`
/// built), and `reassign_slots` brackets its per-slot core transactions in `with_batch`.
#[test]
fn the_picker_commits_the_whole_selection_in_one_undo_group() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let picker = only_body(&code, "fn reassign_picker(");
    assert!(
            picker.contains("reassign_slots(&targets.get_value()"),
            "T-939.2: the picker must commit over the whole target set, not the open slot; body was:\n{picker}"
        );
    let ops = live_code(REASSIGN_RS);
    let apply = only_body(&ops, "pub fn reassign_slots(");
    assert!(
        apply.contains("with_batch("),
        "T-939.2: reassign_slots must bracket its moves in with_batch so one Ctrl+Z reverts the \
             whole batch; body was:\n{apply}"
    );
}

/// REQUIREMENT 3 — slots leaving a squad never delete it. The batch must route through the
/// ADDITIVE keep-source core entry point and must not be able to reach the default
/// `move_slot_to_squad`, whose emptied-source branch takes the row, its place in
/// `faction.squadIds`, and every vehicle attached to it.
#[test]
fn the_batch_uses_the_keep_source_core_path_not_the_garbage_collecting_one() {
    let ops = live_code(concat!(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/squad_reassignment.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/data/store/operations/reassign.rs"
        ))
    ));
    assert!(
        ops.contains("move_slot_to_squad_keep_source("),
        "T-939.2: the batch must move through move_slot_to_squad_keep_source"
    );
    // `move_slot_to_squad_keep_source(` does not contain `move_slot_to_squad(` — the next
    // character is `_`, not `(` — so this is an exact check for the GC-ing call, not a
    // prefix collision.
    assert!(
        !ops.contains("move_slot_to_squad("),
        "T-939.2: the GC-ing move_slot_to_squad must not be reachable from the batch"
    );
    // Nor may it launder the same call through the existing frontend wrapper.
    assert!(
        !ops.contains("refile_slot("),
        "T-939.2: refile_slot wraps the GC-ing core path; the batch must not use it"
    );
}

/// A multi-selection must never be shown the first slot's squad as if it were the group's —
/// the same honesty rule the read-only field already followed, kept through the change.
#[test]
fn a_mixed_selection_reads_mixed_rather_than_the_first_slots_squad() {
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let picker = only_body(&src, "fn reassign_picker(");
    assert_eq!(
        picker.matches("Mixed").count(),
        2,
        "T-939.2: both selects must have a Mixed placeholder; body was:\n{picker}"
    );
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let picker_code = only_body(&code, "fn reassign_picker(");
    // The "all targets agree" test is what makes Mixed truthful: a `first()` with no
    // all-equal check is exactly the defect.
    assert!(
        picker_code.contains("current_squads.iter().all(")
            && picker_code.contains("current_factions.iter().all("),
        "T-939.2: the displayed value must be computed over the whole target set"
    );
}

/// The named reason is rendered where the pick was made. `live_source` (literals kept) because
/// this is about the copy actually reaching the DOM.
#[test]
fn the_refusal_reason_is_rendered_in_the_modal() {
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let picker = only_body(&src, "fn reassign_picker(");
    assert!(
        picker.contains("role=\"alert\""),
        "T-939.2: the refusal must be announced, not printed to the console"
    );
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let picker_code = only_body(&code, "fn reassign_picker(");
    assert!(
        picker_code.contains("refusal.set(reason)") && picker_code.contains("refusal.get()"),
        "T-939.2: the Err arm's reason must be the text the modal shows"
    );
}

/// Wiring proof only; the doc fixture and browser Revert exercise the resulting moves.
#[test]
fn revert_restores_the_open_snapshot_squads_in_one_keep_source_group() {
    let modal = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let revert = only_body(&modal, "fn revert_to_snapshot(");
    assert!(
        revert.contains("restore_slot_squads(&snapshot.get_value())"),
        "T-939.2: Revert must restore each slot's original squad from the OPEN snapshot"
    );
    let ops = live_code(REASSIGN_RS);
    let domain = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/reassign.rs"
    )));
    let restore = [
        only_body(&ops, "pub fn restore_slot_squads("),
        only_body(&domain, "pub fn restore_moves("),
        only_body(&domain, "pub fn restore_slot_squads("),
    ]
    .concat();
    for required in [
        "snap.id",
        "snap.squad",
        "with_batch(",
        "move_slot_to_squad_keep_source(",
    ] {
        assert!(
            restore.contains(required),
            "T-939.2: membership Revert must use {required}; body was:\n{restore}"
        );
    }
    assert!(
        !restore.contains("reassign_slots("),
        "Revert has a destination per slot, not one faction/squad for the whole selection"
    );
}

#[test]
fn reassign_and_revert_read_raw_membership_for_hidden_single_slot_attributes() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/reassign.rs"
    )));
    for entry in ["pub fn reassign_slots(", "pub fn restore_moves("] {
        let body = only_body(&ops, entry);
        assert!(
            body.contains("core.slot_squad_id("),
            "T-939.2: {entry} must read raw membership so hidden slots remain editable"
        );
    }
    assert!(
        !ops.contains(".materialize()"),
        "Reassignment must not mistake a render-filtered slot for a missing slot"
    );
}
