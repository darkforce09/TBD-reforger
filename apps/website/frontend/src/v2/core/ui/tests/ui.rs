//! The shared primitives render what they claim, and one dismiss key closes one overlay.

/// Strip `//` / `/* */` so bans cannot false-red on doc comments.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '*' && matches!(chars.peek(), Some('/')) {
                            chars.next();
                            break;
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// AdminGate must not use browse-mode `has_min_role(None)=>true`.
/// Binds to the live `if` condition (same spirit as wiki Memo bind): a dead
/// `has_min_role_authed(...)` pin beside `if true` must FAIL. Bans browse-mode one-shot.
#[test]
fn admin_gate_uses_authed_reactive_role() {
    let src = crate::v2::core::test_support::pins::ui_source();
    let src: &str = &src;
    let production = src;
    let code = collapse_ws(&strip_rust_comments(production));
    // require the live `if` — presence of the helper call alone is false-green.
    assert!(
        code.contains("if has_min_role_authed(auth.user.get().map(|u| u.role), Role::Admin)"),
        "AdminGate must gate via `if has_min_role_authed(auth.user.get()…, Role::Admin)` \
         (dead pin + if true is a fail; browse-mode None=>true is a fail)"
    );
    // Mask the authed helper so a free `has_min_role(` / one-shot store call stands out.
    let masked = code.replace("has_min_role_authed", "HAS_MIN_ROLE_AUTHED");
    assert!(
        !masked.contains("has_min_role("),
        "AdminGate production must not call browse-mode has_min_role( — use has_min_role_authed only"
    );
    // Split the needle so this assert's own source text cannot false-red the include_str scan.
    let one_shot = format!("auth.has_min_role({}::Admin)", "Role");
    assert!(
        !code.contains(&one_shot),
        "auth.has_min_role(Admin) is browse-mode None=>true (/ contract)"
    );
}

/* ═══════════════ the shared search box ═══════════════ */

/// The primitive's contract, pinned where `cargo test` cannot instantiate a `view!` tree.
///
/// Scrubbed through `class_r_scrub` rather than this module's own `strip_rust_comments`,
/// because `scrub` cuts the TEST MODULE as its first pass — a bare `include_str!` scan would
/// otherwise be satisfied by the needles written in this very function.
#[test]
fn the_search_box_fires_on_every_keystroke_and_clears_through_the_same_callback() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_item};
    let code = only_item(
        &live_code(&crate::v2::core::test_support::pins::ui_source()),
        "pub fn SearchBox(",
    )
    .to_string();
    // `live_source` keeps string literals — every assertion below that is ABOUT a class recipe,
    // an element type or user-visible copy has to read this one, not `code`.
    let src = only_item(
        &live_source(&crate::v2::core::test_support::pins::ui_source()),
        "pub fn SearchBox(",
    )
    .to_string();
    // (1) It narrows as you type. `change` on a text input does not fire until blur, so a
    // search box bound to the settle event is a search box that does nothing while you use it.
    assert!(
        code.contains("on:input=move |ev| on_input.run(event_target_value(&ev))"),
        "SearchBox must emit on every keystroke; body was:\n{code}"
    );
    assert!(
        !code.contains("on:change="),
        "SearchBox must not bind the settle event — that is Slider/Select's constraint, not \
         a filter's"
    );
    // (2) Clearing is not a second, separate event. A caller wires ONE callback and gets both.
    assert!(
        code.contains("on:click=move |_| on_input.run(String::new())"),
        "the clear button must route through the same on_input callback"
    );
    // (3) No state of its own — the caller owns the query, exactly like Slider and Select.
    assert!(
        !code.contains("RwSignal::new(") && !code.contains("signal("),
        "SearchBox must stay uncontrolled: a private copy of the query is a second source of \
         truth for it"
    );
    // (4) the earlier precedent: the state vocabulary is CONSUMED, not re-typed here.
    assert!(
        code.contains("DISABLED_GLYPH") && code.contains("HOVER_FILL"),
        "SearchBox must compose the eden_layout state recipes"
    );
    assert!(
        !src.contains("disabled:opacity") && !src.contains("hover:bg-white/10"),
        "a second definition of a  recipe is the drift  imported them to prevent"
    );
    // (5) The element itself, and the two switched-off WebKit parts. The recipe is a const
    // beside the component (the earlier `SLIDER_*` idiom), so the composition is asserted on the
    // fn and the contents on the const — a `cn(&[…])` that dropped the const would pass a
    // whole-file scan.
    let search_type = format!("type=\"{}\"", "search");
    assert!(
        src.contains(&search_type),
        "the element stays a real search input — that is what assistive tech reads"
    );
    assert!(
        src.contains("SEARCH_UA_PARTS"),
        "the input must compose the UA-parts recipe"
    );
    let recipe = live_source(&crate::v2::core::test_support::pins::ui_source());
    let recipe = recipe
        .split("SEARCH_UA_PARTS: &str =")
        .nth(1)
        .and_then(|t| t.split(';').next())
        .expect("SEARCH_UA_PARTS must be declared");
    for part in [
        "::-webkit-search-cancel-button",
        "::-webkit-search-decoration",
    ] {
        assert!(
            recipe.contains(part),
            "{part} must be switched off — the clear affordance is ours, and WebKit's does \
             not exist in Firefox at all; recipe was:\n{recipe}"
        );
    }
    // (6) Tooltip survives `disabled` ( rule 3), same as Slider/Select.
    assert!(
        src.contains("title=label"),
        "a control that cannot act must still explain itself"
    );
}

