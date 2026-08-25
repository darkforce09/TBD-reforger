use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

/// Both lanes must be driven from the same unfiltered `ids`, and the T-425 slot-only pre-filter
/// must be gone from the drag branch — that filter WAS the bug (vehicles never previewed).
#[test]
fn drag_preview_feeds_the_whole_mixed_selection_to_both_lanes() {
    let tool = live_code(include_str!("../tools/select_tool.rs"));
    let push = only_body(&tool, "pub fn push_drag_preview(");
    assert!(
        push.contains("e.set_drag(ids.to_vec()"),
        "the slot lane must get the WHOLE id list — set_drag skips ids it cannot resolve, so \
         filtering vehicles out first only ever cost the vehicle preview"
    );
    assert!(
        push.contains("pack_vehicle_drag_preview("),
        "the vehicle lane must be re-packed with the dragged rows offset"
    );
    assert!(
        push.contains("bind_vehicle_preview_lane("),
        "…and uploaded, or the re-pack never reaches the GPU (T-808 moved the upload itself \
         into the shared binder so preview and restore cannot bind different lanes)"
    );
    assert!(
        !push.contains("is_vehicle_id"),
        "a preview that filters by kind is the defect this ticket cures"
    );

    // The un-committed exits must put the vehicle lane back: it is live state during a drag now.
    let clear = only_body(&tool, "pub fn clear_drag_preview(");
    assert!(
        clear.contains("e.set_drag(Vec::new()") && clear.contains("bind_vehicle_preview_lane("),
        "clearing the preview must drop BOTH lanes, not just the slot overlay"
    );

    // `class_r_scrub::cut_test_module` cuts from the **first** `#[cfg(test)]` to EOF, and this
    // file has one at ~line 88 (`registry_session::clear_for_test`, a test-only helper inside a
    // production module). Scrubbing the whole file therefore examines only its first ~90 lines
    // and would report every needle below as absent — which is how this assertion first failed,
    // and why the scrubber is worth having: it refused rather than guessed. So hand it the
    // region from the next top-level item onward. The cut is at brace depth 0 between complete
    // items, so the slice stays balanced and the scrubber's own cut still fires on the real
    // test modules below.
    // Split so the anchor literal is not itself a second occurrence in this file (the t427
    // pin below uses the same trick for the same reason).
    let anchor = format!("{}{}", "const REGISTRY_", "COLD_PAGE");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "the scrub anchor must be unambiguous — 0 or 2+ means this pin is reading a region it \
         cannot identify"
    );
    let editor = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    assert!(
        editor.contains("pub fn MissionEditorPage"),
        "canary: the scrubbed region must still contain the editor page, or the anchor moved \
         and this pin is examining almost nothing"
    );
    assert!(
        editor.contains("st::push_drag_preview("),
        "the pointermove drag branch must push the preview through the shared helper"
    );
    assert!(
        !editor.contains("e.set_drag(slot_ids"),
        "the drag branch must no longer feed set_drag a vehicle-filtered id list"
    );
    assert!(
        editor.contains("st::clear_drag_preview("),
        "the no-move release and the pointercancel must restore the vehicle lane"
    );
}

/// **Calibration.** Every needle above must stop being satisfied once the code it names is
/// dead, or this pin could report a live mixed-drag preview over code the build never runs —
/// which is the exact shape of defect the ticket is about, relocated into the test.
#[test]
fn the_preview_pin_rejects_every_dead_code_wrapper() {
    let needle = "pack_vehicle_drag_preview(";
    let attacks: [(&str, String); 8] = [
        ("if false", format!("if false {{ {needle}); }}")),
        (
            "if true == false",
            format!("if true == false {{ {needle}); }}"),
        ),
        ("while false", format!("while false {{ {needle}); }}")),
        ("loop { break; … }", format!("loop {{ break; {needle}); }}")),
        (
            "#[cfg(any())] item",
            format!("#[cfg(any())] fn d() {{ {needle}); }}"),
        ),
        (
            "#[cfg(any())] mod shadow",
            format!("#[cfg(any())] mod s {{ fn d() {{ {needle}); }} }}"),
        ),
        ("after return", format!("fn d() {{ return; {needle}); }}")),
        ("comment", format!("// {needle})")),
    ];
    for (label, body) in attacks {
        let forged = format!("pub fn push_drag_preview() {{\n    {body}\n}}\n#[cfg(test)]\n");
        assert!(
            !live_code(&forged).contains(needle),
            "{label}: the vehicle re-pack needle survived scrubbing — this pin would report a \
             live mixed-drag preview over code that never runs"
        );
    }
    // A second definition is how a pin gets fed a pristine decoy while the real one is gutted.
    let two = "pub fn push_drag_preview() { good(); }\n\
               mod real { pub fn push_drag_preview() { bad(); } }\n#[cfg(test)]\n";
    let scrubbed = live_code(two);
    let caught =
        std::panic::catch_unwind(|| only_body(&scrubbed, "pub fn push_drag_preview(")).is_err();
    assert!(
        caught,
        "two definitions must be RED, not a coin flip over which one ships"
    );
    // …and the honest shape must still satisfy the needle, or the battery proves nothing.
    let live = format!("pub fn push_drag_preview() {{\n    {needle});\n}}\n#[cfg(test)]\n");
    assert!(live_code(&live).contains(needle));
}
