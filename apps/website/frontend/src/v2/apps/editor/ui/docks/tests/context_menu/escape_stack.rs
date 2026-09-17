use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

#[test]
fn context_menu_gates_escape_on_modal_stack() {
    let code = live_code(super::test_source::raw_context_menu());
    let body = only_body(&code, "pub fn ContextMenuOverlay(");
    let reg = ["modal_stack", "::", "register("].concat();
    let top = ["modal_stack", "::", "is_topmost_open(modal_id)"].concat();
    let unreg = ["modal_stack", "::", "unregister(modal_id)"].concat();
    assert!(
        body.contains(&reg),
        "T-726: ContextMenuOverlay must register"
    );
    assert!(
        body.contains(&top),
        "T-726: ContextMenuOverlay must gate keys on is_topmost_open"
    );
    assert!(
        body.contains(&unreg),
        "T-726: ContextMenuOverlay must unregister"
    );
}
