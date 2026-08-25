use crate::editor::arsenal::class_r_scrub::live_code;
/// T-703/T-738 — THE keydown arm-list extractor, consumed rather than re-copied. This module
/// carried the raw-text variant of it; the shared one scrubs comments, which is strictly
/// stronger for the census below (a note that MENTIONS `KeyA` can no longer read as a binding).
use crate::editor::panels::help_modal::keymap_census::keydown_arms;

/// Everything after the editor page's own signature — the live editor body.
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

/// All whitespace removed. `rustfmt` is free to break a Leptos `view!` expression across lines
/// wherever it likes (`gate\n.opt\n.map(`), so any pin on an EXPRESSION rather than on a
/// statement is matched against this form — otherwise the pin is really a formatting pin.
fn squash(src: &str) -> String {
    src.chars().filter(|c| !c.is_whitespace()).collect()
}

/// One top-level `fn`'s source, signature through the closing brace at column 0.
fn fn_source(src: &str, sig: &str) -> String {
    let at = src
        .find(sig)
        .unwrap_or_else(|| panic!("`{sig}` must exist in the live source"));
    let rest = &src[at..];
    let end = rest.find("\n}\n").map_or(rest.len(), |i| i + 3);
    rest[..end].to_string()
}

// ── SEL-ALL-001 ───────────────────────────────────────────────────────────────────────────

/// KEY CENSUS: Ctrl/Cmd+A is claimed by THIS editor keydown and by nothing else. The two
/// window-level editor keydowns are this file's and `mission_history`'s (Ctrl+Z/Y); the other
/// one must not also bind it, or the two listeners would both fire on one keypress.
#[test]
fn t649_ctrl_a_census() {
    let this_arms = keydown_arms(include_str!("../mission_editor.rs"));
    let history_arms = keydown_arms(include_str!("../state/history.rs"));
    // Assembled so the literal never appears verbatim in this test's own source.
    let key_a = format!("\"{}\"", "KeyA");
    assert!(
        !history_arms.contains(&key_a),
        "census: mission_history's keydown (Ctrl+Z/Y) must not claim A"
    );
    // The arm is modifier-gated (Ctrl/Cmd) and rejects Alt/Shift, exactly like Ctrl+C / Ctrl+V
    // beside it — a BARE `a` must stay free.
    assert!(
        this_arms.contains(&format!(
            "{key_a} if modk && !ev.alt_key() && !ev.shift_key() =>"
        )),
        "SEL-ALL-001: Ctrl/Cmd+A (not bare A, not Alt/Shift combos) must be the Select All arm"
    );
    // Ctrl+C / Ctrl+V are untouched neighbours — this slice added an arm, it did not re-key one.
    for k in ["KeyC", "KeyV"] {
        let key = format!("\"{k}\"");
        assert!(
            this_arms.contains(&format!(
                "{key} if modk && !ev.alt_key() && !ev.shift_key() =>"
            )),
            "the clipboard arms must be unchanged by T-649"
        );
    }
}

/// The Ctrl+A arm measures the CANVAS and delegates to `select_all_in_view`; it never reaches
/// into the doc itself. Returning the "acted" bool is what earns the shared `prevent_default`
/// below the match — without it the browser's own Select All would blue-wash the chrome.
#[test]
fn ctrl_a_hands_the_container_rect_to_select_all_in_view() {
    let ed = editor_live();
    let arms = keydown_arms(include_str!("../mission_editor.rs"));
    assert!(
        arms.contains("container.get_bounding_client_rect()")
            && arms.contains("editor_ops::select_all_in_view(rect.width(), rect.height())"),
        "SEL-ALL-001: the Ctrl+A arm must pass the live container CSS size to select_all_in_view"
    );
    // The closure has to capture the container for that to be possible.
    assert!(
        ed.contains("let container = container.clone();"),
        "the keydown closure must clone the container in to measure it"
    );
    // The whole closure is behind the shared editable-field guard, so Ctrl+A still means
    // "select the text" while the operator is typing in an Attributes field.
    assert!(
        ed.contains("mission_history::in_editable_field()"),
        "the editor keydown must keep its editable-field guard"
    );
}

