//! T-692 — the editor's HELP SURFACE: the shortcut reference behind the top strip's **Help** menu
//! and the toggleable **Controls Hint** overlay (rows MENU-BAR-008 / MENU-VIEW-017 /
//! MENU-HELP-001).
//!
//! **The defect this closes.** The Mission Creator binds twenty-two distinct `KeyboardEvent` codes
//! across thirteen window-level keydown listeners in eight editor-surface modules and, before this
//! ticket, documented **none** of them anywhere in the UI: no Help menu, no hint overlay, and
//! `context_menu`'s `with_shortcut` builder had zero callers. An operator's only route to `G`, `[`,
//! `]`, `1`, `2`, `3`, `E`, `R` or Backspace was reading the Rust source.
//!
//! (T-795 added the third widget digit — `Digit3` — renumbering the widget row to Eden's `No Widget
//! (1) / Translate (2) / Rotate (3)`, which is why the distinct-code count is twenty-two, not the
//! twenty-one T-740 last derived.)
//!
//! Those four numbers are **derived, not typed**: `the_prose_census_numbers_are_derived` (T-740)
//! spells the live census counts out in words and asserts this paragraph contains them, because
//! this sentence has already gone stale twice by being retyped. If you widen what counts as a
//! binding, the pin tells you the new numbers — it does not let you guess them. The fourth is the
//! total the distinct-code count hides: those thirteen listeners carry thirty-five bindings in
//! total, most of the surplus being the Escape channel. (T-774 settled that one by measurement —
//! the T-703 slice reported "39" and the wave-119 verifier's parser reported 32; the verifier was
//! right, and 32 was the count over the eleven-listener input this ticket widened.)
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

use crate::eden_layout::HOVER_FILL;
use crate::ui::{cn, MaterialIcon};

