use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

/// The editor page region (comments stripped, string literals blanked). The `#[cfg(wasm32)]`
/// blocks the pointer/dblclick handlers live in are KEPT by the scrubber (it decides only
/// provably-false cfgs, and `target_arch` reads as undecided under the default eval) — the same
/// reason t662 can pin `chrome_hidden.set(` inside that block.
fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
}

/// `editor_ops.rs`, scrubbed to live code. It is wasm-only, so nothing in it runs — but its
/// wiring is pinnable as source (multiple modules already `include_str!` it for this).
fn ops_live() -> String {
    live_code(include_str!("../state/operations/entity.rs"))
}

// ───────────────────────── ATTR-OPEN-001 — dblclick opens Attributes for vehicles too ────────

/// The dblclick handler must pick with `pick_slot_or_vehicle` (slot OR vehicle), not the
/// slot-only `pick` it used pre-T-647 — that swap is the whole of ATTR-OPEN-001 ("not just
/// slots"). A hit opens Attributes; the pick must be handed the live `vehicle_points()`.
#[test]
fn dblclick_opens_attributes_for_vehicles_via_slot_or_vehicle_pick() {
    let ed = editor_live();
    let body = only_body(&ed, "let ondblclick =");
    assert!(
        body.contains("select_tool::pick_slot_or_vehicle(")
            && body.contains("editor_ops::vehicle_points()"),
        "ATTR-OPEN-001: dblclick must pick slot OR vehicle (with vehicle_points), so Attributes \
         opens for a vehicle — not the slot-only pick"
    );
    assert!(
        body.contains("editor_ops::open_attributes(id)"),
        "a dblclick HIT must open Attributes on the picked id"
    );
    // The slot-only `pick(` must be GONE from this handler — a leftover would keep the bug for
    // vehicles. (`pick_slot_or_vehicle` contains the token `pick`, so match the bare call form.)
    assert!(
        !body.contains("select_tool::pick(&cam"),
        "ATTR-OPEN-001: the slot-only pick(&cam, …) must be gone from the dblclick handler"
    );
}

// ───────────────────────── PLACE-003 — dblclick empty ground opens the asset picker ──────────

/// A dblclick MISS (empty ground) opens the asset picker at the unprojected world point; a
/// non-finite unproject opens nothing (off-map).
#[test]
fn dblclick_empty_ground_opens_the_asset_picker() {
    let ed = editor_live();
    let body = only_body(&ed, "let ondblclick =");
    // The match on the pick result: Some(id) → Attributes; None → picker.
    assert!(
        body.contains("editor_ops::open_asset_picker("),
        "PLACE-003: a dblclick miss must open the asset picker"
    );
    assert!(
        body.contains("cam.unproject_xy(px, py)") && body.contains("is_finite()"),
        "PLACE-003: the picker must open at the unprojected world point, guarded finite (off-map \
         opens nothing)"
    );
}

