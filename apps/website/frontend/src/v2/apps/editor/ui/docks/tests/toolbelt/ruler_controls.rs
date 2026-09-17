use crate::v2::core::test_support::class_r_scrub::{live_code, live_source};

/// This file with comments blanked but strings KEPT (class strings + labels survive as landmarks).
fn src_kept() -> String {
    live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/toolbelt.rs"
    )))
}

/// (button enable) THE RULE: the Ruler button must NOT be a disabled stub any more — it drops
/// `disabled=true` and becomes a real `tool_mode` toggle. Proven by slicing the ModeToolbar body
/// and checking the Ruler button's window carries an `on:pointerdown` that sets `EditorTool::Ruler`
/// and NO `disabled=true`.
///
/// T-643 (wave 109) — the LoS button is now ALSO enabled (its wave-108 disabled forward-guard is
/// flipped): the same honesty rule now permits it because `los_tool` works end-to-end. The LoS
/// assertion below therefore mirrors the Ruler one — sets `EditorTool::LoS`, NO `disabled=true`.
#[test]
fn ruler_button_is_enabled_and_toggles_tool_mode() {
    let src = src_kept();
    let mode_at = src
        .find(&format!("fn {}", "ModeToolbar("))
        .expect("ModeToolbar present");
    // Body from ModeToolbar to the next component (StatusBar-adjacent code follows it here; the
    // TOOL_BASE const sits above, so slice forward to the next `pub fn`).
    let body_end = src[mode_at + 1..]
        .find("pub fn ")
        .map(|i| mode_at + 1 + i)
        .unwrap_or(src.len());
    let body = &src[mode_at..body_end];
    // The Ruler button toggles tool_mode to Ruler on press…
    let ruler_set = format!("tool_mode.set(EditorTool::{})", "Ruler");
    assert!(
        body.contains(&ruler_set),
        "T-642: the Ruler button must set tool_mode = Ruler on pointerdown"
    );
    // …and Select returns to Select.
    assert!(
        body.contains(&format!("tool_mode.set(EditorTool::{})", "Select")),
        "T-642: the Select button must set tool_mode = Select"
    );
    // The Ruler button's own window (its `straighten` glyph → the button close) carries NO
    // `disabled=true` — THE RULE this ticket exists to honour.
    let straighten_at = body.find("straighten").expect("Ruler glyph present");
    // Walk back to the <button that owns this glyph, forward to its glyph — the region between the
    // button open and the icon is where a `disabled` attr would live.
    let btn_open = body[..straighten_at]
        .rfind("<button")
        .expect("Ruler button open tag");
    let ruler_btn_head = &body[btn_open..straighten_at];
    let disabled_true = ["disabled=", "true"].concat();
    assert!(
        !ruler_btn_head.contains(&disabled_true),
        "T-642: the Ruler button must NOT be `disabled=true` — enabling a non-working stub is the \
         lie THE RULE forbids; it is enabled only because ruler_tool works end-to-end"
    );
    // T-643 — the LoS button is NOW enabled too (wave 109): its window sets tool_mode = LoS on
    // pointerdown and carries NO `disabled=true`, exactly like the Ruler check above. The
    // wave-108 "LoS must stay disabled" forward-guard is retired here — the honesty rule is met
    // because `los_tool` works end-to-end.
    assert!(
        body.contains(&format!("tool_mode.set(EditorTool::{})", "LoS")),
        "T-643: the LoS button must set tool_mode = LoS on pointerdown"
    );
    let los_at = body.find("visibility").expect("LoS glyph present");
    let los_open = body[..los_at].rfind("<button").expect("LoS button open");
    assert!(
        !body[los_open..los_at].contains(&disabled_true),
        "T-643: the LoS button must NOT be `disabled=true` — it is enabled only because los_tool \
         works end-to-end (the honesty rule this ticket, like T-642, exists to honour)"
    );
}

/// (status readout) The status bar renders the ruler's running-total + last-leg readout
/// (Decision 1) in the readout section, off a `ruler_status` prop, behind a `Some`-gate so it is
/// absent when no ruler is placed. Pinned on scrubbed code so the needle is the real prop + slot,
/// not a comment.
#[test]
fn status_bar_renders_the_ruler_readout() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/toolbelt.rs"
    )));
    // StatusBar accepts the ruler_status signal…
    assert!(
        code.contains("ruler_status"),
        "T-642: StatusBar must accept a ruler_status prop"
    );
    // …and the readout slot exists (data hook) and is Some-gated.
    let src = src_kept();
    assert!(
        src.contains(&format!("data-status-{}", "ruler")),
        "T-642: the status bar must have a ruler readout slot"
    );
    // The slot reads the signal (`.get()`), so it is live, not a static string.
    let hook = format!("data-status-{}", "ruler");
    let at = src.find(&hook).expect("ruler slot present");
    // The gate expression precedes the slot in the same view arm.
    let region_start = src[..at]
        .rfind("ruler_status")
        .expect("ruler_status read before slot");
    assert!(
        src[region_start..at].contains(".get()"),
        "T-642: the ruler readout must read ruler_status.get() (live, not a fixed string)"
    );
}

/// (Decision 4 — session-local, NOT doc state) `eden_toolbelt` renders the ruler readout from a
/// signal only; it must not reach into any document mutation. A light guard that the readout path
/// carries no doc-write token (the real no-doc-writes proof is the engine's
/// `the_ruler_never_writes_the_document` source scrub over `editing::tools::ruler` + the
/// compiler: `ruler_tool` never imports a doc mutator).
#[test]
fn readout_is_display_only_no_doc_writes() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/toolbelt.rs"
    )));
    for banned in ["move_entities", "add_slot", "store.rs", "MissionDocCore"] {
        assert!(
            !code.contains(banned),
            "T-642: the toolbelt ruler readout must be display-only — found `{banned}`"
        );
    }
}
