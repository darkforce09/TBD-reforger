//! The editor page fills the lower layers' hooks before anything can fire them.
//!
//! The draft-persist hook fires from the undo driver's edit tail, which exists only once the
//! canvas mount installs the history context; the right-click opener fires from a gesture the
//! canvas mount attaches; the compile-findings publisher fires from the top strip's export row.
//! All three registrations therefore have to sit in `MissionEditorPage` ahead of
//! `install_canvas_mount(` and of the view that renders the top strip.

use frontend_test_support::class_r_scrub::{live_code, only_body};

const REGISTRATIONS: [&str; 3] = [
    "persist::register_edit_persist()",
    "context_menu::register_canvas_context_menu()",
    "validation_panel::register_compile_findings_publisher()",
];

#[test]
fn the_page_registers_every_lower_layer_hook_before_the_canvas_mount_and_the_top_strip() {
    let page = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mission_editor.rs"
    )));
    let body = only_body(&page, "pub fn MissionEditorPage(");
    let mount = body
        .find("install_canvas_mount(")
        .expect("the page installs the canvas mount");
    let strip = body
        .find("TopCommandStrip")
        .expect("the page renders the top strip");
    for registration in REGISTRATIONS {
        let at = body
            .find(registration)
            .unwrap_or_else(|| panic!("the page never calls `{registration}`"));
        assert!(
            at < mount && at < strip,
            "`{registration}` must run before the canvas mount and the top strip exist"
        );
        assert_eq!(
            body.matches(registration).count(),
            1,
            "`{registration}` runs once per mount; a remount replaces the registration"
        );
    }
}