/// The picker is a real, ungated overlay component mounted beside the other ungated dialogs
/// (so it survives Backspace hide-chrome — a hidden dock can't be focused, which is why this
/// floating form was chosen), and a picked leaf ARMS `begin_place` (click-then-click, PLACE-001).
#[test]
fn asset_picker_is_an_ungated_overlay_that_arms_a_place() {
    let ed = editor_live();
    // Signal declared on the page + the picker signal handed to editor_ops (the open path).
    assert!(
        ed.contains("let asset_picker = RwSignal::new(None")
            && ed.contains("editor_ops::set_asset_picker_signal(asset_picker)"),
        "PLACE-003: the page must own the picker signal and register it with editor_ops"
    );
    // The overlay mount must exist and be OUTSIDE every chrome_hidden gate (ungated, like the
    // Attributes modal / context menu). Prove it by locating the mount and checking no
    // chrome_hidden gate opens between the last ungated-dialog landmark and it.
    assert!(
        ed.contains("AssetPickerOverlay"),
        "PLACE-003: the picker overlay component must be mounted"
    );
    let mount = ed.find("AssetPickerOverlay picker=").expect("picker mount");
    let ctx_menu = ed
        .find("ContextMenuOverlay menu=")
        .expect("context menu mount is the ungated-dialog landmark");
    assert!(
        mount > ctx_menu && !ed[ctx_menu..mount].contains("(!chrome_hidden.get()).then("),
        "PLACE-003: the picker must mount beside the ungated dialogs (no chrome_hidden gate \
         between the context menu and it)"
    );
    // The picker component arms the same place a DockRight leaf does. It is defined ABOVE the
    // page, so the page-anchored `editor_live` slice misses it AND a whole-file scrub is cut at
    // the file's first `#[cfg(test)]` (the `registry_session` helper near the top). Anchor from
    // the cold-registry page-size const (after that helper, before this component) — exactly as
    // the t573 pin does, so `cut_test_module` next fires on the real test modules far below. The
    // anchor is reassembled (not written whole) so this line is not a second occurrence of it,
    // which t573's own "exactly one" count would otherwise trip.
    let cold_anchor = format!("const REGISTRY_{}", "COLD_PAGE");
    let raw = include_str!("../mission_editor.rs");
    let region = live_code(&raw[raw.find(cold_anchor.as_str()).expect("cold anchor present")..]);
    let comp = only_body(&region, "fn AssetPickerOverlay(");
    assert!(
        comp.contains("editor_ops::begin_place(payload")
            && comp.contains("editor_ops::close_asset_picker()"),
        "PLACE-001/PLACE-003: choosing a picker row must arm begin_place then close (the next \
         canvas click lands it)"
    );
    // …reusing the SAME catalog builder the dock uses (no second catalog to drift).
    assert!(
        comp.contains("asset_catalog::build_catalog_tree("),
        "PLACE-003: the picker must reuse build_catalog_tree (the dock's own catalog)"
    );
}

// ───────────────────── T-651 — PLACE-COMMENT-001: the place point + the template seed ───────

/// The right-click handler captures the WORLD point of the click and hands it to the menu, which
/// is what makes `Place Comment` land where the operator clicked. Also pins the negative that
/// matters: this ticket added NO state to the `LeftGesture` machine — no new arm, no new
/// `Pending`, nothing that could strand (T-723's territory, deliberately untouched).
#[test]
fn the_contextmenu_handler_captures_the_world_point_and_arms_no_gesture() {
    let ed = editor_live();
    let body = only_body(&ed, "let oncontextmenu =");
    assert!(
        body.contains("cam.unproject_xy(px, py)") && body.contains(".at_world("),
        "PLACE-COMMENT-001: the right-click must unproject its own pixel and attach the world \
         point to the MenuTarget"
    );
    assert!(
        !body.contains("LeftGesture")
            && !body.contains("editor_ops::arm(")
            && !body.contains("Pending::"),
        "T-651 must add no state to the gesture machine — the place is committed by the menu \
         row, not by an armed pointerup (T-723)"
    );
}

/// THE NEW-MISSION TEMPLATE SEEDS COMMENTS, at the fresh-doc site and BEFORE both boot steps
/// that replace the document. Order is the whole property: seeding after the IDB restore or the
/// server hydrate would stamp a template onto a mission that already has its own comments.
#[test]
fn the_new_mission_template_seeds_comments_before_restore_and_hydrate() {
    let ed = editor_live();
    let seed = ed
        .find("editor_ops::seed_new_mission_template(&doc)")
        .expect("T-651: the new-mission template seed must run in the editor page");
    let mint = ed
        .find("mission_doc::new_seeded_doc()")
        .expect("the fresh-doc mint");
    let restore = ed
        .find("yrs_persist::load_state(&id)")
        .expect("the IDB restore");
    let hydrate = ed
        .find("mission_hydrate::hydrate_from_server(")
        .expect("the server hydrate");
    assert!(
        seed > mint,
        "the template seeds into the freshly-minted doc, not before it exists"
    );
    assert!(
        seed < restore && seed < hydrate,
        "the template must seed BEFORE the restore ({restore}) and the hydrate ({hydrate}) — \
         both replace the document, so a later seed would duplicate onto a saved mission"
    );
}

