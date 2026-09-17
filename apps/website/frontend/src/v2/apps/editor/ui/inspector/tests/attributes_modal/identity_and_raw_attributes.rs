// Everything this ticket touches in the modal lives behind `#[cfg(target_arch = "wasm32")]` — it
// builds `view!` trees over `web_sys` nodes and cannot be instantiated by `cargo test`, which runs
// native. So the modal half is pinned the way the rest of this crate pins its wasm-only surfaces
// (`mission_editor.rs`, `arsenal.rs`): against the SCRUBBED live source, with comments and dead
// `cfg` items removed so a pin can never be satisfied by the prose that describes the code.
//
// The BEHAVIOUR half is not pinned this way and does not need to be: `MissionDocCore::
// update_slot_object` and `slot_layer_is_locked` are native, and `store.rs`'s own tests fire them
// against a real document (`update_slot_object_sets_clears_and_leaves_none_fields_alone`,
// `slot_layer_is_locked_agrees_with_the_update_slot_position_refusal`). These pins cover the wiring
// those tests cannot see: that the modal actually calls them, on the fields it claims to.
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn attrs_src() -> String {
    live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )))
}

/// ATTR-FIELD-OBJ-TYPE + ATTR-FIELD-OBJ-ROLE-DESC — both fields exist in the Identity tab and
/// BOTH route through `commit_slot`'s new argument slots.
///
/// The argument position is the assertion, not the presence of a `text_field` call: the two
/// fields are the 5th and 6th `Option` of one six-argument commit, and a description wired into
/// the `asset_id` slot would compile, render, and silently overwrite the entity type. Pinned on
/// `live_code` (string literals blanked) so a label in a comment or a placeholder cannot satisfy
/// it — this must be a CALL.
#[test]
fn identity_tab_commits_type_and_role_description_through_their_own_argument_slots() {
    let src = attrs_src();
    let body = only_body(&src, "fn identity_tab(");
    assert!(
        body.contains("commit_slot(targets, None, None, None, Some(asset_id), None)"),
        "the Type field must commit into the asset_id slot alone; body was:\n{body}"
    );
    assert!(
        body.contains("commit_slot(targets, None, None, None, None, Some(desc))"),
        "the Role Description field must commit into the description slot alone; body was:\n{body}"
    );
    // And the pre-existing three still commit into theirs — the widening must not have shifted
    // Role into Tag's position, which is the one way this edit breaks silently.
    assert!(
        body.contains("commit_slot(targets, Some(role), None, None, None, None)"),
        "Role must still commit into the role slot"
    );
    assert!(
        body.contains("commit_slot(targets, None, Some(tag), None, None, None)"),
        "Tag must still commit into the tag slot"
    );
}

/// The two new fields read from `SlotAttrs`'s new columns and participate in the T-649 per-field
/// multi-edit opt-in (`g(diff.…, opts.…)`) rather than bypassing it — the seam T-649 left.
#[test]
fn the_new_fields_read_their_own_columns_and_take_the_multi_edit_gate() {
    let src = attrs_src();
    let body = only_body(&src, "fn identity_tab(");
    for needle in [
        "a.asset_id.clone()",
        "g(diff.asset_id, opts.asset_id)",
        "a.description.clone()",
        "g(diff.description, opts.description)",
    ] {
        assert!(
            body.contains(needle),
            "identity_tab must contain `{needle}`"
        );
    }
}

/// The labels an operator actually reads. `live_source` KEEPS string literals — this pin is
/// about user-visible copy, which is the one thing `live_code` deliberately cannot see.
#[test]
fn the_two_new_fields_are_labelled_type_and_role_description() {
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&src, "fn identity_tab(");
    assert!(body.contains("\"Type\""), "the type field is labelled Type");
    assert!(
        body.contains("\"Role Description\""),
        "the description field is labelled Role Description, not Description — it is the \
             description OF the role, and `Role` above is the short label it is distinct from"
    );
}