/* ═══════════════ Esc must reach exactly one dialog ═══════════════ */

use super::modal_stack;
use std::cell::Cell;
use std::rc::Rc;

/// A stand-in for a `RwSignal<bool>` the component reads at keydown time.
fn overlay(open: bool) -> (Rc<Cell<bool>>, u64) {
    let flag = Rc::new(Cell::new(open));
    let read = Rc::clone(&flag);
    let id = modal_stack::register(move || read.get());
    (flag, id)
}

/// The bug, stated as a test. Previously every open overlay answered Escape, so *both* of
/// these would have closed; the guard is `is_topmost_open`, and only the confirm may say true.
///
/// RED on the unfixed component is the second half of this: the source pin below proves the
/// guard is actually in the keydown closure, because a passing stack with an unwired component
/// is exactly the "reports success over an input it never examined" failure this repo hunts.
#[test]
fn only_the_topmost_open_overlay_answers_escape() {
    let start = modal_stack::depth();
    // Mount order = paint order: the form is declared first, the confirm last (the arrangement
    // `event_manager.rs` documents at its detach dialog).
    let (form, form_id) = overlay(true);
    let (confirm, confirm_id) = overlay(true);

    assert!(
        modal_stack::is_topmost_open(confirm_id),
        "the last-opened open overlay owns Escape ( open-order)"
    );
    assert!(
        !modal_stack::is_topmost_open(form_id),
        "the form behind an open confirm must not answer Escape — this is /"
    );

    // Esc dismisses the confirm only; the form is still open and now becomes the target.
    confirm.set(false);
    assert!(modal_stack::is_topmost_open(form_id));
    assert!(!modal_stack::is_topmost_open(confirm_id));

    // Nothing open → Escape has no owner at all.
    form.set(false);
    assert!(!modal_stack::is_topmost_open(form_id));
    assert!(!modal_stack::is_topmost_open(confirm_id));

    modal_stack::unregister(form_id);
    modal_stack::unregister(confirm_id);
    assert_eq!(modal_stack::depth(), start, "registrations must not leak");
}

/// Closed out of order: the overlay *underneath* goes away first (its own Cancel button, or a
/// save that dismisses the form while the confirm it launched is still up). The survivor must
/// still own Escape, and the departed one must not come back as topmost when it reopens under
/// something else.
#[test]
fn a_dialog_closed_out_of_order_leaves_the_stack_consistent() {
    let start = modal_stack::depth();
    let (form, form_id) = overlay(true);
    let (confirm, confirm_id) = overlay(true);

    form.set(false); // the middle of the stack leaves while the top is still up
    assert!(modal_stack::is_topmost_open(confirm_id));
    assert!(!modal_stack::is_topmost_open(form_id));

    // …and reopens as a fresh open: open-order Esc promotes it over the still-open confirm.
    form.set(true);
    assert!(
        modal_stack::is_topmost_open(form_id),
        "a reopen is a fresh open and takes Escape; mount-order would have kept confirm"
    );
    assert!(!modal_stack::is_topmost_open(confirm_id));

    // Unmount out of order too: the top component is torn down first, by id, not by popping.
    modal_stack::unregister(confirm_id);
    assert!(
        modal_stack::is_topmost_open(form_id),
        "removing a registration from the top must promote the one below, not orphan it"
    );
    assert!(
        !modal_stack::is_topmost_open(confirm_id),
        "an unregistered id must never be reported as topmost"
    );
    confirm.set(true); // a stale handle to a torn-down overlay changes nothing
    assert!(modal_stack::is_topmost_open(form_id));

    modal_stack::unregister(form_id);
    modal_stack::unregister(form_id); // double cleanup is a no-op, not a panic
    assert_eq!(modal_stack::depth(), start, "registrations must not leak");
    assert!(!modal_stack::is_topmost_open(form_id));
}