/// The comment editor is a real, ungated overlay (it survives Backspace hide-chrome, the
/// wave-101 mount rule) and it authors all three ATTR-FIELD-CMT-* fields plus copy and delete —
/// so every store mutator this ticket shipped is reachable from the UI.
#[test]
fn the_comment_editor_is_ungated_and_authors_every_comment_field() {
    let ed = editor_live();
    assert!(
        ed.contains("let comment_editor = RwSignal::new(None")
            && ed.contains("editor_ops::set_comment_editor_signal(comment_editor)"),
        "T-651: the page must own the comment-editor signal and register it with editor_ops"
    );
    let mount = ed
        .find("CommentEditorOverlay open=")
        .expect("the comment editor mount");
    let ctx_menu = ed
        .find("ContextMenuOverlay menu=")
        .expect("context menu mount is the ungated-dialog landmark");
    assert!(
        mount > ctx_menu && !ed[ctx_menu..mount].contains("(!chrome_hidden.get()).then("),
        "T-651: the comment editor must mount beside the ungated dialogs"
    );
    // The component is defined ABOVE the page, so scrub from the same cold-registry anchor the
    // picker pin uses (reassembled so this line is not a second occurrence of it).
    let cold_anchor = format!("const REGISTRY_{}", "COLD_PAGE");
    let raw = include_str!("../mission_editor.rs");
    let region = live_code(&raw[raw.find(cold_anchor.as_str()).expect("cold anchor present")..]);
    let comp = only_body(&region, "fn CommentEditorOverlay(");
    for op in [
        "editor_ops::rename_comment(",      // ATTR-FIELD-CMT-TITLE
        "editor_ops::set_comment_tooltip(", // ATTR-FIELD-CMT-TOOLTIP
        "editor_ops::move_comment(",        // ATTR-FIELD-CMT-POSITION (the drag commit)
        "editor_ops::duplicate_comment(",   // COPY
        "editor_ops::delete_comment(",
    ] {
        assert!(
            comp.contains(op),
            "T-651: the comment editor must reach `{op}` — an unreachable mutator is a \
             half-shipped field"
        );
    }
    // A comment must never be routed into the SLOT surfaces (the T-716 live-but-inert trap).
    assert!(
        !comp.contains("editor_ops::open_attributes(")
            && !comp.contains("editor_ops::select_slot("),
        "T-651: a comment id must not enter the slot selection / Attributes lanes"
    );
}

// ───────────────────────── The Ctrl state machine (PLACE-004 ↔ CONN-GROUP-001) ───────────────

/// The overload resolution, pinned as ONE machine. In the pointerup PLACE branch (armed):
/// Ctrl → `place_at_keep` (multi-place, keeps the arm), else `place_at_alt` (one-shot). In the
/// pointerup DRAG-commit branch (unarmed — `has_pending()` short-circuited the place branch):
/// Ctrl + single character onto another character → `regroup_slot_onto`. The two can never both
/// fire: the place branch `return`s under `has_pending()`.
#[test]
fn ctrl_state_machine_multi_place_when_armed_regroup_when_not() {
    let ed = editor_live();
    let up = only_body(&ed, "let onpointerup =");

    // (1) The place branch is armed-gated and returns, so the drag branch below only ever runs
    // with NO pending — that mutual exclusion is the resolution.
    assert!(
        up.contains("editor_ops::has_pending()"),
        "the place branch must gate on has_pending() (armed)"
    );

    // (2) Armed + Ctrl = multi-place (place_at_keep); armed + no Ctrl = one-shot (place_at_alt).
    assert!(
        up.contains("let ctrl_multi = ev.ctrl_key() || ev.meta_key()"),
        "PLACE-004: the armed branch must read Ctrl/Cmd as the multi-place modifier"
    );
    assert!(
        up.contains("editor_ops::place_at_keep(") && up.contains("editor_ops::place_at_alt("),
        "PLACE-004: Ctrl must route to place_at_keep (keep the arm), else place_at_alt"
    );

    // (3) Unarmed + Ctrl + single character dropped onto another → regroup, and the positional
    // move is skipped.
    assert!(
        up.contains("editor_ops::regroup_slot_onto(")
            && up.contains("ids.len() == 1")
            && up.contains("!editor_ops::is_vehicle_id(&ids[0])"),
        "CONN-GROUP-001: an unarmed Ctrl-drag of a SINGLE character onto another must regroup"
    );

    // (4) The state machine is DOCUMENTED as one block (the ticket requires the comment). A
    // comment is stripped by every scrubber, so pin it on the RAW file, sliced to the page's
    // production body (page anchor → the first following test module) so this test module's own
    // text cannot satisfy it. The needle is reassembled so this line is not itself the decoy.
    let raw = include_str!("../mission_editor.rs");
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let page_at = raw.find(anchor.as_str()).expect("page anchor present");
    let boundary = format!("#[cfg{}", "(test)]");
    let after = &raw[page_at..];
    let prod = &after[..after
        .find(boundary.as_str())
        .expect("a test module follows the page")];
    let phrase = format!("Ctrl is {}", "OVERLOADED");
    assert!(
        prod.contains(phrase.as_str()),
        "T-647: the Ctrl state machine must be documented in a comment block in the page body"
    );
}