/// Eden scopes Select All to what is ON SCREEN. This pins that the implementation is a
/// VIEWPORT-RECT query through the marquee's own primitive — not a "hand back every id in the
/// document" shortcut, which is the obvious wrong implementation of this ticket.
#[test]
fn select_all_is_viewport_scoped_through_the_marquee_primitive() {
    let tool = live_code(include_str!("../tools/select_tool.rs"));
    let view_fn = fn_source(&tool, "pub fn view_ids_with_vehicles(");
    // The near corner is the top-left CSS pixel unprojected; the far corner is the viewport
    // size in PIXELS — the exact (world start, px end) shape `marquee_ids_with_vehicles` takes.
    assert!(
        view_fn.contains("cam.size_px()") && view_fn.contains("cam.unproject_xy(0.0, 0.0)"),
        "SEL-ALL-001: the select-all rect must be the viewport — unproject (0,0), far corner \
         from the camera's own size_px()"
    );
    assert!(
        view_fn.contains("marquee_ids_with_vehicles(cam, soa, vehicle_points,"),
        "SEL-ALL-001: it must reuse the marquee primitive, not define a second 'inside the box'"
    );
    // A degenerate camera yields nothing, exactly like the marquee — never a full-mission dump.
    assert!(
        view_fn.contains("is_finite()") && view_fn.contains("return Vec::new()"),
        "a non-finite unproject must select NOTHING (the marquee's own behaviour)"
    );

    let ops = live_code(include_str!("../state/operations/entity.rs"));
    let sel_fn = fn_source(&ops, "pub fn select_all_in_view(");
    assert!(
        sel_fn.contains("select_tool::view_ids_with_vehicles(")
            && sel_fn.contains("select_tool::frozen_camera("),
        "select_all_in_view must snapshot a frozen camera and run the viewport-rect query"
    );
    assert!(
        !sel_fn.contains("soa.ids.clone()") && !sel_fn.contains(".ids.clone()"),
        "SEL-ALL-001: Select All is scoped to the VIEWPORT — it must never hand back the whole \
         document's id list"
    );
    // Selection-only change: the SEL readout refreshes, but nothing enters the undo history.
    assert!(
        sel_fn.contains("mission_history::refresh_selection()")
            && !sel_fn.contains("after_local_edit"),
        "a selection change is not a doc edit — refresh_selection only, never a history step"
    );
}

// ── ATTR-MULTI-001 / ATTR-MULTI-CHK-001 ───────────────────────────────────────────────────

/// THE INVERTED GUARD. Before this slice both `open_attributes` and `open_arsenal` opened with
/// an identical three-line `if ctx.selection.borrow().len() > 1 { return; }`, so a
/// multi-selection suppressed the modal entirely — which in turn made `context_menu.rs`'s
/// unconditionally-enabled "Attributes..." / "Edit Loadout..." rows live-but-inert (T-716).
/// Both guards must be gone, and BOTH entry points must route through the one shared opener.
#[test]
fn multi_selection_no_longer_suppresses_the_attributes_modal() {
    // T-934.7 — the ops module was split; concatenate every submodule so the file-wide
    // absence / uniqueness assertions keep their whole-module meaning.
    let ops = live_code(
        &[
            include_str!("../state/operations/attrs.rs"),
            include_str!("../state/operations/cargo.rs"),
            include_str!("../state/operations/compositions.rs"),
            include_str!("../state/operations/context.rs"),
            include_str!("../state/operations/entity.rs"),
            include_str!("../state/operations/transform.rs"),
        ]
        .concat(),
    );
    assert!(
        !ops.contains("if ctx.selection.borrow().len() > 1 {"),
        "ATTR-MULTI-001: the suppress-on-multi guard must be gone from editor_ops"
    );
    for entry in ["pub fn open_attributes(", "pub fn open_arsenal("] {
        let f = fn_source(&ops, entry);
        assert!(
            f.contains("open_attrs_modal("),
            "{entry} must route through the shared opener, not a re-copied guard"
        );
    }
    // Arsenal still lands on tab 3; Attributes still leaves the tab alone.
    let opener = fn_source(&ops, "fn open_attrs_modal(");
    assert!(
        opener.contains("ctx.attrs_tab.set(3)") && opener.contains("if arsenal_tab {"),
        "open_arsenal must still select the Arsenal tab"
    );
    // The multi path must PRESERVE the selection — replacing it with `[id]` would collapse the
    // very set the operator is about to multi-edit.
    assert!(
        opener.contains("sel.len() > 1 && sel.contains(&id)")
            && opener.contains("if !keep_selection {"),
        "ATTR-MULTI-001: opening over a multi-selection must not collapse it to one id"
    );
}

