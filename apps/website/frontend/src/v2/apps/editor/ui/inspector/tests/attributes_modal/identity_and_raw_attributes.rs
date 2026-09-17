//! Attributes modal identity and raw attributes tests.

use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn attrs_src() -> String {
    live_code(super::ATTRIBUTES_MODAL_SOURCE)
}

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
    assert!(
        body.contains("commit_slot(targets, Some(role), None, None, None, None)"),
        "Role must still commit into the role slot"
    );
    assert!(
        body.contains("commit_slot(targets, None, Some(tag), None, None, None)"),
        "Tag must still commit into the tag slot"
    );
}

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

#[test]
fn the_two_new_fields_are_labelled_type_and_role_description() {
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&src, "fn identity_tab(");
    assert!(body.contains("\"Type\""), "the type field is labelled Type");
    assert!(
        body.contains("\"Role Description\""),
        "the description field is labelled Role Description, not Description — it is the \
             description OF the role, and `Role` above is the short label it is distinct from"
    );
}

#[test]
fn a_refused_transform_field_is_disabled_whatever_the_multi_edit_latch_says() {
    let src = attrs_src();
    let locked = only_body(&src, "fn locked(self) -> bool");
    assert!(
        locked.contains("self.shut || self.opt.is_some_and(|o| !o.get())"),
        "Gate::locked must short-circuit on `shut`; body was:\n{locked}"
    );
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
    let forged = body.replacen(raw_gate, "false /* delete-prod */", 1);
    assert!(
        !forged.contains(raw_gate),
        "delete-prod control: stripping the raw gate must remove the existence needle"
    );
}

#[test]
fn attributes_modal_none_arm_still_closes_on_true_absence() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
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
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let host_src = only_body(&src, "pub fn AttributesModal(");
    assert!(
        host_src.contains("read_attrs(&id)") && host_src.matches("close_attributes()").count() >= 2,
        "live_source pin: read_attrs + dual close_attributes must remain real call sites"
    );
}

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

#[test]
fn attrs_update_slot_noops_when_all_none_or_id_missing() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
    )));
    let body = only_body(&ops, "pub fn attrs_update_slot(");

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