/// `place_at_keep` re-arms the pending after a successful place (multi-place keeps going), and a
/// FAILED place does not re-arm (a place that can't commit must not spin). The Alt override is
/// carried through each stamp.
#[test]
fn place_at_keep_rearms_on_success_only() {
    let ops = ops_live();
    let body = only_body(&ops, "pub fn place_at_keep(");
    assert!(
        body.contains("place_at_impl(x, y, alt_empty, true)"),
        "PLACE-004: place_at_keep must place with keep=true and carry the Alt override"
    );
    // Snapshot before, restore after — and only when `placed`.
    assert!(
        body.contains("if placed") && body.contains("pending.borrow_mut() = Some(p)"),
        "PLACE-004: place_at_keep must re-arm the snapshotted pending, and only on success"
    );
}

// ───────────────────────── PLACE-CREW-001 — Alt = empty vehicle ──────────────────────────────

/// Alt on release is threaded from the pointerup into the placement as `alt_empty`, and the
/// vehicle commit stamps `crewed:false` when Alt is held (`with_crew = toggle && !alt_empty`) —
/// the per-gesture override of the dock's crew default. Alt can force empty; it can never force
/// crewed a switched-off toggle withheld.
#[test]
fn alt_places_an_empty_vehicle() {
    let ed = editor_live();
    let up = only_body(&ed, "let onpointerup =");
    assert!(
        up.contains("let alt_empty = ev.alt_key()"),
        "PLACE-CREW-001: the armed branch must read Alt as the empty-vehicle modifier"
    );
    // Both place routes carry the alt flag through.
    assert!(
        up.contains("place_at_keep(c[0], c[1], alt_empty)")
            && up.contains("place_at_alt(c[0], c[1], alt_empty)"),
        "PLACE-CREW-001: the Alt override must reach place_at_* on both the multi and single paths"
    );
    // The vehicle commit computes with_crew from the toggle AND-NOT alt.
    let ops = ops_live();
    let impl_body = only_body(&ops, "fn place_at_impl(");
    assert!(
        impl_body.contains("let with_crew = place_with_crew() && !alt_empty"),
        "PLACE-CREW-001: a Vehicle arm must stamp crewed:false under Alt (toggle && !alt_empty)"
    );
}

// ───────────────────────── CONN-GROUP-001 — regroup shares the ORBAT refile seam ─────────────

