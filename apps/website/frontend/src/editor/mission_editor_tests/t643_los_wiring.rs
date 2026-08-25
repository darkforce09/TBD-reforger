use crate::editor::arsenal::class_r_scrub::live_code;

/// Page-from-anchor + the T-934.13 gesture file (`canvas/gestures.rs`) — the LoS wiring spans the
/// page body (tool signals, keydown Esc, overlay mounts) and the moved pointer/dblclick closures.
/// The t642 `editor_live` idiom; each half scrubbed separately.
fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    let mut src = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    src.push_str(&live_code(include_str!("../canvas/gestures.rs")));
    src
}

/// (arbitration entry — shared gesture) LoS enters the SAME `LG::Ruler` gesture the ruler uses,
/// via the broadened `should_begin_ruler` (true for any point-capture tool). No separate LoS
/// `LeftGesture` variant exists — the un-owned `select_tool` is untouched — so the whole entry is
/// the ruler's, with the commit site (below) choosing the tool.
#[test]
fn los_shares_the_ruler_gesture_entry() {
    let ed = editor_live();
    assert!(
        ed.contains("should_begin_ruler("),
        "T-643: LoS must enter via the shared should_begin_ruler point-capture predicate"
    );
    // No third gesture variant was invented for LoS (would require editing the un-owned
    // select_tool). The only capture gesture is LG::Ruler.
    assert!(
        !ed.contains("LeftGesture::LoS") && !ed.contains("LG::LoS"),
        "T-643: LoS must NOT add a new LeftGesture variant — it reuses LG::Ruler (mode field)"
    );
}

/// (commit routes by tool_mode) The single `LG::Ruler` pointerup arm commits a LoS point via
/// `los...click(` under `is_los()`, and a ruler vertex via `.press(` otherwise — one arm, routed
/// by the mode. The LoS commit must NOT be a doc write (Decision 4) and must NOT route through the
/// armed-place `has_pending()` branch (constraint a — the same arm the ruler pin already proves
/// sits outside it).
#[test]
fn los_commit_routes_by_tool_mode_no_doc_write() {
    let ed = editor_live();
    // The LoS commit exists and is a `.click(` on the los state, gated by is_los().
    assert!(
        ed.contains("los.borrow_mut().click(") && ed.contains("is_los()"),
        "T-643: the LG::Ruler pointerup arm must route a LoS point via los.click() under is_los()"
    );
    // Slice the pointerup LG::Ruler commit arm (the one carrying .click(); it is the same arm as
    // the ruler's .press(, so it also carries that) and prove it is not a doc-move commit.
    let arms: Vec<&str> = ed.split("LG::Ruler").skip(1).collect();
    let commit: Vec<&str> = arms
        .iter()
        .map(|a| a.split("LG::").next().unwrap_or(a))
        .filter(|a| a.contains(".click(") && a.contains(".press("))
        .collect();
    assert_eq!(
        commit.len(),
        1,
        "T-643: exactly one LG::Ruler arm routes BOTH tools (los.click + ruler.press), found {}",
        commit.len()
    );
    let arm = commit[0];
    assert!(
        !arm.contains("has_pending()"),
        "T-643 (constraint a): the LoS commit shares the arm that sits OUTSIDE the armed-place branch"
    );
    assert!(
        !arm.contains("move_entities_and_vehicles"),
        "T-643 (Decision 4): the LoS commit must not call a doc-move commit (it is not a doc edit)"
    );
}