/// The per-field checkbox. `attributes.rs` had ZERO checkbox inputs before this slice; a field
/// whose values DIFFER across the selection must now be blank, disabled, and behind one.
#[test]
fn differing_fields_are_locked_behind_a_per_field_checkbox() {
    let raw_attrs = include_str!("../panels/attributes_modal.rs");
    let attrs = live_code(raw_attrs);
    // The checkbox itself (string literal ⇒ pinned on the RAW source), assembled so this test's
    // own text is not the match.
    let checkbox = format!("type=\"{}\"", "checkbox");
    assert!(
        raw_attrs.contains(&checkbox),
        "ATTR-MULTI-CHK-001: the multi-edit opt-in checkbox must exist in the modal"
    );
    let label = squash(&fn_source(&attrs, "fn field_label("));
    assert!(
        label.contains("gate.opt.map(|o|") && label.contains("o.set(event_target_checked(&ev))"),
        "the checkbox must be bound to the field's own opt-in latch"
    );
    // Locked ⇒ disabled. Both field primitives, plus the stance select.
    for f in ["fn number_field(", "fn text_field("] {
        let src = fn_source(&attrs, f);
        assert!(
            src.contains("disabled=move || gate.locked()"),
            "{f} must disable while the field differs and its checkbox is unticked"
        );
        assert!(
            src.contains("gate.differs()"),
            "{f} must blank the value when the selection disagrees — showing one member's \
             value would be a lie about the other N-1"
        );
    }
    let xform = fn_source(&attrs, "fn transform_tab(");
    assert!(
        xform.contains("disabled=move || stance_gate.locked()"),
        "the Stance select must obey the same gate as the text/number fields"
    );
    // A gate is minted ONLY under a multi-selection AND only where the values actually differ,
    // so single-slot editing is byte-for-byte the pre-T-649 behaviour.
    assert!(
        xform.contains("Gate::maybe(is_multi && differs, latch)"),
        "a field the selection AGREES on must stay live with no checkbox"
    );
    // Every editable field is wired to its own latch — a shared one would tick them together.
    for latch in [
        "opts.x",
        "opts.y",
        "opts.z",
        "opts.rotation",
        "opts.stance",
        "opts.role",
        "opts.tag",
    ] {
        assert!(
            attrs.contains(latch),
            "{latch} must gate its own field (one checkbox per field, not one for the modal)"
        );
    }
    // The latches must survive a commit: they are minted on the COMPONENT and re-armed off
    // `attrs_open` only, never off `doc_tick` (which every commit bumps).
    let modal = fn_source(&attrs, "pub fn AttributesModal(");
    assert!(
        modal.contains("let opts = MultiOpts::new();") && modal.contains("opts.reset()"),
        "the opt-in latches must live on the component and re-arm when the modal reopens"
    );
}

