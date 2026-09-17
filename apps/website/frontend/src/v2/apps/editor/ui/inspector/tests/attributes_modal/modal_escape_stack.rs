/// T-726 — Attributes modal Esc through the modal stack.
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

#[test]
fn attributes_modal_gates_escape_on_modal_stack() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "pub fn AttributesModal(");
    let reg = ["modal_stack", "::", "register("].concat();
    let top = ["modal_stack", "::", "is_topmost_open(modal_id)"].concat();
    let unreg = ["modal_stack", "::", "unregister(modal_id)"].concat();
    assert!(body.contains(&reg), "T-726: AttributesModal must register");
    assert!(
        body.contains(&top),
        "T-726: AttributesModal must gate Escape on is_topmost_open"
    );
    assert!(
        body.contains(&unreg),
        "T-726: AttributesModal must unregister"
    );
}