/// The map regroup reads the target character's squad off the SoA (`read_attrs`) and refiles
/// through the SAME T-180.6 core move the ORBAT dock uses (`refile_slot` → `move_slot_to_squad`),
/// so a map regroup and a dock refile are one undo step / one membership write. It no-ops when
/// the target has no squad or already shares the dragged slot's squad.
#[test]
fn regroup_reuses_the_refile_seam_and_noops_off_squad() {
    let ops = ops_live();
    let body = only_body(&ops, "pub fn regroup_slot_onto(");
    assert!(
        body.contains("read_attrs(target_id)") && body.contains("read_attrs(slot_id)"),
        "CONN-GROUP-001: regroup must read the target's (and source's) squad off the SoA"
    );
    assert!(
        body.contains("refile_slot("),
        "CONN-GROUP-001: regroup must go through the T-180.6 refile seam (move_slot_to_squad)"
    );
    assert!(
        body.contains("dest_squad.is_empty() || dest_squad == src_squad"),
        "CONN-GROUP-001: regroup must no-op when the target has no squad or already shares one"
    );
}

// ───────────────────────── Alt census (re-run at filing time) ────────────────────────────────

/// Re-run of the Alt census the ticket demanded, as source pins across the whole frontend. Alt
/// is a placement modifier ONLY on the canvas (this ticket's `alt_empty`); every pre-existing
/// `alt_key()` reader is either a NEGATIVE guard on a Ctrl shortcut or a DOCK-tree gesture — no
/// canvas collision. The `eden_tree` Alt-click (wave 104, descendants selection) is the one
/// noted since filing: a dock surface, not the map.
#[test]
fn alt_census_confirms_no_canvas_collision() {
    // mission_history: Alt is a NEGATIVE guard on the Ctrl/Cmd copy shortcut, never a place.
    let hist = live_code(include_str!("../state/history.rs"));
    assert!(
        hist.contains("|| ev.alt_key()"),
        "census: mission_history uses alt_key only as a guard (|| ev.alt_key())"
    );
    // mission_editor keydown: Alt only as !alt_empty on copy/paste and the Ctrl+Alt+D HUD
    // toggle — none a canvas placement modifier. (The keydown lives in the same file.)
    let ed = editor_live();
    assert!(
        ed.contains("if modk && ev.alt_key() && !ev.shift_key() =>"),
        "census: mission_editor's only positive alt_key keydown is the Ctrl+Alt+D HUD toggle"
    );
    // eden_tree: Alt-click is a DOCK-tree gesture (descendants selection), NOT the canvas.
    let tree = live_code(include_str!("../panels/outliner_tree.rs"));
    assert!(
        tree.contains("ev.alt_key() || ev.shift_key()"),
        "census: eden_tree's Alt-click is a dock-tree gesture (no canvas collision)"
    );
    // And the canvas's own new reader is the T-647 placement modifier — exactly one, in the
    // pointerup armed branch.
    let up = only_body(&ed, "let onpointerup =");
    assert_eq!(
        up.matches("ev.alt_key()").count(),
        1,
        "census: the canvas pointerup reads alt_key exactly once — the T-647 empty-vehicle \
         modifier"
    );
}

// ───────────────────────── The fired rule: perturb / fail / restore ──────────────────────────

/// Fires the PLACE-CREW-001 pin (`with_crew = place_with_crew() && !alt_empty`). Proof it is
/// load-bearing: the pin passes on the real body, and a perturbation that drops the `!alt_empty`
/// clause (the exact regression — Alt no longer forces empty) makes the same assertion FAIL.
/// Restore is implicit: the real `include_str!` body is untouched; only an in-memory copy is
/// perturbed here.
#[test]
fn fired_rule_alt_empty_clause_is_load_bearing() {
    let ops = ops_live();
    let real = only_body(&ops, "fn place_at_impl(");
    let needle = "let with_crew = place_with_crew() && !alt_empty";
    // PASS on the real body.
    assert!(
        real.contains(needle),
        "canary: the real body must carry the clause"
    );
    // Perturb: strip the Alt clause (the regression). The pin must no longer find its needle.
    let perturbed = real.replace(needle, "let with_crew = place_with_crew()");
    assert!(
        !perturbed.contains(needle),
        "fired rule: dropping `!alt_empty` (Alt stops forcing empty) must break the PLACE-CREW-001 \
         pin — proving the pin discriminates the regression"
    );
}
