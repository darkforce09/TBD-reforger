use crate::editor::arsenal::class_r_scrub::{live_code, only_item};

/// The editor page region onward, comments stripped and string literals blanked — the same
/// slice `t635_debug_hud` uses. `start_raf` is defined after `MissionEditorPage`, so it is in.
fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    live_code(&raw[raw.find(anchor.as_str()).expect("anchor present")..])
}

/// The signal is a real signal seeded from the shared `m_per_px` conversion (not a bare float
/// literal), and it is threaded into the status bar — so the cell reads a true value before the
/// engine mounts, and on native, where `start_raf` never runs.
#[test]
fn the_scale_signal_is_seeded_and_reaches_the_status_bar() {
    let ed = editor_live();
    assert!(
        ed.contains(&format!(
            "let scale_mpp = RwSignal::new(crate::editor::panels::toolbelt::{}(-2.0))",
            "m_per_px"
        )),
        "T-670: scale_mpp must be a real signal seeded from eden_toolbelt::m_per_px at the \
         editor's default deck zoom"
    );
    let belt = ed
        .find("crate::editor::panels::toolbelt::StatusBar")
        .expect("StatusBar mount present");
    let close = ed[belt..]
        .find("/>")
        .map(|i| belt + i)
        .expect("the StatusBar mount closes");
    assert!(
        ed[belt..close].contains("scale_mpp"),
        "T-670: the scale signal must be passed into the StatusBar mount"
    );
}

/// **THE GUARD.** The sampler writes `scale_mpp` exactly once, and only inside an inequality
/// against the last PUBLISHED readout string. Delete the guard and this fails — which is the
/// point: the failure mode it prevents (a 60 fps Leptos write from a per-frame closure) is
/// invisible to a compile and to every other test in this crate.
#[test]
fn the_sampler_writes_the_scale_only_when_the_readout_changes() {
    let ed = editor_live();
    let raf = only_item(&ed, &format!("fn {}", "start_raf("));
    let set = format!("scale_mpp.{}(", "set");
    assert_eq!(
        raf.matches(set.as_str()).count(),
        1,
        "T-670: the sampler must have exactly ONE scale write — a second, unguarded one would \
         reintroduce the per-frame re-render"
    );
    let at = raf.find(set.as_str()).expect("counted above");
    // The write's enclosing block is the change guard, and the guard updates the remembered
    // string in the same block (otherwise it would fire on every frame after the first change).
    let guard = format!("if text != {} {{", "last_scale_text");
    let g = raf
        .find(guard.as_str())
        .unwrap_or_else(|| panic!("T-670: the scale write must sit behind `{guard}`"));
    assert!(
        g < at,
        "T-670: the change guard must OPEN before the scale write, not after it"
    );
    assert!(
        raf[g..at].contains(&format!("{} = text;", "last_scale_text")),
        "T-670: the guard must remember the published readout, or it fires every frame"
    );
    // The remembered value is a per-closure `mut` local, not a fresh binding each frame.
    assert!(
        raf.contains(&format!("let mut {} = String::new()", "last_scale_text")),
        "T-670: the last-published readout must live ACROSS frames (a closure-captured local)"
    );
}

/// The scale is read every frame and published promptly — it does NOT ride the ~1 Hz debug-HUD
/// sample. A zoom gesture must show on the next frame; hanging the readout off the 1 Hz block
/// would make it up to a second stale, and would also make the guard above pointless, hiding
/// the regression this ticket is about.
#[test]
fn the_scale_does_not_ride_the_one_hz_hud_sample() {
    let ed = editor_live();
    let raf = only_item(&ed, &format!("fn {}", "start_raf("));
    let scale = raf
        .find(&format!("scale_mpp.{}(", "set"))
        .expect("scale write present");
    let hud = raf
        .find(&format!("debug_hud.{}(", "set"))
        .expect("HUD write present");
    let sample_gate = raf
        .find("now - last_sample >= 1000.0")
        .expect("the ~1 Hz sample gate is still there");
    assert!(
        scale < sample_gate && scale < hud,
        "T-670: the scale must be published BEFORE (and outside) the ~1 Hz HUD sample block"
    );
}
