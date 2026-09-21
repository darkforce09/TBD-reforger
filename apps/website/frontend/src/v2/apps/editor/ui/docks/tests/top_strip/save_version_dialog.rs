//! Save version dialog tests for the top command strip.

/// T-789 F-04 — the Save Version dialog: clamped on-screen, fresh state on reopen, focus moved into
/// the version input and Tab trapped inside the dialog. Source-scrub pins (the mechanical lane); the
/// live-DOM acceptance (activeElement / 8-Tab / two-viewport rects) is the operator playtest lane.
///
/// STACK-REGISTER DECISION (recorded here so the pin file carries it): the Save dialog stays
/// **unregistered** with `modal_stack`. It is hand-rolled markup, not the shared `ui::Dialog`, and
/// its Esc-close already runs on the strip's proven window listener (`save_open.set(false)`, guarded
/// by the T-814 `escape_consumed()` ladder pinned in `t726_top_strip_esc_stack`). Registering it
/// would make that same guard swallow its own Esc (an open overlay ⇒ `escape_consumed()` true ⇒ the
/// strip arm returns before `save_open.set(false)`) — the exact T-726 trap the wave-200 note flags —
/// forcing a re-proof of the whole ladder for no acceptance-criteria gain: fresh-state/focus/trap are
/// all achievable dialog-locally (these pins lock them), and the wave-203 Portal move does NOT change
/// that — the Esc-close still rides the window-level keydown listener (position-independent), so the
/// T-726/T-814 ladder is untouched by teleporting the dialog to `document.body`. The on-screen clamp
/// is the one property that could NOT be proven by construction (an ancestor `backdrop-filter` broke
/// it, wave-203 MAJOR); it is now proven by the live-rect smoke in `tools_v2/developer-tools`, not a class pin.
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// FRESH STATE. `save_status` is a shared prop (it also paints inline in the strip) and
/// `save_now` writes it to `Saved v{semver}`, where it stays — so on reopen the dialog would
/// greet the author with the *previous* save's line. An Effect on `save_open` must clear both
/// `save_status` and the `save_findings` list on the closed→open edge. Scrubbed `live_code`, so
/// the needles are the actual sets, not a mention in a comment/string.
#[test]
fn clears_stale_status_on_the_reopen_edge() {
    let code = live_code(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    // The rising-edge guard: an Effect that reads save_open and a was-open cell.
    assert!(
        body.contains("save_was_open"),
        "T-789: the strip must track a closed→open edge for the Save dialog (save_was_open cell)"
    );
    let eff_at = body
        .find("Effect::new")
        .and_then(|start| body[start..].find("save_open.get()").map(|o| start + o))
        .expect("T-789: an Effect must read save_open to catch the reopen edge");
    let region = &body[eff_at..];
    // Within that Effect, both the status line and the findings list are cleared.
    for needle in ["save_status.set(", "save_findings.set(", "rising"] {
        assert!(
            region.contains(needle),
            "T-789 F-04: the reopen Effect must run `{needle}` so a stale `Saved vX` (and the \
             rejected-save findings) do not survive into the next open. Hollow: delete the \
             clear → this pin goes RED and the dialog reopens showing the last save."
        );
    }
}

/// FOCUS-IN. The version input is the one decision the dialog demands; it must own focus the
/// instant the dialog paints (the review's blind-typeable-offscreen note is why focus-first
/// matters). NodeRef + on_load focus/select — the T-785/T-811 pattern (a bare autofocus on a
/// reactive insert does not fire). Same shape the eden_tree / eden_dock_left rename pins assert.
#[test]
fn version_input_takes_focus_on_open() {
    let code = live_code(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    assert!(
        body.contains("let version_ref = NodeRef::<leptos::html::Input>::new()"),
        "T-789: the Version input needs a NodeRef so on_load can focus it"
    );
    assert!(
        body.contains("node_ref=version_ref"),
        "T-789: the NodeRef must be attached to the Version input via node_ref=version_ref"
    );
    // The on_load handler that owns the version_ref calls focus() (and select()).
    let onload_at = body
        .find("version_ref")
        .and_then(|s| body[s..].find(".on_load(").map(|o| s + o))
        .expect("T-789: version_ref must carry an on_load");
    let region = &body[onload_at..onload_at + 200.min(body.len() - onload_at)];
    assert!(
        region.contains(".focus()") && region.contains(".select()"),
        "T-789 F-04: version_ref.on_load must focus() (and select()) the input so activeElement \
         is the Version field on open, not the opener button. Hollow: drop the focus() call \
         → RED, and focus stays on the opener."
    );
}

/// TAB TRAP. Before this the Tab cycle ✕ → version → notes → Save walked out into the left dock
/// with no wrap. The dialog container must carry `on:keydown=trap_tab`, and `trap_tab_in_dialog`
/// must (a) act only on Tab, (b) enumerate the dialog's own focusables, and (c) wrap at both
/// edges (prevent_default + refocus). `trap_tab_in_dialog` has two cfg-gated defs, so scrub the
/// whole source and match on substrings rather than `only_body`.
#[test]
fn traps_tab_within_the_dialog_subtree() {
    let code = live_code(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    assert!(
        body.contains("on:keydown=trap_tab"),
        "T-789 F-04: the Save dialog container must wire on:keydown=trap_tab so Tab is trapped. \
         Hollow: remove the handler → RED, and Tab walks into the left dock."
    );
    assert!(
        body.contains("node_ref=dialog_ref"),
        "T-789: the trap needs the dialog container NodeRef (node_ref=dialog_ref) to scope its \
         focusables to this subtree"
    );
    // The wasm trap body: Tab-only, queries focusables, wraps at the edges.
    let full = live_code(super::test_source::top_strip_source());
    let trap_at = full
        .find("fn trap_tab_in_dialog")
        .expect("T-789: trap_tab_in_dialog must exist");
    let region = &full[trap_at..];
    for needle in [
        "ev.key()",             // guard: only Tab is acted on
        "query_selector_all",   // enumerate this dialog's focusables
        "ev.prevent_default()", // stop the browser's default Tab move at the edge
        "ev.shift_key()",       // both directions
        "within(",              // pull focus back if it escaped the set
    ] {
        assert!(
            region.contains(needle),
            "T-789 F-04: trap_tab_in_dialog must contain `{needle}` — the trap enumerates the \
             dialog focusables and wraps at both edges (Shift+Tab off first → last; Tab off \
             last → first)."
        );
    }
}

/// CLAMP — CLASS GUARD ONLY; THE REAL GUARD IS THE LIVE-RECT SMOKE.
///
/// wave-203 correction: this pin's old name and old prose claimed the Version field was on-screen
/// "by construction" from `top-1/2 … -translate-y-1/2` + `max-h-[85vh]`. That was FALSE, and
/// "construction" was exactly what lied: `position:fixed` centers on the nearest containing block,
/// and the strip's `backdrop-filter` glass root (`STRIP_ROWS`) — an ANCESTOR of this dialog —
/// established one, so the dialog centered on the 48px strip and the Version input rendered at
/// y=-22 (1920×1080) / y=-184 (1366×768), off the top edge (verifier wave203 MAJOR, CDP-measured;
/// removing the ancestor filter snapped it to y=423 — causation proven). The fix PORTALS the
/// dialog to `document.body` (see the mount), so the containing block is now the viewport.
///
/// A class-string pin CANNOT catch that class of failure — the offending classes were all present
/// and correct; the geometry was wrong because of an ancestor. So the AUTHORITATIVE guard is now
/// the live-Chrome rect smoke `smoke_save_dialog_rect` (`gate smoke save-dialog-rect`) in
/// `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests.rs` (real `getBoundingClientRect`, both viewports, in the wave
/// gate). This test remains only as a cheap source-scrub sentinel: it holds the centering classes
/// in place and forbids the upward-anchored (`top-full`) regression — but it does NOT and cannot
/// prove on-screen-ness. Never re-add a "by construction" claim here. `live_source` (classes).
#[test]
fn dialog_carries_the_centering_classes_rect_is_smoke_proven() {
    let code = live_source(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    // Anchor on the dialog's unique description copy (the button label "Save Version" also
    // appears earlier, so it is not a unique anchor). The description sits INSIDE the popup, so
    // the nearest centered-container class before it is the Save dialog's own.
    let desc_at = body
        .find("Versions are immutable")
        .expect("T-789: the Save Version dialog description copy must exist");
    let before = &body[..desc_at];
    let popup_at = before
        .rfind("fixed top-1/2 left-1/2")
        .expect("T-789 F-04: the Save dialog popup must be centered (fixed top-1/2 left-1/2)");
    let popup = &before[popup_at..];
    for needle in ["-translate-y-1/2", "max-h-[85vh]"] {
        assert!(
            popup.contains(needle),
            "T-789 F-04: the Save dialog popup must carry `{needle}` so it centers and caps its \
             height. (On-screen-ness is proven by the rect smoke, not here.) A regression to an \
             upward-anchored panel (top-full) would drop this."
        );
    }
    // And it must NOT be anchored upward from the button (the mechanism the review described).
    assert!(
        !popup.contains("top-full"),
        "T-789 F-04: the Save dialog popup must not anchor upward (top-full) — that is the \
         offscreen-Version-field mechanism the fix forbids."
    );
    // wave-203: the dialog must be teleported OUT of the strip's `backdrop-filter` glass root so
    // its `fixed` centering resolves against the viewport, not the 48px strip. The `<Portal>`
    // open tag is the escape hatch; deleting it re-nests the fixed dialog under
    // `STRIP_ROWS`'s `backdrop-blur-xl` (which establishes a containing block) → the exact
    // wave203 MAJOR (Version input at y=-22 / y=-184). This is a cheap source companion to the
    // authoritative rect smoke; keep both.
    assert!(
        body.contains("<Portal>"),
        "T-789 (wave-203): the Save dialog must be wrapped in a leptos::portal::Portal so it \
         mounts on document.body and escapes the strip's backdrop-filter containing block. \
         Removing the Portal reintroduces the off-top-of-viewport MAJOR — the rect smoke \
         (gate smoke save-dialog-rect) is the live proof; this is the source sentinel."
    );
}
