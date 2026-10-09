use super::*;

/// The runbook the font-cache diagnostic names exists in the checkout.
///
/// A relocation of the documentation tree rewrites this value, and a value left behind fails
/// quietly: the diagnostic points the operator at a dead runbook.
#[test]
fn the_editor_gate_runbook_exists_in_the_checkout() {
    let root = ::repository_root::find_repository_root().expect("active checkout");
    assert!(
        root.join(EDITOR_GATE_RUNBOOK).is_file(),
        "missing: {EDITOR_GATE_RUNBOOK}"
    );
}
