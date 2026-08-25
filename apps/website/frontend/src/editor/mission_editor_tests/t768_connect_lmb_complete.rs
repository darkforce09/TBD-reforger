use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

fn page() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

fn pointerup_body() -> String {
    only_body(&page(), "let onpointerup =").to_string()
}

/// The Pending click path must call `complete_connect` when a connect is armed — the only new
/// CALLER this ticket adds. RMB Complete (context_menu) must remain a separate caller.
#[test]
fn pending_click_calls_complete_connect_when_armed() {
    let up = pointerup_body();
    let pending_gate = ["editor_ops", "::", "pending_connect()"].concat();
    let complete = ["editor_ops", "::", "complete_connect("].concat();
    assert!(
        up.contains(&pending_gate),
        "T-768: pointerup must consult pending_connect() before an LMB complete"
    );
    assert!(
        up.contains(&complete),
        "T-768: LG::Pending click must call complete_connect(id) — the Eden LMB-target half. Hollow: delete that call → RED."
    );
    // Order: gate before complete inside the Pending arm.
    let pending_arm = up
        .split("LG::Pending")
        .nth(1)
        .expect("pointerup has an LG::Pending arm")
        .split("LG::Move")
        .next()
        .expect("Pending arm bounded by Move");
    let gate_at = pending_arm
        .find(&pending_gate)
        .expect("pending_connect in Pending arm");
    let complete_at = pending_arm
        .find(&complete)
        .expect("complete_connect in Pending arm");
    assert!(
        gate_at < complete_at,
        "T-768: pending_connect() must gate the LMB complete_connect call"
    );
}

/// Esc disarms an armed connect (same seam as T-723 place cancel).
#[test]
fn escape_arm_cancels_pending_connect() {
    let code = page();
    let cancel = ["editor_ops", "::", "cancel_connect()"].concat();
    assert!(
        code.contains(&cancel),
        "T-768: Esc arm must call cancel_connect() — Hollow: delete it → RED."
    );
    // Esc cancel sits with place cancel, before measure .escape() calls.
    let place_cancel = ["editor_ops", "::", "cancel_pending()"].concat();
    let place_at = code.find(&place_cancel).expect("place cancel in Esc arm");
    let connect_at = code.find(&cancel).expect("connect cancel present");
    let ruler = format!("{}{}", "ruler.borrow_mut().", "escape()");
    let ruler_at = code.find(&ruler).expect("ruler escape");
    assert!(
        place_at < connect_at && connect_at < ruler_at,
        "T-768: cancel_connect must sit with place disarm, before measure Esc"
    );
}

/// pointercancel must never commit a connect — drop the arm like place.
#[test]
fn pointercancel_cancels_pending_connect() {
    let code = page();
    let body = only_body(&code, "let onpointercancel =");
    let cancel = ["editor_ops", "::", "cancel_connect()"].concat();
    assert!(
        body.contains(&cancel),
        "T-768: pointercancel must cancel_connect (never a commit). Hollow: delete → RED."
    );
}

/// Hollow canary: stripping complete_connect from an in-memory pointerup copy breaks the pin.
#[test]
fn complete_connect_caller_is_load_bearing() {
    let up = pointerup_body();
    let complete = ["editor_ops", "::", "complete_connect("].concat();
    assert!(
        up.contains(&complete),
        "canary: real pointerup carries complete_connect"
    );
    let perturbed = up.replacen(&complete, "/* hollow */", 1);
    assert!(
        !perturbed.contains(&complete),
        "fired rule: deleting complete_connect must break the T-768 LMB pin"
    );
}
