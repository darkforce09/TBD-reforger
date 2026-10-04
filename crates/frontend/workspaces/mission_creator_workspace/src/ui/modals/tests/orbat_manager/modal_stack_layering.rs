//! The ORBAT manager's overlay layering against the shared modal stack.

/// **The wiring for O-3.** The stack utility only fixes the paint order if the ORBAT surface
/// actually consumes it. `OrbatManagerDialog`'s scrim and panel must derive their z from
/// `modal_stack::z_class` rather than a literal `z-50`; a body that still hard-codes `z-50` on
/// the ORBAT overlay is the unfixed component (the Arsenal keeps its literal `z-50` by design — it
/// is the surface ORBAT yields *to*).
#[test]
fn orbat_manager_overlay_derives_z_from_the_modal_stack() {
    use frontend_test_support::class_r_scrub::{live_code, only_body};
    let scrubbed = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/ui/modals/orbat_manager/dialog.rs"
    )));
    let body = only_body(&scrubbed, "pub fn OrbatManagerDialog(");
    assert!(
        body.contains("modal_stack::z_class(modal_id)"),
        "OrbatManagerDialog must take its overlay z from modal_stack::z_class. \
         Body was: {body}"
    );
}