/// A multi-edit commit reaches EVERY selected slot, field-by-field, under ONE history tail —
/// and an un-opted field stays `None`, so ticking Rotation cannot also stamp one member's X
/// onto the rest.
///
/// T-736: a bare `after_local_edit()` count == 1 is blind to "one tail *inside* the loop"
/// (wave-112 MINOR-2). The pin must prove the call sits AFTER the balanced `for id in ids`
/// body closes — locate by SYMBOL on the scrubbed `editor_ops` source.
///
/// RED (tail-in-loop): move the single `after_local_edit()` inside `for id in ids { … }` →
/// "must fire the history/persist tail OUTSIDE the fan-out loop".
#[test]
fn multi_edit_commits_fan_out_to_every_selected_id() {
    let attrs = live_code(include_str!("../panels/attributes_modal.rs"));
    for (seam, single, multi) in [
        (
            "fn commit_position(",
            "attrs_update_position(",
            "attrs_update_position_multi(",
        ),
        (
            "fn commit_slot(",
            "attrs_update_slot(",
            "attrs_update_slot_multi(",
        ),
    ] {
        let f = fn_source(&attrs, seam);
        assert!(
            f.contains(multi) && f.contains(single) && f.contains("ids.len() > 1"),
            "{seam} must fan out on a multi-selection and keep the ORIGINAL single-slot call \
             otherwise"
        );
    }
    let ops = live_code(include_str!("../state/operations/attrs.rs"));
    // T-732 — position multi is ONE LOCAL txn via update_entity_transforms (not N×
    // update_slot_position). F-26 (T-788) — identity multi is now ATOMIC too, via
    // `update_slots_attr_batch` (one txn, one undo step); the per-id fan-out moved INTO the core.
    {
        let f = "pub fn attrs_update_position_multi(";
        let src = fn_source(&ops, f);
        let batch = ["update_entity", "_transforms("].concat();
        let per_id = ["core.update_slot", "_position(id,"].concat();
        assert!(
            src.contains(&batch),
            "T-732: {f} must commit via update_entity_transforms (one LOCAL txn)"
        );
        assert!(
            !src.contains(&per_id),
            "T-732: {f} must NOT loop per-id update_slot_position — that is N undo steps"
        );
        assert!(
            src.contains("for id in ids {") && src.contains("EntityTransformPatch"),
            "{f} must build a patch list over every id in the target set"
        );
        assert_eq!(
            src.matches("after_local_edit()").count(),
            1,
            "{f} must fire exactly ONE history/persist tail for the whole commit"
        );
        assert_after_local_edit_outside_ids_loop(f, &src);
        assert!(
            src.contains("is_none()") && src.contains("return;"),
            "{f} must no-op when nothing was opted in"
        );
    }
    {
        // F-26 (T-788) — the NEW contract: the identity/type multi-commit is ONE LOCAL txn via
        // `update_slots_attr_batch` (one undo step for an apply-to-all), NOT a per-id loop of
        // `core.update_slot(id, …)` (which was N undo steps under `capture_timeout_millis = 0` —
        // the review's measured 9→8). The per-slot byte-semantics are unchanged: the batch runs
        // the same `update_slot` / `update_slot_object` logic per id inside the core (pinned
        // native by `update_slots_attr_batch_is_one_undo_step_across_many_slots` in `store.rs`).
        let f = "pub fn attrs_update_slot_multi(";
        let batch = ["update_slots_attr", "_batch("].concat();
        let per_id = ["core.update_slot", "(id,"].concat();
        let src = fn_source(&ops, f);
        assert!(
            src.contains(&batch),
            "F-26: {f} must commit via update_slots_attr_batch (one LOCAL txn, one undo step)"
        );
        assert!(
            !src.contains(&per_id),
            "F-26: {f} must NOT loop per-id core.update_slot(id, …) — that is N undo steps for \
             an apply-to-all (the review's 9→8). The fan-out lives in the core batch now."
        );
        assert!(
            src.contains("slot_half") && src.contains("ids,"),
            "{f} must hand the whole id set (and its opt-in `slot_half`) to the batch"
        );
        assert_eq!(
            src.matches("after_local_edit()").count(),
            1,
            "{f} must fire exactly ONE history/persist tail for the whole commit"
        );
        assert!(
            src.contains("is_none()") && src.contains("return;"),
            "{f} must no-op when nothing was opted in"
        );
    }
    // The "which fields differ" read is one snapshot over one materialize, and it compares
    // dict-coded columns by TEXT (two rows can carry the same role under different indices).
    let diff = fn_source(&ops, "pub fn read_attrs_diff(");
    assert!(
        diff.matches("core.materialize()").count() == 1 && diff.contains("&soa.roles)"),
        "read_attrs_diff must compare one snapshot, resolving dict columns to their strings"
    );
}

