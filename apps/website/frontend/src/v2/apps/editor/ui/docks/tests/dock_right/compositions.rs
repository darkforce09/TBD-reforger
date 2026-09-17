use super::*;

/// T-650 (RIGHT-MODE-002) — the Compositions palette mode + tab exist and map to their own
/// sub-mode. The pure pins: the tab-index → sub-mode function reports Compositions for tab 4, and
/// tab 4 is NOT one of the pre-existing surfaces (so it did not silently reuse another tab's
/// slot).
#[test]
fn compositions_tab_maps_to_its_own_submode() {
    assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);
    // Objects mode on the Compositions tab does not turn it into Objects (that split is the
    // Factions tab's alone) — tab index wins.
    assert_eq!(EdenSubmode::from_tab(4, true), EdenSubmode::Compositions);
    // The four pre-existing surfaces keep their tabs.
    assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
    assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
    assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
    assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
}

/// T-650 — the Compositions palette is a LIVE surface (not a T-069-style stub) wired to the
/// editor-ops save/arm/edit seam. Source inspection, following `vehicles_tab_places_instead_of
/// _promising`, because `compositions_panel` is a wasm-only view a native test cannot mount.
///
/// **Every needle is assembled at run time** (the file's own hard-won rule): this test's source
/// is part of the haystack it searches, so a contiguous literal would make an absence check
/// unfailable and a presence check unpassable. Each needle is therefore split and re-joined.
#[test]
fn compositions_tab_is_wired_not_stubbed() {
    // T-791 — scrub comments out of both haystacks before matching. This file's own doc comments
    // now NAME every one of these tokens (the armed-state block, `begin_place_composition`,
    // `save_composition`), so a raw `include_str!` haystack would let a comment satisfy a
    // presence check — the T-759 hollow-pin class. `live_source` blanks comments and KEEPS string
    // literals, which the `"Compositions"` / `ops::<fn>(` needles below need.
    use crate::v2::core::test_support::class_r_scrub::live_source;
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    )));
    let src = src.as_str();
    // The library mutators are the engine's, aliased `engine_ops`; the arm is still the
    // frontend's, aliased `ops`. Both read `<alias>::<fn>(`.
    let call = |f: &str| format!("engine_ops::{f}(");
    let arm_call = |f: &str| format!("armed_placement::{f}(");

    // The tab strip renders a Compositions tab at index 4.
    assert!(
        src.contains(&format!("tab_btn(4, {:?})", "Compositions")),
        "a Compositions tab must be in the tab strip"
    );
    // The panel dispatch routes tab 4 to the compositions panel.
    assert!(
        src.contains(&format!("4 => {}(", "compositions_panel")),
        "tab 4 must dispatch to the compositions panel"
    );
    // SAVE (COMP-SAVE-001): the panel reaches the capture seam.
    assert!(
        src.contains(&call("save_composition")),
        "the Save affordance must call save_composition"
    );
    // PLACE (COMP-PLACE-001): a row press ARMS via the T-647 arm seam.
    assert!(
        src.contains(&arm_call("begin_place_composition")),
        "a composition row must arm the place"
    );
    // EDIT (COMP-EDIT-001) + the three ATTR-FIELD-COMP-* metadata fields, inline.
    for f in [
        "rename_composition",
        "recategorize_composition",
        "set_composition_author",
        "delete_composition",
    ] {
        assert!(
            src.contains(&call(f)),
            "the inline edit must call {f} (COMP-EDIT-001 / metadata fields)"
        );
    }

    // The seam actually exposes those functions AND the place reaches the core one-undo-step
    // mutator (the claim the store round-trip test rests on). Scrubbed the same way (T-791):
    // these `pub fn`s are documented in prose right above them. The save lives in the engine's
    // composition library, the arm + place in the entity operations.
    let ops = live_source(
        &[
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/editing/hosted_commands/composition_library.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/data/store/operations/compositions.rs"
            )),
            crate::v2::core::test_support::editor_operations::ENTITY,
            crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY,
        ]
        .concat(),
    );
    assert!(
        ops.contains("pub fn save_composition") && ops.contains("pub fn begin_place_composition"),
        "editor_ops must expose the composition save + arm"
    );
    assert!(
        ops.contains("core.place_composition("),
        "the composition place must reach the core mutator"
    );
}