/// **The wiring.** The stack above is only a fix if the components consult it. Both `Dialog`
/// and `Sheet` install a window-level listener, so both must carry the guard; a keydown closure
/// that still reads `if open.get_untracked() && ev.key == "Escape"` and nothing else is the
/// unfixed component.
///
/// Scrubbed source (/ `class_r_scrub`), and `live_code` at that, so the needle cannot
/// be satisfied by this doc comment, by a string literal, or by an item the build drops.
#[test]
fn both_overlay_components_gate_escape_on_the_modal_stack() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let prod = live_code(&crate::v2::core::test_support::pins::ui_source());
    for component in ["pub fn Dialog(", "pub fn Sheet("] {
        let body = only_body(&prod, component);
        assert!(
            body.contains("modal_stack::register("),
            "{component} must register with the modal stack. Body was: {body}"
        );
        assert!(
            body.contains("modal_stack::is_topmost_open(modal_id)"),
            "{component} must gate its Escape handler on being topmost. \
             Body was: {body}"
        );
        assert!(
            body.contains("modal_stack::unregister(modal_id)"),
            "{component} must release its registration on cleanup or the registry leaks \
             one dead entry per mount. Body was: {body}"
        );
    }
}
/// stacked dialogs: prefs over settings. One Escape must belong to the topmost only.
/// Mirrors EditorPreferencesDialog mounting after MissionSettingsDialog (eden_settings).
#[test]
fn stacked_dialogs_only_topmost_answers_escape() {
    let start = modal_stack::depth();
    // Mount order = paint order: settings first, prefs on top (sibling mount in eden_settings).
    let (settings, settings_id) = overlay(true);
    let (prefs, prefs_id) = overlay(true);
    assert!(
        modal_stack::is_topmost_open(prefs_id),
        "prefs stacked over settings must own Escape"
    );
    assert!(
        !modal_stack::is_topmost_open(settings_id),
        "settings under open prefs must not answer Escape — this is "
    );
    assert!(
        modal_stack::any_open(),
        "any_open must be true while either dialog is open"
    );

    // First Esc would close prefs only.
    prefs.set(false);
    assert!(modal_stack::is_topmost_open(settings_id));
    assert!(!modal_stack::is_topmost_open(prefs_id));
    assert!(modal_stack::any_open());

    // Second Esc would close settings; then nobody owns Escape.
    settings.set(false);
    assert!(!modal_stack::is_topmost_open(settings_id));
    assert!(!modal_stack::any_open());

    modal_stack::unregister(settings_id);
    modal_stack::unregister(prefs_id);
    assert_eq!(modal_stack::depth(), start, "registrations must not leak");
}

/* ═══════════════  O-3 /  — z-index and Esc both follow open order ══════════ */

/// The O-3 defect, stated as a test. `AttributesModal` mounts *before* `OrbatManagerDialog`
/// (`mission_editor.rs`), so with equal `z-50` the browser paints ORBAT (later in the DOM) on
/// top — hit-testing the Arsenal's centre returns ORBAT, and the author's click hits nothing.
/// The fix drives z from OPEN order: the Arsenal, opened last, must be the top tier and ORBAT
/// must drop below it. routes Esc by the same open order so Esc1 closes the Arsenal the
/// operator sees, not the hidden ORBAT.
#[test]
fn arsenal_opened_over_orbat_wins_z_and_escape() {
    let start = modal_stack::depth();
    // Mount order (registration): Attributes first, ORBAT second — as `mission_editor` mounts
    // them. Both start closed; the editor mounts both and toggles `open`.
    let (attrs, attrs_id) = overlay(false);
    let (orbat, orbat_id) = overlay(false);

    // Author opens ORBAT, then OPEN ARSENAL from a slot → Attributes opens on top, LAST.
    orbat.set(true);
    assert_eq!(
        modal_stack::z_class(orbat_id),
        "z-50",
        "ORBAT alone is the last-opened surface, so it holds the top modal tier"
    );
    attrs.set(true);

    // z-index: the Arsenal (opened last) is the top of open order; ORBAT drops one tier so the
    // Arsenal's z-50 wins the hit-test. This is the RED assertion before the fix — the old code
    // had every overlay pinned at z-50 with no way to drop.
    assert_eq!(
        modal_stack::z_class(attrs_id),
        "z-50",
        "the Arsenal opened last and must paint on top"
    );
    assert_eq!(
        modal_stack::z_class(orbat_id),
        "z-40",
        "ORBAT opened first and must drop below the Arsenal that opened over it"
    );
    assert!(
        modal_stack::is_top_by_open_order(attrs_id),
        "open order: Arsenal is on top"
    );
    assert!(!modal_stack::is_top_by_open_order(orbat_id));

    // Esc follows the same OPEN order as z: Arsenal owns Escape while it is on top.
    assert!(
        modal_stack::is_topmost_open(attrs_id),
        "Esc routing matches open order: the Arsenal opened last owns Escape"
    );
    assert!(!modal_stack::is_topmost_open(orbat_id));

    // Close the Arsenal → ORBAT is the last-open again and climbs back to the top tier.
    attrs.set(false);
    assert_eq!(
        modal_stack::z_class(orbat_id),
        "z-50",
        "with the Arsenal gone, ORBAT is last-opened again and returns to the top tier"
    );

    modal_stack::unregister(attrs_id);
    modal_stack::unregister(orbat_id);
    assert_eq!(modal_stack::depth(), start, "registrations must not leak");
}

