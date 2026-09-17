//! Menu state vocabulary tests for the top command strip.

/// T-668 — the top strip speaks the one state vocabulary. The headline conversion is here: the OPEN
/// menu-bar button now wears TOGGLED_PLATE (plate + dark top border) where before it wore a bare
/// `bg-white/10` — byte-identical to every neighbour's hover, so "open" and "hovered" were
/// indistinguishable. Source-inspection pins on scrubbed source, since the strip is a Leptos view a
/// native test cannot render. Needles are assembled from fragments so the file's own prose can never
/// satisfy an absence check (the house rule).
use super::{MenuAction, MENUS};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// This file with comments blanked but class STRINGS kept, so the Tailwind literals survive as
/// the structural landmarks the class pins read.
fn src_kept() -> String {
    live_source(include_str!("../../top_strip.rs"))
}

/// The open menu-bar button consumes TOGGLED_PLATE (via `cn`), and the closed one HOVER_FILL —
/// the one state vocabulary, so "this menu is open" reads like every other toggle and can never
/// be confused with "the pointer is over this menu". Proven on scrubbed CODE (literals blanked)
/// so the needle is the real `cn(&[…, TOGGLED_PLATE])` call, not a mention.
#[test]
fn open_menu_wears_the_toggled_plate_not_the_hover_fill() {
    let code = live_code(include_str!("../../top_strip.rs"));
    assert!(
        code.contains("TOGGLED_PLATE"),
        "the open menu must consume TOGGLED_PLATE (plate + 1px dark top border)"
    );
    assert!(
        code.contains("HOVER_FILL"),
        "the closed menu (and other neutral controls) must consume HOVER_FILL"
    );
}

/// THE FIX, stated as an absence: the open-menu branch must NOT be a bare `bg-white/10` string
/// (the defect — an active state wearing the neutral hover fill). The needle is assembled so this
/// test's own source cannot satisfy it, and it is checked on the string-kept source where a class
/// literal is real. TOGGLED_PLATE carries `bg-primary/20`, not `bg-white/10`, so a compliant
/// strip has no `bg-white/10` literal used as a persistent (non-`hover:`) fill.
#[test]
fn no_active_state_wears_the_bare_neutral_fill() {
    let src = src_kept();
    // A persistent neutral fill would appear as ` bg-white/10` WITHOUT a `hover:` prefix. Every
    // legitimate use in the chrome is `hover:bg-white/10` (a hover) or the DIVIDER hairline
    // (`h-5 w-px bg-white/10`, non-interactive). Assemble the needle so prose can't be the match.
    let persistent = ["bg-", "white/10"].concat();
    let hover = ["hover:bg-", "white/10"].concat();
    let hairline = ["w-px bg-", "white/10"].concat(); // the DIVIDER recipe — allowlisted
                                                      // Count bare occurrences that are neither a hover nor the hairline divider.
    let mut bare = 0usize;
    let mut i = 0usize;
    while let Some(off) = src[i..].find(&persistent) {
        let at = i + off;
        let is_hover = at >= 6 && src[at - 6..].starts_with(&hover);
        let is_hairline = at >= 5 && src[at - 5..].starts_with(&hairline);
        if !is_hover && !is_hairline {
            bare += 1;
        }
        i = at + persistent.len();
    }
    assert_eq!(
        bare, 0,
        "T-668: no active/persistent `bg-white/10` may remain in the strip — an active state \
         must wear TOGGLED_PLATE (bg-primary/20 + top border), not the neutral hover fill"
    );
}

/// Convention — every top-strip menu row reserves the checkmark gutter UNCONDITIONALLY, so labels
/// do not shift between menus (Eden's jumping indent is the bug NOT to copy). Both the enabled and
/// the disabled (future-command) row branches lead with a `MENU_GUTTER` cell.
#[test]
fn menu_rows_reserve_the_checkmark_gutter() {
    let code = live_code(include_str!("../../top_strip.rs"));
    assert!(
        code.contains("MENU_GUTTER"),
        "menu rows must reserve MENU_GUTTER (the always-present checkmark cell)"
    );
    // Both branches use it — count ≥ 2 gutter cells in the rendered rows.
    let cells = code.matches("class=MENU_GUTTER").count();
    assert!(
        cells >= 2,
        "both the enabled and the future-command menu rows must lead with the gutter (found {cells})"
    );
}