/// Wave-112 MINOR-2 / T-736: `matches("after_local_edit()").count() == 1` cannot tell "one
/// tail after the loop" from "one tail inside it". Brace-match `for id in ids {…}` and
/// require the call to sit strictly after that span.
fn assert_after_local_edit_outside_ids_loop(fn_name: &str, src: &str) {
    const LOOP: &str = "for id in ids {";
    let at = src
        .find(LOOP)
        .unwrap_or_else(|| panic!("{fn_name} must fan out with `{LOOP}`"));
    let open = at + LOOP.len() - 1;
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    let mut end = None;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let end = end.unwrap_or_else(|| panic!("{fn_name}: unbalanced `{LOOP}`"));
    let inside = &src[open..=end];
    assert!(
        !inside.contains("after_local_edit()"),
        "{fn_name} must fire the history/persist tail OUTSIDE the fan-out loop — found \
         after_local_edit() inside `for id in ids`"
    );
    assert!(
        src[end + 1..].contains("after_local_edit()"),
        "{fn_name} must fire the history/persist tail after the fan-out loop closes"
    );
}

/// T-788 F-27 — a PLAIN (non-additive) click that lands INSIDE the current multi-selection must
/// NOT collapse it to `[hit]`. That collapse is what made double-click-to-multi-edit unreachable:
/// the first click of the dbl-click re-selected the one slot on pointerup BEFORE `dblclick` →
/// `open_attributes` ran, so `attrs_multi_ids` saw SEL1 and the modal opened single-edit. The
/// group-drag path (`compute_move_ids` in the `LG::Move` arm) has the same dependency: the whole
/// selection must survive the press.
///
/// The pin is on the pointerup click arm because a wasm pointer Closure cannot run in a native
/// test (same reason every dbl-click/pointerup pin in this file is a scrubbed-source pin). It
/// requires the guard to gate on ALL of: not-additive, a real multi-selection, and the hit being
/// a MEMBER of it — and to still call `apply_click` otherwise, so a click OUTSIDE the selection
/// (or a Ctrl-click) keeps the exact Eden replace/toggle semantics.
///
/// RED (drop the guard): restore a bare `st::apply_click(&mut sel, hit, additive);` with no
/// `keep_multi` → "the plain-inside-selection click must be guarded so it does not collapse".
#[test]
fn t788_plain_click_inside_a_multi_selection_does_not_collapse_it() {
    let ed = editor_live();
    // The guard's three conjuncts, on the scrubbed source (comments/strings gone).
    for needle in [
        "let keep_multi = !additive",
        "sel.len() > 1",
        "sel.iter().any(|s| s == h)",
    ] {
        assert!(
            ed.contains(needle),
            "F-27: the click arm must gate collapse on `{needle}` (plain click, real \
             multi-selection, hit is a member)"
        );
    }
    // The gate wraps the replace/toggle: `apply_click` runs only when NOT keeping the multi, so
    // an outside click / Ctrl-click still flows through Eden's `apply_click` untouched.
    assert!(
        squash(&ed).contains(&squash(
            "if !keep_multi {\n st::apply_click(&mut sel, hit, additive);"
        )),
        "F-27: `st::apply_click` must run only under `if !keep_multi` — outside/additive clicks \
         keep Eden replace/toggle; an inside plain click preserves the selection"
    );
    // HOLLOW-PIN: a bare unconditional apply_click (the pre-fix shape) would collapse SEL9→SEL1
    // and must not return. The only apply_click in this arm is the guarded one.
    assert_eq!(
        ed.matches("st::apply_click(&mut sel, hit, additive)")
            .count(),
        1,
        "F-27: exactly one apply_click call in the click arm, and it is the guarded one"
    );
    // The OUTLINER half of the same defect: the tree/ORBAT rows route their single click
    // through `editor_ops::select_slot` and their dblclick through `open_attributes`, so the
    // unconditional `= vec![id]` replace in select_slot collapsed SEL9→SEL1 before activate
    // fired and the modal could only ever open single-edit from a row. Same guard, same
    // outside-click-still-replaces Eden semantics (the contract context_menu::open documents).
    let ops = live_code(include_str!("../state/operations/entity.rs"));
    let sel_fn = fn_source(&ops, "pub fn select_slot(");
    assert!(
        squash(&sel_fn).contains(&squash("sel.len() > 1 && sel.iter().any(|s| *s == id)")),
        "F-27: select_slot must keep a multi-selection that already contains the clicked id"
    );
    assert!(
        squash(&sel_fn).contains(&squash("if !keep_multi {")),
        "F-27: select_slot's replace must be gated — a row click outside the selection still \
         replaces (Eden semantics)"
    );
}

