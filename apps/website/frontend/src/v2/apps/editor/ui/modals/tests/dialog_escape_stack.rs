use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

fn prod() -> String {
    live_code(include_str!("../settings_modal.rs"))
}

fn gate_needles() -> (String, String, String) {
    (
        ["modal_stack", "::", "register("].concat(),
        ["modal_stack", "::", "is_topmost_open(modal_id)"].concat(),
        ["modal_stack", "::", "unregister(modal_id)"].concat(),
    )
}

#[test]
fn settings_dialogs_gate_escape_on_modal_stack() {
    let code = prod();
    let (reg, top, unreg) = gate_needles();
    for component in [
        "pub fn MissionSettingsDialog(",
        "fn EditorPreferencesDialog(",
        "fn AllSettingsDialog(",
    ] {
        let body = only_body(&code, component);
        assert!(
            body.contains(&reg),
            "T-726: {component} must register with the modal stack"
        );
        assert!(
            body.contains(&top),
            "T-726: {component} must gate Escape on is_topmost_open (stacked Esc)"
        );
        assert!(
            body.contains(&unreg),
            "T-726: {component} must unregister on cleanup"
        );
    }
}

/// Prefs mounts after settings in the parent view — registration order IS paint order.
#[test]
fn prefs_and_all_settings_mount_after_settings_body() {
    let code = prod();
    let body = only_body(&code, "pub fn MissionSettingsDialog(");
    let prefs_at = body
        .find("EditorPreferencesDialog")
        .expect("prefs mount in MissionSettingsDialog");
    let all_at = body
        .find("AllSettingsDialog")
        .expect("all-settings mount in MissionSettingsDialog");
    // Both sibling mounts must appear after the settings dialog's own modal_stack::register
    // so they paint/register on top when open.
    let reg = ["modal_stack", "::", "register("].concat();
    let reg_at = body.find(&reg).expect("settings registers");
    assert!(
        prefs_at > reg_at && all_at > reg_at,
        "T-726: prefs/all-settings must mount after settings registers (topmost when open)"
    );
}

/// T-936.6 — a registered panel that is never mounted cannot fire. Needle is assembled so this
/// test cannot become its own haystack.
#[test]
fn spawn_modules_panel_is_mounted_in_mission_settings() {
    let code = prod();
    let body = only_body(&code, "pub fn MissionSettingsDialog(");
    let needle = ["spawn_modules", "_panel("].concat();
    assert!(
        body.contains(&needle),
        "T-936.6: spawn_modules_panel must be mounted in MissionSettingsDialog"
    );
}