/// **wave-202 — the Edit menu's widget / snap / Select-All rows are LIVE COMMANDS now.** They
/// shipped `action: None` (chord-labelled, disabled) because the editor state they drive had no
/// cross-file bridge; wave-202 added `register_editor_toolbar_dispatch` (the `register_widget_pivot`
/// pattern), so each row now carries a real `MenuAction` that dispatches through it. This pin was
/// "a disabled keyboard-only row keeps its tooltip" (rule 3, for `action: None`); the honest
/// inversion is that NO Edit row is `action: None` any more — every one of these five is an
/// enabled command whose label still shows its chord. The T-668 dead-control rule and its
/// `disabled=true`+tooltip idiom stay green on the controls where a dispatch is GENUINELY absent
/// (History's version-list glyph, the scaffold-only gear / ORBAT), pinned by their own tests.
#[test]
fn edit_menu_widget_snap_rows_dispatch_not_disabled() {
    // The five rows that were `action: None` now carry live actions.
    let edit = MENUS
        .iter()
        .find(|(name, _)| *name == "Edit")
        .expect("T-797: the bar must carry an Edit menu");
    for (label, want) in [
        ("Select All on Screen (Ctrl+A)", MenuAction::SelectAll),
        // T-795 renumbering: `1` No Widget / `2` Translate / `3` Rotate.
        ("Widget: No Widget (1)", MenuAction::SetWidget(1)),
        ("Widget: Translation (2)", MenuAction::SetWidget(2)),
        ("Widget: Rotation (3)", MenuAction::SetWidget(3)),
        ("Toggle Snap Grid (G)", MenuAction::ToggleSnap),
        ("Snap Step — Decrease ([)", MenuAction::SnapStep(-1)),
        ("Snap Step — Increase (])", MenuAction::SnapStep(1)),
    ] {
        let row = edit
            .1
            .iter()
            .find(|it| it.label == label)
            .unwrap_or_else(|| panic!("T-797: the Edit menu must keep the `{label}` row"));
        assert!(
            matches!(row.action, Some(a) if std::mem::discriminant(&a) == std::mem::discriminant(&want)),
            "wave-202: `{label}` must DISPATCH (a live MenuAction), not ship `action: None`"
        );
    }
    // …and no Edit row is a dead keyboard-only affordance any longer.
    assert!(
        edit.1.iter().all(|it| it.action.is_some()),
        "wave-202: every Edit row is a live command now — none may be `action: None`"
    );
    // The dispatch reaches the editor's registered bridge (the write path, wasm-gated).
    let src = live_code(include_str!("../../top_strip.rs"));
    assert!(
        src.contains("with_editor_toolbar_dispatch"),
        "wave-202: the widget/snap/select-all actions must route through the editor bridge"
    );
}

/// **wave-202 MAJOR — the row-2 toggle plates are actually REACTIVE (the pin the verifier said was
/// missing).** The regression was a SUBSCRIPTION-ORDER bug, so this pins the order, not just the
/// presence of a signal: each plate getter (`widget_is` / `snap_on`) must read the dispatch
/// GENERATION signal (`toolbar_dispatch_generation()`) BEFORE it reads through
/// `with_editor_toolbar_dispatch`.
///
/// Why the order is the whole fix: the strip renders — and these `class=move || …` closures run —
/// BEFORE `mission_editor`'s `on_load` registers the dispatch. On that first pass the dispatch is
/// `None`, so `with_editor_toolbar_dispatch` fires nothing and the getter reads no tracked signal;
/// with no dependency, Leptos never re-runs the closure and the plate freezes at its first-render
/// default (Translate stuck lit, Snap dark — the verifier's live finding). Reading the generation
/// FIRST gives the closure a dependency that exists from frame one regardless of the dispatch, so a
/// register/unregister bump re-runs it — and THAT run, with the dispatch now present, subscribes to
/// the tracked `widget_variant` / `snap` getters. If the generation read is removed or moved AFTER
/// the dispatch read, the frozen shape returns and this pin goes RED.
///
/// Anti-hollow: the getter bodies are extracted from `live_code`-scrubbed source (comments folded,
/// string literals blanked, this test module cut) via `only_body`, which panics unless its marker
/// occurs EXACTLY once — a rename (0) or a shadow copy (2+) is RED, not a guess. The assertion is a
/// byte-offset ordering of two real calls, which no comment or literal can forge.
#[test]
fn plates_subscribe_to_dispatch_generation_before_reading_it() {
    let code = live_code(include_str!("../../top_strip.rs"));
    let gen_read = "toolbar_dispatch_generation";
    let dispatch_read = "with_editor_toolbar_dispatch";
    // Markers carry NO trailing `{` on purpose: `only_body` splits at the FIRST `{` after the
    // marker, so a brace in the marker would hand back the wrong (inner) block and drop the
    // generation read that sits above it. The bare signature lets `only_body` grab the closure's
    // own body — and it stays unique after the scrub cuts this test module + blanks its literals.
    for (marker, getter) in [
        ("let widget_is = move |digit: u8| -> bool", "widget_is"),
        ("let snap_on = move || -> bool", "snap_on"),
    ] {
        // `only_body` isolates THIS getter's balanced body and panics on 0 (renamed/deleted) or
        // 2+ (shadow decoy) — the pin refuses to examine code it cannot unambiguously find.
        let body = only_body(&code, marker);
        let gen_at = body.find(gen_read).unwrap_or_else(|| {
            panic!(
                "wave-202: `{getter}` must read the dispatch generation \
                 (`{gen_read}()`) so its plate closure subscribes from frame one — not found"
            )
        });
        let dispatch_at = body.find(dispatch_read).unwrap_or_else(|| {
            panic!(
                "wave-202: `{getter}` must still read the live state through `{dispatch_read}` \
                 — not found"
            )
        });
        assert!(
            gen_at < dispatch_at,
            "wave-202: `{getter}` must read the generation signal BEFORE reading through the \
             dispatch (the subscription order that makes the plate re-runnable). Found the \
             generation read at byte {gen_at}, the dispatch read at {dispatch_at} — reading the \
             dispatch first restores the frozen-plate regression."
        );
    }
}