/// Wave-102 F-7 — a Transform field the core will REFUSE must be disabled, and no multi-edit
/// latch may re-open it.
///
/// Pinned on `Gate::locked`, which is the single place the `disabled` attribute is decided for
/// every field in this modal: `shut ||` must come FIRST and must be an unconditional `||`, so
/// that `refused()` overrides the opt-in rather than being one vote among two. Ticking "Apply
/// to all" on a locked slot re-enabling the input is exactly the lie F-7 banked.
#[test]
fn a_refused_transform_field_is_disabled_whatever_the_multi_edit_latch_says() {
    let src = attrs_src();
    let locked = only_body(&src, "fn locked(self) -> bool");
    assert!(
        locked.contains("self.shut || self.opt.is_some_and(|o| !o.get())"),
        "Gate::locked must short-circuit on `shut`; body was:\n{locked}"
    );
    // `refused()` must actually be reachable from the Transform tab, and only from there.
    let transform = only_body(&src, "fn transform_tab(");
    assert!(
        transform.contains("base.refused()"),
        "transform_tab must hard-shut its gates when every target is locked"
    );
    let identity = only_body(&src, "fn identity_tab(");
    assert!(
        !identity.contains("refused()"),
        "identity fields must NOT be lock-gated: T-665 locks TRANSFORM only, and `update_slot` \
             / `update_slot_object` carry no lock check, so a role or type edit on a locked slot \
             really does land"
    );
}

/// F-7's other half: the count must come from the CORE's own predicate, and `all_locked` must
/// mean every target — a partially-locked selection still moves its unlocked members, so
/// disabling the fields there would be the same lie in reverse.
#[test]
fn the_lock_affordance_asks_the_core_and_distinguishes_all_locked_from_some_locked() {
    let src = attrs_src();
    let modal = only_body(&src, "fn modal_view(");
    assert!(
        modal.contains("engine_ops::attrs_locked_count(&targets.get_value())"),
        "the modal must ask the core over the same id set the commits fan out to"
    );
    let transform = only_body(&src, "fn transform_tab(");
    assert!(
        transform.contains("let all_locked = n > 0 && locked_n == n;"),
        "all_locked must require EVERY target to be locked; body was:\n{transform}"
    );
}

/// `read_attrs` must read the two new fields off the RAW slot rows. This is the defect the
/// ticket named: the type was unreadable because the read path was the SoA, which has no such
/// column — not because the mutator was missing.
#[test]
fn read_attrs_reads_asset_id_and_description_from_the_raw_slot_rows() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/attrs.rs"
    )));
    let body = only_body(
        &ops,
        "pub fn read_attrs(core: &MissionDocCore, id: &str) -> Option<SlotAttrs>",
    );
    assert!(
        body.contains("raw_slot_rows(core)"),
        "read_attrs must consult the raw rows, not `materialize()` alone"
    );
    for needle in ["asset_id: row_str(", "description: row_str("] {
        assert!(body.contains(needle), "read_attrs must fill `{needle}…`");
    }
}

/* ─────────── T-744 — hide must not close Attributes like undo-away ─────────── */

/// wave-113 F-2 / T-744: `read_attrs` Option-gates on RAW membership, not SoA membership.
///
/// `materialize()` drops layer-hidden / `editorHidden` slots. The pre-fix body used
/// `soa.ids.iter().position(|s| s == id)?` as the Option gate, so Hide returned `None` and the
/// modal's `None` arm called `close_attributes()` — the same path as "slot was undone away".
///
/// Hollow-pin rules: `live_code` blanks comments + string literals, so a docstring claiming the
/// fix cannot green these needles. delete-prod: stripping the raw gate from a forged copy must
/// drop the existence needle (proves the pin is about production, not this test module).
#[test]
fn read_attrs_gates_existence_on_raw_rows_not_soa_membership() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/attrs.rs"
    )));
    let body = only_body(
        &ops,
        "pub fn read_attrs(core: &MissionDocCore, id: &str) -> Option<SlotAttrs>",
    );
    let raw_gate = "!rows.contains_key(id)";
    assert!(
        body.contains(raw_gate),
        "T-744: Option must gate on raw membership; body was:\n{body}"
    );
    assert!(
        !body.contains("soa.ids.iter().position(|s| s == id)?"),
        "T-744: SoA position must not be the Option gate (that made hide look like undo-away)"
    );
    assert!(
        body.contains("core.materialize()"),
        "T-744: SoA fields still come from materialize() when the slot is visible"
    );
    assert!(
        body.contains("slot_attrs_from_raw(&rows, id)"),
        "T-744: hidden-but-present slots must fall back to raw field values, not invent zeros"
    );
    // F1: the needle alone is not an exit — an empty `if !rows.contains_key(id) {}` arm kept
    // the pin green while missing ids still yielded Some. Require a real absence return in that
    // arm (wave-135 adversarial).
    let after_gate = body.split(raw_gate).nth(1).expect("raw gate present above");
    let brace = after_gate
        .find('{')
        .expect("T-744: raw gate must open an if-arm");
    let arm_tail = &after_gate[brace + 1..];
    let close = arm_tail.find('}').expect("T-744: raw-gate arm must close");
    let arm = arm_tail[..close].trim();
    assert!(
        arm.contains("return None")
            || arm.split_whitespace().collect::<Vec<_>>().join(" ") == "None",
        "T-744: raw absence arm must exit with None (empty arm must RED); arm was:\n{arm}"
    );
    // delete-prod control: remove the raw gate from a forged production body → needle gone →
    // the positive assert above would RED. (This forged copy is never compiled; it proves the
    // pin is load-bearing on the production token, not on a comment or this test's own source.)
    let forged = body.replacen(raw_gate, "false /* delete-prod */", 1);
    assert!(
        !forged.contains(raw_gate),
        "delete-prod control: stripping the raw gate must remove the existence needle"
    );
}

