use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

fn gate_needles() -> (String, String, String) {
    (
        ["modal_stack", "::", "register("].concat(),
        ["modal_stack", "::", "is_topmost_open(modal_id)"].concat(),
        ["modal_stack", "::", "unregister(modal_id)"].concat(),
    )
}

/// The overlay components live in `editor/canvas/overlays.rs` (T-934.11). That file carries no
/// `#[cfg(test)]`, so `live_code` scrubs it whole.
fn overlays_region() -> String {
    live_code(include_str!("../canvas/overlays.rs"))
}

/// Page body — hosts the shared measure-tool Escape arm.
fn page() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

/// Asset picker / comment editor / connections panel each register and gate Escape.
#[test]
fn editor_overlay_esc_listeners_gate_on_modal_stack() {
    let region = overlays_region();
    let (reg, top, unreg) = gate_needles();
    // Markers reassembled so this module is not a second occurrence for only_body.
    let components = [
        format!("{}{}", "fn Asset", "PickerOverlay("),
        format!("{}{}", "fn Comment", "EditorOverlay("),
        format!("{}{}", "fn Connections", "PanelOverlay("),
    ];
    for component in &components {
        let body = only_body(&region, component);
        assert!(
            body.contains(&reg),
            "T-726: {component} must register with the modal stack"
        );
        assert!(
            body.contains(&top),
            "T-726: {component} must gate Escape on is_topmost_open — else stacked Esc pile-up"
        );
        assert!(
            body.contains(&unreg),
            "T-726: {component} must unregister on cleanup"
        );
    }
}

/// The shared ruler/LoS/viewshed Escape arm yields while any overlay claims Esc.
#[test]
fn measure_tool_escape_arm_yields_when_modal_stack_has_open() {
    let code = page();
    let any = ["modal_stack", "::", "any_open()"].concat();
    assert!(
        code.contains(&any),
        "T-726: measure Esc arm must consult modal_stack::any_open() so an open menu/dialog \
         consumes Esc alone (wave108 MAJOR-2). Hollow: delete the any_open guard → RED."
    );
    let any_at = code.find(&any).expect("any_open present");
    let ruler = format!("{}{}", "ruler.borrow_mut().", "escape()");
    let los = format!("{}{}", "los.borrow_mut().", "escape()");
    let viewshed = format!("{}{}", "viewshed.borrow_mut().", "escape()");
    let ruler_at = code.find(&ruler).expect("ruler escape in arm");
    let los_at = code.find(&los).expect("los escape in arm");
    let viewshed_at = code.find(&viewshed).expect("viewshed escape in arm");
    assert!(
        any_at < ruler_at && any_at < los_at && any_at < viewshed_at,
        "T-726: any_open() must precede measure .escape() calls (yield before act)"
    );
}

/// Hollow-pin canary: stripping `any_open` from an in-memory copy must break the pin needle.
#[test]
fn measure_any_open_guard_is_load_bearing() {
    let code = page();
    let any = ["modal_stack", "::", "any_open()"].concat();
    assert!(code.contains(&any), "canary: real page carries any_open");
    let perturbed = code.replacen(&any, "false /* hollow */", 1);
    assert!(
        !perturbed.contains(&any),
        "fired rule: deleting any_open must break the T-726 measure Esc pin"
    );
}
