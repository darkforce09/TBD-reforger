//! T-692 — the editor's HELP SURFACE: the shortcut reference behind the top strip's **Help** menu
//! and the toggleable **Controls Hint** overlay (rows MENU-BAR-008 / MENU-VIEW-017 /
//! MENU-HELP-001).
//!
//! **The defect this closes.** The Mission Creator binds twenty-six distinct `KeyboardEvent` codes
//! across fifteen window-level keydown listeners in twelve editor-surface modules and, before this
//! ticket, documented **none** of them anywhere in the UI: no Help menu, no hint overlay, and
//! `context_menu`'s `with_shortcut` builder had zero callers. An operator's only route to `G`, `[`,
//! `]`, `1`, `2`, `3`, `E`, `R` or Backspace was reading the Rust source.
//!
//! (T-795 added the third widget digit — `Digit3` — renumbering the widget row to Eden's `No Widget
//! (1) / Translate (2) / Rotate (3)`. T-939.4 then added the six Arrange chords on a fourteenth
//! listener, four of them on codes nothing bound before — which is why the distinct-code count is
//! twenty-six, not the twenty-two T-795 last derived.)
//!
//! Those four numbers are **derived, not typed**: `the_prose_census_numbers_are_derived` (T-740)
//! spells the live census counts out in words and asserts this paragraph contains them, because
//! this sentence has already gone stale twice by being retyped. If you widen what counts as a
//! binding, the pin tells you the new numbers — it does not let you guess them. The fourth is the
//! total the distinct-code count hides: those listeners carry forty-two bindings in
//! total, most of the surplus being the Escape channel. (T-774 settled that one by measurement —
//! the T-703 slice reported "39" and the wave-119 verifier's parser reported 32; the verifier was
//! right, and 32 was the count over the eleven-listener input that ticket widened.)
//!
//! **T-774 widened the INPUT, which is where the last two lies came from.** T-703 replaced a census
//! that read two listeners with one that read eleven and called that the editor surface. It was
//! not: `faction_manager` and `orbat_manager` each install a window-level keydown too, and both are
//! live whenever the Mission Creator is up (`mission_editor` mounts `FactionManagerDialog` and —
//! through a bare re-export in `eden_chrome` — `OrbatManagerDialog`, which is why a symbol search
//! for the component never found the file). Neither was censused and neither was covered by the
//! scope note, which excuses only `ui`'s `Dialog`/`Sheet` and `layout`'s nav. Nothing was
//! exploitable — each binds Escape alone, gated on `open.get_untracked()`, and Escape is the
//! declared shared channel — but both sat outside the growth tripwire, so a key added to either
//! would have shipped undocumented and collision-unchecked with every pin green. They are in the
//! surface now, and every number above is derived over the widened input.
//!
//! **Why the list cannot drift.** A hand-typed prose list re-creates that defect one file over: it
//! goes stale the first time a ticket adds an arm, and nothing goes red. So [`SHORTCUTS`] carries
//! the `KeyboardEvent` codes each row documents, and the `t692_help_covers_every_binding` pins
//! below take the real bindings out of [`keymap_census`] — the ONE extractor (T-703/T-738) that
//! reads every window-level keydown in the editor surface, both the `match ev.code().as_str()`
//! blocks and the `ev.key()` listeners — and assert the two sets are EQUAL. A new binding with no
//! help row fails the first pin; a help row for a binding that does not exist fails the second.
//! Neither direction is a judgement call, and neither can be satisfied by a comment.
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

use crate::v2::apps::editor::shell::layout::HOVER_FILL;
use crate::v2::core::ui::{cn, MaterialIcon};

/// T-772 — ControlsHint close-button geometry. Dense strip/dock rows keep
/// [`crate::v2::apps::editor::shell::layout::BTN_ICON`]'s `p-0.5`; this overlay dismiss is not in a dense row, so the
/// comfortable `p-1.5` hit box lives at the call site rather than widening the shared recipe.
/// Same bright rest + shrink/rounded shape as `BTN_ICON`, different padding only.
const HINT_CLOSE_BTN: &str = "shrink-0 rounded p-1.5 text-on-surface";

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
///
/// T-939.4 added `Arrange`. It sits beside `Transform & snapping` rather than inside it because the
/// two answer different questions: snapping is about the grid a single drag quantises to, Arrange is
/// about the relationship BETWEEN several selected objects. An author hunting for "align these six"
/// should not have to read past the snap-step rows to find it.
pub const GROUPS: [&str; 7] = [
    "Selection",
    "View",
    "Transform & snapping",
    "Arrange",
    "History",
    "Tools",
    "Context menu",
];