/// The modal's `None` arm still closes — but only for true absence. Esc alone must not green
/// this pin: require the `read_attrs` match and **two** `close_attributes()` call sites in the
/// host (Esc listener + None arm). `live_code` blanks comments; `live_source` keeps call paths.
#[test]
fn attributes_modal_none_arm_still_closes_on_true_absence() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let host = only_body(&code, "pub fn AttributesModal(");
    assert!(
        host.contains("read_attrs(&id)"),
        "AttributesModal must ask read_attrs for existence; body was:
{host}"
    );
    assert!(
        host.matches("close_attributes()").count() >= 2,
        "Esc path AND the None/undo-away arm must both call close_attributes; found {} in:
{host}",
        host.matches("close_attributes()").count()
    );
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let host_src = only_body(&src, "pub fn AttributesModal(");
    assert!(
        host_src.contains("read_attrs(&id)") && host_src.matches("close_attributes()").count() >= 2,
        "live_source pin: read_attrs + dual close_attributes must remain real call sites"
    );
}

/// The write path: both new fields land through `update_slot_object`, and `update_slot`'s three
/// original columns are not dragged along by a commit that only touches a new one.
#[test]
fn attrs_update_slot_routes_the_new_fields_through_update_slot_object() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/attrs.rs"
    )));
    let body = only_body(&ops, "pub fn attrs_update_slot(");
    assert!(
        body.contains("core.update_slot_object(id, asset_id, description)"),
        "the object half must go through the core mutator that leaves None keys alone"
    );
    assert!(
        body.contains("if role.is_some() || tag.is_some() || stance.is_some() {"),
        "a type-only or description-only commit must not open an update_slot transaction"
    );
}

/// T-745 Class-R: `attrs_update_slot` must no-op on all-None and on a missing id.
///
/// Production already carries both guards (editor_ops.rs). Without this lasting pin a one-hunk
/// revert ships green — the sibling route pin only requires `update_slot_object` / slot-half
/// gating (wave-136 F1).
///
/// RED: strip the five-field all-None early `return` before `let did`.
/// RED: strip `!raw_slot_rows(core).contains_key(id) → false`.
#[test]
fn attrs_update_slot_noops_when_all_none_or_id_missing() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
    )));
    let body = only_body(&ops, "pub fn attrs_update_slot(");

    // (1) five-field all-None early `return` before `let did`
    let before_did = body
        .split("let did")
        .next()
        .expect("attrs_update_slot must bind `let did`");
    for field in [
        "role.is_none()",
        "tag.is_none()",
        "stance.is_none()",
        "asset_id.is_none()",
        "description.is_none()",
    ] {
        assert!(
                before_did.contains(field),
                "T-745: all-None guard must check `{field}` before `let did`; prelude was:\n{before_did}"
            );
    }
    assert!(
        before_did.contains("return"),
        "T-745: all-None must early-return before `let did`; prelude was:\n{before_did}"
    );

    // (2) `!raw_slot_rows(core).contains_key(id)` → false arm
    let domain = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/attrs.rs"
    )));
    let body = only_body(&domain, "pub fn attrs_update_slot(");
    let raw_gate = "!raw_slot_rows(core).contains_key(id)";
    assert!(
        body.contains(raw_gate),
        "T-745: missing id must gate on raw membership; body was:\n{body}"
    );
    let after_gate = body.split(raw_gate).nth(1).expect("raw gate present above");
    let brace = after_gate
        .find('{')
        .expect("T-745: raw gate must open an if-arm");
    let arm_tail = &after_gate[brace + 1..];
    let close = arm_tail.find('}').expect("T-745: raw-gate arm must close");
    let arm = arm_tail[..close].trim();
    assert!(
        arm.contains("return false")
            || arm.split_whitespace().collect::<Vec<_>>().join(" ") == "false",
        "T-745: raw absence arm must yield false (empty arm must RED); arm was:\n{arm}"
    );
}