/// **T-781** — a composition captures COMMENTS (the composable clause of `PLACE-COMMENT-001`)
/// and every placeable entry's AUTHORED ELEVATION, and the two ends of that seam agree.
///
/// Source inspection, and it has to be. `editor_ops::capture_selection_entities` is wasm-only,
/// so no native test in this crate can call it; the BEHAVIOUR it feeds is pinned natively in
/// `map-engine-core` (`a_composed_comment_is_placed_but_never_reaches_the_mod_document` and
/// `placing_a_composition_keeps_each_entrys_authored_elevation`). What neither of those can see
/// is whether the frontend capture ever EMITS the rows they consume, and whether both sides
/// spell the elevation key the same — a divergence there leaves the core tests green, the
/// frontend pin green, and every placed composition on the ground. That gap is this test.
///
/// The haystacks are `class_r_scrub`-scrubbed, so a needle satisfied by a code comment — this
/// ticket wrote several that name these very tokens — cannot make it pass, and the test modules
/// are cut so a needle cannot match the assertion hunting for it. Needles are assembled at run
/// time, this file's standing rule.
#[test]
fn a_composition_captures_comments_and_authored_elevation() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

    // `capture_selection_entities` lives in the engine's composition operations and `mint_ids`
    // in its entity operations; the haystack is their concatenation with the hosted library.
    let ops_all = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/composition_library.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/data/store/operations/compositions.rs"
        )),
        crate::v2::core::test_support::editor_operations::ENTITY,
    ]
    .concat();
    let ops_all: &str = &ops_all;
    // The elevation key is a CONTRACT string crossing a crate boundary: both ends below are
    // checked against this one value, so a rename that touches only one side is red.
    let elevation_key = format!("{:?}", "elevation");
    let comment_kind = format!("{:?}", "comment");
    let capture = format!("fn {}(", "capture_selection_entities");

    // ── The CAPTURE half (`website-frontend`, wasm-only) ─────────────────────────────────────
    let ops_text = live_source(ops_all);
    let ops_code = live_code(ops_all);
    let capture_text = only_body(&ops_text, &capture);
    let capture_code = only_body(&ops_code, &capture);
    assert!(
        capture_text.contains(&format!("{:?}", "commentsById")),
        "the capture must read the comments root, or a selected comment is dropped in \
         silence — the T-748 diagnosis; body was:\n{capture_text}"
    );
    assert!(
        capture_text.contains(&comment_kind),
        "the capture must emit a comment-kinded entry; body was:\n{capture_text}"
    );
    assert!(
        capture_text.contains(&elevation_key),
        "the capture must carry the authored elevation; body was:\n{capture_text}"
    );
    // Resolved through the SHARED reader, not a third z vocabulary, and not the f32 SoA (which
    // rounds the value and omits hidden-layer slots, T-665). `live_code` blanks literals, so
    // this needle means a CALL and cannot be satisfied by a mention.
    assert!(
        capture_code.contains(&format!("{}(", "slot_z")),
        "the elevation must be resolved through the shared slot_z reader; body was:\n{capture_code}"
    );
    // A composed comment is minted an id by `mint_ids`, so the comments root joins the
    // uniqueness union — otherwise a second placement can upsert an earlier note away.
    let domain = live_source(crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY);
    let mint = only_body(&domain, &format!("fn {}(", "mint_ids"));
    assert!(
        mint.contains(&format!("{:?}", "commentsById")),
        "mint_ids must prove uniqueness against the comments root too; body was:\n{mint}"
    );

    // ── The PLACE half (`map-engine-core`) — the same two keys, read back ────────────────────
    let store = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/rows/compositions.rs"
    )));
    let place = only_body(&store, &format!("fn {}(", "place_composition"));
    assert!(
        place.contains(&elevation_key),
        "place_composition must read back the SAME elevation key the capture writes"
    );
    assert!(
        place.contains(&comment_kind) && place.contains(&format!("{}(", "comment_row")),
        "the comment arm must stamp through the shared comment_row constructor, so a composed \
         note is the same shape as a hand-placed one"
    );
}