/// T-788 F-29 — with the Attributes modal open, a SELECTION change (Ctrl+A, a pick, a marquee)
/// must re-render or close the modal, because the modal body's only render triggers are
/// `attrs_open` and `doc_tick` and a selection change bumps NEITHER — so the panel kept showing
/// the single slot it opened on while SEL climbed to 9.
///
/// The fix lives in `refresh_selection_mirrors` — the ONE funnel every selection-only change
/// flows through (`mission_history::refresh_selection` → here) — and NOT in `mirror_selection`,
/// which `refresh_docks` also runs on every DOCUMENT change; re-poking `attrs_open` there would
/// wipe the operator's per-field opt-in ticks on every commit (the T-649 latch re-arms off
/// `attrs_open` alone, so a commit must leave the ticks intact).
///
/// RED (delete the sync): drop the `attrs_open` re-poke/close from `refresh_selection_mirrors`
/// → "must re-render (or close) the open Attributes modal when the selection changes".
#[test]
fn t788_open_attributes_modal_follows_a_selection_change() {
    let ops = live_code(include_str!("../state/operations/context.rs"));
    let f = fn_source(&ops, "pub fn refresh_selection_mirrors(");
    // Reads the open id WITHOUT subscribing (untracked — this is a plain fn, not an effect).
    assert!(
        f.contains("ctx.attrs_open.get_untracked()"),
        "F-29: refresh_selection_mirrors must read the open modal id (get_untracked)"
    );
    // Still-selected id ⇒ re-poke `attrs_open` to force the modal body to re-render against the
    // live selection (single-edit flips to multi within the frame).
    assert!(
        f.contains("ctx.attrs_open.set(Some(open_id))"),
        "F-29: when the open id is still selected, re-poke attrs_open so the modal re-renders \
         (header flips to `N slots · multi-edit`)"
    );
    // Deselected id ⇒ close, rather than strand a single-edit view on a slot the operator just
    // deselected (spec: re-render against the live selection OR close on selection change).
    assert!(
        f.contains("ctx.attrs_open.set(None)"),
        "F-29: when the open id left the selection, close the modal"
    );
    // Gated on the LIVE selection membership — the branch reads the selection, not a constant.
    assert!(
        squash(&f).contains(&squash(
            "ctx.selection.borrow().iter().any(|s| *s == open_id)"
        )),
        "F-29: the re-render/close choice must test whether the live selection still contains \
         the open id"
    );
    // SEPARATION PIN (the whole point): the sync must NOT be in `mirror_selection`, or every
    // commit's `refresh_docks` → `mirror_selection` would reset the modal's opt-in latches.
    let mirror = fn_source(&ops, "fn mirror_selection(");
    assert!(
        !mirror.contains("attrs_open"),
        "F-29: mirror_selection must NOT touch attrs_open — it also runs on every doc change \
         (refresh_docks), which would wipe the operator's per-field ticks on every commit"
    );
}

/// HONESTY (T-649 / T-771): inverting the `open_arsenal` guard makes the context menu's
/// "Edit Loadout..." row open something under a multi-selection. Pick and cargo rows still
/// edit ONE slot; T-699's Copy / Apply / Remove Everything fan out to the WHOLE selection.
/// The banner must say both — the old unqualified "one entity, not the whole selection" claim
/// is false for those three verbs and must not return.
#[test]
fn the_arsenal_tab_discloses_one_entity_picks_and_whole_selection_buffer_verbs() {
    let raw = include_str!("../panels/attributes_modal.rs");
    let modal_raw = fn_source(raw, "fn modal_view(");
    let one = "Pick and cargo edits apply to this one entity";
    let whole = "Copy, Apply, and Remove Everything act on the whole selection";
    assert!(
        modal_raw.contains(one) && modal_raw.contains(whole),
        "banner must disclose one-entity picks/cargo AND whole-selection buffer verbs"
    );
    // Hollow-pin: restoring the T-649 unqualified claim would go green on a weaker pin.
    assert!(
        !modal_raw.contains("Loadout edits apply to this one entity"),
        "do not restore the unqualified one-entity claim — it contradicts the buffer verbs"
    );
    let attrs = live_code(raw);
    let modal = squash(&fn_source(&attrs, "fn modal_view("));
    assert!(
        modal.contains("is_multi.then("),
        "the disclosure must render only under a multi-selection"
    );
}
