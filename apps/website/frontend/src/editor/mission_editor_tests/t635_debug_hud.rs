use crate::editor::arsenal::class_r_scrub::{live_code, live_source};

/// The editor page region with comments stripped but string literals KEPT (so Tailwind class
/// strings survive as structural landmarks). Same slice boundary as `editor_live`.
fn editor_src() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    live_source(&raw[raw.find(anchor.as_str()).expect("anchor present")..])
}

/// The editor page region with comments stripped and string literals blanked — same slice the
/// t662 module uses (from `pub fn MissionEditorPage()` onward, at a brace-0 boundary).
fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
}

/// (a) Ctrl/Cmd+Alt+D is a keydown arm that toggles the HUD, and (a') the closure honours the
/// editable-field guard so it never fires while typing.
#[test]
fn ctrl_alt_d_toggles_the_hud_behind_the_editable_guard() {
    let ed = editor_live();
    // The arm: modifier + Alt + not Shift, keyed on KeyD (literals blanked, so the guard shape
    // is what we pin, not the "KeyD" string — the code idiom, not a mention).
    assert!(
        ed.contains("if modk && ev.alt_key() && !ev.shift_key() =>")
            && ed.contains("debug_hud_shown.set(!debug_hud_shown.get_untracked())"),
        "T-635: Ctrl/Cmd+Alt+D must be a keydown arm that toggles debug_hud_shown"
    );
    // The whole keydown closure sits behind the editable-field guard (shared with copy/paste/
    // Backspace) — a HUD toggle must not fire while the operator types in an Attributes field.
    assert!(
        ed.contains("if mission_history::in_editable_field() {"),
        "T-635: the keydown closure must guard on in_editable_field() before acting"
    );
    // The literal binding is present on the raw file too (live_code blanks it above).
    let raw = include_str!("../mission_editor.rs");
    assert!(
        raw.contains("\"KeyD\" if modk && ev.alt_key()"),
        "T-635: the toggle must be bound to the D key"
    );
}

/// (b) The HUD defaults HIDDEN: `debug_hud_shown` is a real signal seeded `false`.
#[test]
fn the_hud_defaults_hidden() {
    let ed = editor_live();
    assert!(
        ed.contains("let debug_hud_shown = RwSignal::new(false)"),
        "T-635: debug_hud_shown must be a real RwSignal defaulting to false (hidden)"
    );
}

/// (c) T-636/T-719: the HUD is NO LONGER a free-floating overlay corner — it moved into the
/// full-width status bar's right section (its real visible home; the old `right-3 bottom-3`
/// overlay div had no z-index and was painted over by DockRight's z-20 column). From
/// `mission_editor`'s side the proof is: (1) the standalone overlay HUD div is gone, and (2) the
/// HUD signals are fed into `StatusBar`, which sits behind a `chrome_hidden` gate — so the
/// chrome-hidden half of the T-635 gate is preserved. The `hud_shown`-AND-non-empty half is
/// pinned inside `eden_toolbelt` (see `t636_status_bar`).
#[test]
fn the_hud_moved_into_the_gated_status_bar() {
    let src = editor_src();
    // (1) The retired overlay corner must be gone — no free-floating `right-3 bottom-3` HUD div.
    assert!(
        !src.contains("absolute right-3 bottom-3 font-mono"),
        "T-636: the free-floating overlay HUD corner must be gone (it moved into the status bar)"
    );
    // (2) The status-bar mount passes the HUD signals in, and it sits behind a chrome_hidden
    // gate. Pinned on `live_code` (comments/strings blanked) so this is the real wiring, not a
    // comment: the `debug_hud` + `hud_shown=debug_hud_shown` props reach `StatusBar`.
    let ed = editor_live();
    assert!(
        ed.contains("StatusBar")
            && ed.contains("debug_hud")
            && ed.contains("hud_shown=debug_hud_shown"),
        "T-636: the HUD text + toggle must be threaded into StatusBar (debug_hud + hud_shown)"
    );
    // The StatusBar mount must be one of the `(!chrome_hidden.get()).then(` gated wrappers, so
    // hiding the chrome unmounts the HUD too (the chrome_hidden half of the T-635 gate stack).
    let belt = ed
        .find("crate::editor::panels::toolbelt::StatusBar")
        .expect("StatusBar mount present");
    let gate = ed[..belt]
        .rfind("(!chrome_hidden.get()).then(")
        .expect("StatusBar must be preceded by a chrome_hidden gate");
    // Nothing but the wrapper div opens between the gate and the StatusBar mount — i.e. the gate
    // is the StatusBar's own wrapper, not an earlier mount's.
    assert!(
        !ed[gate..belt].contains("crate::editor::panels::toolbelt::ModeToolbar")
            && !ed[gate..belt].contains("crate::editor::eden_chrome::Dock"),
        "T-636: the chrome_hidden gate immediately preceding StatusBar must be its OWN wrapper"
    );
}

/// (d) The telemetry-vs-diagnostics distinction is stated explicitly in a PRODUCTION code
/// comment — the framework_synthesis §D.4 #7 requirement that this key-gating pattern not be
/// copied onto mission-correctness diagnostics. The scrubbers blank comments, so this is pinned
/// on the raw file, sliced to the `MissionEditorPage` production body (from its anchor to the
/// first `#[cfg(test)]` module that follows it) so the test modules' own text — including this
/// docstring — cannot satisfy the pin. The comment must really ship in the page's source.
#[test]
fn the_telemetry_vs_diagnostics_distinction_is_documented() {
    let raw = include_str!("../mission_editor.rs");
    // Window: `MissionEditorPage`'s definition … first test module after it. The file's FIRST
    // `#[cfg(test)]` is a `clear_for_test` helper near the top (well above the page), so slice
    // from the page anchor forward, then cut at the next test module. (Both needles split so
    // this line is not itself the boundary it searches for.)
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let page_at = raw.find(anchor.as_str()).expect("page anchor present");
    let boundary = format!("#[cfg{}", "(test)]");
    let after_page = &raw[page_at..];
    let prod_end_rel = after_page
        .find(boundary.as_str())
        .expect("a test module follows the page");
    let prod = &after_page[..prod_end_rel];
    // The rule reference (reassembled so this line is not a decoy match inside the window if the
    // production comment were deleted — the needle must be found in real shipped source).
    let rule = format!("framework_synthesis {}D.4 #7", "\u{a7}");
    assert!(
        prod.contains(rule.as_str()),
        "T-635: the §D.4 #7 rule reference must be present in a production comment"
    );
    assert!(
        prod.contains("Mission-correctness diagnostics") && prod.contains("NEVER gated"),
        "T-635: the comment must state that mission-correctness diagnostics are never gated"
    );
    // And it must frame the HUD itself as telemetry (the thing that IS legitimately gated).
    assert!(
        prod.contains("TELEMETRY"),
        "T-635: the comment must classify the HUD as telemetry (the gatable kind)"
    );
}