/// Wave 144 F-1 — **both id minters must prove uniqueness against a universe that includes
/// HIDDEN slots.**
///
/// `materialize()` is a VIEW: it drops slots on a hidden layer (T-665) and slots carrying
/// `editorHidden` (T-701). Sourcing the mint's `existing` set from it makes a hidden slot's id
/// look free — and with `next_id` resetting to 0 on every editor mount, an IDB-restored document
/// hands `n0` straight back to the next placement, whose insert is an UPSERT. The hidden row is
/// destroyed silently. This is the wave-127 rule for the id namespace: read `slots_json` (exact,
/// complete), never the SoA, which is lossy about hidden slots.
///
/// The consequence is fired for real — on a document with two hidden slots, both universes, and
/// the upsert — by `map-engine-core`'s
/// `a_hidden_slot_is_invisible_to_a_materialize_sourced_id_mint`. What that test cannot see is
/// which universe the SHIPPED minters read, because `editor_ops.rs` is
/// `#![cfg(target_arch = "wasm32")]` from line one and no native test in this crate can call
/// into it. That is this pin: it binds `mint_id`, `mint_ids` and their shared `live_slot_ids`
/// helper to `slots_json`, so reverting either minter to `materialize()` reddens here.
///
/// `live_code` blanks string literals, so every needle below means a CALL, not a mention, and
/// the doc comments this ticket wrote naming these very tokens cannot make it pass. Needles are
/// assembled at run time, this file's standing rule.
#[test]
fn both_id_minters_prove_uniqueness_against_hidden_slots_too() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

    let ops_code = live_code(crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY);
    let helper_call = format!("{}(", "live_slot_ids");
    let helper_body = only_body(&ops_code, &format!("fn {helper_call}"));

    // The shared helper reads the EXACT row map.
    assert!(
        helper_body.contains(&format!("{}()", "slots_json")),
        "live_slot_ids must read the exact slots_json row map; body was:\n{helper_body}"
    );

    // …and BOTH minters go through it, so there is one universe, not two.
    for name in ["mint_id", "mint_ids"] {
        let body = only_body(&ops_code, &format!("fn {name}("));
        assert!(
            body.contains(&helper_call),
            "{name} must source its slot half from live_slot_ids (slots_json), not the \
             materialized SoA — a hidden slot is absent from the SoA, so a mint proven against \
             it can re-mint and upsert away an id a hidden row already holds; body was:\n{body}"
        );
    }

    // Belt to the braces above: the mint's whole id-universe path — the two minters and the
    // helper they share, which is the entire closure that decides `existing` — must not reach
    // for the lossy view at all. Scoped deliberately narrowly (`materialize` is the right
    // reader nearly everywhere else in this file); the positive pins above are what actually
    // hold the fix down.
    for name in ["mint_id", "mint_ids", "live_slot_ids"] {
        let body = only_body(&ops_code, &format!("fn {name}("));
        assert!(
            !body.contains(&format!("{}()", "materialize")),
            "{name} must not build an id universe from the materialized SoA (T-665 / T-701 drop \
             hidden slots from it); body was:\n{body}"
        );
    }
}

