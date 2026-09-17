use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

fn page() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../../mission_editor.rs");
    live_code(&raw[raw.find(anchor.as_str()).expect("MissionEditorPage")..])
}

#[test]
fn first_bind_uploads_vehicle_symbology_not_the_disc_lane() {
    let mount = page();
    let bind = format!("{}{}", "vehicles_bind_", "symbology(");
    assert!(
        mount.contains(&bind),
        "T-930: engine-mount first bind must upload vehicle glyphs; otherwise a placed /
         restored vehicle stays on pack_vehicle_instances' yellow disc until a later move"
    );
    let disc = format!("{}{}", "vehicles_bind", "(&");
    assert!(
        !mount.contains(&disc),
        "T-930: first bind must not call the disc lane"
    );
}

#[test]
fn place_path_invalidates_vehicle_lane() {
    let raw = crate::v2::core::test_support::editor_operations::ENTITY;
    let impl_body = only_body(raw, "fn place_at_impl(");
    let rebind = format!("{}{}", "rebind_vehicle_lane_", "after_place");
    assert!(
        impl_body.contains("placed_vehicle"),
        "T-930: catalog Vehicle arm must flag a vehicle place; body:\n{impl_body}"
    );
    assert!(
        impl_body.contains(&rebind),
        "T-930: place_at_impl must call the place-time vehicle invalidate; body:\n{impl_body}"
    );
    let helper = only_body(raw, &format!("fn rebind_vehicle_lane_{}", "after_place()"));
    assert!(
        helper.contains(&format!("{}{}", "vehicles_bind_", "symbology(")),
        "T-930: place-time invalidate must bind silhouettes, not discs; body:\n{helper}"
    );
    assert!(
        helper.contains("mark_dirty()"),
        "T-930: place-time invalidate must mark damage so the first frame repaints; body:\n{helper}"
    );
    assert!(
        !helper.contains("vehicles_bind(&"),
        "T-930: place-time invalidate must not use the disc packer; body:\n{helper}"
    );
}