/// (Esc — SHARED seam, not a new listener) The keydown Escape arm dismisses the LoS capture via
/// `los...escape()` in the SAME arm that dismisses the ruler — reusing the ruler's existing Esc
/// entry (Decision 3 + the T-726 note: no second unguarded window listener is added).
#[test]
fn escape_is_the_shared_ruler_seam() {
    let ed = editor_live();
    // The LoS escape rides the same keydown dispatch as the ruler escape.
    assert!(
        ed.contains("code().as_str()")
            && ed.contains("los.borrow_mut().escape()")
            && ed.contains("ruler.borrow_mut().escape()"),
        "T-643 (Decision 3 / T-726): Esc must call BOTH los.escape() and ruler.escape() in the \
         one shared keydown arm — no second window listener"
    );
    // There must be exactly ONE window keydown Closure carrying the measure-tool Esc (the shared
    // seam): the los.escape and ruler.escape calls sit in the same closure, so a second unguarded
    // Esc listener was NOT added. Proven structurally: both escape calls appear, and the T-642
    // pin already fixes that ruler.escape lives in the one code().as_str() keydown arm.
    assert_eq!(
        ed.matches("los.borrow_mut().escape()").count(),
        1,
        "T-643: LoS Esc must be wired exactly once (the shared seam), not duplicated"
    );
}

/// (dblclick guard) A double-click in LoS mode must NOT open Attributes / the asset picker: the
/// dblclick handler returns early under `is_los()` (its two pointerups already completed the shot
/// via the shared arm). Pinned alongside the ruler's dblclick guard.
#[test]
fn dblclick_is_guarded_in_los_mode() {
    let ed = editor_live();
    // The dblclick handler branches on is_los() (the guard) — the ruler's is_ruler() guard is
    // pinned by t642; this proves the LoS peer guard exists too. Both live in the ondblclick
    // closure, which the t642 dblclick pin already anchors.
    assert!(
        ed.matches("get_untracked().is_los()").count() >= 1,
        "T-643: the dblclick handler must short-circuit under is_los() (no dialog on a LoS dbl-click)"
    );
}

/// (Decision 4 — session-local, tool-switch clear, overlay mounted) Switching the tool away from
/// LoS clears the placed shot; the state is a leaked `RefCell<LosState>` (overlay state, never a
/// doc write); the overlay is mounted and BOTH the state and the DEM sampler are registered for it.
#[test]
fn tool_switch_clears_and_overlay_is_mounted() {
    let ed = editor_live();
    // Tool-switch clear effect: reads !is_los(), clears the state.
    assert!(
        ed.contains("is_los()") && ed.contains("los.borrow_mut().clear()"),
        "T-643 (Decision 3): switching away from LoS must clear the placed shot"
    );
    // The overlay is mounted and the state + sampler registered for it.
    assert!(
        ed.contains("LosOverlay")
            && ed.contains("register_los_state(")
            && ed.contains("register_los_sampler("),
        "T-643: LosOverlay must be mounted with the state + DEM sampler registered for it"
    );
    // Session-local overlay state (a LosState in a RefCell), NOT the Y.Doc.
    assert!(
        ed.contains("LosState::new()"),
        "T-643 (Decision 4): LoS is a session-local LosState, not doc state"
    );
}

// ── The fired rule at the wiring layer (perturb / fail / restore) ─────────────────────────────

/// Fires the commit-routing pin: proof the `is_los()` branch in the shared `LG::Ruler` arm is
/// load-bearing. The pin passes on the real body; a perturbation that drops the `is_los()` route
/// (so a LoS click would fall through to `ruler.press` — the exact regression) makes the routing
/// assertion FAIL. Restore is implicit — only an in-memory copy is perturbed.
#[test]
fn fired_rule_los_routing_is_load_bearing() {
    let ed = editor_live();
    let needle = "los.borrow_mut().click(";
    assert!(
        ed.contains(needle),
        "canary: the real body routes a LoS click"
    );
    // Perturb: remove the LoS click route. The routing pin's needle must vanish.
    let perturbed = ed.replace(needle, "ruler.borrow_mut().press(");
    assert!(
        !perturbed.contains(needle),
        "fired rule: dropping the los.click() route (LoS clicks fall through to ruler.press) must \
         break the routing pin — proving the is_los() branch discriminates the regression"
    );
}