/// T-650 / T-791 — the composition arm rides the SAME T-647 armed-state machine as the object
/// place: its arm is a `Pending::Composition`, `has_pending()` (which gates the map's place
/// branch and ghost) is true while one is armed, and the canvas release stamps the members as
/// ONE undo step and SELECTS the stamp. Also the T-791 feedback seam: the arm is READABLE
/// (`armed_composition_id`) and both arming (`arm`) and disarming (`cancel_pending`) nudge the
/// dock tick, so the panel's armed hint appears on arm and is GONE after Esc.
///
/// **T-791 — the haystacks are `class_r_scrub`-scrubbed.** This ticket's own doc comments now
/// name every token below (`arm(Pending::Composition(`, `place_composition`, `bump_doc_tick`,
/// `armed_composition_id`), so a raw `include_str!` would let this file's PROSE satisfy the pin —
/// the T-759 hollow-pin class two of these were caught as this week. `live_code` blanks comments
/// AND string/char literals, so every needle here means a real code token, never a mention. The
/// needles that carry no literal are assembled at run time so this test body cannot match itself.
#[test]
fn composition_arm_rides_the_shared_pending_machine() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    // T-934.7 — the Pending enum lives in operations/context.rs, the arm verbs and
    // place_at_impl in operations/entity.rs; the haystack is their concatenation.
    let ops = live_code(
        &[
            crate::v2::core::test_support::editor_operations::CONTEXT,
            crate::v2::core::test_support::editor_operations::ENTITY,
        ]
        .concat(),
    );
    // The arm variant exists and the public arm fn routes through the shared `arm(…)`.
    assert!(
        ops.contains("Composition(String)"),
        "the Pending enum must carry a Composition arm"
    );
    let arm_call = format!("arm(Pending::{}(", "Composition");
    assert!(
        ops.contains(&arm_call),
        "begin_place_composition must route through the shared arm() like begin_place_object"
    );
    // place_at_impl handles the Composition arm (the one-shot consume + place).
    assert!(
        ops.contains(&format!("Pending::{}(comp_id)", "Composition")),
        "place_at_impl must consume the Composition arm on a canvas release"
    );

    // ── T-791 — the acceptance wiring, pinned on the scrubbed bodies of the fns that own it ────
    // The consume: ONE core mutator (`place_composition` = one txn, one undo step — never a
    // per-member loop, the F-26 mistake), reached from the engine's release machine, AND the
    // host writes `ctx.selection` with what the stamp minted (select the stamp: OBJ +N,
    // SEL == N). Extracted from `place_at_impl` so a match elsewhere can't stand in.
    let consume = only_body(&ops, &format!("fn {}(", "place_at_impl"));
    let place_call = format!("place_saved_{}(", "composition");
    let domain = live_code(crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY);
    assert!(
        only_body(&domain, "pub fn place_saved_composition(").contains("core.place_composition(")
    );
    let release = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/entity/armed_placement.rs"
    )));
    assert!(
        only_body(&release, "pub fn commit_armed_placement(").contains(&place_call),
        "the composition consume must stamp via the single core place_composition (one undo \
         step)"
    );
    assert!(
        consume.contains(&format!("ctx.{}.borrow_mut()", "selection")),
        "the composition consume must select the stamped set; body was:\n{consume}"
    );

    // The armed state is READABLE for the panel hint, and it reads the Composition arm.
    let armed = only_body(&ops, &format!("fn armed_{}_id(", "composition"));
    assert!(
        armed.contains(&format!("Pending::{}(id)", "Composition")),
        "armed_composition_id must report the armed Composition id; body was:\n{armed}"
    );

    // Arm + disarm both nudge the dock tick — the appear/GONE halves of the hint. `cancel_pending`
    // is the single choke point for Esc / RMB / release-over-chrome (the sibling-owned
    // mission_editor Esc seam routes through it), so pinning the bump here covers the Esc case
    // without this slice touching that file.
    let bump = format!("bump_doc_{}()", "tick");
    let arm_body = only_body(&ops, "fn arm(");
    assert!(
        arm_body.contains(&bump),
        "arm() must bump the dock tick so the armed hint appears; body was:\n{arm_body}"
    );
    let cancel_body = only_body(&ops, &format!("fn cancel_{}()", "pending"));
    assert!(
        cancel_body.contains(&bump),
        "cancel_pending() must bump the dock tick so the armed hint is GONE after Esc; body \
         was:\n{cancel_body}"
    );

    // The panel actually RENDERS an armed hint gated on the live arm (not just static copy).
    // `live_code` here too: the block is documented in prose right above it, and it BLANKS the
    // `"composition"` literal this test uses to assemble the needle — so the scrubbed haystack
    // cannot contain this assertion's own text, only the real `armed_placement::armed_composition_id()` call
    // in the panel. Whole-file (not `only_body`): `compositions_panel` has a wasm def AND a
    // native stub, which `only_body` rejects as a shadow pair by design.
    let dock = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    )));
    let gated = format!("armed_{}_id()", "composition");
    assert!(
        dock.contains(&gated),
        "the compositions panel must render a hint gated on the live arm (armed_composition_id)"
    );
}