/// A reopen is a FRESH open: a surface that closes and reopens jumps to the top of open order,
/// even though its mount position never moved. This is the open-order counterpart to the
/// mount-order `a_dialog_closed_out_of_order_leaves_the_stack_consistent` above — and the reason
/// z could not simply reuse `is_topmost_open`, which (correctly, for Esc) keeps the reopened
/// surface underneath.
#[test]
fn a_reopen_takes_the_top_of_open_order() {
    let start = modal_stack::depth();
    let (lower, lower_id) = overlay(false);
    let (upper, upper_id) = overlay(false);

    lower.set(true);
    upper.set(true);
    assert!(
        modal_stack::is_top_by_open_order(upper_id),
        "upper opened last"
    );

    // Lower closes, then (in a later frame) reopens — now it is the most-recently-opened. The
    // close and the reopen are separate renders in the live editor, and z is recomputed each
    // render, so a reconcile lands between them; the query here stands in for that render and is
    // what lets the reopen register as a fresh open edge (see `reconcile_open_order`'s cadence
    // note — a close+reopen collapsed into one frame with no render between is not a real UI
    // event and is not tracked).
    lower.set(false);
    assert!(
        !modal_stack::is_top_by_open_order(lower_id),
        "with lower closed, upper is on top"
    );
    lower.set(true);
    assert!(
        modal_stack::is_top_by_open_order(lower_id),
        "a reopen is a fresh open and takes the top of open order (z)"
    );
    // Esc agrees with open order, so the reopened lower owns Escape too.
    assert!(
        modal_stack::is_topmost_open(lower_id),
        "Esc follows open order: a reopen takes Escape, not mount order"
    );
    assert!(!modal_stack::is_topmost_open(upper_id));

    modal_stack::unregister(lower_id);
    modal_stack::unregister(upper_id);
    assert_eq!(modal_stack::depth(), start, "registrations must not leak");
}

/// a closed→open edge fires registered transient closers (strip menu/export/hint).
#[test]
fn opening_an_overlay_fires_registered_transient_closers() {
    use std::cell::Cell;
    use std::rc::Rc;
    let fired = Rc::new(Cell::new(0u32));
    let f = Rc::clone(&fired);
    let closer_id = modal_stack::register_transient_closer(move || {
        f.set(f.get() + 1);
    });
    let (flag, id) = overlay(false);
    assert_eq!(fired.get(), 0, "closed overlay must not fire closers");
    flag.set(true);
    // reconcile runs inside is_topmost_open / z_class / flush
    assert!(modal_stack::is_topmost_open(id));
    assert_eq!(
        fired.get(),
        1,
        "closed→open must fire transient closers exactly once"
    );
    // still open — no second edge
    assert!(modal_stack::is_topmost_open(id));
    assert_eq!(fired.get(), 1, "staying open must not re-fire");
    flag.set(false);
    // Reconcile between close and reopen (mirrors a real UI frame) so open_seq clears.
    assert!(!modal_stack::is_topmost_open(id));
    flag.set(true);
    assert!(modal_stack::is_topmost_open(id));
    assert_eq!(fired.get(), 2, "a reopen is a fresh edge");
    modal_stack::unregister(id);
    modal_stack::unregister_transient_closer(closer_id);
}

/// **The wiring for O-3.** The stack utility only fixes the paint order if the ORBAT surface
/// actually consumes it. `OrbatManagerDialog`'s scrim and panel must derive their z from
/// `modal_stack::z_class` rather than a literal `z-50`; a body that still hard-codes `z-50` on
/// the ORBAT overlay is the unfixed component (the Arsenal keeps its literal `z-50` this wave by
/// design — it is the surface ORBAT yields *to*).
#[test]
fn orbat_manager_overlay_derives_z_from_the_modal_stack() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let scrubbed = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/modals/orbat_manager/dialog.rs"
    )));
    let body = only_body(&scrubbed, "pub fn OrbatManagerDialog(");
    assert!(
        body.contains("modal_stack::z_class(modal_id)"),
        "OrbatManagerDialog must take its overlay z from modal_stack::z_class. \
         Body was: {body}"
    );
}
