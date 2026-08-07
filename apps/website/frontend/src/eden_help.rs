//! T-692 — the editor's HELP SURFACE: the shortcut reference behind the top strip's **Help** menu
//! and the toggleable **Controls Hint** overlay (rows MENU-BAR-008 / MENU-VIEW-017 /
//! MENU-HELP-001).
//!
//! **The defect this closes.** The Mission Creator binds eighteen `KeyboardEvent.code` values across
//! its two window-level keydowns (`mission_editor`'s editor handler and `mission_history`'s
//! Ctrl+Z/Y handler) and, before this ticket, documented **none** of them anywhere in the UI: no
//! Help menu, no hint overlay, and `context_menu`'s `with_shortcut` builder had zero callers. An
//! operator's only route to `G`, `[`, `]`, `1`, `2`, `E`, `R` or Backspace was reading the Rust
//! source.
//!
//! **Why the list cannot drift.** A hand-typed prose list re-creates that defect one file over: it
//! goes stale the first time a ticket adds an arm, and nothing goes red. So [`SHORTCUTS`] carries
//! the `KeyboardEvent.code` values each row documents, and the `t692_help_covers_every_binding`
//! pins below EXTRACT the real arm patterns out of both keydown `match ev.code().as_str()` blocks
//! (the `t648_keydown_census` technique — scrubbed source, string literals kept, sliced to the arm
//! list) and assert the two sets are EQUAL. A new binding with no help row fails the first pin; a
//! help row for a binding that does not exist fails the second. Neither direction is a judgement
//! call, and neither can be satisfied by a comment.
//!
//! The pins live here rather than in `mission_editor` because they must fail when *that* file
//! changes and *this* one does not — the whole point is that the two are yoked.
//!
//! **Chrome gating.** The overlay is chrome: it must vanish with the rest of it on Backspace. It
//! gets that by CONSTRUCTION rather than by a second gate — [`ControlsHint`] is mounted from
//! `eden_top_strip`'s `TopCommandStrip`, and `mission_editor` mounts the whole strip behind
//! `(!chrome_hidden.get()).then(`, so hiding the chrome unmounts the card with it. The
//! `overlay_hides_with_the_rest_of_the_chrome` pin holds that mount path in place. The open/closed
//! latch is mirrored into the [`HINT_SHOWN`] thread-local (the `eden_layout` latch idiom) so a
//! hide/show cycle — which unmounts and remounts the strip — brings the card back exactly as the
//! operator left it, matching how the debug HUD survives the same cycle.

use std::cell::Cell;

use leptos::prelude::*;

use crate::eden_layout::{BTN_ICON, HOVER_FILL};
use crate::ui::{cn, MaterialIcon};

/// One documented editor shortcut.
pub struct Shortcut {
    /// The `KeyboardEvent.code` value(s) whose keydown ARM this row documents. This is the field
    /// that makes the table checkable: the pins below compare this set against the arm patterns
    /// extracted from the live keydowns, so a row can neither invent a binding nor miss one. It is
    /// also emitted as the rendered row's `data-codes` attribute, so a browser gate can read the
    /// same mapping off the DOM.
    pub codes: &'static [&'static str],
    /// The chord as an operator reads it (`"Ctrl/Cmd + C"`). Human-facing; not parsed.
    pub chord: &'static str,
    /// What the arm does.
    pub action: &'static str,
    /// Which [`GROUPS`] heading the row files under.
    pub group: &'static str,
}

/// The overlay's section headings, in render order. A group naming no row renders nothing.
pub const GROUPS: [&str; 5] = [
    "Selection",
    "View",
    "Transform & snapping",
    "History",
    "Tools",
];

