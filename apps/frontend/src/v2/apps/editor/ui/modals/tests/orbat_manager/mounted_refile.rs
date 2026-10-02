#[test]
fn the_mounted_manager_arms_and_consumes_the_shared_set() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let code = live_code(&super::source::production_source());
    let row = only_body(&code, "fn stitch_row(");
    assert!(row.contains("tree::drag_set_for("));
    assert!(row.contains("drag::begin_refile(drag)"));
    assert!(row.contains("drag::complete_multi_refile_onto_squad(&id_drop)"));
    let dialog = only_body(&code, "pub fn OrbatManagerDialog(");
    assert!(dialog.contains("drag::cancel_layer_drag()"));
    assert!(dialog
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains("tree_panel( squad_nodes, orbat,"));
    let panel = only_body(&code, "fn tree_panel(");
    assert!(panel
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains("stitch_row( r, drag_nodes,"));
}