/// T-772 — ControlsHint close-button geometry. Dense strip/dock rows keep
/// [`crate::eden_layout::BTN_ICON`]'s `p-0.5`; this overlay dismiss is not in a dense row, so the
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
pub const GROUPS: [&str; 6] = [
    "Selection",
    "View",
    "Transform & snapping",
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
    // ── History (mission_history's keydown — the second window-level editor listener) ──────────
    Shortcut {
        codes: &["KeyZ"],
        chord: "Ctrl/Cmd + Z",
        action: "Undo",
        group: "History",
    },
    Shortcut {
        codes: &["KeyY"],
        // T-740 — mission_history redo uses ctrl || meta on KeyY (same mod as undo). Bare
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
/// editor keydown and `mission_history`'s Ctrl+Z/Y one). The editor binds keys in **eleven more
/// window-level listeners** it could not see, all through `ev.key()`: the asset picker, the comment
/// editor and the connections panel (`mission_editor`), the Attributes modal (`attributes`), the
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
///   by position and is often deliberate (`mission_history` matches `"KeyZ" if shift` before bare
///   `"KeyZ"`). The bug there is a later arm being *entirely* shadowed by an earlier one, which is a
///   binding that can never fire. [`no_arm_is_shadowed_within_its_own_listener`].
///
/// # Escape, the one declared shared channel
///
/// Escape is a shared channel by design, not by accident: twelve listeners claim Escape, each
/// claimant reads its own live state first (`get_untracked()`), and the editor keydown's arm only
/// "acts" when a measurement was actually dismissed. That count is derived, not typed: T-774 put
/// this doc block under `the_prose_census_numbers_are_derived` after finding it still said "nine"
/// — a number that was wrong even before the census input was widened.
///
/// [`SHARED_CHANNELS`] is where that decision is written down,
/// and it cannot rot in either direction — [`every_shared_channel_is_really_shared`] fails if a
/// code is exempted without being multiply-claimed, and
/// [`every_shared_channel_claimant_reads_live_state`] fails if a claimant stops gating itself.
#[cfg(test)]
pub(crate) mod keymap_census {
    use crate::arsenal::class_r_scrub::live_source;
    use std::collections::{BTreeMap, BTreeSet};

    /// A modifier PREDICATE, as read out of a live arm guard. `Some(true)` = the modifier is
    /// required, `Some(false)` = forbidden, `None` = the binding does not constrain it (and so
    /// claims the key with the modifier held **and** released).
    ///
    /// `modk` is the editor's `ctrl || meta`, the one abstraction the arms already use.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) struct Mods {
        pub(crate) modk: Option<bool>,
        pub(crate) alt: Option<bool>,
        pub(crate) shift: Option<bool>,
    }

    impl Mods {
        /// No constraint at all — what a bare `ev.key() == "Escape"` listener claims.
        pub(crate) const ANY: Self = Self {
            modk: None,
            alt: None,
            shift: None,
        };

        /// Does a real keypress with these modifier states reach this binding?
        pub(crate) fn accepts(self, modk: bool, alt: bool, shift: bool) -> bool {
            self.modk.is_none_or(|w| w == modk)
                && self.alt.is_none_or(|w| w == alt)
                && self.shift.is_none_or(|w| w == shift)
        }

        /// Every `(modk, alt, shift)` triple this predicate accepts. Three booleans, so the space is
        /// eight events wide and enumerating it is exact — no reasoning about guard algebra.
        fn matrix(self) -> Vec<(bool, bool, bool)> {
            let mut out = Vec::new();
            for modk in [false, true] {
                for alt in [false, true] {
                    for shift in [false, true] {
                        if self.accepts(modk, alt, shift) {
                            out.push((modk, alt, shift));
                        }
                    }
                }
            }
            out
        }

        /// Is there a keypress BOTH bindings answer? This is the collision relation.
        pub(crate) fn overlaps(self, other: Self) -> bool {
            self.matrix()
                .into_iter()
                .any(|(m, a, s)| other.accepts(m, a, s))
        }

        /// Is every keypress `self` answers already answered by `other`? Within one ordered `match`
        /// that means `self` is dead code.
        pub(crate) fn covered_by(self, other: Self) -> bool {
            self.matrix()
                .into_iter()
                .all(|(m, a, s)| other.accepts(m, a, s))
        }

        /// The arm guard AND the listener-level precondition above it.
        fn and(self, pre: Self) -> Self {
            fn one(arm: Option<bool>, pre: Option<bool>, what: &str) -> Option<bool> {
                match (arm, pre) {
                    (Some(a), Some(p)) => {
                        assert_eq!(
                            a, p,
                            "T-703: an arm guard and its listener's precondition contradict on \
                             {what} — the arm can never run"
                        );
                        Some(a)
                    }
                    (Some(a), None) => Some(a),
                    (None, p) => p,
                }
            }
            Self {
                modk: one(self.modk, pre.modk, "ctrl/meta"),
                alt: one(self.alt, pre.alt, "alt"),
                shift: one(self.shift, pre.shift, "shift"),
            }
        }

        /// Human form for a failure message.
        fn describe(self) -> String {
            let part = |v: Option<bool>, name: &str| match v {
                Some(true) => format!("+{name}"),
                Some(false) => format!("-{name}"),
                None => format!("?{name}"),
            };
            format!(
                "[{} {} {}]",
                part(self.modk, "mod"),
                part(self.alt, "alt"),
                part(self.shift, "shift")
            )
        }
    }

    /// Read a `match` arm guard (everything between `if` and `=>`) as a [`Mods`].
    ///
    /// **Fail-closed.** A term this does not recognise PANICS rather than being ignored: a guard
    /// silently read as "no constraint" would widen the binding and could turn a real collision
    /// into a phantom one, and a guard silently dropped could hide one. Teach it the term.
    fn parse_guard(guard: &str) -> Mods {
        let mut m = Mods::ANY;
        for raw in guard.split("&&") {
            let term: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
            if term.is_empty() {
                continue;
            }
            let (slot, want) = match term.as_str() {
                "modk" | "ev.ctrl_key()||ev.meta_key()" => (0, true),
                "!modk" | "!(ev.ctrl_key()||ev.meta_key())" => (0, false),
                "ev.alt_key()" => (1, true),
                "!ev.alt_key()" => (1, false),
                "ev.shift_key()" => (2, true),
                "!ev.shift_key()" => (2, false),
                other => panic!(
                    "T-703: unreadable modifier guard term `{other}`. The collision census refuses \
                     to guess: a term it cannot read would silently widen or narrow the binding, \
                     and either way the answer it gives about collisions would be about code that \
                     is not there. Teach `parse_guard` the term."
                ),
            };
            let slotted = match slot {
                0 => &mut m.modk,
                1 => &mut m.alt,
                _ => &mut m.shift,
            };
            *slotted = Some(want);
        }
        m
    }

    /// One binding: a key code, the modifier predicate that reaches it, and where it lives.
    #[derive(Clone, Debug)]
    pub(crate) struct Binding {
        pub(crate) code: String,
        pub(crate) mods: Mods,
        pub(crate) file: &'static str,
        /// Index of the window-level listener within its file, in source order.
        pub(crate) listener: usize,
        /// Position within that listener — `match` arms are ordered and the order is load-bearing.
        pub(crate) order: usize,
        /// `ev.code()` or `ev.key()` — the accessor the listener reads.
        pub(crate) via: &'static str,
    }

    impl Binding {
        fn site(&self) -> String {
            format!("{}#{} ({})", self.file, self.listener, self.via)
        }
    }

    /// One window-level `keydown` closure: its scrubbed source and the bindings inside it.
    pub(crate) struct Listener {
        pub(crate) file: &'static str,
        pub(crate) index: usize,
        pub(crate) src: String,
        pub(crate) bindings: Vec<Binding>,
    }

    /// The EDITOR SURFACE: every module that installs a window-level `keydown` while the Mission
    /// Creator is up, with the number of such listeners each one is expected to install.
    ///
    /// The count is declared, not merely observed, so that ADDING a listener is red until someone
    /// has looked at whether it collides — which is the whole ticket. Modules whose Escape belongs
    /// to the suite rather than the editor (`ui`'s `Dialog`/`Modal`, `layout`'s nav) are out of
    /// scope here and stay out; they are gated on `modal_stack::is_topmost_open`, which is a
    /// different (and stricter) discipline from this one.
    ///
    /// **The membership test is "does `mission_editor` mount it", not "is it named like an editor
    /// module".** T-774 found the list short by two on exactly that confusion: `faction_manager` and
    /// `orbat_manager` each install a raw window-level keydown, both are mounted by
    /// `MissionEditorPage`, and neither is gated on the modal stack — so both belong here, and
    /// neither the scope note above nor the tripwire below had any way to say so. `orbat_manager`
    /// hid the longest because `mission_editor` reaches it through a bare `pub use` re-export in
    /// `eden_chrome`, so a symbol search for `OrbatManagerDialog` lands on the re-export and never
    /// on the file that owns the listener. Grep for the LISTENER HEADS, not for the component.
    fn editor_surface() -> Vec<(&'static str, &'static str, usize)> {
        vec![
            ("mission_editor.rs", include_str!("mission_editor.rs"), 4),
            ("mission_history.rs", include_str!("mission_history.rs"), 1),
            ("attributes.rs", include_str!("attributes.rs"), 1),
            ("eden_top_strip.rs", include_str!("eden_top_strip.rs"), 1),
            ("context_menu.rs", include_str!("context_menu.rs"), 1),
            ("eden_settings.rs", include_str!("eden_settings.rs"), 3),
            ("faction_manager.rs", include_str!("faction_manager.rs"), 1),
            ("orbat_manager.rs", include_str!("orbat_manager.rs"), 1),
        ]
    }

    /// How a window-level keydown closure is registered. Both idioms the frontend uses; a third
    /// would be invisible to the census, so `every_editor_surface_listener_is_censused` counts.
    const LISTENER_HEADS: [&str; 2] = [
        "window_event_listener(leptos::ev::keydown,",
        "Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(",
    ];

    /// Listener-level modifier PRECONDITIONS: an early `return` above the arm list narrows every arm
    /// in that closure, and reading only the arm heads would therefore over-claim.
    /// `mission_history` bails on anything that is not Ctrl/Cmd-without-Alt before it looks at the
    /// code at all, which is why its bare `"KeyZ"` arm is not the modifier-free claim it appears to
    /// be.
    ///
    /// Keyed by a needle that must appear in the closure's own source, so the entry can only apply
    /// to code that is really there — and if the source is reworded, the precondition simply stops
    /// applying and the arms widen, which produces MORE collisions, not fewer. Fail-closed.
    const PRECONDITIONS: [(&str, Mods); 1] = [(
        "if !(ev.ctrl_key() || ev.meta_key()) || ev.alt_key() {",
        Mods {
            modk: Some(true),
            alt: Some(false),
            shift: None,
        },
    )];

    /// Escape is claimed by every dismissable surface in the editor, deliberately. See the module
    /// docs; the two pins below stop this from becoming a place to bury a real collision.
    pub(crate) const SHARED_CHANNELS: [&str; 1] = ["Escape"];

    /// Small-integer spelling, for the prose counts the census derives. One copy, consumed by
    /// `mission_editor`'s help-blurb pin too — the same discipline this whole module is about.
    /// Deliberately narrow: a count outside the range panics with instructions rather than
    /// silently spelling nothing.
    pub(crate) fn spell(n: usize) -> String {
        const ONES: [&str; 20] = [
            "zero",
            "one",
            "two",
            "three",
            "four",
            "five",
            "six",
            "seven",
            "eight",
            "nine",
            "ten",
            "eleven",
            "twelve",
            "thirteen",
            "fourteen",
            "fifteen",
            "sixteen",
            "seventeen",
            "eighteen",
            "nineteen",
        ];
        const TENS: [&str; 4] = ["twenty", "thirty", "forty", "fifty"];
        assert!(
            n < 60,
            "extend `spell` past {n} before the editor gets there"
        );
        if n < 20 {
            return ONES[n].to_string();
        }
        let tens = TENS[n / 10 - 2];
        if n.is_multiple_of(10) {
            tens.to_string()
        } else {
            format!("{tens}-{}", ONES[n % 10])
        }
    }

    /// Byte index of the `}` closing the `{` at `open`.
    fn balanced(src: &str, open: usize) -> Option<usize> {
        let mut depth = 0i32;
        for (i, c) in src.char_indices() {
            if i < open {
                continue;
            }
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Every window-level keydown closure body in `raw`, comment-scrubbed, in source order.
    ///
    /// The closure body is sliced from the RAW source and scrubbed after, not before: `live_source`
    /// cuts at the first `#[cfg(test)]` it sees and `mission_editor` has one on an inner
    /// `clear_for_test` helper at line 88, so scrubbing the whole file first would hand back an
    /// empty editor. Scrubbing from the listener head onward is safe — every one of these closures
    /// closes long before its file's test module.
    fn listener_bodies(raw: &str) -> Vec<String> {
        let mut heads: Vec<usize> = Vec::new();
        for head in LISTENER_HEADS {
            let mut from = 0usize;
            while let Some(i) = raw[from..].find(head) {
                heads.push(from + i);
                from += i + head.len();
            }
        }
        heads.sort_unstable();
        heads
            .into_iter()
            .filter_map(|start| {
                let live = live_source(&raw[start..]);
                let open = live.find('{')?;
                let end = balanced(&live, open)?;
                let body = live[open..=end].to_string();
                // T-776 — a keydown registration that reads neither `ev.key()` nor `ev.code()`
                // (or whose event parameter is not named `ev`) used to be silently DROPPED from
                // discovery. That is the hollow shape the census exists to eliminate: the listener
                // never inflated `found`, so every per-file count and the empty-bindings check
                // stayed green over an incomplete input. Fail closed — teach the census the new
                // idiom, or rename the parameter to `ev` and use those accessors.
                assert!(
                    body.contains("ev.key()") || body.contains("ev.code()"),
                    "T-776: a window-level keydown closure reads neither `ev.key()` nor                      `ev.code()` — it would have been silently dropped from the census. Body                      starts:\n{}",
                    &body[..body.len().min(160)]
                );
                Some(body)
            })
            .collect()
    }

    /// Arm heads of the `match` opened by `head` inside `body`: `(literal, guard)`, in source order.
    ///
    /// Only literals in ARM-HEAD position (the next non-space text is `=>`, `if ` or `|`) count — a
    /// string constant inside an arm body is not a binding and must never be read as one.
    fn match_arms(body: &str, head: &str) -> Vec<(String, Mods)> {
        let Some(at) = body.find(head) else {
            return Vec::new();
        };
        let rest = &body[at..];
        let end = rest.find("_ =>").map_or(rest.len(), |i| i + 4);
        let a: Vec<char> = rest[..end].chars().collect();
        let mut out = Vec::new();
        let mut i = 0usize;
        while i < a.len() {
            if a[i] != '"' {
                i += 1;
                continue;
            }
            let start = i + 1;
            let mut j = start;
            // A `KeyboardEvent` code literal carries no escapes, so a plain scan to the next quote
            // is exact for the arm heads; anything weirder is not an arm head anyway.
            while j < a.len() && a[j] != '"' {
                j += 1;
            }
            let lit: String = a[start..j].iter().collect();
            let mut k = j + 1;
            while k < a.len() && a[k].is_whitespace() {
                k += 1;
            }
            let peek: String = a[k..a.len().min(k + 3)].iter().collect();
            if peek.starts_with("=>") || peek.starts_with("if ") || peek.starts_with('|') {
                // The guard is whatever sits between this literal and the arm's `=>`. Taking the
                // LAST `if ` means an alternation (`"A" | "B" if g =>`) gives both literals the
                // guard that really applies to them.
                let mut p = k;
                while p + 1 < a.len() && !(a[p] == '=' && a[p + 1] == '>') {
                    p += 1;
                }
                let head_tail: String = a[k..p].iter().collect();
                let mods = match head_tail.rfind("if ") {
                    Some(g) => parse_guard(&head_tail[g + 3..]),
                    None => Mods::ANY,
                };
                out.push((lit, mods));
            }
            i = j + 1;
        }
        out
    }

    /// Every `ev.key() == "X"` comparison in `body`. These carry no modifier guard whatsoever —
    /// they answer `Ctrl+Esc` and `Shift+Esc` as readily as `Esc` — which is why they are recorded
    /// as [`Mods::ANY`] rather than being quietly assumed bare.
    fn key_equals(body: &str) -> Vec<String> {
        let needle = "ev.key() == \"";
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(i) = body[from..].find(needle) {
            let s = from + i + needle.len();
            let e = body[s..].find('"').map_or(body.len(), |d| s + d);
            out.push(body[s..e].to_string());
            from = e;
        }
        out
    }

    fn precondition(body: &str) -> Mods {
        let mut m = Mods::ANY;
        for (needle, pre) in PRECONDITIONS {
            if body.contains(needle) {
                m = m.and(pre);
            }
        }
        m
    }

    /// THE census: every window-level keydown listener in the editor surface, with its bindings.
    pub(crate) fn listeners() -> Vec<Listener> {
        let mut out = Vec::new();
        for (file, raw, _expected) in editor_surface() {
            for (index, body) in listener_bodies(raw).into_iter().enumerate() {
                let pre = precondition(&body);
                let mut bindings = Vec::new();
                let mut push = |code: String, mods: Mods, via: &'static str, order: &mut usize| {
                    bindings.push(Binding {
                        code,
                        mods,
                        file,
                        listener: index,
                        order: *order,
                        via,
                    });
                    *order += 1;
                };
                let mut order = 0usize;
                for (code, mods) in match_arms(&body, "match ev.code().as_str() {") {
                    push(code, mods.and(pre), "ev.code()", &mut order);
                }
                for (code, mods) in match_arms(&body, "match ev.key().as_str() {") {
                    push(code, mods.and(pre), "ev.key()", &mut order);
                }
                for code in key_equals(&body) {
                    push(code, pre, "ev.key()", &mut order);
                }
                out.push(Listener {
                    file,
                    index,
                    src: body,
                    bindings,
                });
            }
        }
        out
    }

    /// Every binding in the editor surface, flattened.
    pub(crate) fn all_bindings() -> Vec<Binding> {
        listeners().into_iter().flat_map(|l| l.bindings).collect()
    }

    /// Every distinct key code the editor binds. This is what the T-692 coverage pins compare
    /// [`super::SHORTCUTS`] against.
    pub(crate) fn all_bound_codes() -> BTreeSet<String> {
        all_bindings().into_iter().map(|b| b.code).collect()
    }

    /// The window-level editor keydown's ARM LIST as TEXT, comment-scrubbed with the `"KeyX"` arm
    /// literals kept.
    ///
    /// This is the shape the older census pins in `mission_editor` grep against (they assert on
    /// exact guard spellings such as `"KeyA" if modk && !ev.alt_key() && !ev.shift_key() =>`), and
    /// it is the function that existed in four copies before T-703. It stays here, beside the
    /// structured census, so there is exactly one of it.
    ///
    /// The assertion is new: the first `match ev.code().as_str()` in a file must sit in the
    /// PRODUCTION half. A census that slices a fixture out of a test module and reports on that is
    /// the hollow-pin failure this programme keeps finding, and it costs one line to refuse.
    pub(crate) fn keydown_arms(src: &str) -> String {
        let head = "match ev.code().as_str() {";
        let at = src.find(head).expect("an editor keydown match is present");
        assert!(
            !src[..at].contains("\n#[cfg(test)]"),
            "T-703: the first `match ev.code().as_str()` in this source sits after a top-level \
             `#[cfg(test)]` — the census would be reading a test fixture instead of the shipped \
             listener, which is a pin that proves nothing"
        );
        let rest = &src[at..];
        let end = rest.find("_ =>").map_or(rest.len(), |i| i + 4);
        live_source(&rest[..end])
    }

    // ── THE COLLISION TEST ────────────────────────────────────────────────────────────────────

    /// **The ticket.** Two window-level listeners must not both answer one keypress.
    ///
    /// This is the case that has bitten twice already (Backspace: delete-the-selection vs
    /// hide-the-interface; Space: flyTo vs cycle-the-widget). Nothing orders two separate `keydown`
    /// closures — the browser runs both — so an overlap here is not a precedence question, it is two
    /// actions firing on one key.
    #[test]
    fn no_two_listeners_claim_the_same_chord() {
        let all = all_bindings();
        let mut clashes: Vec<String> = Vec::new();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                if a.code != b.code
                    || (a.file == b.file && a.listener == b.listener)
                    || SHARED_CHANNELS.contains(&a.code.as_str())
                    || !a.mods.overlaps(b.mods)
                {
                    continue;
                }
                clashes.push(format!(
                    "`{}` is claimed by {} {} AND by {} {}",
                    a.code,
                    a.site(),
                    a.mods.describe(),
                    b.site(),
                    b.mods.describe()
                ));
            }
        }
        assert!(
            clashes.is_empty(),
            "T-703: KEYBINDING COLLISION — two window-level editor listeners answer the same \
             keypress, so BOTH fire and the operator gets two actions from one key:\n  {}\n\
             Fix it by re-keying one of them, by narrowing a modifier guard so the two predicates \
             no longer overlap, or — if the pile-up is deliberate and every claimant state-gates \
             itself — by adding the code to `SHARED_CHANNELS` with the reason written down.",
            clashes.join("\n  ")
        );
    }

    /// Within ONE listener, `match` order resolves an overlap deterministically — that is why
    /// `mission_history` can put `"KeyZ" if ev.shift_key()` in front of a bare `"KeyZ"` and mean it.
    /// What is never intentional is an arm whose every keypress was already taken by an arm above
    /// it: that binding can never fire, and `rustc` will not warn because guards make arm
    /// reachability undecidable for it.
    #[test]
    fn no_arm_is_shadowed_within_its_own_listener() {
        let mut dead: Vec<String> = Vec::new();
        for l in listeners() {
            for (i, later) in l.bindings.iter().enumerate() {
                for earlier in &l.bindings[..i] {
                    if earlier.code == later.code && later.mods.covered_by(earlier.mods) {
                        dead.push(format!(
                            "{} arm #{} `{}` {} is entirely covered by arm #{} {}",
                            l.file,
                            later.order,
                            later.code,
                            later.mods.describe(),
                            earlier.order,
                            earlier.mods.describe()
                        ));
                    }
                }
            }
        }
        assert!(
            dead.is_empty(),
            "T-703: DEAD BINDING — an arm is fully shadowed by an earlier arm on the same key in \
             the same listener, so it can never run:\n  {}",
            dead.join("\n  ")
        );
    }

    /// The exemption list cannot rot into a dumping ground: a code sits in [`SHARED_CHANNELS`] only
    /// while it really is claimed by two or more listeners. Exempt a key that is singly bound and
    /// this is red — which means the list can only ever describe a pile-up that exists.
    #[test]
    fn every_shared_channel_is_really_shared() {
        let all = all_bindings();
        for code in SHARED_CHANNELS {
            let sites: BTreeSet<(&str, usize)> = all
                .iter()
                .filter(|b| b.code == code)
                .map(|b| (b.file, b.listener))
                .collect();
            assert!(
                sites.len() >= 2,
                "T-703: `{code}` is exempted from the collision rule but only {} listener(s) claim \
                 it. An exemption for a key that is not actually shared is a hole waiting for the \
                 next real collision to fall into — delete the entry.",
                sites.len()
            );
        }
    }

    /// T-776 — the source region that answers ONE shared-channel claim, not the whole listener.
    /// An unrelated `get_untracked()` in another arm (cursor, snap, chrome, …) must not satisfy
    /// the live-state pin: that was the hollow shape NIT-1 named.
    fn key_equals_claim_site(src: &str, code: &str) -> Option<String> {
        let needle = format!("ev.key() == \"{code}\"");
        let at = src.find(&needle)?;
        let before = &src[..at];
        let if_at = before.rfind("if ").unwrap_or(0);
        let after_if = &src[if_at..];
        let rel_open = after_if.find('{')?;
        let open = if_at + rel_open;
        let end = balanced(src, open)?;
        Some(src[if_at..=end].to_string())
    }

    /// Match-arm claim: `(prelude before the match, arm body including the literal head)`.
    fn match_arm_claim(src: &str, code: &str) -> Option<(String, String)> {
        let lit = format!("\"{code}\"");
        let mut from = 0usize;
        while let Some(i) = src[from..].find(&lit) {
            let at = from + i;
            let trimmed = src[at + lit.len()..].trim_start();
            if trimmed.starts_with("=>") || trimmed.starts_with("if ") || trimmed.starts_with('|') {
                let arrow_rel = src[at..].find("=>")?;
                let after_arrow = &src[at + arrow_rel + 2..];
                let body_start_rel = after_arrow.find(|c: char| !c.is_whitespace()).unwrap_or(0);
                let abs = at + arrow_rel + 2 + body_start_rel;
                let end = if src.as_bytes().get(abs) == Some(&b'{') {
                    balanced(src, abs)?
                } else {
                    abs + after_arrow[body_start_rel..]
                        .find('\n')
                        .unwrap_or(after_arrow[body_start_rel..].len())
                };
                let match_at = src[..at].rfind("match ev.").unwrap_or(0);
                return Some((src[..match_at].to_string(), src[at..=end].to_string()));
            }
            from = at + lit.len();
        }
        None
    }

    /// Open/closed latch in the listener prelude: a `get_untracked()` whose nearby window
    /// actually `return`s. An unrelated untracked read (cursor position, …) does not count.
    fn early_return_live_gate(prelude: &str) -> bool {
        let mut from = 0usize;
        while let Some(i) = prelude[from..].find("get_untracked()") {
            let at = from + i;
            let window = &prelude[at..prelude.len().min(at + 80)];
            if window.contains("return") {
                return true;
            }
            from = at + 1;
        }
        false
    }

    /// True when `get_untracked()` / `.escape()` appears in an `if` PREDICATE (the text between
    /// `if` / `if let` and its opening `{`), not merely somewhere in the claim body.
    ///
    /// Wave-134 F1: a decoy `let _ = open.get_untracked()` inside an ungated Escape body used to
    /// green the pin while the act stayed unconditional — the exact shared-channel collision the
    /// exemption narrates.
    fn live_state_in_if_predicate(site: &str) -> bool {
        let mut from = 0usize;
        while let Some(rel) = site[from..].find("if ") {
            let if_at = from + rel;
            if if_at > 0 {
                let prev = site.as_bytes()[if_at - 1];
                if prev.is_ascii_alphanumeric() || prev == b'_' {
                    from = if_at + 2;
                    continue;
                }
            }
            let after = if_at + 3;
            let Some(open_rel) = site[after..].find('{') else {
                from = after;
                continue;
            };
            let cond = &site[after..after + open_rel];
            if cond.contains("get_untracked()") || cond.contains(".escape()") {
                return true;
            }
            from = after + open_rel + 1;
        }
        false
    }

    /// Does this listener's claim path for `code` read live state before acting?
    fn shared_channel_claim_gated(src: &str, code: &str) -> bool {
        if let Some(site) = key_equals_claim_site(src, code) {
            // Predicate that guards the act — not a body-side decoy read (wave-134 F1).
            return live_state_in_if_predicate(&site);
        }
        if let Some((prelude, arm)) = match_arm_claim(src, code) {
            if live_state_in_if_predicate(&arm) {
                return true;
            }
            // `.escape()` as the act's deciding call (editor measure-tool OR-chain).
            if arm.contains(".escape()") {
                return true;
            }
            return early_return_live_gate(&prelude);
        }
        panic!(
            "T-776: could not locate the `{code}` claim site in a shared-channel claimant — the              census saw the binding but the live-state pin cannot find the path that answers it"
        );
    }

    /// What makes the Escape pile-up sound rather than a collision: every claimant reads its own
    /// live state before it acts, so at most one surface is ever dismissed. Pin that, or the
    /// exemption is a wish rather than an argument.
    #[test]
    fn every_shared_channel_claimant_reads_live_state() {
        // T-776 — per CLAIM, not per listener. A substring over `l.src` was satisfied by any
        // unrelated `get_untracked()` in the same closure (cursor, snap, chrome toggles, …) while
        // the Escape path itself stayed unconditional. Wave-134 F1: a decoy untracked read *inside*
        // an ungated Escape body is the same hollow — the latch must sit in the predicate that
        // guards the act. The exemption this pin guards is what keeps a many-claimant Escape
        // channel legal — the last place a weak guard should sit.
        for l in listeners() {
            for b in &l.bindings {
                if !SHARED_CHANNELS.contains(&b.code.as_str()) {
                    continue;
                }
                assert!(
                    shared_channel_claim_gated(&l.src, &b.code),
                    "T-703/T-776: {}#{} claims shared channel `{}` but its claim path never gates the act on                      live state (a `get_untracked()` / `.escape()` in the `if` predicate, `.escape()` as the act, or an early-return latch) — it will                      fire alongside every other claimant on one keypress, which is a collision and                      not a shared channel",
                    l.file,
                    l.index,
                    b.code
                );
            }
        }
        // The editor keydown's own Escape arm is the one claimant with no open/closed latch: it is
        // gated on the measure tools having something to dismiss. `.escape()` returns false when a
        // tool is empty and the arm returns the OR, so an Escape with nothing placed falls through
        // untouched instead of swallowing the key from the dialogs above.
        let arms = keydown_arms(include_str!("mission_editor.rs"));
        let esc = arms
            .find(&format!("\"{}\" if !modk", "Escape"))
            .expect("the editor keydown's Escape arm");
        // Just this arm: from its head to the head of the next one. Every arm in that match is
        // guarded, so `" if ` is where the next one starts.
        let rest = &arms[esc..];
        let body = &rest[..rest[16..].find("\" if ").map_or(rest.len(), |i| i + 16)];
        assert!(
            body.contains(".escape()") && body.contains("||"),
            "T-703: the editor keydown's Escape arm must ACT only when a measurement was really \
             dismissed (the OR of the tools' `.escape()` results) — an arm that returns a bare \
             `true` would swallow Escape from every dialog that shares the channel"
        );
    }

    /// The census must see every listener there is. This is T-738's finding as a standing pin: the
    /// old extractor read two listeners out of the thirteen the editor runs and reported total
    /// coverage, and nothing was red. A new window-level keydown anywhere in the editor surface now
    /// fails here until someone bumps the count — which is the moment to check whether it collides.
    ///
    /// **This pin only sees what [`editor_surface`] hands it, and T-774 is the proof.** The tripwire
    /// was green while `faction_manager` and `orbat_manager` each ran an uncensused window-level
    /// keydown, because neither file was in the list — a growth tripwire over an incomplete input
    /// reports on its own input, not on the editor. Adding a MODULE to the surface is therefore the
    /// one move this pin cannot prompt you to make; the scope note above [`editor_surface`] is what
    /// bounds it, and it is exhaustive on purpose.
    #[test]
    fn every_editor_surface_listener_is_censused() {
        let mut total = 0usize;
        for (file, raw, expected) in editor_surface() {
            let found = listener_bodies(raw).len();
            assert_eq!(
                found, expected,
                "T-703: `{file}` installs {found} window-level keydown listener(s), the census \
                 expects {expected}. If a listener was ADDED, bump the count here — and read \
                 `no_two_listeners_claim_the_same_chord` before you do, because a new listener is \
                 exactly how the Backspace and Space collisions got in. If one was REMOVED, drop \
                 the count."
            );
            total += found;
        }
        assert_eq!(
            total, 13,
            "T-703: the editor surface should carry 13 window-level keydown listeners, found \
             {total}"
        );
        // A listener that yields no binding means the slicer lost the closure body (an unbalanced
        // brace in a literal, a reworded registration) — that is a silently EMPTY census, the
        // exact shape of a pin that passes forever while the UI rots.
        for l in listeners() {
            assert!(
                !l.bindings.is_empty(),
                "T-703: {}#{} was discovered but yielded no binding — the extractor lost its body",
                l.file,
                l.index
            );
        }
    }

    /// Every declared precondition must still match live source. A needle that no longer appears is
    /// a stale entry, and a stale entry means the census is narrowing arms on the strength of a
    /// guard that has been deleted.
    #[test]
    fn every_declared_precondition_is_still_in_the_source() {
        let bodies: Vec<String> = listeners().into_iter().map(|l| l.src).collect();
        for (needle, _) in PRECONDITIONS {
            assert!(
                bodies.iter().any(|b| b.contains(needle)),
                "T-703: the listener precondition `{needle}` matches no listener any more. Either \
                 the guard was reworded (update the needle — until you do, every arm under it is \
                 censused as modifier-free) or it is gone (delete the entry)."
            );
        }
    }

    /// The T-669 lesson, stated as arithmetic: a census that compares bare CODES cannot see the
    /// collisions this ticket is about. `Ctrl+V` and `Ctrl+Shift+V` share a code and do not
    /// collide; `Ctrl+V` and "V with any modifiers" share a code and do. Any rule phrased on codes
    /// alone gets both of those wrong, in opposite directions.
    #[test]
    fn overlap_is_modifier_aware_not_code_aware() {
        let ctrl_v = Mods {
            modk: Some(true),
            alt: Some(false),
            shift: Some(false),
        };
        let ctrl_shift_v = Mods {
            modk: Some(true),
            alt: Some(false),
            shift: Some(true),
        };
        assert!(
            !ctrl_v.overlaps(ctrl_shift_v),
            "Ctrl+V and Ctrl+Shift+V partition the key — a rule that called this a collision would \
             have blocked T-669"
        );
        assert!(
            ctrl_v.overlaps(Mods::ANY),
            "an unguarded `ev.key() == \"…\"` listener claims the key under EVERY modifier, so it \
             collides with a Ctrl-guarded arm on the same code"
        );
        assert!(
            ctrl_v.overlaps(Mods {
                modk: Some(true),
                alt: None,
                shift: None
            }),
            "a Ctrl-anything arm swallows Ctrl+V — this is the shape T-669's Ctrl+Shift+V pin \
             could not see, because both are `KeyV`"
        );
        // Coverage, the other relation, is strictly stronger than overlap.
        assert!(ctrl_v.covered_by(Mods::ANY));
        assert!(!Mods::ANY.covered_by(ctrl_v));
        assert!(!ctrl_v.covered_by(ctrl_shift_v));
    }

    /// T-738 banked this: consume the extractor, do not write a fifth copy. By wave 113 it existed
    /// four times (`mission_editor` ×3, `eden_help` ×1) in two variants, and each copy was one more
    /// place a census could drift from the code it censuses. Exactly one definition, forever.
    #[test]
    fn there_is_exactly_one_extractor() {
        // Assembled so the needle never appears verbatim in this test's own source.
        let needle = format!("fn keydown{}(", "_arms");
        // T-776 — scan the WHOLE crate, not `editor_surface()` + self. A fifth copy in
        // `eden_dock_left` / `editor_ops` / `ui` sat outside the old six-file list and would have
        // passed; this pin enforces T-738's banked "one extractor" instruction, so its input is
        // the crate.
        let src_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut copies: Vec<String> = Vec::new();
        fn walk(dir: &std::path::Path, needle: &str, copies: &mut Vec<String>) {
            let entries = std::fs::read_dir(dir)
                .unwrap_or_else(|e| panic!("T-776: cannot read {}: {e}", dir.display()));
            for ent in entries {
                let ent = ent.expect("read_dir entry");
                let path = ent.path();
                if path.is_dir() {
                    walk(&path, needle, copies);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let raw = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("T-776: cannot read {}: {e}", path.display()));
                let n = raw.matches(needle).count();
                if n > 0 {
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?")
                        .to_string();
                    copies.push(format!("{name} ×{n}"));
                }
            }
        }
        walk(&src_root, &needle, &mut copies);
        copies.sort();
        assert_eq!(
            copies,
            vec!["eden_help.rs ×1".to_string()],
            "T-738/T-776: the keydown-arm extractor must be defined ONCE, in `keymap_census`.              Found: {copies:?}. Consume it (`use crate::eden_help::keymap_census::…`) and widen              it there — a second copy is a second answer to the same question."
        );
    }

    /// T-740 — the module's prose census numbers must be DERIVED from this census, not retyped.
    /// That sentence has gone stale twice; a number nobody can check is a comment pretending to be
    /// a measurement.
    ///
    /// T-774 widened it in both directions. The **input** grew (`faction_manager` and
    /// `orbat_manager` were missing from `editor_surface`, so every count below was true of what the
    /// census SCANNED and false of what the editor RAN), and the **prose under pin** grew: this used
    /// to read the `//!` header alone, which left `keymap_census`'s own doc block free to state
    /// numbers — "nine listeners claim Escape" — that no pin ever checked. Both regions are read
    /// now.
    #[test]
    fn the_prose_census_numbers_are_derived() {
        let raw = include_str!("eden_help.rs");
        // Doc prose is hard-wrapped at 100 columns, so a claim phrase routinely straddles a line
        // break and a naive `contains` then fails on prose that is perfectly correct. Flatten the
        // comment markers and collapse every whitespace run to one space, so what is matched is the
        // SENTENCE and the pin is indifferent to where the wrap lands.
        let flatten = |s: &str| {
            s.split_whitespace()
                .filter(|w| *w != "//!" && *w != "///")
                .collect::<Vec<_>>()
                .join(" ")
        };
        // Only the `//!` header, so this pin can never read its own assertion strings back as
        // evidence (the hollow-pin failure a bare whole-file `include_str!` walks into).
        let header = flatten(
            &raw.lines()
                .take_while(|l| l.starts_with("//!") || l.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        // `keymap_census`'s own doc block: everything before the module opens, which is strictly
        // before this test's source and so is subject to the same no-self-reading guarantee.
        let opens = "pub(crate) mod keymap_census {";
        let census_doc = flatten(
            &raw[..raw.find(opens).expect("the census module")]
                .lines()
                .filter(|l| l.trim_start().starts_with("///"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let ls = listeners();
        let files: BTreeSet<&str> = ls.iter().map(|l| l.file).collect();
        let escape_claimants = ls
            .iter()
            .filter(|l| l.bindings.iter().any(|b| b.code == "Escape"))
            .count();
        let claims = [
            (
                all_bound_codes().len(),
                "distinct `KeyboardEvent` codes",
                &header,
            ),
            (ls.len(), "window-level keydown listeners", &header),
            (files.len(), "editor-surface modules", &header),
            // T-774 — the TOTAL, not just the distinct codes. The T-703 slice reported "39
            // bindings" in prose nobody could check and the wave-119 verifier's own parser said 32;
            // two unchecked numbers about the same census is exactly the defect this module exists
            // to kill, so the total now lives in the header and is derived like the other three.
            (all_bindings().len(), "bindings in total", &header),
            // The Escape pile-up is the census's own headline number and it was typed, not derived.
            (escape_claimants, "listeners claim Escape", &census_doc),
        ];
        for (n, what, prose) in claims {
            let phrase = format!("{} {what}", spell(n));
            assert!(
                prose.contains(&phrase),
                "T-740/T-774: the module prose must say `{phrase}` — the census counts {n}. Do not \
                 retype the old number; this is the third time."
            );
        }
    }

    /// A per-listener breakdown, so a reader can see what the census actually holds rather than
    /// trusting that it holds something. Also a floor: a census that collapsed to a handful of
    /// bindings would make every coverage pin above pass vacuously.
    #[test]
    fn the_census_reports_what_it_found() {
        let ls = listeners();
        let mut by_file: BTreeMap<&str, usize> = BTreeMap::new();
        for l in &ls {
            *by_file.entry(l.file).or_default() += l.bindings.len();
        }
        let total: usize = by_file.values().sum();
        assert!(
            total >= 25,
            "T-703: the editor binds well over two dozen chords; a census finding {total} \
             ({by_file:?}) has broken, and a broken census makes every pin built on it vacuous"
        );
        // Both accessors must be represented, or the widening T-738 asked for has been undone.
        assert!(
            ls.iter()
                .flat_map(|l| &l.bindings)
                .any(|b| b.via == "ev.code()"),
            "the census must read the `match ev.code().as_str()` keydowns"
        );
        assert!(
            ls.iter()
                .flat_map(|l| &l.bindings)
                .any(|b| b.via == "ev.key()"),
            "T-738: the census must read the `ev.key()` listeners too — that widening IS this ticket"
        );
    }
}

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
#[cfg(test)]
mod t692_help_covers_every_binding {
    use super::keymap_census;
    use super::{Shortcut, GROUPS, SHORTCUTS};
    use crate::arsenal::class_r_scrub::live_code;
    use std::collections::BTreeSet;

    /// Every window-level editor keydown listener, as one set of bound codes. One line, because the
    /// extractor lives in exactly one place now (T-738).
    fn all_bound() -> BTreeSet<String> {
        keymap_census::all_bound_codes()
    }

    fn documented() -> BTreeSet<String> {
        SHORTCUTS
            .iter()
            .flat_map(|s| s.codes.iter().map(|c| (*c).to_string()))
            .collect()
    }

    /// The extractor itself must be honest before either coverage assertion means anything: an
    /// extractor that returns nothing would make "every binding is documented" vacuously true, and
    /// that is exactly the shape of a pin that passes forever while the UI rots. Pin arms that
    /// prove each SOURCE was really parsed and a floor on the count.
    ///
    /// T-703 STRENGTHENED this rather than replacing it. It used to prove two files were read;
    /// there were never two, there were six, and the four it could not see were the reason a
    /// binding could ship undocumented with every pin green. `ArrowUp` is the witness that carries
    /// the widening: no `ev.code()` match anywhere binds it, so it can only have come from an
    /// `ev.key()` listener.
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
            bound.contains("ArrowUp") && bound.contains("Enter"),
            "T-738: the extractor must see the `ev.key()` listeners too — `ArrowUp` and `Enter` are \
             bound by `context_menu`'s window keydown and by no `ev.code()` match at all, so their \
             absence means the widening has been undone (found {bound:?})"
        );
        assert!(
            bound.contains("Escape"),
            "the shared Escape channel must be censused (found {bound:?})"
        );
        assert!(
            bound.len() >= 20,
            "the editor binds twenty-odd codes; an extractor finding {} has broken, and a broken \
             extractor makes the coverage pins pass vacuously",
            bound.len()
        );
        // A body literal must never be read as a binding. `center_on_selection` is called from the
        // Space arm's body; no arm-head literal may look like a call fragment.
        assert!(
            bound.iter().all(|c| !c.contains('(')),
            "arm-head extraction picked up something that is not a key code: {bound:?}"
        );
    }

    /// T-738 — the wave-112 MINOR-4 sites (and the two T-774 added) must each appear in the census
    /// as an `ev.key()` Escape claim. `ArrowUp`/`Enter` already witness that *some* `ev.key()`
    /// listener is read; this pin fails if a *known* Escape site is dropped from the scrape while
    /// Escape still arrives from the editor keydown's `ev.code()` arm alone — the exact false-green
    /// shape the ticket names.
    #[test]
    fn known_escape_ev_key_sites_are_censused() {
        let required: &[(&str, &str)] = &[
            // wave-112 MINOR-4
            ("mission_editor.rs", "asset picker / comment / connections"),
            ("attributes.rs", "Attributes modal"),
            ("eden_top_strip.rs", "menus / Save / Controls Hint"),
            // already on the surface when T-703 widened; still a drop-from-scrape trap
            ("context_menu.rs", "context menu"),
            ("eden_settings.rs", "settings dialogs"),
            // T-774 — the two the eleven-listener census still missed
            ("faction_manager.rs", "Faction Manager"),
            ("orbat_manager.rs", "ORBAT Manager"),
        ];
        let all = keymap_census::all_bindings();
        for (file, what) in required {
            assert!(
                all.iter()
                    .any(|b| b.file == *file && b.code == "Escape" && b.via == "ev.key()"),
                "T-738: Escape via `ev.key()` in {file} ({what}) is invisible to the census — that                  is the wave-112 MINOR-4 false-green. Re-add the file to `editor_surface` / teach                  the extractor the idiom; do not document Escape from the `ev.code()` arm alone."
            );
        }
    }

    /// T-738 — the Escape help row must document the SHARED channel, not only measurement dismissal.
    /// The Controls Hint's own close button advertises Esc; a row that names only the ruler lies
    /// to the operator standing on that button.
    #[test]
    fn escape_help_documents_the_shared_channel() {
        let row = SHORTCUTS
            .iter()
            .find(|s| s.codes.contains(&"Escape"))
            .expect("SHORTCUTS must document Escape");
        for needle in [
            "measurement",
            "Save",
            "Attributes",
            "asset picker",
            "settings",
            "context menu",
            "Faction",
            "ORBAT",
            "this card",
        ] {
            assert!(
                row.action.contains(needle),
                "T-738: Escape help action must document the shared channel (missing `{needle}` in                  `{}`); naming only measurement dismissal is the live defect wave-112 MINOR-4 saw",
                row.action
            );
        }
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

    /// T-740 — mission_history redo accepts ctrl OR meta on KeyY (same as undo on KeyZ). A help
    /// chord that documents bare `Ctrl + Y` alone lies to Mac operators and is RED.
    #[test]
    fn redo_chord_documents_cmd_for_key_y() {
        let row = SHORTCUTS
            .iter()
            .find(|s| s.codes.contains(&"KeyY") && s.action == "Redo")
            .expect("SHORTCUTS must document KeyY redo");
        assert!(
            row.chord.contains("Ctrl/Cmd + Y"),
            "T-740: KeyY redo chord must document Cmd (got `{}`); mission_history uses \
             ctrl_key() || meta_key() — bare `Ctrl + Y` alone is a lie on Mac",
            row.chord
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
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        let ed = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
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

#[cfg(test)]
mod t772_controls_hint_close_hitbox {
    //! Wave-118 NIT-1 / T-772 — the ControlsHint close button must keep a comfortable hit box
    //! without widening `eden_layout::BTN_ICON` (dense strip/dock rows need `p-0.5`).
    //!
    //! Hollow-pin discipline:
    //! 1. RED under the original defect (`cn(&[BTN_ICON, HOVER_FILL])` alone).
    //! 2. RED under a second hollow (comment/string decoy or conflicting `p-0.5`+`p-1.5` via
    //!    shared recipe) — pins require the live `HINT_CLOSE_BTN` identifier in the component body
    //!    under `live_code` (literals blanked) and exclusive `p-1.5` on the call-site recipe.

    use super::HINT_CLOSE_BTN;
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};
    use crate::eden_layout::BTN_ICON;

    fn hint_body_source() -> String {
        let src = live_source(include_str!("eden_help.rs"));
        only_body(&src, "pub fn ControlsHint(").to_string()
    }

    fn hint_body_code() -> String {
        let src = live_code(include_str!("eden_help.rs"));
        only_body(&src, "pub fn ControlsHint(").to_string()
    }

    /// Original defect: sizing the overlay dismiss from the dense shared recipe alone.
    #[test]
    fn close_button_uses_call_site_padding_not_shared_recipe_alone() {
        assert!(
            HINT_CLOSE_BTN.contains("p-1.5") && !HINT_CLOSE_BTN.contains("p-0.5"),
            "T-772: HINT_CLOSE_BTN must be comfortable p-1.5 without the dense p-0.5"
        );
        assert!(
            BTN_ICON.contains("p-0.5") && !BTN_ICON.contains("p-1.5"),
            "T-772: do not widen BTN_ICON — dense-row sizing stays p-0.5"
        );
        assert!(
            HINT_CLOSE_BTN.contains("shrink-0")
                && HINT_CLOSE_BTN.contains("rounded")
                && HINT_CLOSE_BTN.contains("text-on-surface")
                && !HINT_CLOSE_BTN.contains("text-on-surface-variant"),
            "T-772: call-site recipe keeps BTN_ICON's bright rest + geometry, padding excepted"
        );

        let body = hint_body_source();
        assert!(
            body.contains("HINT_CLOSE_BTN") && body.contains("HOVER_FILL"),
            "T-772: ControlsHint close must compose HINT_CLOSE_BTN + HOVER_FILL"
        );
        assert!(
            !body.contains("BTN_ICON"),
            "T-772: ControlsHint close must not size from BTN_ICON (wave118 NIT-1 original defect)"
        );
        // `cn` does not twMerge — forbidding a BTN_ICON co-compose also blocks p-0.5+p-1.5
        // conflict. Do not substring-match `p-0.5` on the whole body: shortcut rows use `py-0.5`.
        assert!(
            !body.contains("\"p-0.5\"") && !body.contains(" p-0.5"),
            "T-772: close call site must not also carry dense padding class p-0.5 (cn cannot \
             resolve the conflict with HINT_CLOSE_BTN's p-1.5)"
        );
    }

    /// Second hollow: a comment / blanked-literal decoy must not green the pin. `live_code` blanks
    /// string literals and comments, so only a real identifier use of `HINT_CLOSE_BTN` survives.
    #[test]
    fn call_site_padding_identifier_is_live_not_hollow() {
        let body = hint_body_code();
        assert!(
            body.contains("HINT_CLOSE_BTN"),
            "T-772: HINT_CLOSE_BTN must appear as a live cn argument in ControlsHint (not a \
             comment or string decoy — live_code blanks those)"
        );
        assert!(
            body.contains("HOVER_FILL"),
            "T-772: close must still compose HOVER_FILL (behaviour preserved)"
        );
        assert!(
            !body.contains("BTN_ICON"),
            "T-772 hollow: BTN_ICON must not return to the ControlsHint close call"
        );
    }
}