/// Every keyboard shortcut the editor binds. Kept EQUAL to the live keydown arms by the pins at the
/// bottom of this file — add an arm without adding a row here and `cargo test -p website-frontend`
/// goes red naming the orphaned code.
///
/// Chords spell out the guard each arm actually carries: `modk` is `ctrl || meta` (so "Ctrl/Cmd"),
/// and the bare-key arms (`E`, `R`, `G`, `[`, `]`, `1`, `2`, Space, Delete, Backspace) reject every
/// modifier, which is why they are written without one.
pub const SHORTCUTS: &[Shortcut] = &[
    // ── Selection (mission_editor's editor keydown) ───────────────────────────────────────────
    // T-649 landed the Ctrl+A arm in the same wave as this table. It could not be pre-seeded from
    // T-692's worktree: `no_help_entry_invents_a_binding` correctly refuses a row for a binding
    // that does not exist yet, so the two pins are only both satisfiable once both slices merge.
    Shortcut {
        codes: &["KeyA"],
        chord: "Ctrl/Cmd + A",
        action: "Select all in view",
        group: "Selection",
    },
    Shortcut {
        codes: &["KeyC"],
        chord: "Ctrl/Cmd + C",
        action: "Copy the selection",
        group: "Selection",
    },
    // T-669 — cut is `KeyX`, a code neither keydown bound before; paste-at-original re-uses `KeyV`
    // under Shift, so the code-set pins below CANNOT see a missing row for it (`KeyV` is documented
    // either way). `mission_editor`'s `both_new_chords_are_documented_in_the_help_table` pins the
    // two CHORD strings for that reason.
    Shortcut {
        codes: &["KeyX"],
        chord: "Ctrl/Cmd + X",
        action: "Cut the selection (copy, then remove)",
        group: "Selection",
    },
    Shortcut {
        codes: &["KeyV"],
        chord: "Ctrl/Cmd + V",
        action: "Paste at the cursor",
        group: "Selection",
    },
    Shortcut {
        codes: &["KeyV"],
        chord: "Ctrl/Cmd + Shift + V",
        action: "Paste at the source position instead of the cursor",
        group: "Selection",
    },
    Shortcut {
        codes: &["Delete"],
        chord: "Delete",
        action: "Remove the selection",
        group: "Selection",
    },
    Shortcut {
        codes: &["Space"],
        chord: "Space",
        action: "Centre the camera on the selection",
        group: "Selection",
    },
    // ── View / chrome ─────────────────────────────────────────────────────────────────────────
    Shortcut {
        codes: &["Backspace"],
        chord: "Backspace",
        action: "Hide / show the whole interface (this card included)",
        group: "View",
    },
    Shortcut {
        codes: &["KeyE"],
        chord: "E",
        action: "Collapse / expand the Entity List (left dock)",
        group: "View",
    },
    Shortcut {
        codes: &["KeyR"],
        chord: "R",
        action: "Collapse / expand the Asset Browser (right dock)",
        group: "View",
    },
    Shortcut {
        codes: &["KeyD"],
        chord: "Ctrl/Cmd + Alt + D",
        action: "Toggle the telemetry HUD in the status bar",
        group: "View",
    },
    // ── Transform & snapping (T-648) ──────────────────────────────────────────────────────────
    Shortcut {
        codes: &["KeyG"],
        chord: "G",
        action: "Toggle the snap grid",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["BracketLeft", "BracketRight"],
        chord: "[  /  ]",
        action: "Decrease / increase the snap step of the active widget",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["Digit1"],
        chord: "1",
        action: "Translate widget",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["Digit2"],
        chord: "2",
        action: "Rotate widget",
        group: "Transform & snapping",
    },
    // ── History (mission_history's keydown — the second window-level editor listener) ──────────
    Shortcut {
        codes: &["KeyZ"],
        chord: "Ctrl/Cmd + Z",
        action: "Undo",
        group: "History",
    },
    Shortcut {
        codes: &["KeyY"],
        chord: "Ctrl + Y  or  Ctrl/Cmd + Shift + Z",
        action: "Redo",
        group: "History",
    },
    // ── Tools ─────────────────────────────────────────────────────────────────────────────────
    Shortcut {
        codes: &["Escape"],
        chord: "Esc",
        action: "Dismiss the ruler / line-of-sight / viewshed measurement",
        group: "Tools",
    },
];