/// Every keyboard shortcut the editor binds. Kept EQUAL to the live keydown arms by the pins at the
/// bottom of this file — add an arm without adding a row here and `cargo test -p website-frontend`
/// goes red naming the orphaned code.
///
/// Chords spell out the guard each arm actually carries: `modk` is `ctrl || meta` (so "Ctrl/Cmd"),
/// and the bare-key arms (`E`, `R`, `G`, `[`, `]`, `1`, `2`, Space, Delete, Backspace) reject every
/// modifier, which is why they are written without one. The `ev.key()` listeners (Escape, and the
/// context menu's arrows / Enter) carry NO modifier guard at all — they fire on Ctrl+Esc as
/// readily as on Esc — which is exactly why [`keymap_census`] compares `(code, modifiers)` and not
/// bare codes.
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
    // T-743 — this row's copy is UNCHANGED and is now literally true. It was written by T-669 as a
    // description of the intent while `paste_slots`' no-anchor arm still added a 20 m `PASTE_NUDGE`
    // to both axes, which made "the source position" an overstatement of ±20 m and is half of what
    // T-743 was raised for. The nudge is gone; the paste lands on the source coordinates exactly, so
    // the sentence needed no softening — the code came to meet it.
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
    // T-795 — the widget-select digits are numbered to MATCH Eden's widget row exactly (pixel-verified
    // against Eden frames 164038-164107: `No Widget (1) / Translation (2) / Rotation (3) / Area
    // Scaling (4) / Area (5)`). They were OFF BY ONE before (1=Translate, 2=Rotate, 3=nothing), so an
    // Eden author's muscle memory armed the wrong mode; the renumber makes the keys mean what Eden
    // means. `4`/`5` (Area Scaling / Area) are RESERVED-UNBOUND — no area-scale variant yet — so they
    // get no row here (a phantom row is a lie `no_help_entry_invents_a_binding` would catch) and no
    // keydown arm. These three chords are the same map T-799's toolbar tooltips/Edit-menu chords read.
    Shortcut {
        codes: &["Digit1"],
        chord: "1",
        action: "No widget (bare drag still moves the selection)",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["Digit2"],
        chord: "2",
        action: "Translate widget",
        group: "Transform & snapping",
    },
    Shortcut {
        codes: &["Digit3"],
        chord: "3",
        action: "Rotate widget (drag the ring to rotate the selection)",
        group: "Transform & snapping",
    },
    // ── Arrange (T-939.4 — the page's own chord listener) ─────────────────────────────────────
    // Six chords, `Alt` + a mnemonic letter, and the chord STRINGS here are the ones
    // `top_strip::ARRANGE` prints on both menu surfaces — `arrange_help_rows_match_the_shared_list`
    // below refuses a spelling that drifts, so an operator reading this card and an operator
    // reading the menu row are told the same thing.
    //
    // Every one of the six is inert on a selection of fewer than two, which the action copy says
    // out loud: the tools have nothing to do with one object, and a chord that silently does
    // nothing is the complaint this ticket family exists to answer.
    Shortcut {
        codes: &["KeyL", "KeyR"],
        chord: "Alt + L  /  Alt + R",
        action: "Align the selection to its left / right edge (needs 2+ selected)",
        group: "Arrange",
    },
    Shortcut {
        codes: &["KeyT", "KeyB"],
        chord: "Alt + T  /  Alt + B",
        action: "Align the selection to its top / bottom edge (needs 2+ selected)",
        group: "Arrange",
    },
    Shortcut {
        codes: &["KeyH", "KeyV"],
        chord: "Alt + H  /  Alt + V",
        action: "Space the selection equally, horizontally / vertically (needs 2+ selected)",
        group: "Arrange",
    },
    // ── History (the undo/redo keydown — the second window-level editor listener) ──────────────
    Shortcut {
        codes: &["KeyZ"],
        chord: "Ctrl/Cmd + Z",
        action: "Undo",
        group: "History",
    },
    Shortcut {
        codes: &["KeyY"],
        // T-740 — the undo/redo keydown uses ctrl || meta on KeyY (same mod as undo). Bare
        // "Ctrl + Y" lied to Mac operators; document Cmd on both alternatives.
        chord: "Ctrl/Cmd + Y  or  Ctrl/Cmd + Shift + Z",
        action: "Redo",
        group: "History",
    },
    // ── Tools ─────────────────────────────────────────────────────────────────────────────────
    // T-738/T-703 — Escape is the editor's ONE SHARED CHANNEL. Every dismissable editor-surface
    // listener claims it: the editor keydown's measure-tool dismissal, the asset picker, the
    // comment editor, the connections panel, the Attributes modal, the top strip's menus /
    // export dropdown / Save dialog / Controls Hint, the context menu, the three settings
    // dialogs, and the Faction / ORBAT Manager dialogs (T-774). The claimant COUNT is derived in
    // `keymap_census` (pinned by `the_prose_census_numbers_are_derived`) — do not retype it here.
    // Every claimant state-gates itself, so at most one thing is ever dismissed; that is what
    // makes the pile-up sound rather than a collision, and `keymap_census::SHARED_CHANNELS` is
    // where that decision is written down and pinned. This row used to name only the measurement
    // tools, which advertised a fraction of the truth — including for the Controls Hint's OWN
    // close button, whose tooltip says "Close (Esc)".
    Shortcut {
        codes: &["Escape"],
        chord: "Esc",
        action: "Dismiss whatever is up: a measurement, an open menu or dropdown, the Save dialog, \
                 the Attributes modal, the asset picker, the comment editor, the connections panel, \
                 a settings dialog, the context menu, the Faction or ORBAT Manager — or this card",
        group: "Tools",
    },
    // ── Context menu (context_menu's window keydown — the listener the census could not see) ────
    // T-703 — these three were bound by a window-level `ev.key()` listener that the old code-only
    // extractor never read, so they shipped undocumented for the whole programme. They only act
    // while the context menu is OPEN (the listener returns early otherwise), which is why they can
    // share Enter / the arrows with the rest of the suite without colliding.
    Shortcut {
        codes: &["ArrowUp", "ArrowDown"],
        chord: "↑  /  ↓",
        action: "Move the highlight while the context menu is open",
        group: "Context menu",
    },
    Shortcut {
        codes: &["Enter"],
        chord: "Enter",
        action: "Run the highlighted row (or expand it, if it opens a submenu)",
        group: "Context menu",
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
                                        // T-772: call-site padding — do not size from BTN_ICON alone.
                                        class=cn(&[HINT_CLOSE_BTN, HOVER_FILL])
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

// ═════════════════ T-703 / T-738 — THE keydown census, and the collision test ═════════════════
/// T-703 — **the** keyboard-binding census for the editor, and the collision test built on it.
///
/// # Why this module exists at all
///
/// By wave 119 the keydown-arm extractor had been copy-pasted into FOUR places (`mission_editor`
/// three times, `eden_help` once) in two variants. T-738 banked the instruction: consume it and
/// widen it, do not write a fifth. So this is the one extractor; every census pin in the editor
/// calls in here, and `there_is_exactly_one_extractor` keeps it that way.
///
/// # What T-738 found, which is the substance
///
/// The old extractor scraped only the two `match ev.code().as_str()` blocks (`mission_editor`'s
/// editor keydown and the Ctrl+Z/Y one). The editor binds keys in **eleven more
/// window-level listeners** it could not see, all through `ev.key()`: the asset picker, the
/// comment editor and the connections panel (`overlays`, since T-934.11), the Attributes modal
/// (`attributes`), the
/// menus / export dropdown / Save dialog / **Controls Hint** (`eden_top_strip` — T-692's own close
/// path), the context menu's Escape/arrows/Enter (`context_menu`), three settings dialogs
/// (`eden_settings`), and — added by T-774, which found the input itself short by two — the Faction
/// Manager (`faction_manager`) and the ORBAT Manager (`orbat_manager`). A collision test that cannot
/// see half the bindings is the same lie the ticket exists to kill, so [`listeners`] DISCOVERS every
/// window-level keydown closure in the editor surface rather than being handed a list of two.
///
/// # What a COLLISION is here
///
/// Not "the same code twice" — T-669 proved that test is blind, by adding `Ctrl+Shift+V` on an
/// already-documented `KeyV` and watching every code-set pin stay green. A binding is
/// `(code, modifier predicate)`, the predicate is read out of the live arm guard ([`Mods`]), and
/// two bindings COLLIDE when they name the same code and their predicates OVERLAP — i.e. some real
/// `(ctrl/meta, alt, shift)` combination satisfies both. `Ctrl+V` and `Ctrl+Shift+V` do not
/// overlap; `Ctrl+V` and "V with any modifiers" do.
///
/// Two rules follow, because the two failure modes are genuinely different:
///
/// * **Across listeners** — two separate `keydown` closures both firing on one keypress is the
///   defect the ticket names (Eden's Backspace-hides-the-interface vs Backspace-deletes; Eden's
///   Space-cycles-the-widget vs Space-flyTo). Nothing orders them, so both run.
///   [`no_two_listeners_claim_the_same_chord`].
/// * **Within one listener** — `match` arms are ORDERED, so an overlap is resolved deterministically
///   by position and is often deliberate (the undo/redo listener matches `"KeyZ" if shift` before bare
///   `"KeyZ"`). The bug there is a later arm being *entirely* shadowed by an earlier one, which is a
///   binding that can never fire. [`no_arm_is_shadowed_within_its_own_listener`].
///
/// # Escape, the one declared shared channel
///
/// Escape is a shared channel by design, not by accident: thirteen listeners claim Escape, each
/// claimant reads its own live state first (`get_untracked()`), and the editor keydown's arm only
/// "acts" when a measurement was actually dismissed. That count is derived, not typed: T-774 put
/// this doc block under `the_prose_census_numbers_are_derived` after finding it still said "nine"
/// — a number that was wrong even before the census input was widened.
///
/// [`SHARED_CHANNELS`] is where that decision is written down,
/// and it cannot rot in either direction — [`every_shared_channel_is_really_shared`] fails if a
/// code is exempted without being multiply-claimed, and
/// [`every_shared_channel_claimant_reads_live_state`] fails if a claimant stops gating itself.

/// T-692 — the anti-drift pins. [`SHORTCUTS`] and the editor's real key bindings must name the SAME
/// set of `KeyboardEvent` codes, in both directions.
///
/// **Why source extraction.** Every one of these listeners is a `#[cfg(target_arch = "wasm32")]`
/// closure over `web_sys` events, so no native test can press a key at them; the arm list IS the
/// binding, and reading it out of the scrubbed source is the same technique `t648_keydown_census`
/// already uses to prove a key is free. Comments are stripped (`live_source` keeps the `"KeyX"` arm
/// literals but blanks prose) so a note that MENTIONS a keysym is never mistaken for a binding, and
/// only literals in ARM-HEAD position (followed by `=>`, `if` or `|`) count — a string constant
/// inside an arm body is not a binding and must not be read as one.
///
/// **T-703 widened the input, not the technique.** These pins used to own a private copy of the
/// extractor that read the two `ev.code()` keydowns and nothing else, so eleven `ev.key()`
/// listeners — including the Controls Hint's own Escape — were documented or not entirely by luck.
/// They now consume [`keymap_census`], which reads all thirteen (T-774 added the last two: the
/// Faction and ORBAT Manager dialogs, which the census had never been pointed at).

/* ═════════ T-939.4 — the Arrange chords: one spelling, three surfaces ═══════════════════════════
 *
 * The census pins above already prove the six chords are BOUND and DOCUMENTED. What they cannot see
 * is whether the card and the menus tell the operator the same story: `SHORTCUTS` names codes, and
 * `top_strip::ARRANGE` names the chord an operator reads off a menu row. Both are typed by hand, in
 * different files, and nothing structural stops `Alt + L` on the menu from becoming `Alt + Shift +
 * L` here. These pins close that gap — a chord string that drifts is exactly as misleading as a
 * missing row, and considerably harder to notice.
 */

#[cfg(test)]
#[path = "tests/help_modal/keymap_census/mod.rs"]
pub(crate) mod keymap_census;

#[cfg(test)]
#[path = "tests/help_modal/shortcut_coverage.rs"]
mod help_modal_shortcut_coverage;

#[cfg(test)]
#[path = "tests/help_modal/controls_hint_close.rs"]
mod help_modal_controls_hint_close;

#[cfg(test)]
#[path = "tests/help_modal/arrange_shortcuts.rs"]
mod help_modal_arrange_shortcuts;