thread_local! {
    /// T-692 — is the Controls Hint open right now? The `eden_layout` latch idiom: the reactive
    /// truth is the `RwSignal` `TopCommandStrip` owns, mirrored here so it SURVIVES the strip's
    /// unmount/remount across a Backspace hide/show cycle. Without it the card would silently
    /// close every time the operator peeked at a clean map, which is not what a pinned reference
    /// panel should do (the debug HUD's `debug_hud_shown` persists across the same cycle for the
    /// same reason).
    static HINT_SHOWN: Cell<bool> = const { Cell::new(false) };
}

/// T-692 — is the Controls Hint open right now (across a chrome hide/show cycle)?
#[must_use]
pub fn hint_shown() -> bool {
    HINT_SHOWN.with(Cell::get)
}

/// T-692 — mirror the Controls Hint's open state into the cross-remount latch.
pub fn set_hint_shown(v: bool) {
    HINT_SHOWN.with(|c| c.set(v));
}

/// T-692 — the Controls Hint overlay: every binding in [`SHORTCUTS`], grouped, over the map.
///
/// Mounted from `TopCommandStrip` so it inherits the `chrome_hidden` gate (see the module docs).
/// Renders NO DOM while closed, like the menu dropdowns. The backdrop is `pointer-events-none` and
/// only the card itself takes the pointer, so the map stays pannable around an open hint — it is a
/// reference card, not a modal, and nothing about it needs dismissing before work continues.
#[component]
pub fn ControlsHint(open: RwSignal<bool>) -> impl IntoView {
    view! {
        {move || {
            open.get()
                .then(|| {
                    view! {
                        <div
                            data-controls-hint
                            class="pointer-events-none fixed inset-0 z-50 flex items-start justify-center pt-16"
                        >
                            <div class="glass animate-menu-in pointer-events-auto max-h-[70vh] w-[38rem] max-w-[92vw] overflow-y-auto rounded-xl p-4 shadow-lg">
                                <div class="mb-3 flex items-center justify-between gap-3">
                                    <span class="text-label-md font-semibold text-on-surface">
                                        "Controls — keyboard shortcuts"
                                    </span>
                                    <button
                                        type="button"
                                        aria-label="Close the Controls Hint"
                                        // Rule (3): the control explains itself even though it is
                                        // never disabled — Esc is the other way out.
                                        title="Close (Esc)"
                                        class=cn(&[BTN_ICON, HOVER_FILL])
                                        on:click=move |_| {
                                            open.set(false);
                                            set_hint_shown(false);
                                        }
                                    >
                                        <MaterialIcon name="close" class="block text-base" />
                                    </button>
                                </div>
                                {GROUPS
                                    .iter()
                                    .map(|g| {
                                        let rows = SHORTCUTS
                                            .iter()
                                            .filter(|s| s.group == *g)
                                            .map(|s| {
                                                view! {
                                                    <li
                                                        class="flex items-baseline gap-3 py-0.5"
                                                        data-codes=s.codes.join(" ")
                                                    >
                                                        <kbd class="w-52 shrink-0 text-right font-mono text-code-md text-on-surface">
                                                            {s.chord}
                                                        </kbd>
                                                        <span class="text-label-sm text-on-surface-variant">
                                                            {s.action}
                                                        </span>
                                                    </li>
                                                }
                                            })
                                            .collect_view();
                                        view! {
                                            <div class="mb-3">
                                                <div class="mb-1 text-label-sm font-semibold uppercase tracking-wide text-outline">
                                                    {*g}
                                                </div>
                                                <ul class="flex flex-col">{rows}</ul>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                                <div class="border-t border-white/10 pt-2 text-label-sm text-outline">
                                    "This list is pinned against the editor's real key handlers — a new binding cannot ship undocumented."
                                </div>
                            </div>
                        </div>
                    }
                })
        }}
    }
}

/// T-692 — the anti-drift pins. [`SHORTCUTS`] and the editor's real keydown arms must name the SAME
/// set of `KeyboardEvent.code` values, in both directions.
///
/// **Why source extraction.** Both keydowns are `#[cfg(target_arch = "wasm32")]` closures over
/// `web_sys` events, so no native test can press a key at them; the arm list IS the binding, and
/// reading it out of the scrubbed source is the same technique `t648_keydown_census` already uses
/// to prove a key is free. Comments are stripped (`live_source` keeps the `"KeyX"` arm literals but
/// blanks prose) so a note that MENTIONS a keysym is never mistaken for a binding, and only
/// literals in ARM-HEAD position (followed by `=>`, `if` or `|`) count — a string constant inside
/// an arm body is not a binding and must not be read as one.
#[cfg(test)]
mod t692_help_covers_every_binding {
    use super::{Shortcut, GROUPS, SHORTCUTS};
    use crate::arsenal::class_r_scrub::{live_code, live_source};
    use std::collections::BTreeSet;

    /// Every `KeyboardEvent.code` bound by the keydown in `src`.
    ///
    /// Slices from the `match ev.code().as_str() {` head to the `_ =>` fallthrough (both editor
    /// keydowns end their arm list that way), scrubs comments, then collects string literals that
    /// sit in arm-head position. Position, not shape: a whitelist of "code-looking" strings would
    /// silently miss the first binding on a key nobody anticipated, which is precisely the failure
    /// this whole file exists to prevent.
    fn bound_codes(src: &str) -> BTreeSet<String> {
        let head = "match ev.code().as_str() {";
        let at = src.find(head).expect("an editor keydown match is present");
        let rest = &src[at..];
        let end = rest.find("_ =>").map_or(rest.len(), |i| i + 4);
        let arms: Vec<char> = live_source(&rest[..end]).chars().collect();
        let mut out = BTreeSet::new();
        let mut i = 0usize;
        while i < arms.len() {
            if arms[i] != '"' {
                i += 1;
                continue;
            }
            let start = i + 1;
            let mut j = start;
            // A `KeyboardEvent.code` literal carries no escapes, so a plain scan to the next quote
            // is exact for the arm heads; anything weirder is not an arm head anyway.
            while j < arms.len() && arms[j] != '"' {
                j += 1;
            }
            let lit: String = arms[start..j].iter().collect();
            let mut k = j + 1;
            while k < arms.len() && arms[k].is_whitespace() {
                k += 1;
            }
            let tail: String = arms[k..arms.len().min(k + 3)].iter().collect();
            if tail.starts_with("=>") || tail.starts_with("if ") || tail.starts_with('|') {
                out.insert(lit);
            }
            i = j + 1;
        }
        out
    }

    /// Both window-level editor keydowns, as one set of bound codes.
    fn all_bound() -> BTreeSet<String> {
        let mut bound = bound_codes(include_str!("mission_editor.rs"));
        bound.extend(bound_codes(include_str!("mission_history.rs")));
        bound
    }

    fn documented() -> BTreeSet<String> {
        SHORTCUTS
            .iter()
            .flat_map(|s| s.codes.iter().map(|c| (*c).to_string()))
            .collect()
    }

    /// The extractor itself must be honest before either coverage assertion means anything: an
    /// extractor that returns nothing would make "every binding is documented" vacuously true, and
    /// that is exactly the shape of a pin that passes forever while the UI rots. Pin two arms that
    /// prove BOTH files were parsed (`Backspace` is `mission_editor`'s, `KeyZ` is
    /// `mission_history`'s) and a floor on the count.
    #[test]
    fn the_extractor_actually_reads_both_keydowns() {
        let bound = all_bound();
        assert!(
            bound.contains("Backspace"),
            "extractor must see mission_editor's arms (found {bound:?})"
        );
        assert!(
            bound.contains("KeyZ"),
            "extractor must see mission_history's arms (found {bound:?})"
        );
        assert!(
            bound.len() >= 14,
            "the editor binds well over a dozen codes; an extractor finding {} has broken, and a \
             broken extractor makes the coverage pins pass vacuously",
            bound.len()
        );
        // A body literal must never be read as a binding. `center_on_selection` is called from the
        // Space arm's body; no arm-head literal may look like a call fragment.
        assert!(
            bound.iter().all(|c| !c.contains('(')),
            "arm-head extraction picked up something that is not a key code: {bound:?}"
        );
    }

    /// THE TICKET, as a test: a binding with no help entry is RED. This is the pin that fires when
    /// a future slice adds a keydown arm and forgets the operator.
    #[test]
    fn every_binding_has_a_help_entry() {
        let bound = all_bound();
        let documented = documented();
        let undocumented: Vec<&String> = bound.difference(&documented).collect();
        assert!(
            undocumented.is_empty(),
            "T-692: these editor keydown arms are bound but documented NOWHERE in the UI: \
             {undocumented:?}. Add a `Shortcut` row to `SHORTCUTS` naming each code — that is the \
             whole point of this ticket ('TBD binds keyboard shortcuts and documents none of \
             them'); a new binding must not re-open the defect."
        );
    }

    /// The other direction: the help surface must not promise a shortcut the editor does not bind.
    /// A phantom row is the same lie as a missing one, told the other way round.
    #[test]
    fn no_help_entry_invents_a_binding() {
        let bound = all_bound();
        let documented = documented();
        let phantom: Vec<&String> = documented.difference(&bound).collect();
        assert!(
            phantom.is_empty(),
            "T-692: `SHORTCUTS` documents {phantom:?}, which no editor keydown arm binds. Either \
             the binding was removed (drop the row) or the code was mistyped."
        );
    }

    /// Every row is renderable and files under a real heading — a row whose `group` is a typo would
    /// silently render nowhere, which is a documented-but-invisible shortcut.
    #[test]
    fn every_row_renders_under_a_real_group() {
        for Shortcut {
            codes,
            chord,
            action,
            group,
        } in SHORTCUTS
        {
            assert!(!codes.is_empty(), "row `{chord}` names no key code");
            assert!(
                !chord.is_empty() && !action.is_empty(),
                "row {codes:?} is blank"
            );
            assert!(
                GROUPS.contains(group),
                "row `{chord}` files under `{group}`, which is not a rendered heading — it would \
                 be invisible"
            );
        }
    }

    /// The `chrome_hidden` gate, held in place structurally: the overlay is mounted by
    /// `eden_top_strip`, and `mission_editor` mounts that strip INSIDE a `(!chrome_hidden.get())`
    /// gate with nothing closing the gated block in between. So Backspace takes the hint card with
    /// the rest of the chrome, and no second gate can drift away from the first.
    #[test]
    fn overlay_hides_with_the_rest_of_the_chrome() {
        let strip = live_code(include_str!("eden_top_strip.rs"));
        assert!(
            strip.contains("ControlsHint"),
            "the Controls Hint must be mounted from the top strip (that is what puts it behind the \
             chrome_hidden gate)"
        );
        // `live_code` on the WHOLE editor file would blank the mount: the Eden view is inside a
        // `#[cfg(target_arch = "wasm32")]` item, which the scrubber (correctly) treats as dead on
        // the native shell. Hand it the region from the page fn onward, at a brace-0 boundary —
        // the same `editor_live()` manoeuvre the T-662 pins use for the same reason.
        let raw = include_str!("mission_editor.rs");
        let anchor = "pub fn MissionEditorPage() -> impl IntoView";
        assert_eq!(
            raw.matches(anchor).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        let ed = live_code(&raw[raw.find(anchor).expect("counted above")..]);
        let mount = ed
            .find("TopCommandStrip")
            .expect("mission_editor mounts the top strip");
        let gate = ed[..mount]
            .rfind("(!chrome_hidden.get()).then(")
            .expect("the strip mount must sit behind the chrome_hidden gate");
        assert!(
            !ed[gate..mount].contains("})}"),
            "the chrome_hidden gate must still be OPEN at the TopCommandStrip mount — otherwise \
             the strip (and the Controls Hint inside it) survives Backspace"
        );
    }
}
