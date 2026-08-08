//! T-661 — the Top Command Strip and its `missions`-row mirror, split from `eden_chrome.rs`.
//!
//! Menu bar · editable title · time scrubber + weather · History (disabled) · Undo/Redo · Save
//! dialog · Export · Settings, plus the T-192 `RowMirror` that debounces authored time/weather onto
//! the `missions` row. Not cfg-gated (the doc-driving `on:click` bodies are wasm-gated inside their
//! closures); the mirror's debounce/single-flight state is wasm-only.
#![allow(dead_code)]
use leptos::prelude::*;

// T-192 fix — the row mirror's debounce + single-flight state. Gated because only the wasm build has
// a `setTimeout` to hang a debounce on; the native view shell compiles the components without them.
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

// The inline scrubber/weather author through the same T-193 gate as the Mission Settings dialog.
#[cfg(target_arch = "wasm32")]
use crate::eden_env::author_env;
// T-637 — `STRIP_ROWS` / `ROW_MENUS` / `ROW_TOOLS` and the icon recipe are `eden_layout`'s again
// (the T-634 fold-back); this file renders them rather than redefining them.
use crate::eden_layout::{
    BTN_ICON, DISABLED_GLYPH, DIVIDER, HOVER_FILL, MENU_GUTTER, ROW_MENUS, ROW_TOOLS, STRIP_ROWS,
    TOGGLED_PLATE,
};
// T-633 — the scrubber and the weather picker are the shared Aegis primitives now, not raw
// `<input type="range">` / `<select>`.
use crate::ui::{cn, MaterialIcon, Select, Slider};

// ═══════════════ T-634 — two rows, and one action hierarchy ═══════════════
//
// Eden fits EIGHT menus (`y 0–22`) AND twenty-five tool icons (`y 22–40`, its own row) into 40 px.
// We fit five menus, the title, a scrubber, a weather picker, undo/redo/history, three buttons and a
// gear into ONE 48 px row — which is why it reads as crowded, and why
// `editor_chrome_direction.md` §"Four concrete moves" (1) rescoped this ticket from "no action
// hierarchy" to Eden's structure. Menus on row 1, an icon toolbar on row 2.
//
// **HEIGHT IS A LAYOUT CONTRACT.** `eden_layout::STRIP_TOP_PX` (48) is the top inset four accessors
// and `mission_editor`'s `top-12`/`h-12` are written from, so two rows must SPLIT 48, never add to
// it. The menu row is a fixed `h-6` (24) and the tool row is `flex-1` — it takes whatever is left —
// so the total is 48 BY CONSTRUCTION and nothing downstream of `STRIP_TOP_PX` moves. The two
// `*_PX` consts below state that split as a number the pins can check against `STRIP_TOP_PX`
// itself; they are documentation-with-teeth, not a second source of truth for the height.

// T-637 — the T-634 residue is FOLDED BACK. `STRIP_ROWS`, `ROW_MENUS`, `ROW_TOOLS`, `ROW_MENUS_PX`
// and `ROW_TOOLS_PX` were defined locally here only because `eden_layout` was another slice's `owns`
// in wave 115; they now live in `eden_layout` beside `DOCK_L`/`DOCK_R`/`STRIP_TOP_PX`, and the dead
// one-row `STRIP` they replaced is deleted. This file imports them (see the `use` block below).
// `TOOL_ICON` folded back too — `eden_layout::BTN_ICON` IS the bright, dense recipe now, so the local
// copy that routed around the old muted one has no reason to exist.

/// T-634 — the ONE primary action. `Save Version` earns it: it is the routine, reversible,
/// most-used command, and it is the only FILLED button in the strip. Before this ticket it was one
/// of three buttons at near-equal visual weight (a filled primary and two outlined exports, all
/// `px-3 py-1 text-xs font-medium`), so the routine and the consequential read the same.
const ACTION_PRIMARY: &str = "shrink-0 rounded bg-primary px-2.5 py-0.5 text-xs font-medium text-on-primary transition-colors hover:bg-primary/90";

/// T-634 — the demoted tier: outlined, unfilled, and muted at rest. The two exports wore an outline
/// but full `text-on-surface` next to the primary; they are now ONE `Export` trigger wearing this,
/// with the choice of format a second-level decision inside its menu. Compose with [`HOVER_FILL`].
const ACTION_SECONDARY: &str = "shrink-0 rounded border border-outline-variant/40 px-2.5 py-0.5 text-xs font-medium text-on-surface-variant";

/// T-634 — one dropdown-row recipe, shared by the menu-bar dropdowns and the demoted-export menu, so
/// the demotion lands INSIDE the T-668 menu vocabulary instead of inventing a second dropdown
/// language beside it. Compose with [`HOVER_FILL`] + [`DISABLED_GLYPH`]; every row that uses it
/// leads with the unconditional [`MENU_GUTTER`] cell.
const MENU_ROW: &str = "flex w-full items-center gap-1.5 px-3 py-1.5 text-left text-label-sm text-on-surface disabled:cursor-default disabled:text-outline";

/// T-634 — the dropdown surface itself (menu bar and export menu alike).
const MENU_PANEL: &str =
    "glass animate-menu-in absolute top-full z-50 mt-1 rounded-lg py-1 shadow-lg";

// Top Command Strip (T-172 B9) — menu bar · editable title · time scrubber + weather ·
// History (disabled) · Undo/Redo · Save dialog · Export · Settings.

/// One top-strip menu (T-172 B9). React rendered File/Edit/View/Mission/Environment as dead
/// "(soon)" stubs; these open real dropdowns with the commands that exist. No DOM while closed.
struct MenuItem {
    label: &'static str,
    /// None = disabled row (rendered, not clickable — parity with genuinely-future features).
    action: Option<MenuAction>,
}

#[derive(Clone, Copy)]
enum MenuAction {
    Save,
    /// The editor SUPERSET envelope (`MissionExport`) — re-importable, not loadable by the mod.
    Export,
    /// T-243 — the compiled mod document, the bytes `GET /missions/:id/compiled` serves a game
    /// server. A separate action rather than a replacement for [`MenuAction::Export`]: the two
    /// files answer different questions and both have a caller.
    ExportCompiled,
    Undo,
    Redo,
    Settings,
    // T-645 — the Placement Tools (the "Arrange" menu). Each acts LIVE on the current selection;
    // ops moving > 10 entities confirm (`editor_ops::confirm_bulk`). The dispatch bodies are
    // wasm-gated in `run_action` (like Undo/Redo); the enum + descriptor compile natively.
    /// Apply a placement pattern (Circular / Line / Grid / Fill Area).
    Pattern(crate::place_helpers::PatternKind),
    /// Align the selection to a box edge / centre axis.
    Align(crate::place_helpers::AlignEdge),
    /// Space the selection equally along an axis.
    Space(crate::place_helpers::SpaceAxis),
    /// Orient the selection (N/E/S/W / face-centre / face-away).
    Orient(crate::place_helpers::Orient),
    /// T-692 — toggle the Controls Hint overlay (the keyboard-shortcut reference). A CHECKED
    /// toggle, not a one-shot command: it is the first menu row whose state shows in the T-668
    /// checkmark gutter, and it is deliberately reachable from BOTH the View menu (as the toggle)
    /// and the Help menu (as the reference) — one action, two entry points, because "where do I
    /// find the shortcuts" and "turn the hint off" are different questions.
    ControlsHint,
}

use crate::place_helpers::{AlignEdge, Orient, PatternKind, SpaceAxis};

const MENUS: [(&str, &[MenuItem]); 7] = [
    (
        "File",
        &[
            MenuItem {
                label: "Save Version…",
                action: Some(MenuAction::Save),
            },
            MenuItem {
                label: "Export JSON",
                action: Some(MenuAction::Export),
            },
            // T-634 — the `…` came off. The T-668 convention is that `…` means "opens a dialog",
            // and `export_compiled_now` opens none: it composes the bytes, starts a browser
            // download and reports through a toast. `Save Version…` and `Mission Settings…` keep
            // theirs because they really do put a dialog in front of the operator. Pinned below —
            // a suffix that promises a dialog and delivers a download is the convention leaking.
            //
            // **T-690 re-examined this and the `…` STAYS OFF.** The verb now does more: the compile
            // returns structured findings alongside the bytes, and this row publishes them to the
            // T-655 validation panel. That is not "opening" anything — the panel is a persistent
            // floating card that is already on screen (it survives even hide-chrome, deliberately),
            // so the row updates a surface the operator is already looking at rather than putting a
            // new one in front of them. The `…` promise is about interruption, not about whether the
            // click had a visible effect. Recorded here because the next reader will ask.
            MenuItem {
                label: "Export Compiled Mission",
                action: Some(MenuAction::ExportCompiled),
            },
        ],
    ),
    (
        "Edit",
        &[
            MenuItem {
                label: "Undo",
                action: Some(MenuAction::Undo),
            },
            MenuItem {
                label: "Redo",
                action: Some(MenuAction::Redo),
            },
        ],
    ),
    // T-645 — Placement Tools. Patterns rearrange the selection LIVE; align/space snap it; orient
    // turns it. Ops moving > 10 entities confirm. Disabled (with a "select entities first" tooltip)
    // until at least one entity is selected — the T-668 dead-control rule: no clickable no-op.
    (
        "Arrange",
        &[
            MenuItem {
                label: "Pattern: Circular",
                action: Some(MenuAction::Pattern(PatternKind::Circular)),
            },
            MenuItem {
                label: "Pattern: Line",
                action: Some(MenuAction::Pattern(PatternKind::Line)),
            },
            MenuItem {
                label: "Pattern: Grid",
                action: Some(MenuAction::Pattern(PatternKind::Grid)),
            },
            MenuItem {
                label: "Pattern: Fill Area",
                action: Some(MenuAction::Pattern(PatternKind::FillArea)),
            },
            MenuItem {
                label: "Align Left",
                action: Some(MenuAction::Align(AlignEdge::Left)),
            },
            MenuItem {
                label: "Align Right",
                action: Some(MenuAction::Align(AlignEdge::Right)),
            },
            MenuItem {
                label: "Align Top",
                action: Some(MenuAction::Align(AlignEdge::Top)),
            },
            MenuItem {
                label: "Align Bottom",
                action: Some(MenuAction::Align(AlignEdge::Bottom)),
            },
            MenuItem {
                label: "Align Centres (horizontal)",
                action: Some(MenuAction::Align(AlignEdge::CentreH)),
            },
            MenuItem {
                label: "Align Centres (vertical)",
                action: Some(MenuAction::Align(AlignEdge::CentreV)),
            },
            MenuItem {
                label: "Space Equally (horizontal)",
                action: Some(MenuAction::Space(SpaceAxis::Horizontal)),
            },
            MenuItem {
                label: "Space Equally (vertical)",
                action: Some(MenuAction::Space(SpaceAxis::Vertical)),
            },
            MenuItem {
                label: "Space Equally (along line)",
                action: Some(MenuAction::Space(SpaceAxis::AlongLine)),
            },
            MenuItem {
                label: "Orient North",
                action: Some(MenuAction::Orient(Orient::North)),
            },
            MenuItem {
                label: "Orient East",
                action: Some(MenuAction::Orient(Orient::East)),
            },
            MenuItem {
                label: "Orient South",
                action: Some(MenuAction::Orient(Orient::South)),
            },
            MenuItem {
                label: "Orient West",
                action: Some(MenuAction::Orient(Orient::West)),
            },
            MenuItem {
                label: "Orient: Face Centre",
                action: Some(MenuAction::Orient(Orient::FaceCentre)),
            },
            MenuItem {
                label: "Orient: Face Away",
                action: Some(MenuAction::Orient(Orient::FaceAway)),
            },
        ],
    ),
    (
        "View",
        &[
            // T-692 (MENU-VIEW-017) — the Controls Hint's VIEW-side toggle. A checked row: the
            // T-668 gutter carries a check while the overlay is up, which is what a View menu
            // toggle is supposed to look like (the gutter was reserved for exactly this).
            MenuItem {
                label: "Controls Hint",
                action: Some(MenuAction::ControlsHint),
            },
            MenuItem {
                label: "Map layers — render host (T-159.28)",
                action: None,
            },
        ],
    ),
    (
        "Mission",
        &[
            MenuItem {
                label: "Mission Settings…",
                action: Some(MenuAction::Settings),
            },
            // T-671 — a named route to the two attribute rows this menu previously had no word for.
            // Same pattern, and the same reasoning, as the Environment menu's `Time & Weather
            // (Mission Settings)…` row directly below: one dialog, but an author looking for where
            // the mission's blurb and card picture are set should not have to guess that
            // "Settings" is the answer. Parenthesised destination so the row does not pretend to be
            // a second dialog, and `…` because a dialog is exactly what follows (T-668).
            MenuItem {
                label: "Briefing & Thumbnail (Mission Settings)…",
                action: Some(MenuAction::Settings),
            },
        ],
    ),
    (
        "Environment",
        &[MenuItem {
            label: "Time & Weather (Mission Settings)…",
            action: Some(MenuAction::Settings),
        }],
    ),
    // T-692 (MENU-BAR-008 / MENU-HELP-001) — the Help menu. Eden has one; TBD had none, which is
    // why sixteen bound keys were discoverable only by reading Rust. Last in the bar, the
    // conventional slot. One live row today: the shortcut reference, which is the Controls Hint.
    (
        "Help",
        &[MenuItem {
            label: "Keyboard Shortcuts (Controls Hint)",
            action: Some(MenuAction::ControlsHint),
        }],
    ),
];

/// T-645 — how many entities are currently selected (the Placement Tools' enable gate). Reads the
/// live selection off `editor_ops` under wasm; the native view shell has no doc/selection, so it
/// reports 0 (the placement rows render disabled there — the menu is a wasm-only affordance anyway).
#[must_use]
fn selection_count() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        crate::editor_ops::selection_len()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

/// Minutes-since-midnight ↔ `HH:MM` for the time scrubber (T-172 B9). Pure + tested.
pub fn minutes_to_hhmm(min: u32) -> String {
    format!("{:02}:{:02}", (min / 60) % 24, min % 60)
}

/// `HH:MM` → minutes since midnight; `None` when it is not a clock.
///
/// **T-192 — the trailing `:SS` is accepted on purpose.** `missions.time_of_day` is a Postgres
/// `time` selected as `::text`, so `mission_hydrate::apply_row_meta` writes `06:00:00` into
/// `meta.environment.time` on every load. The old `split_once` parse read the remainder as `00:00`,
/// failed, and silently parked the scrubber at the 06:00 default — so after a reload an author who
/// had set 21:45 saw the slider claim 06:00. That is the same "your setting was quietly reverted"
/// symptom this ticket exists to remove, on the same value.
pub fn hhmm_to_minutes(s: &str) -> Option<u32> {
    let mut parts = s.split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    if let Some(sec) = parts.next() {
        let sec: u32 = sec.parse().ok()?;
        if sec > 59 {
            return None;
        }
    }
    if parts.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// A clock from any of the controls (or from the row hydrate) → canonical `HH:MM`; `None` when it
/// is not one. The shape [`RowMirror::set_time`] sends to `PATCH /missions/{id}`.
pub fn normalize_clock(s: &str) -> Option<String> {
    hhmm_to_minutes(s).map(minutes_to_hhmm)
}

/// Is the route `:id` a real mission row? T-192.
///
/// The editor also mounts on synthetic ids — `mission_editor` falls back to `draft`, and the gate
/// route drives a smoke id — where a row PATCH is a guaranteed 400. Cheap shape check rather than a
/// `uuid` dependency the SPA does not otherwise carry.
///
/// **T-746 — `pub(crate)`.** `eden_settings::ShapeMirror` needs the same rule for its shape GET/PATCH
/// guards. Keeping a second copy as `is_row_id` invited drift; one predicate, one owner.
pub(crate) fn is_mission_row_id(s: &str) -> bool {
    s.len() == 36
        && s.as_bytes().iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// One `missions` column the editor mirrors, plus the name the author knows it by. T-192.
///
/// The label exists because a failure has to be reported in the user's vocabulary: they changed
/// "Time of day" on a slider, not `time_of_day` on a row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct MirroredField {
    column: &'static str,
    label: &'static str,
}

/// T-633 — the inline weather picker's `(wire value, label)` table. A const rather than four
/// `<option>` tags in the view, because [`crate::ui::Select`] takes its options as data: the wire
/// enum and the words the author reads then have ONE definition in this file instead of markup that
/// drifts. Values are the schema's snake_case weather enum — the same strings `MIRROR_WEATHER`
/// mirrors onto the `missions` row, so the picker and the PATCH cannot disagree by construction.
const WEATHER_OPTIONS: &[(&str, &str)] = &[
    ("clear", "Clear"),
    ("overcast", "Overcast"),
    ("heavy_rain", "Heavy Rain"),
    ("dense_fog", "Dense Fog"),
];

const MIRROR_TIME: MirroredField = MirroredField {
    column: "time_of_day",
    label: "Time of day",
};
const MIRROR_WEATHER: MirroredField = MirroredField {
    column: "weather",
    label: "Weather",
};

/// How long a burst of authored values coalesces before the row PATCH goes out.
///
/// **Trailing edge only, on purpose.** A held time scrubber emits ~30 distinct values a second, and
/// the row only ever needs the one the author settles on; every intermediate value is already in the
/// document, and Save Version is the durable path. So the mirror waits for the hand to stop moving
/// rather than narrating the journey — one PATCH per settle instead of thirty per second, none of
/// which can then land out of order and leave the row holding a value the author scrubbed past.
const MIRROR_DEBOUNCE_MS: i32 = 400;

/// The toast a mirror PATCH raises when it does not land.
///
/// **Why a toast at all.** The shipped version only `warn!`ed. `PATCH /missions/{id}` gates on
/// *ownership* (`handlers/missions.rs` `can_edit` — author or admin) while the editor route gates on
/// *role* (`router.rs`, `mission_maker`), so a mission_maker who legitimately opens someone else's
/// **live** mission is refused every mirror PATCH. They watch the setting apply, reload, and
/// `mission_hydrate::apply_row_meta` writes the row back over the document — precisely the bug T-192
/// exists to remove, with the console as its only witness.
///
/// **Why two texts.** A 403 and a dropped connection ask different things of the user. The 403 is
/// structural: retrying cannot help, and neither can Save Version (`create_version` gates on the
/// same `can_edit`), so the only way to keep the change is to own the mission. Anything else — a
/// flaky connection, a restarting API — is worth another go, so it names what the server said.
///
/// Both say the setting will revert, because it will: the row still holds the old value, and the row
/// wins on the next hydrate.
fn mirror_failure_message(field: MirroredField, err: &crate::client::ApiErr) -> String {
    let label = field.label;
    if err.0 == 403 {
        return format!(
            "{label} was not saved — you are not this mission's author. It will revert when the editor reloads."
        );
    }
    format!(
        "Could not save {}: {}. It will revert when the editor reloads — try again.",
        label.to_lowercase(),
        crate::client::api_error_message(err, "the server did not respond")
    )
}

/// T-192 — mirrors an authored environment field onto the `missions` row.
///
/// **Why this exists.** The Mission Settings dialog and the top-strip scrubber below write
/// `meta.environment.{time,weather}` into the CRDT document and nowhere else. Nothing PATCHed the
/// row, and `mission_hydrate::apply_row_meta` re-applies the row over the document on every
/// hydrate — so the author's setting was dropped on the wire (the compile read the row) **and**
/// reverted locally on the next reload. The compile half is fixed in
/// `api/src/services/mission_compile.rs`, which now prefers the environment carried by the saved
/// payload; this is the half that stops the row from reverting it, and keeps the library dossier
/// (which renders the row's Weather/Time) telling the truth.
///
/// **It rides the same event that writes the document.** Not a later `change`: the Mission Settings
/// dialog re-runs its whole view closure on every `doc_tick`, and `update_environment` bumps
/// `doc_tick`, so a `change` handler on a control the rebuild may have re-created is exactly the
/// kind of thing that works until it doesn't — and its failure mode is this ticket's bug again,
/// silently. Mirroring from the doc-write handler makes "the row disagrees with the document"
/// unreachable rather than unlikely. The sequencing that keeps that flood sane lives in
/// [`MirrorState`], not here.
///
/// `Copy` so each control's handler can capture it. Built at component **setup**: `expect_context`,
/// `use_toasts` and `use_params_map` all resolve through the reactive owner, which a plain DOM event
/// handler does not have.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(crate) struct RowMirror {
    auth: crate::auth::AuthStore,
    mission_id: StoredValue<String>,
    /// Where a failed mirror is reported. Resolved at setup for the same reason `auth` is —
    /// `use_toasts()` is an `expect_context` and would panic from a DOM handler or a timer.
    toasts: crate::toast::Toasts,
}

/// Per-column mirror bookkeeping: the dedupe memory, the debounce queue, and the single-flight slot.
///
/// **Why it is a module singleton and not a field of [`RowMirror`].** `TopCommandStrip` and
/// `MissionSettingsDialog` each build their own handle and both write `time_of_day`. If the queue
/// lived in the handle those two would be independent sequencers and could put two PATCHes for the
/// same column on the wire at once — the exact out-of-order hazard this state exists to close. wasm
/// is single-threaded, so a `RefCell` map is the sound analogue of the shared mutable box (the
/// `yrs_persist` idiom).
///
/// **The three transitions below are the whole sequencer, and they are pure** — no timer, no
/// network, no `Toasts` — so the ordering guarantee is provable on the native `cargo test` shell
/// instead of only in a browser. [`RowMirror`] supplies the `setTimeout` and the PATCH around them.
#[derive(Default)]
struct MirrorState {
    /// Last value the row is believed to hold, so an unchanged commit costs nothing. Cleared on a
    /// failed PATCH so the next commit of the same value retries instead of assuming it landed.
    last: String,
    /// Newest authored value not yet on the wire. `None` = nothing waiting.
    pending: Option<String>,
    /// Bumped on every authored value. The response to generation `g` no longer speaks for the
    /// field once `generation != g` — the author has moved on and a successor is queued.
    generation: u64,
    /// The generation on the wire; `None` = idle. **Exactly one PATCH per column at a time**, which
    /// is what makes out-of-order landing unreachable rather than unlikely: a second value cannot
    /// start until the first has settled, so the row's last write is always the author's last edit.
    inflight: Option<u64>,
    /// The live debounce timer — the handle plus the `Closure` it fires, kept alive here so it is
    /// not leaked per-call. Dropped when re-armed, never from inside its own fire. wasm-only:
    /// the native view shell has no `setTimeout` to hang a debounce on.
    #[cfg(target_arch = "wasm32")]
    timer: Option<MirrorTimer>,
}

/// What a settled response is still allowed to do — see [`MirrorState::settle`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Settled {
    /// The author has already moved past the value this response describes. It must not touch
    /// `last` and must not raise a toast — its successor is queued and will report its own outcome.
    stale: bool,
    /// Something is waiting for the wire; re-arm the debounce.
    queued: bool,
}

impl MirrorState {
    /// A control committed `value`. `true` when it is genuinely new, i.e. the debounce must (re)arm.
    fn queue(&mut self, value: String) -> bool {
        // Dedupe against the newest INTENT, not just what the row holds: while a burst is queued,
        // `pending` is what the row is about to hold, so a scrub that wanders back to the queued
        // value is still a no-op.
        if self.pending.as_deref().unwrap_or(self.last.as_str()) == value {
            return false;
        }
        self.generation += 1;
        self.pending = Some(value);
        true
    }

    /// The debounce window closed. `Some((generation, value))` when this column is idle and has
    /// something to send; `None` when a PATCH is already in flight (single flight — the completion
    /// re-arms) or nothing is queued.
    fn take_for_send(&mut self) -> Option<(u64, String)> {
        if self.inflight.is_some() {
            return None;
        }
        let value = self.pending.take()?;
        self.inflight = Some(self.generation);
        Some((self.generation, value))
    }

    /// A response for `generation` came back; `ok` is whether it landed. Frees the single-flight
    /// slot, then records the outcome only if this generation still speaks for the field.
    fn settle(&mut self, generation: u64, value: String, ok: bool) -> Settled {
        if self.inflight == Some(generation) {
            self.inflight = None;
        }
        let stale = self.generation != generation;
        if !stale {
            if ok {
                self.last = value;
            } else {
                self.last.clear(); // did not land — let the next commit of the same value retry
            }
        }
        Settled {
            stale,
            queued: self.pending.is_some(),
        }
    }
}

/// A live debounce timer. The `Closure` is owned here (not `.forget()`) so re-arming drops it.
#[cfg(target_arch = "wasm32")]
struct MirrorTimer {
    handle: i32,
    _closure: wasm_bindgen::closure::Closure<dyn FnMut()>,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Keyed by `MirroredField::column` — two entries, shared by every `RowMirror` on the page.
    static MIRROR: RefCell<HashMap<&'static str, MirrorState>> = RefCell::new(HashMap::new());
}

/// Cancel and drop any armed timer for `column`. Called only from `arm` — never from inside a firing
/// timer, so this cannot drop a `Closure` that is currently running.
#[cfg(target_arch = "wasm32")]
fn clear_mirror_timer(column: &'static str) {
    let armed = MIRROR.with(|m| m.borrow_mut().get_mut(column).and_then(|f| f.timer.take()));
    if let Some(t) = armed {
        if let Some(win) = web_sys::window() {
            win.clear_timeout_with_handle(t.handle);
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl RowMirror {
    pub(crate) fn from_route() -> Self {
        use leptos_router::hooks::use_params_map;
        let id = use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self {
            auth: expect_context::<crate::auth::AuthStore>(),
            mission_id: StoredValue::new(id),
            toasts: crate::toast::use_toasts(),
        }
    }

    /// Queue one field of the row. Runs in the control's event handler, so this is also the only
    /// place the route id is read — everything downstream carries it, and none of it touches a
    /// `StoredValue` that may have been disposed by a navigation mid-debounce.
    fn commit(self, field: MirroredField, value: String) {
        let id = self.mission_id.get_value();
        if value.is_empty() || !is_mission_row_id(&id) {
            return;
        }
        let queued = MIRROR.with(|m| m.borrow_mut().entry(field.column).or_default().queue(value));
        if queued {
            self.arm(field, id);
        }
    }

    /// (Re)start the debounce window. Each commit restarts it, so a burst collapses to one PATCH.
    fn arm(self, field: MirroredField, id: String) {
        use wasm_bindgen::JsCast;
        clear_mirror_timer(field.column);
        let Some(win) = web_sys::window() else {
            return;
        };
        let closure = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            self.fire(field, id.clone());
        });
        let handle = win
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                MIRROR_DEBOUNCE_MS,
            )
            .unwrap_or(0);
        MIRROR.with(|m| {
            if let Some(f) = m.borrow_mut().get_mut(field.column) {
                f.timer = Some(MirrorTimer {
                    handle,
                    _closure: closure,
                });
            }
        });
    }

    /// The window closed: put the queued value on the wire — unless this column already has a PATCH
    /// in flight, in which case `pending` stays put and the in-flight completion re-arms. That is
    /// the single-flight rule; two PATCHes for one column are never open at the same time.
    ///
    /// Deliberately does **not** clear its own timer entry: that would drop the `Closure` currently
    /// running. The stale entry is harmless and is cleared by the next `arm`.
    fn fire(self, field: MirroredField, id: String) {
        let next = MIRROR.with(|m| {
            m.borrow_mut()
                .get_mut(field.column)
                .and_then(MirrorState::take_for_send)
        });
        if let Some((generation, value)) = next {
            self.send(field, id, generation, value);
        }
    }

    /// PATCH one field of the row. Fire-and-forget in the sense that matters — the document already
    /// holds the authored value, so a failed mirror never blocks or undoes the edit — but **not**
    /// silent: see [`mirror_failure_message`].
    fn send(self, field: MirroredField, id: String, generation: u64, value: String) {
        let auth = self.auth;
        let column = field.column;
        leptos::task::spawn_local(async move {
            let body = serde_json::json!({ column: value.clone() });
            let res = crate::client::api_patch::<serde_json::Value>(
                auth,
                &format!("/missions/{id}"),
                body,
            )
            .await;
            // Settle this generation, then ask whether it still speaks for the field.
            let settled = MIRROR.with(|m| {
                m.borrow_mut()
                    .get_mut(column)
                    .map(|f| f.settle(generation, value, res.is_ok()))
                    .unwrap_or(Settled {
                        stale: true,
                        queued: false,
                    })
            });
            if let Err(e) = &res {
                leptos::logging::warn!(
                    "T-192: could not mirror {} onto the mission row: {}",
                    column,
                    crate::client::api_error_message(e, "PATCH /missions/:id failed")
                );
                // A stale generation describes a value the author has already replaced, and its
                // successor is queued and will report its own outcome. Toasting here would stack one
                // per scrub tick and name a value nobody is looking at any more.
                if !settled.stale {
                    self.toasts.error(mirror_failure_message(field, e));
                }
            }
            // Either a value arrived while this one was on the wire, or `fire` found the slot busy
            // and left it queued. Re-arm rather than send now: it costs one debounce window on a
            // value that is only a mirror, and it keeps the PATCH rate bounded under a slow server.
            if settled.queued {
                self.arm(field, id);
            }
        });
    }

    /// A clock from a control (`HH:MM`) or a row hydrate (`HH:MM:SS`) → `missions.time_of_day`.
    /// `<input type="time">` reports a half-entered value as `""`, and [`normalize_clock`] rejects
    /// anything else that is not a whole clock, so a partial edit never leaves the tab.
    pub(crate) fn set_time(self, raw: &str) {
        if let Some(t) = normalize_clock(raw) {
            self.commit(MIRROR_TIME, t);
        }
    }

    /// The weather select value → `missions.weather` (the PATCH rejects anything off the enum).
    pub(crate) fn set_weather(self, raw: &str) {
        self.commit(MIRROR_WEATHER, raw.to_string());
    }
}

#[component]
pub fn TopCommandStrip(
    /// Mission title fallback — the `:id` route param; the doc's `meta.title` wins once read.
    title: String,
    can_undo: RwSignal<bool>,
    can_redo: RwSignal<bool>,
    save_semver: RwSignal<String>,
    save_status: RwSignal<String>,
    /// T-159.26 — unsaved-changes flag; a `•` after the title marks dirty (React's `isDirty` dot).
    #[prop(optional)]
    dirty: Option<RwSignal<bool>>,
    /// T-159.26 — the Mission Settings dialog's open flag (gear button toggles it).
    #[prop(optional)]
    settings_open: Option<RwSignal<bool>>,
    /// T-172 B9 — doc revision; re-reads the env (scrubber/weather) + title after undo/redo.
    #[prop(optional)]
    doc_tick: Option<RwSignal<u64>>,
    /// T-172 B9 — obj count for the Save dialog's size line.
    #[prop(optional)]
    obj_count: Option<RwSignal<usize>>,
    /// T-177 B2 / T-071.0 — the ORBAT Manager modal's open flag (the top-strip button toggles it).
    /// Disabled in the scaffold-only case, like `settings_open`.
    #[prop(optional)]
    orbat_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let open_menu = RwSignal::new(None::<usize>);
    // T-634 — the demoted exports' dropdown. A second latch rather than an eighth `MENUS` entry:
    // the export menu hangs off a BUTTON in the tool row, not off the menu bar, and the two are
    // mutually exclusive (opening either closes the other) so only one dropdown is ever up.
    let export_open = RwSignal::new(false);
    let save_open = RwSignal::new(false);
    let save_notes = RwSignal::new(String::new());
    // T-692 — the Controls Hint's open latch. SEEDED from `eden_help`'s thread-local rather than
    // from `false`, because this whole component unmounts and remounts on every Backspace
    // hide/show cycle (`mission_editor` gates the strip on `chrome_hidden`); seeding from the
    // latch is what makes the card come back the way the operator left it, the way the debug HUD
    // does. Every writer below mirrors back into the latch.
    let hint_open = RwSignal::new(crate::eden_help::hint_shown());
    let set_hint = move |v: bool| {
        hint_open.set(v);
        crate::eden_help::set_hint_shown(v);
    };
    // T-192 — row mirror for the inline scrubber / weather select. Setup-time, not handler-time.
    #[cfg(target_arch = "wasm32")]
    let row_mirror = RowMirror::from_route();
    // T-243 — where the server-truth Export reports its outcome. Resolved at setup for the same
    // reason `row_mirror` is: `use_toasts()` is an `expect_context` and a DOM click handler has no
    // reactive owner to resolve it through.
    #[cfg(target_arch = "wasm32")]
    let toasts = crate::toast::use_toasts();
    // T-181.44 — per-problem lines from a rejected Save (`details` from the 400). Local to the
    // strip because the Save dialog is the only place they are read; `save_status` stays a
    // one-liner because it also renders in the strip itself.
    let save_findings = RwSignal::new(Vec::<String>::new());
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if ev.key() == "Escape" {
                if open_menu.get_untracked().is_some() {
                    open_menu.set(None);
                }
                // T-634 — the export dropdown joins the strip's ONE Escape closure, for the same
                // reason the Controls Hint did: a third window listener is what T-726 is about.
                if export_open.get_untracked() {
                    export_open.set(false);
                }
                if save_open.get_untracked() {
                    save_open.set(false);
                }
                // T-692 — Esc also closes the Controls Hint. It rides THIS listener rather than
                // adding a third window Escape (T-726, the Esc pile-up, is still pending) — the
                // strip already owns an Escape closure, so the hint joins it.
                if hint_open.get_untracked() {
                    set_hint(false);
                }
            }
        });
        on_cleanup(move || esc.remove());
    }
    // Env mirror for the inline scrubber/weather — re-read on every doc change.
    let env = Memo::new(move |_| {
        if let Some(t) = doc_tick {
            t.track();
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::editor_ops::read_env()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::dto::MissionEnv::default()
        }
    });
    // T-659 — per-side slot census + generated summary line, on the SAME `doc_tick` channel as `env`
    // above. `refresh_docks` (`editor_ops.rs:1055`) bumps `doc_tick` from `refresh_signals`
    // (`mission_history.rs:452`) at every mutation site, so this recomputes on slot add/remove/refile
    // with no manual refresh — the "live" the ticket requires. The census is pure over the snapshot
    // rows (`census_from_rows`); the summary composes it with the terrain (from `env`, same memo the
    // scrubber reads) and the game mode when the document carries one.
    let census = Memo::new(move |_| {
        if let Some(t) = doc_tick {
            t.track();
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (factions, squads, slot_squad_ids) = crate::editor_ops::census_input();
            census_from_rows(&factions, &squads, &slot_squad_ids)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            SlotCensus::default()
        }
    });
    // The generated one-liner. Terrain rides `env` (already `doc_tick`-tracked); `mode` is read only
    // under wasm and only when the document actually carries one (it is not a first-class editor
    // field today — see `summary_line`'s "if present" note).
    let summary = Memo::new(move |_| {
        let c = census.get();
        let terrain = env.get().terrain;
        #[cfg(target_arch = "wasm32")]
        let mode = crate::editor_ops::read_env_value("mode")
            .and_then(|v| v.as_str().map(str::to_string))
            .filter(|s| !s.trim().is_empty());
        #[cfg(not(target_arch = "wasm32"))]
        let mode: Option<String> = None;
        summary_line(&c, &terrain, mode.as_deref())
    });
    let run_action = move |a: MenuAction| {
        open_menu.set(None);
        // T-634 — the export menu dispatches through this same function, so it closes here too.
        export_open.set(false);
        match a {
            MenuAction::Save => save_open.set(true),
            MenuAction::Export => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_commands::export_now(&save_semver.get_untracked());
            }
            MenuAction::ExportCompiled => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_commands::export_compiled_now(toasts);
            }
            MenuAction::Undo => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_history::undo();
            }
            MenuAction::Redo => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_history::redo();
            }
            MenuAction::Settings => {
                if let Some(s) = settings_open {
                    s.set(true);
                }
            }
            // T-645 — the Placement Tools act on the live selection through `editor_ops`, which reads
            // the selection/positions from its `OPS_CTX` (like Undo/Redo reach the undo stack). The
            // confirm (> 10 entities) lives inside each `editor_ops` fn. Wasm-gated bodies; the native
            // build compiles the match arms but does nothing (no doc).
            MenuAction::Pattern(kind) => {
                #[cfg(target_arch = "wasm32")]
                {
                    crate::editor_ops::apply_pattern_to_selection(kind);
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = kind;
            }
            MenuAction::Align(edge) => {
                #[cfg(target_arch = "wasm32")]
                {
                    crate::editor_ops::align_selection(edge);
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = edge;
            }
            MenuAction::Space(axis) => {
                #[cfg(target_arch = "wasm32")]
                {
                    crate::editor_ops::space_selection(axis);
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = axis;
            }
            MenuAction::Orient(cmd) => {
                #[cfg(target_arch = "wasm32")]
                {
                    crate::editor_ops::orient_selection(cmd);
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = cmd;
            }
            // T-692 — a TOGGLE, not a one-shot: picking it from either menu flips the overlay, so
            // the same row that opens the reference also puts it away. Ungated (no doc, no
            // web-sys) — the native view shell toggles it too.
            MenuAction::ControlsHint => set_hint(!hint_open.get_untracked()),
        }
    };
    let title_fallback = StoredValue::new(title);
    view! {
        <div class=STRIP_ROWS>
            // ═══════════ ROW 1 — menus · title · census (Eden `y 0–22`) ═══════════
            // Identity and commands: what this mission IS and the eight ways into it. Nothing that
            // acts on the map lives here any more — that is row 2's job.
            <div class=ROW_MENUS>
            // Menu bar (screen 05: File / Edit / Arrange / View / Mission / Environment / Help).
            <div class="flex shrink-0 items-center">
                {MENUS
                    .iter()
                    .enumerate()
                    .map(|(i, (name, items))| {
                        view! {
                            <div class="relative">
                                <button
                                    type="button"
                                    // T-668 — the OPEN menu wears TOGGLED_PLATE (plate + 1px dark top
                                    // border); a closed menu wears HOVER_FILL. Before this the open
                                    // state was a bare `bg-white/10` — byte-identical to every
                                    // neighbour's hover, so "this menu is open" and "the pointer is
                                    // over this menu" were indistinguishable. That was the ticket's
                                    // headline confusion, on the top strip.
                                    // T-634 — `py-0.5` not `py-1`: a 16 px `text-label-sm` line box
                                    // in a 24 px row leaves 4 px, not 8. The state classes are
                                    // untouched.
                                    class=move || {
                                        if open_menu.get() == Some(i) {
                                            cn(&["rounded px-2 py-0.5 text-label-sm", TOGGLED_PLATE])
                                        } else {
                                            cn(&[
                                                "rounded px-2 py-0.5 text-label-sm text-on-surface-variant",
                                                HOVER_FILL,
                                            ])
                                        }
                                    }
                                    on:click=move |_| {
                                        export_open.set(false);
                                        open_menu
                                            .update(|m| {
                                                *m = if *m == Some(i) { None } else { Some(i) };
                                            });
                                    }
                                >
                                    {*name}
                                </button>
                                {move || {
                                    (open_menu.get() == Some(i))
                                        .then(|| {
                                            view! {
                                                <div class=cn(&[MENU_PANEL, "left-0 w-64"])>
                                                    {items
                                                        .iter()
                                                        .map(|it| {
                                                            let label = it.label;
                                                            // T-668 conventions — every menu row leads with the
                                                            // UNCONDITIONAL checkmark gutter (MENU_GUTTER), so
                                                            // labels never shift between menus (Eden's jumping
                                                            // indent is the bug NOT to copy). T-692 is the future
                                                            // the note below anticipated: Controls Hint is a real
                                                            // CHECKED toggle, and its glyph drops INTO this cell
                                                            // without moving the label — which is only true
                                                            // because the gutter was reserved unconditionally.
                                                            // The `…` "opens a dialog" suffix lives in the MENUS
                                                            // labels themselves.
                                                            match it.action {
                                                                Some(a) => {
                                                                    let disabled = move || match a {
                                                                        MenuAction::Undo => !can_undo.get(),
                                                                        MenuAction::Redo => !can_redo.get(),
                                                                        // T-645 — a Placement Tool is dead
                                                                        // without a selection. The dropdown is
                                                                        // conditionally rendered on `open_menu`,
                                                                        // so this closure re-runs at OPEN time
                                                                        // and reads the live selection count
                                                                        // then — correct exactly when the
                                                                        // operator is about to click. (No
                                                                        // clickable no-op — the T-668 rule.)
                                                                        MenuAction::Pattern(_)
                                                                        | MenuAction::Align(_)
                                                                        | MenuAction::Space(_)
                                                                        | MenuAction::Orient(_) => {
                                                                            selection_count() == 0
                                                                        }
                                                                        _ => false,
                                                                    };
                                                                    // Rule (3): a disabled row keeps a tooltip
                                                                    // that explains why it is dark rather than
                                                                    // going silent; an enabled row has none.
                                                                    let title = move || {
                                                                        if !disabled() {
                                                                            ""
                                                                        } else {
                                                                            match a {
                                                                                MenuAction::Pattern(_)
                                                                                | MenuAction::Align(_)
                                                                                | MenuAction::Space(_)
                                                                                | MenuAction::Orient(_) => {
                                                                                    "Select entities first"
                                                                                }
                                                                                _ => "Nothing to do yet",
                                                                            }
                                                                        }
                                                                    };
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            title=title
                                                                            class=cn(
                                                                                &[MENU_ROW, HOVER_FILL, DISABLED_GLYPH],
                                                                            )
                                                                            disabled=disabled
                                                                            on:click=move |_| run_action(a)
                                                                        >
                                                                            <span class=MENU_GUTTER>
                                                                                // T-692 — the gutter's first real
                                                                                // occupant: a check while the
                                                                                // Controls Hint is up. Reactive on
                                                                                // `hint_open`, so toggling it from
                                                                                // the Help menu is reflected in the
                                                                                // View menu and vice versa.
                                                                                {move || {
                                                                                    (matches!(a, MenuAction::ControlsHint)
                                                                                        && hint_open.get())
                                                                                        .then(|| {
                                                                                            view! {
                                                                                                <MaterialIcon
                                                                                                    name="check"
                                                                                                    class="block text-base leading-none"
                                                                                                />
                                                                                            }
                                                                                        })
                                                                                }}
                                                                            </span>
                                                                            <span>{label}</span>
                                                                        </button>
                                                                    }
                                                                        .into_any()
                                                                }
                                                                None => {
                                                                    // A genuinely-future command: rendered as a
                                                                    // DISABLED button (not an inert span) so it
                                                                    // keeps rule (3)'s tooltip AND the gutter — it
                                                                    // explains itself instead of going silent.
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            disabled=true
                                                                            title="Not available yet"
                                                                            class=cn(&[MENU_ROW, DISABLED_GLYPH])
                                                                        >
                                                                            <span class=MENU_GUTTER></span>
                                                                            <span>{label}</span>
                                                                        </button>
                                                                    }
                                                                        .into_any()
                                                                }
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                        })
                                }}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            <span class=DIVIDER></span>
            // Editable mission title (React setTitle) + the dirty dot.
            <div class="flex min-w-0 flex-1 items-center">
                {move || {
                    if let Some(t) = doc_tick {
                        t.track();
                    }
                    #[cfg(target_arch = "wasm32")]
                    let doc_title = {
                        let t = crate::editor_ops::read_title();
                        if t.is_empty() { title_fallback.get_value() } else { t }
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let doc_title = title_fallback.get_value();
                    view! {
                        <input
                            type="text"
                            aria-label="Mission title"
                            // T-634 — `py-0` not `py-0.5`: a 20 px `text-label-md` line box plus
                            // the 1 px focus border is 22, which clears a 24 px row; `py-0.5`
                            // made it exactly 24 and the focus ring touched both edges.
                            class="w-full min-w-0 truncate rounded border border-transparent bg-transparent px-1.5 py-0 text-label-md font-semibold text-on-surface outline-none transition-colors focus:border-outline-variant/40 focus:bg-surface-container"
                            prop:value=doc_title
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let v = event_target_value(&ev);
                                    if !v.trim().is_empty() {
                                        crate::editor_ops::set_title(v.trim());
                                    }
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = &ev;
                            }
                        />
                    }
                }}
                {dirty
                    .map(|d| {
                        view! {
                            <span
                                class=move || if d.get() { "ml-1.5 text-primary" } else { "hidden" }
                                title="Unsaved changes"
                                aria-label="Unsaved changes"
                            >
                                "•"
                            </span>
                        }
                    })}
            </div>
            // T-659 — per-side slot census + generated summary line. Same mono / tabular-nums /
            // `title=`-hook idiom as the `eden_toolbelt` StatusBar OBJ/SEL readout it mirrors, on the
            // `doc_tick` reactivity channel (see the `census`/`summary` memos above). The census sits
            // on top; the generated one-liner (the stable community-naming format) sits below it as a
            // truncating line whose full text is also its tooltip. Both `shrink-0` so the flex title
            // to the left keeps the elastic width.
            //
            // T-634 — the two lines became ONE. They stacked inside a 48 px strip; a 24 px menu row
            // cannot hold a 25 px two-line block, and a readout that overflows its row is the
            // crowding this ticket exists to remove. Same two elements, same `data-` hooks, same
            // tooltips, same truncation — laid side by side across the row instead of down it, with
            // the hairline between them. Nothing is dropped: the summary's full text was already its
            // own tooltip, because it was already truncated at `max-w-[22rem]`.
            <div class="flex shrink-0 items-center gap-2 leading-none">
                <div
                    class="flex items-center gap-1.5 font-mono text-[11px] leading-none tabular-nums text-on-surface-variant"
                    title="Per-side slot census (WEST · EAST · IND · TOTAL)"
                    data-slot-census
                >
                    <span title="WEST (BLUFOR) slots">
                        "WEST "
                        <span class="text-on-surface">{move || census.get().west}</span>
                    </span>
                    <span class="text-outline">"·"</span>
                    <span title="EAST (OPFOR) slots">
                        "EAST "
                        <span class="text-on-surface">{move || census.get().east}</span>
                    </span>
                    <span class="text-outline">"·"</span>
                    <span title="IND (INDFOR) slots">
                        "IND "
                        <span class="text-on-surface">{move || census.get().ind}</span>
                    </span>
                    // Unassigned — rendered ONLY when nonzero (spec): a slot whose squad resolves to
                    // no known side. Hidden entirely when the roster is clean.
                    {move || {
                        let u = census.get().unassigned;
                        (u > 0)
                            .then(|| {
                                view! {
                                    <span class="text-outline">"·"</span>
                                    <span class="text-tactical-yellow" title="Slots with no side">
                                        "UNA "
                                        <span>{u}</span>
                                    </span>
                                }
                            })
                    }}
                    <span class="text-outline">"·"</span>
                    <span title="Total placed slots">
                        "TOTAL "
                        <span class="text-on-surface">{move || census.get().total}</span>
                    </span>
                </div>
                <span class=DIVIDER></span>
                // The generated one-liner — the stable format other tools parse (`summary_line`).
                <div
                    class="max-w-[22rem] truncate font-mono text-[10px] leading-none text-outline"
                    title=move || summary.get()
                    data-mission-summary
                >
                    {move || summary.get()}
                </div>
            </div>
            </div>
            // ═══════════ ROW 2 — the icon toolbar (Eden `y 22–40`, its own row) ═══════════
            // Everything that ACTS: history/undo/redo, the ORBAT Manager, the environment cluster,
            // and the actions at the far right in one hierarchy — one primary, one demoted.
            <div class=ROW_TOOLS>
            // History — present-but-disabled (React parity; version list lands with the history
            // lane). T-634 — first in the row, with Undo/Redo: the three history glyphs are one
            // cluster and they lead the toolbar, where Eden puts its own first tool group.
            <button
                type="button"
                aria-label="History"
                title="Version history (soon)"
                class=cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])
                disabled=true
            >
                <MaterialIcon name="history" class="block text-base leading-none" />
            </button>
            // `aria-label` is the gate's DOM handle for the button path (smoke_undo_editor A3/A6) —
            // a real a11y name, not a test-only attribute.
            <button
                type="button"
                aria-label="Undo"
                title="Undo (Ctrl+Z)"
                class=cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])
                disabled=move || !can_undo.get()
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::mission_history::undo();
                    }
                }
            >
                <MaterialIcon name="undo" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Redo"
                title="Redo (Ctrl+Shift+Z)"
                class=cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])
                disabled=move || !can_redo.get()
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::mission_history::redo();
                    }
                }
            >
                <MaterialIcon name="redo" class="block text-base leading-none" />
            </button>
            <span class=DIVIDER></span>
            // T-177 B2 / T-071.0 — ORBAT Manager: opens the modal shell (browse/select the live
            // faction → squad → slot tree). Disabled in the scaffold-only case (no `orbat_open`
            // signal), mirroring the settings gear. T-634 — it moved off the menu row: it is a
            // command, not a menu, and a command sitting inside the menu bar is exactly the mixing
            // that made one 48 px row unreadable. Its styling is UNCHANGED (T-668 chose that
            // primary-CTA text-button idiom deliberately) — only its row is new.
            <button
                type="button"
                aria-label="ORBAT Manager"
                // T-668 — rule (3): the disabled scaffold-only case keeps its tooltip (a disabled
                // control that explains itself beats one that goes silent). `text-primary` +
                // `hover:bg-primary/15` is the primary-CTA text-button hover idiom (a solid tinted
                // fill, no border — never confusable with TOGGLED_PLATE); the disabled half is
                // DISABLED_GLYPH.
                title="Open the ORBAT Manager"
                class=cn(
                    &[
                        "shrink-0 rounded px-2 py-0.5 text-label-sm font-semibold text-primary transition-colors hover:bg-primary/15",
                        DISABLED_GLYPH,
                    ],
                )
                disabled=orbat_open.is_none()
                on:click=move |_| {
                    if let Some(o) = orbat_open {
                        o.set(true);
                    }
                }
            >
                "ORBAT Manager"
            </button>
            <span class=DIVIDER></span>
            // Inline time scrubber + weather (screen 05 center) — same doc fields as the
            // Mission Settings dialog (`update_environment`, one undo step per commit), and
            // T-192 the same `missions` row mirror, so the two entry points cannot disagree.
            // T-633 — both controls are now the Aegis primitives from `ui`, not browser chrome. The
            // scrubber painted its track and thumb in the UA accent (`accent-[--color-primary]`
            // only tints the widget; the shape stayed the browser's) and the weather picker carried
            // a native arrow. The handlers below are UNCHANGED in substance: the same `author_env`
            // write and the same `RowMirror` commit, still on the SETTLE event only — `Slider`
            // exposes `on_change` (native `change`) and no `input`, so the ~30 values/second a drag
            // produces reach the mirror's dedupe→debounce→single-flight exactly as before, and the
            // primitive adds no signal and no re-render of its own to that path.
            <div class="flex shrink-0 items-center gap-2">
                <Slider
                    label="Time of day"
                    min=0
                    max=1439
                    class="w-28"
                    value=Signal::derive(move || {
                        hhmm_to_minutes(&env.get().time).unwrap_or(360) as i32
                    })
                    on_change=Callback::new(move |mins: i32| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let hhmm = minutes_to_hhmm(mins.clamp(0, 1439) as u32);
                            author_env("time", hhmm.as_str().into());
                            row_mirror.set_time(&hhmm);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = mins;
                    })
                />
                // The live readout IS the drag feedback — the reason `Slider` needs no per-pixel
                // callback. It re-reads the same `env` memo the scrubber paints from.
                <span class="font-mono text-xs tabular-nums text-on-surface-variant">
                    {move || env.get().time}
                </span>
                <Select
                    label="Weather"
                    options=WEATHER_OPTIONS
                    value=Signal::derive(move || env.get().weather)
                    on_change=Callback::new(move |w: String| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            author_env("weather", w.as_str().into());
                            row_mirror.set_weather(&w);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = w;
                    })
                />
                // T-159.26 — Mission Settings (environment). Opens the dialog when a
                // `settings_open` signal is threaded (the editor); disabled in the scaffold-only
                // case.
                //
                // T-634 — the gear used to sit ALONE at the far right of the strip, past the export
                // buttons, belonging to nothing. It belongs HERE: the scrubber and the weather
                // picker are two fields of the Mission Settings dialog rendered inline, and the gear
                // opens the rest of them. Grouping it with the two it extends turns a stranded glyph
                // into the third member of the environment cluster.
                <button
                    type="button"
                    aria-label="Mission settings"
                    title="Mission Settings — the rest of the environment"
                    class=cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])
                    disabled=settings_open.is_none()
                    on:click=move |_| {
                        if let Some(s) = settings_open {
                            s.set(true);
                        }
                    }
                >
                    <MaterialIcon name="settings" class="block text-base leading-none" />
                </button>
            </div>
            // T-634 — the elastic gap. Tools left, actions right: the two ends of the toolbar are
            // the two kinds of thing it holds, and the space between them is what says so.
            <div class="min-w-4 flex-1"></div>
            // The save readout keeps its `min-w-24` reservation so the actions to its right do not
            // shuffle sideways every time the status text changes length.
            <span class="min-w-24 shrink-0 font-mono text-xs text-on-surface-variant">
                {move || save_status.get()}
            </span>
            // ── T-634: the action hierarchy ──────────────────────────────────────────────────────
            //
            // Three buttons stood here at near-equal visual weight — a filled `Save Version` and two
            // outlined exports, all `px-3 py-1 text-xs font-medium text-on-surface`. Weight is the
            // only signal a top strip has, so spending it evenly said the three commands are
            // equivalent, when one is the routine reversible save an author makes twenty times a
            // session and the other two produce files that leave the product.
            //
            // ONE primary: `Save Version`. Chosen over an export because it is the frequent one and
            // the safe one — a primary should be the button you want the operator to reach for
            // without thinking, and `editor_chrome_direction.md` §Open already names Save Version as
            // the candidate for Eden's loudest slot.
            //
            // The two exports are DEMOTED INTO A MENU behind one secondary trigger. A menu rather
            // than "two smaller buttons" because the choice between them is a real question with a
            // real answer that needs prose — the superset envelope re-imports here and the mod
            // cannot read it; the compiled document is what a game server receives — and a dropdown
            // row has room for that where a 90 px button does not. Rather than the File menu alone
            // (which also carries both): losing the one-click export from the strip would be a
            // discoverability regression, and this keeps it one click away while spending a third of
            // the weight. The rows reuse `MENU_ROW`/`MENU_GUTTER`/`MENU_PANEL`, so the demotion
            // lands inside the T-668 menu vocabulary instead of beside it.
            <button
                type="button"
                title="Save an immutable version of this mission"
                class=ACTION_PRIMARY
                on:click=move |_| save_open.set(true)
            >
                "Save Version"
            </button>
            <div class="relative shrink-0">
                <button
                    type="button"
                    aria-label="Export"
                    aria-haspopup="menu"
                    title="Download this mission — pick a format"
                    class=move || {
                        if export_open.get() {
                            cn(&[ACTION_SECONDARY, TOGGLED_PLATE])
                        } else {
                            cn(&[ACTION_SECONDARY, HOVER_FILL])
                        }
                    }
                    on:click=move |_| {
                        open_menu.set(None);
                        export_open.update(|o| *o = !*o);
                    }
                >
                    "Export"
                    // `text-sm`, not `text-base`: a 16 px glyph would grow the 16 px `text-xs` line
                    // box and push the button past the tool row's 24 px.
                    <MaterialIcon
                        name="expand_more"
                        class="ml-0.5 inline-block align-middle text-sm leading-none"
                    />
                </button>
                {move || {
                    export_open
                        .get()
                        .then(|| {
                            view! {
                                <div class=cn(&[MENU_PANEL, "right-0 w-72"])>
                                    <button
                                        type="button"
                                        title="The editor superset envelope — re-imports here; the mod cannot load it"
                                        class=cn(&[MENU_ROW, HOVER_FILL])
                                        on:click=move |_| run_action(MenuAction::Export)
                                    >
                                        <span class=MENU_GUTTER></span>
                                        <span>"Export JSON"</span>
                                    </button>
                                    // T-243 — the compiled mod document (what `/compiled` serves a
                                    // game server), beside the superset envelope above.
                                    // `/compiled` is service-token-only, so this row is the only
                                    // way an author can see these bytes at all.
                                    <button
                                        type="button"
                                        title="The compiled mission document the game server receives"
                                        class=cn(&[MENU_ROW, HOVER_FILL])
                                        on:click=move |_| run_action(MenuAction::ExportCompiled)
                                    >
                                        <span class=MENU_GUTTER></span>
                                        <span>"Export Compiled"</span>
                                    </button>
                                </div>
                            }
                        })
                }}
            </div>
            </div>
            // ═══════════ Overlays — outside both rows, inside the strip ═══════════
            // T-692 — the Controls Hint overlay (MENU-VIEW-017). Mounted HERE, inside the strip,
            // rather than beside it in `mission_editor`: the strip is already one of the four
            // mounts `mission_editor` gates on `chrome_hidden`, so hosting the card in this
            // subtree gives it the Backspace hide/show behaviour BY CONSTRUCTION — there is no
            // second gate that could drift out of step with the first (the debug HUD gets its
            // gating the same way, from inside the status bar). Renders no DOM while closed.
            // T-634 moved it out of row 1 and up to the shell — it is `fixed inset-0`, so it never
            // belonged to a row, and a `fixed` child of a 24 px flex row is a trap for the next
            // edit. The subtree — which is what the gate is — is unchanged.
            <crate::eden_help::ControlsHint open=hint_open />
            // Click-away scrim for an open dropdown (below the dropdowns' z-50). T-634 — it now
            // covers the export menu too, so both dropdowns dismiss the same way.
            {move || {
                (open_menu.get().is_some() || export_open.get())
                    .then(|| {
                        view! {
                            <div
                                class="fixed inset-0 z-40"
                                on:click=move |_| {
                                    open_menu.set(None);
                                    export_open.set(false);
                                }
                            ></div>
                        }
                    })
            }}
            // Save Version dialog (React SaveVersionDialog: semver + notes + size estimate +
            // indeterminate bar while saving). Renders no DOM while closed.
            {move || {
                save_open
                    .get()
                    .then(|| {
                        let estimate = {
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::editor_ops::slots_json()
                                    .as_deref()
                                    .and_then(crate::mission_size::estimate_compiled_bytes)
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                None::<usize>
                            }
                        };
                        let obj = obj_count.map_or(0, |o| o.get());
                        let size_line = match estimate {
                            Some(b) => {
                                format!(
                                    "~{} · {} objects",
                                    crate::mission_size::format_bytes(b),
                                    obj,
                                )
                            }
                            None => format!("{obj} objects"),
                        };
                        let big = estimate.is_some_and(|b| b > 200_000_000);
                        view! {
                            <div
                                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
                                on:click=move |_| save_open.set(false)
                            ></div>
                            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
                                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                                    <div class="min-w-0">
                                        <h2 class="text-headline-sm text-on-surface">"Save Version"</h2>
                                        <p class="mt-1 text-label-md text-on-surface-variant">
                                            "Versions are immutable — pick a new semver."
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        aria-label="Close"
                                        on:click=move |_| save_open.set(false)
                                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                                    >
                                        <MaterialIcon name="close" />
                                    </button>
                                </div>
                                <div class="flex flex-col gap-3 px-6 py-5">
                                    <label class="flex flex-col gap-1">
                                        <span class="text-label-sm uppercase tracking-wider text-outline">
                                            "Version"
                                        </span>
                                        <input
                                            type="text"
                                            aria-label="Version"
                                            class="w-32 rounded border border-outline-variant/40 bg-surface-container px-2 py-1 font-mono text-xs text-on-surface"
                                            prop:value=move || save_semver.get()
                                            on:input=move |ev| save_semver.set(event_target_value(&ev))
                                        />
                                    </label>
                                    <label class="flex flex-col gap-1">
                                        <span class="text-label-sm uppercase tracking-wider text-outline">
                                            "Notes"
                                        </span>
                                        <textarea
                                            aria-label="Editor notes"
                                            rows="2"
                                            class="w-full resize-none rounded border border-outline-variant/40 bg-surface-container px-2 py-1 text-xs text-on-surface"
                                            prop:value=move || save_notes.get()
                                            on:input=move |ev| save_notes.set(event_target_value(&ev))
                                        ></textarea>
                                    </label>
                                    <p class=if big {
                                        "font-mono text-xs text-tactical-yellow"
                                    } else {
                                        "font-mono text-xs text-on-surface-variant"
                                    }>{size_line}</p>
                                    {move || {
                                        save_status
                                            .get()
                                            .starts_with("Saving")
                                            .then(|| {
                                                view! {
                                                    <div class="h-1 w-full overflow-hidden rounded-full bg-surface-variant/40">
                                                        <div class="animate-mc-load-bar h-full w-1/4 rounded-full bg-primary"></div>
                                                    </div>
                                                }
                                            })
                                    }}
                                    <p class="min-h-4 font-mono text-xs text-on-surface-variant">
                                        {move || save_status.get()}
                                    </p>
                                    // T-181.44 — the backend's `details`, one row each. Before
                                    // this the author saw "Save failed (400)" and nothing else,
                                    // and a control character in a callsign only ever surfaced as
                                    // a /compiled 500 in an API log they never read.
                                    {move || {
                                        let rows = save_findings.get();
                                        (!rows.is_empty())
                                            .then(|| {
                                                view! {
                                                    <ul class="max-h-32 list-disc space-y-1 overflow-y-auto rounded border border-error/40 bg-error/5 py-1 pl-5 pr-2 font-mono text-[11px] leading-snug text-error">
                                                        {rows
                                                            .into_iter()
                                                            .map(|r| view! { <li>{r}</li> })
                                                            .collect_view()}
                                                    </ul>
                                                }
                                            })
                                    }}
                                    <button
                                        type="button"
                                        class="self-end rounded bg-primary px-4 py-1.5 text-xs font-medium text-on-primary"
                                        on:click=move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            crate::mission_commands::save_now(
                                                save_semver.get_untracked(),
                                                save_notes.get_untracked(),
                                                save_status,
                                                save_findings,
                                            );
                                        }
                                    >
                                        "Save"
                                    </button>
                                </div>
                            </div>
                        }
                    })
            }}
        </div>
    }
}

// ── Slot census + generated mission summary line (T-659) ─────────────────────────────────────────
//
// The header near the OBJ/SEL census pattern (`eden_toolbelt` StatusBar) now also carries a PER-SIDE
// live slot census (`WEST 78 · EAST 74 · IND 8 · TOTAL 160`) and, below it, a one-line mission
// summary composed from the document. Both ride the SAME reactivity the inline scrubber does — the
// `env` `Memo` above re-reads on every `doc_tick`, and `editor_ops::refresh_docks` bumps `doc_tick`
// from `mission_history::refresh_signals` (`mission_history.rs:452`) at EVERY mutation site (place /
// drag / undo / redo / refile / the IDB restore swap), so the badge is live: it updates on slot
// add/remove/refile with no manual refresh (`editor_ops.rs:1055` is where the bump happens).
//
// **Why this replaces two MissionAnalyzer rules rather than adding a warning.** A side count derived
// straight off the ORBAT snapshot cannot show a malformed state — an unresolved slot lands in the
// UNASSIGNED bucket by construction, not by a rule that might not run — so the "counts disagree" /
// "orphan slot" analyzer checks become unrepresentable rather than caught. That is the same
// "should-be-unrepresentable" move as the T-192 row mirror above.
//
// The derivation is a PURE function over plain rows (`census_from_rows`) so it is testable on the
// native `cargo test` shell; the wasm reader that feeds it the live snapshot is
// `editor_ops::census_input` (which reuses `orbat_manager_snapshot`, not a second doc read).

/// The three Eden sides, in header order, paired with the schema faction `key` each derives from.
///
/// The `key` half is the value `factionsById[..].key` holds (`asset_catalog` `EDEN_SIDES`, and the
/// `orbat_add_squad` guard on `editor_ops.rs:1735`); the `label` half is the milsim-facing word the
/// header shows. WOG's 94%-consistent community naming convention grew out of exactly this label
/// vocabulary, so the labels are part of the stable format the summary line pins below.
const CENSUS_SIDES: [(&str, &str); 3] = [("BLUFOR", "WEST"), ("OPFOR", "EAST"), ("INDFOR", "IND")];

/// A per-side slot tally plus the unassigned remainder — the census the header badge renders.
///
/// `west` / `east` / `ind` are the BLUFOR / OPFOR / INDFOR slot counts; `unassigned` is every slot
/// whose `squadId` does not resolve through a squad to a faction carrying one of the three side keys
/// (a dangling `squadId`, or a faction with an empty/unknown `key`). `total` counts EVERY slot, so
/// `west + east + ind + unassigned == total` always — the invariant that makes the malformed
/// "counts don't add up" state unrepresentable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotCensus {
    pub west: usize,
    pub east: usize,
    pub ind: usize,
    /// Slots that resolve to no known side. Shown in the badge ONLY when nonzero (spec).
    pub unassigned: usize,
    pub total: usize,
}

impl SlotCensus {
    /// The per-side count for a schema faction `key`, or 0 for a key that is not one of the three
    /// Eden sides (which is what makes such a slot land in `unassigned`, not in a side bucket).
    fn count_for_key(&self, key: &str) -> usize {
        match key {
            "BLUFOR" => self.west,
            "OPFOR" => self.east,
            "INDFOR" => self.ind,
            _ => 0,
        }
    }
}

/// Derive the per-side census PURELY from the ORBAT rows — the header's single source of truth.
///
/// Reuses the snapshot's own rows (fed by `editor_ops::census_input`, which reads them once via
/// `orbat_manager_snapshot`); it never re-parses the document. Each `(slot, squadId)` walks
/// squad → faction → `key`; an id that dangles at any hop (deleted squad, faction with no side key)
/// falls through to `unassigned`. `slot_squad_ids` is one entry per slot — its length IS `total`, so
/// the buckets can never disagree with the slot set.
///
/// Pure + total (no panics, no I/O): the whole reason it lives here and not behind the wasm gate.
#[must_use]
pub fn census_from_rows(
    factions: &[crate::outliner::FactionRow],
    squads: &[crate::outliner::SquadRow],
    slot_squad_ids: &[String],
) -> SlotCensus {
    // squadId → side key, resolved once so the per-slot loop is O(1) per slot rather than O(squads).
    let mut side_of_squad: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for sq in squads {
        if let Some(f) = factions.iter().find(|f| f.id == sq.faction_id) {
            side_of_squad.insert(sq.id.as_str(), f.key.as_str());
        }
    }
    let mut c = SlotCensus::default();
    for squad_id in slot_squad_ids {
        c.total += 1;
        match side_of_squad.get(squad_id.as_str()).copied() {
            Some("BLUFOR") => c.west += 1,
            Some("OPFOR") => c.east += 1,
            Some("INDFOR") => c.ind += 1,
            // Dangling squadId, or a squad under a faction with no/unknown side key.
            _ => c.unassigned += 1,
        }
    }
    c
}

/// Human terrain name for the summary (`everon` → `Everon`). Mirrors the `terrain_label` idiom used
/// across the mission pages (`event_hub.rs:50`, `create_mission_dialog.rs:18`) — capitalize the
/// first char — kept local so this owned file carries no cross-module dependency for a one-liner.
fn terrain_label(t: &str) -> String {
    let mut ch = t.chars();
    match ch.next() {
        Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
        None => String::new(),
    }
}

/// **The community naming format — KEEP STABLE. Other tools parse this string.**
///
/// The generated one-liner other tooling reads (the seed of WOG's community naming convention, which
/// came out of the counter alone — so the *format*, not just the counts, is the deliverable). Shape:
///
/// ```text
/// [MODE ]TOTAL on Terrain — WEST w v EAST e[ (+i IND)][ (u unassigned)]
/// ```
///
/// - `MODE` is prefixed with a trailing space ONLY when the document carries a game mode
///   (`editor_ops::read_env_value("mode")`); it is omitted entirely otherwise. Game mode is not a
///   first-class field of the editor document today, so "if present" is literal — most missions emit
///   no mode segment, and that absence is part of the pinned format, not a bug.
/// - `TOTAL` is the whole slot count; `on Terrain` names the map.
/// - The `WEST w v EAST e` core is ALWAYS present (zeros included) so a parser can rely on the two
///   anchor words `on` and ` v ` being there regardless of the roster.
/// - `(+i IND)` appears only when the IND count is nonzero, and `(u unassigned)` only when there are
///   unassigned slots — both are strictly additive suffixes so appending them never moves an earlier
///   field a parser has already located.
///
/// The em-dash separator and the ` v ` / `(+ IND)` punctuation are load-bearing: changing them is a
/// breaking change to every downstream parser. New optional segments must be APPENDED, never
/// inserted, and the anchors above must not move. (This paragraph is the stability pin the ticket
/// asks the tests to hold to via a golden string.)
#[must_use]
pub fn summary_line(census: &SlotCensus, terrain: &str, mode: Option<&str>) -> String {
    let mut out = String::new();
    if let Some(m) = mode {
        let m = m.trim();
        if !m.is_empty() {
            out.push_str(m);
            out.push(' ');
        }
    }
    let terrain = terrain_label(terrain);
    let terrain = if terrain.is_empty() {
        "Unknown".to_string()
    } else {
        terrain
    };
    out.push_str(&format!(
        "{} on {} — WEST {} v EAST {}",
        census.total, terrain, census.west, census.east
    ));
    if census.ind > 0 {
        out.push_str(&format!(" (+{} IND)", census.ind));
    }
    if census.unassigned > 0 {
        out.push_str(&format!(" ({} unassigned)", census.unassigned));
    }
    out
}

// ── Tree rows (T-159.22 / T-172 B6+B7) ──────────────────────────────────────────────────────────
// Both trees collapse: container rows carry a chevron toggle (span, not a nested button — rows are
// `<button>`s) + open/closed folder icons, and depth renders as border-l guide-line runs instead of
// bare padding (the React `TreeView` look). The outliner/ORBAT collapsed sets start EMPTY (fully
// expanded — the T-169 windowing smoke's totals depend on it); the palette seeds from
// `CatalogNode::default_expanded` (only depth-0 faction folders open, `buildCatalogTree` rule 3).
//
// Rows are `<button>`s with a real `aria-label` — focusable, activatable, and the gates' DOM handle,
// the `aria-label="Undo"` precedent above (NOT a test-only attribute).

#[cfg(test)]
mod tests {
    use super::{
        census_from_rows, hhmm_to_minutes, is_mission_row_id, minutes_to_hhmm,
        mirror_failure_message, normalize_clock, summary_line, MirrorState, SlotCensus,
        CENSUS_SIDES, MIRROR_DEBOUNCE_MS, MIRROR_TIME, MIRROR_WEATHER,
    };
    use crate::outliner::{FactionRow, SquadRow};

    // ── T-659 census/summary fixtures ────────────────────────────────────────────────────────────

    /// One faction row carrying a side `key` (`BLUFOR`/`OPFOR`/`INDFOR`). Ids mirror the live shape
    /// `editor_ops::ensure_side_faction` mints (`faction-{SIDE}`), but the census keys off `key`, not
    /// the id, so any id works — the test proves that by using the real shape.
    fn faction(id: &str, key: &str) -> FactionRow {
        FactionRow {
            id: id.to_string(),
            key: key.to_string(),
            name: key.to_string(),
            squad_ids: Vec::new(),
        }
    }

    fn squad(id: &str, faction_id: &str) -> SquadRow {
        SquadRow {
            id: id.to_string(),
            name: id.to_string(),
            faction_id: faction_id.to_string(),
            slot_ids: Vec::new(),
            leader_slot_id: String::new(),
            vehicle_ids: Vec::new(),
        }
    }

    /// Build `slot_squad_ids` — `n` slots pointing at `squad_id` — as `census_input` hands it over.
    fn slots_in(squad_id: &str, n: usize) -> Vec<String> {
        vec![squad_id.to_string(); n]
    }

    #[test]
    fn time_scrubber_roundtrip() {
        assert_eq!(minutes_to_hhmm(0), "00:00");
        assert_eq!(minutes_to_hhmm(360), "06:00");
        assert_eq!(minutes_to_hhmm(1439), "23:59");
        assert_eq!(hhmm_to_minutes("06:00"), Some(360));
        assert_eq!(hhmm_to_minutes("23:59"), Some(1439));
        assert_eq!(hhmm_to_minutes("24:00"), None);
        assert_eq!(hhmm_to_minutes("nope"), None);
        for m in [0u32, 1, 59, 60, 719, 1439] {
            assert_eq!(hhmm_to_minutes(&minutes_to_hhmm(m)), Some(m));
        }
    }

    /// T-192 — `missions.time_of_day` is a Postgres `time`, so the row hydrate puts `HH:MM:SS` in
    /// the document. The scrubber has to read it, or a reload silently shows 06:00 for a mission
    /// set to 21:45 — the same reverted-setting symptom the ticket removes.
    #[test]
    fn scrubber_reads_the_row_hydrate_clock() {
        assert_eq!(hhmm_to_minutes("21:45:00"), Some(1305));
        assert_eq!(hhmm_to_minutes("06:00:00"), Some(360));
        assert_eq!(hhmm_to_minutes("00:00:59"), Some(0));
        // Still a clock parser, not a "contains digits" parser.
        assert_eq!(hhmm_to_minutes("12:00:60"), None);
        assert_eq!(hhmm_to_minutes("12:00:00:00"), None);
        assert_eq!(hhmm_to_minutes("12"), None);
        assert_eq!(hhmm_to_minutes("12:0a"), None);
        assert_eq!(hhmm_to_minutes(""), None);
    }

    /// What [`super::RowMirror::set_time`] sends to `PATCH /missions/{id}`: a canonical `HH:MM`, or
    /// nothing at all. A half-typed clock must never reach the row.
    #[test]
    fn normalize_clock_is_the_patch_shape() {
        assert_eq!(normalize_clock("21:45").as_deref(), Some("21:45"));
        assert_eq!(normalize_clock("21:45:00").as_deref(), Some("21:45"));
        assert_eq!(normalize_clock("6:5").as_deref(), Some("06:05"));
        for bad in ["", "2", "24:00", "12:60", "noon", "12:00:00:00"] {
            assert_eq!(normalize_clock(bad), None, "{bad:?} must not be PATCHed");
        }
    }

    /// T-746 — the row-id predicate is crate-visible so `eden_settings` does not keep a twin.
    #[test]
    fn t746_row_id_predicate_is_crate_visible() {
        use crate::arsenal::class_r_scrub::live_code;
        let src = live_code(include_str!("eden_top_strip.rs"));
        assert!(
            src.contains("pub(crate) fn is_mission_row_id"),
            "T-746: is_mission_row_id must be pub(crate), not a private twin in eden_settings"
        );
    }

    /// The mirror only fires on a real row. `mission_editor` falls back to `draft` and the editor
    /// gate mounts on a smoke id — a PATCH there is a guaranteed 400 and pure console noise.
    #[test]
    fn only_a_uuid_route_id_gets_mirrored() {
        assert!(is_mission_row_id("3f2504e0-4f89-11d3-9a0c-0305e82c3301"));
        assert!(is_mission_row_id("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"));
        for bad in [
            "",
            "draft",
            "smoke",
            "3f2504e0-4f89-11d3-9a0c-0305e82c330",   // short
            "3f2504e0-4f89-11d3-9a0c-0305e82c33011", // long
            "3f2504e0x4f89-11d3-9a0c-0305e82c3301",  // dash in the wrong place
            "3f2504e0-4f89-11d3-9a0c-0305e82c330g",  // non-hex
        ] {
            assert!(!is_mission_row_id(bad), "{bad:?} is not a mission row id");
        }
    }

    /// The columns the mirror PATCHes, and the words the failure toast uses for them. Pinned
    /// because the column half is the API contract (`PatchMissionInput`) and the label half is what
    /// the author reads — `viewDistance` / `thermals` are absent because T-193 stopped the editor
    /// authoring them at all, not because the mirror declined to carry them.
    #[test]
    fn mirrored_fields_are_the_two_row_columns() {
        assert_eq!(MIRROR_TIME.column, "time_of_day");
        assert_eq!(MIRROR_WEATHER.column, "weather");
        assert_eq!(MIRROR_TIME.label, "Time of day");
        assert_eq!(MIRROR_WEATHER.label, "Weather");
        assert_ne!(MIRROR_TIME.column, MIRROR_WEATHER.column, "one queue each");
    }

    /// A failed mirror must SAY so — the shipped version only `warn!`ed, which is how a
    /// mission_maker editing someone else's live mission watched the setting apply and revert with
    /// no feedback at all. Every failure names the setting and says it will revert.
    #[test]
    fn every_mirror_failure_names_the_setting_and_the_revert() {
        for field in [MIRROR_TIME, MIRROR_WEATHER] {
            for err in [
                (403u16, Some("not your mission".to_string())),
                (400, Some("invalid weather".to_string())),
                (500, None),
                (0, None), // transport: no response at all
            ] {
                let msg = mirror_failure_message(field, &err);
                assert!(
                    msg.to_lowercase().contains(&field.label.to_lowercase()),
                    "{err:?} must name the setting: {msg}"
                );
                assert!(
                    msg.contains("revert"),
                    "{err:?} must warn of the revert: {msg}"
                );
            }
        }
    }

    /// 403 and "the server fell over" call for different action, so they must not read the same.
    /// The ownership refusal is structural — `PATCH /missions/:id` gates on `can_edit` (author or
    /// admin) while the editor route gates on role — so retrying cannot help and the text must not
    /// suggest it. Everything else is worth another go and carries what the server said.
    #[test]
    fn forbidden_and_transport_failures_read_differently() {
        let denied = mirror_failure_message(MIRROR_TIME, &(403, Some("not your mission".into())));
        let dropped = mirror_failure_message(MIRROR_TIME, &(0, None));
        assert_ne!(denied, dropped);
        assert!(
            denied.contains("author"),
            "403 must name the cause: {denied}"
        );
        assert!(
            !denied.contains("try again"),
            "403 is not retryable: {denied}"
        );
        assert!(
            dropped.contains("try again"),
            "a transport failure is: {dropped}"
        );
        assert!(
            dropped.contains("the server did not respond"),
            "a bodyless failure still says what happened: {dropped}"
        );
        // A backend message is surfaced verbatim (capitalized), not flattened to one house string.
        let bad = mirror_failure_message(MIRROR_WEATHER, &(400, Some("invalid weather".into())));
        assert!(bad.contains("Invalid weather"), "{bad}");
    }

    /// The rate bound. A held scrubber emits ~30 distinct values a second; the window has to be long
    /// enough to swallow that burst and short enough that a settled value lands while the author is
    /// still looking at the dialog.
    #[test]
    fn mirror_debounce_bounds_the_patch_rate() {
        assert!(
            MIRROR_DEBOUNCE_MS >= 200,
            "a 30 Hz scrub must collapse to one PATCH"
        );
        assert!(MIRROR_DEBOUNCE_MS <= 1000, "a settle must feel immediate");
    }

    /// A held scrubber: 30 distinct values inside one debounce window must reach the wire as ONE
    /// PATCH carrying the value the author stopped on. Every intermediate value is in the document
    /// already, so none of them is worth a round trip — and 30 of them racing is how the row ends up
    /// holding one the author scrubbed past.
    #[test]
    fn a_burst_collapses_to_the_settled_value() {
        let mut f = MirrorState::default();
        let mut rearms = 0;
        for m in 0..30u32 {
            if f.queue(minutes_to_hhmm(360 + m)) {
                rearms += 1; // each commit restarts the window; the timer fires once, at the end
            }
        }
        assert_eq!(rearms, 30, "every distinct value re-arms");
        let (generation, value) = f
            .take_for_send()
            .expect("the window closed with work queued");
        assert_eq!(
            value, "06:29",
            "the wire gets the settled value, not the first"
        );
        assert_eq!(f.take_for_send(), None, "and only that one");
        assert!(
            !f.settle(generation, value, true).queued,
            "nothing left over"
        );
        assert_eq!(f.last, "06:29");
    }

    /// The single-flight rule, which is what actually makes out-of-order landing unreachable: while
    /// one PATCH is on the wire, a newer value cannot start a second one. It waits, and the
    /// completion hands it the slot — so the row's last write is always the author's last edit.
    #[test]
    fn a_second_patch_cannot_start_while_one_is_in_flight() {
        let mut f = MirrorState::default();
        assert!(f.queue("06:00".into()));
        let (first, first_value) = f.take_for_send().expect("first goes out");

        assert!(f.queue("21:45".into()), "the author moves on mid-flight");
        assert_eq!(
            f.take_for_send(),
            None,
            "the window may close again, but the slot is busy"
        );

        // The first response comes back. It no longer speaks for the field, so it must not write
        // `last` — otherwise the losing value is what the next hydrate believes.
        let settled = f.settle(first, first_value, true);
        assert!(settled.stale, "the author has moved past 06:00");
        assert!(settled.queued, "21:45 is still waiting");
        assert_eq!(f.last, "", "a stale response must not record a value");

        let (second, second_value) = f.take_for_send().expect("the slot is free now");
        assert_eq!(second_value, "21:45");
        assert!(second > first, "generations are monotonic");
        assert!(!f.settle(second, second_value, true).stale);
        assert_eq!(f.last, "21:45", "the row ends on the author's last edit");
    }

    /// A stale FAILURE is just as silent as a stale success: it must not clear `last` (which
    /// describes a different generation) and must not toast (its successor will). The newest
    /// failure always speaks — that is the whole MAJOR.
    #[test]
    fn only_the_newest_generation_reports_a_failure() {
        let mut f = MirrorState::default();
        assert!(f.queue("clear".into()));
        let (first, first_value) = f.take_for_send().unwrap();
        assert!(!f.settle(first, first_value, true).stale);
        assert_eq!(f.last, "clear");

        assert!(f.queue("overcast".into()));
        let (second, second_value) = f.take_for_send().unwrap();
        assert!(f.queue("dense_fog".into()), "author moves on mid-flight");
        let stale_failure = f.settle(second, second_value, false);
        assert!(stale_failure.stale, "no toast for a value already replaced");
        assert_eq!(f.last, "clear", "a stale failure must not rewrite last");

        let (third, third_value) = f.take_for_send().unwrap();
        let live_failure = f.settle(third, third_value, false);
        assert!(!live_failure.stale, "THIS one must reach the user");
        assert!(!live_failure.queued);
        // Cleared so the very next commit of "dense_fog" retries rather than deduping away.
        assert_eq!(f.last, "");
        assert!(f.queue("dense_fog".into()), "the retry is not deduped away");
    }

    /// The T-192 dedupe, extended to cover the queue: a rebuild replaying a value, or a scrub that
    /// wanders back to what is already queued, must not cost a PATCH or bump the generation.
    #[test]
    fn an_unchanged_value_costs_nothing() {
        let mut f = MirrorState::default();
        assert!(f.queue("06:00".into()));
        assert!(!f.queue("06:00".into()), "same value, already queued");
        assert_eq!(f.generation, 1, "a no-op must not bump the generation");

        let (g, v) = f.take_for_send().unwrap();
        f.settle(g, v, true);
        assert!(!f.queue("06:00".into()), "same value, already on the row");
        assert_eq!(f.take_for_send(), None, "and nothing to send");
    }

    // ── T-659 — census derivation ────────────────────────────────────────────────────────────────

    /// The header example, verbatim: `WEST 78 · EAST 74 · IND 8 · TOTAL 160`. A multi-side roster
    /// tallies each side off its faction `key`, and the total equals the slot set. This is the pure
    /// derivation the badge renders — no doc, no wasm.
    #[test]
    fn census_counts_each_side_and_totals() {
        let factions = [
            faction("faction-BLUFOR", "BLUFOR"),
            faction("faction-OPFOR", "OPFOR"),
            faction("faction-INDFOR", "INDFOR"),
        ];
        let squads = [
            squad("sq-w", "faction-BLUFOR"),
            squad("sq-e", "faction-OPFOR"),
            squad("sq-i", "faction-INDFOR"),
        ];
        let mut slot_squad_ids = slots_in("sq-w", 78);
        slot_squad_ids.extend(slots_in("sq-e", 74));
        slot_squad_ids.extend(slots_in("sq-i", 8));

        let c = census_from_rows(&factions, &squads, &slot_squad_ids);
        assert_eq!(
            c,
            SlotCensus {
                west: 78,
                east: 74,
                ind: 8,
                unassigned: 0,
                total: 160,
            }
        );
        // The invariant that makes "counts don't add up" unrepresentable.
        assert_eq!(c.west + c.east + c.ind + c.unassigned, c.total);
    }

    /// The zero state — an empty document (no factions, no squads, no slots) — is a clean all-zero
    /// census, not a panic or an unassigned pile. This is the mount-time state the badge renders
    /// before the first place.
    #[test]
    fn census_zero_state_is_all_zero() {
        let c = census_from_rows(&[], &[], &[]);
        assert_eq!(c, SlotCensus::default());
        assert_eq!(c.total, 0);
        assert_eq!(c.unassigned, 0);
    }

    /// Unassigned handling — the whole point of deriving the census off the snapshot rather than a
    /// rule. Three ways a slot resolves to no side, all landing in `unassigned` and none in a side
    /// bucket: (1) a `squadId` with no squad in the map (the seed-slot "dangling squadId" case,
    /// `editor_ops.rs:15`); (2) a squad under a faction with an empty side `key`; (3) an empty
    /// `squadId`. The malformed state is a bucket, not a caught error.
    #[test]
    fn census_unassigned_covers_every_unresolved_slot() {
        let factions = [
            faction("faction-BLUFOR", "BLUFOR"),
            // A faction the doc kept but whose side key never got written.
            faction("faction-mystery", ""),
        ];
        let squads = [
            squad("sq-w", "faction-BLUFOR"),
            squad("sq-mystery", "faction-mystery"),
        ];
        let mut ids = slots_in("sq-w", 5); // → WEST
        ids.extend(slots_in("sq-ghost", 3)); // dangling squadId (no such squad)
        ids.extend(slots_in("sq-mystery", 2)); // squad under a keyless faction
        ids.push(String::new()); // slot with no squadId at all

        let c = census_from_rows(&factions, &squads, &ids);
        assert_eq!(c.west, 5);
        assert_eq!(c.east, 0);
        assert_eq!(c.ind, 0);
        assert_eq!(c.unassigned, 6, "3 ghost + 2 keyless + 1 empty");
        assert_eq!(c.total, 11);
        assert_eq!(c.west + c.east + c.ind + c.unassigned, c.total);
    }

    /// A single-side roster is exactly what it says: WEST populated, EAST/IND zero, no unassigned.
    /// Guards against an off-by-one that would leak the other sides' zeros into `unassigned`.
    #[test]
    fn census_single_side_leaves_others_zero() {
        let factions = [faction("faction-OPFOR", "OPFOR")];
        let squads = [squad("sq-e", "faction-OPFOR")];
        let c = census_from_rows(&factions, &squads, &slots_in("sq-e", 12));
        assert_eq!(c.east, 12);
        assert_eq!(c.west, 0);
        assert_eq!(c.ind, 0);
        assert_eq!(c.unassigned, 0);
        assert_eq!(c.total, 12);
    }

    /// The side→label table is the vocabulary the community naming convention rides. Pin it so a
    /// rename (WEST→BLUEFOR, say) is a deliberate, test-breaking act, not a silent drift — the WOG
    /// lesson the ticket calls out. `count_for_key` must agree with the table.
    #[test]
    fn census_side_labels_are_pinned() {
        assert_eq!(
            CENSUS_SIDES,
            [("BLUFOR", "WEST"), ("OPFOR", "EAST"), ("INDFOR", "IND")]
        );
        let c = SlotCensus {
            west: 1,
            east: 2,
            ind: 3,
            unassigned: 0,
            total: 6,
        };
        assert_eq!(c.count_for_key("BLUFOR"), 1);
        assert_eq!(c.count_for_key("OPFOR"), 2);
        assert_eq!(c.count_for_key("INDFOR"), 3);
        assert_eq!(
            c.count_for_key("CIV"),
            0,
            "a non-side key counts to no bucket"
        );
    }

    // ── T-659 — summary-line format (STABLE — other tools parse this) ─────────────────────────────

    /// **GOLDEN — the community naming format. Changing this string is a breaking change.**
    ///
    /// The full-roster line, matching the ticket's illustrative shape (`"COOP 160 on Everon — WEST
    /// 78 v EAST 74 (+8 IND)"`). This golden pins the em-dash, the ` v ` core, the `(+ IND)` suffix,
    /// the mode prefix, and the `everon`→`Everon` label — the load-bearing punctuation
    /// `summary_line` documents as the parser contract.
    #[test]
    fn summary_golden_full_roster_with_mode() {
        let c = SlotCensus {
            west: 78,
            east: 74,
            ind: 8,
            unassigned: 0,
            total: 160,
        };
        assert_eq!(
            summary_line(&c, "everon", Some("COOP")),
            "COOP 160 on Everon — WEST 78 v EAST 74 (+8 IND)"
        );
    }

    /// No mode present → no mode segment, and the ` v ` core plus the two anchor words `on` / ` v `
    /// are still there so a parser can locate the fields regardless of the mode's absence. This is
    /// the common case (game mode is not a first-class editor field today).
    #[test]
    fn summary_omits_mode_when_absent() {
        let c = SlotCensus {
            west: 10,
            east: 10,
            ind: 0,
            unassigned: 0,
            total: 20,
        };
        let line = summary_line(&c, "arland", None);
        assert_eq!(line, "20 on Arland — WEST 10 v EAST 10");
        assert!(!line.contains(" (+"), "no IND suffix when IND is zero");
        assert!(line.contains(" on "), "the `on` anchor is always present");
        assert!(line.contains(" v "), "the ` v ` anchor is always present");
    }

    /// The optional suffixes are strictly APPEND-ONLY and ordered `(+i IND)` then `(u unassigned)`,
    /// so adding either never moves an earlier field. Both present at once here; the IND suffix
    /// precedes the unassigned one.
    #[test]
    fn summary_suffixes_are_append_only_and_ordered() {
        let c = SlotCensus {
            west: 5,
            east: 4,
            ind: 3,
            unassigned: 2,
            total: 14,
        };
        let line = summary_line(&c, "everon", Some("TVT"));
        assert_eq!(
            line,
            "TVT 14 on Everon — WEST 5 v EAST 4 (+3 IND) (2 unassigned)"
        );
        // The core prefix is byte-identical to the no-suffix line — proof the suffixes only append.
        let core = "TVT 14 on Everon — WEST 5 v EAST 4";
        assert!(line.starts_with(core), "suffixes must not perturb the core");
        let ind_at = line.find("(+3 IND)").expect("IND suffix present");
        let una_at = line
            .find("(2 unassigned)")
            .expect("unassigned suffix present");
        assert!(ind_at < una_at, "IND suffix precedes the unassigned suffix");
    }

    /// A blank/unknown terrain still yields a parseable line (`Unknown` placeholder), never an empty
    /// map name that would leave the `on ` anchor dangling.
    #[test]
    fn summary_handles_blank_terrain() {
        let c = SlotCensus::default();
        assert_eq!(summary_line(&c, "", None), "0 on Unknown — WEST 0 v EAST 0");
    }

    /// The census and the summary compose end-to-end: the roster the pure census produces is exactly
    /// what the summary reports. This is the live-update pin's pure half — the header memos feed
    /// `census_from_rows`'s output straight into `summary_line`, and the reactivity source is the
    /// `doc_tick` channel documented on the `census`/`summary` memos (`refresh_docks` →
    /// `editor_ops.rs:1055`), which native tests cannot drive but which the memo wiring pins.
    #[test]
    fn census_and_summary_compose() {
        let factions = [
            faction("faction-BLUFOR", "BLUFOR"),
            faction("faction-INDFOR", "INDFOR"),
        ];
        let squads = [
            squad("sq-w", "faction-BLUFOR"),
            squad("sq-i", "faction-INDFOR"),
        ];
        let mut ids = slots_in("sq-w", 2);
        ids.extend(slots_in("sq-i", 1));
        let c = census_from_rows(&factions, &squads, &ids);
        assert_eq!(
            summary_line(&c, "everon", None),
            "3 on Everon — WEST 2 v EAST 0 (+1 IND)"
        );
    }

    /// FIRE THE RULE ONCE (perturb / fail / restore). This census REPLACES the two MissionAnalyzer
    /// rules by making the malformed state a bucket rather than a caught error, so "firing the rule"
    /// is: a clean roster shows `unassigned == 0`; perturbing a slot onto a dangling squad makes the
    /// census REPORT the orphan (`unassigned > 0`, and the badge would light UNA); restoring the slot
    /// to a real squad returns the census to clean. The malformed state is observable by
    /// construction — there is no analyzer pass that could fail to run.
    #[test]
    fn census_fires_on_an_orphan_then_clears_on_restore() {
        let factions = [faction("faction-BLUFOR", "BLUFOR")];
        let squads = [squad("sq-w", "faction-BLUFOR")];

        // Clean: every slot resolves to WEST.
        let clean = census_from_rows(&factions, &squads, &slots_in("sq-w", 4));
        assert_eq!(clean.unassigned, 0, "clean roster: nothing unassigned");
        assert_eq!(clean.west, 4);

        // Perturb: one slot now points at a squad that isn't in the map (the orphan the old rule
        // existed to catch). The census FIRES — the orphan surfaces in `unassigned`.
        let mut perturbed_ids = slots_in("sq-w", 3);
        perturbed_ids.push("sq-deleted".to_string());
        let perturbed = census_from_rows(&factions, &squads, &perturbed_ids);
        assert_eq!(
            perturbed.unassigned, 1,
            "the orphan is reported, not silently dropped"
        );
        assert_eq!(perturbed.west, 3);
        assert_eq!(perturbed.total, 4, "total still counts every slot");

        // Restore: refile the orphan back onto the real squad — the census returns to clean.
        let restored = census_from_rows(&factions, &squads, &slots_in("sq-w", 4));
        assert_eq!(restored, clean, "restoring clears the fired state");
    }
}

/// T-668 — the top strip speaks the one state vocabulary. The headline conversion is here: the OPEN
/// menu-bar button now wears TOGGLED_PLATE (plate + dark top border) where before it wore a bare
/// `bg-white/10` — byte-identical to every neighbour's hover, so "open" and "hovered" were
/// indistinguishable. Source-inspection pins on scrubbed source, since the strip is a Leptos view a
/// native test cannot render. Needles are assembled from fragments so the file's own prose can never
/// satisfy an absence check (the house rule).
#[cfg(test)]
mod t668_state_vocabulary {
    use crate::arsenal::class_r_scrub::{live_code, live_source};

    /// This file with comments blanked but class STRINGS kept, so the Tailwind literals survive as
    /// the structural landmarks the class pins read.
    fn src_kept() -> String {
        live_source(include_str!("eden_top_strip.rs"))
    }

    /// The open menu-bar button consumes TOGGLED_PLATE (via `cn`), and the closed one HOVER_FILL —
    /// the one state vocabulary, so "this menu is open" reads like every other toggle and can never
    /// be confused with "the pointer is over this menu". Proven on scrubbed CODE (literals blanked)
    /// so the needle is the real `cn(&[…, TOGGLED_PLATE])` call, not a mention.
    #[test]
    fn open_menu_wears_the_toggled_plate_not_the_hover_fill() {
        let code = live_code(include_str!("eden_top_strip.rs"));
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
        let code = live_code(include_str!("eden_top_strip.rs"));
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

    /// Rule (3) — a disabled top-strip menu row keeps a tooltip that explains why it is dark, rather
    /// than going silent. The future-command (`None`-action) row is a DISABLED button carrying a
    /// `title=`, not the inert `<span>` it used to be. Proven on the string-kept source where the
    /// title literal survives.
    #[test]
    fn disabled_controls_keep_their_tooltip() {
        let src = src_kept();
        // The future-command row: a disabled button whose tooltip says it is not available yet.
        assert!(
            src.contains("Not available yet"),
            "a disabled future-command menu row must keep a tooltip (rule 3 — it must not go silent)"
        );
        // …and it is a real button (so the tooltip shows and the row keeps its slot), not a span.
        let code = live_code(include_str!("eden_top_strip.rs"));
        assert!(
            code.contains("disabled=true") && code.contains("title="),
            "the disabled row is a button with a retained title, not an inert span"
        );
    }
}

/// T-692 — the help surface's TOP-STRIP half: a Help menu in the bar (MENU-BAR-008 /
/// MENU-HELP-001) and a View-menu toggle (MENU-VIEW-017), both reaching the one Controls Hint.
///
/// The list's contents are pinned against the real keydown arms in `eden_help`; what this module
/// pins is that the surface is REACHABLE — a shortcut table nothing opens documents nothing. The
/// menus are a `const` table, so these read it directly rather than scraping source.
#[cfg(test)]
mod t692_help_surface {
    use super::{MenuAction, MENUS};

    /// Which menu labels carry a row that opens the Controls Hint.
    fn menus_reaching_the_hint() -> Vec<&'static str> {
        MENUS
            .iter()
            .filter(|(_, items)| {
                items
                    .iter()
                    .any(|it| matches!(it.action, Some(MenuAction::ControlsHint)))
            })
            .map(|(name, _)| *name)
            .collect()
    }

    /// MENU-BAR-008 / MENU-HELP-001 — there is a Help menu, and it is not a stub: its rows are all
    /// live commands. A Help menu whose only row is disabled would be the same silence, dressed up.
    #[test]
    fn the_bar_has_a_live_help_menu() {
        let help = MENUS
            .iter()
            .find(|(name, _)| *name == "Help")
            .expect("T-692: the top strip menu bar must carry a Help menu (MENU-BAR-008)");
        assert!(
            !help.1.is_empty() && help.1.iter().all(|it| it.action.is_some()),
            "T-692: every Help row must be a live command — a disabled-only Help menu documents \
             nothing (MENU-HELP-001)"
        );
    }

    /// MENU-VIEW-017 — and the same overlay is a View-menu toggle, so it can be put away from the
    /// menu that shows chrome state, not only from the card's own close button.
    #[test]
    fn the_hint_is_reachable_from_both_help_and_view() {
        let reaching = menus_reaching_the_hint();
        assert!(
            reaching.contains(&"Help"),
            "T-692: the Help menu must open the Controls Hint (found {reaching:?})"
        );
        assert!(
            reaching.contains(&"View"),
            "T-692 (MENU-VIEW-017): the View menu must carry the Controls Hint toggle (found \
             {reaching:?})"
        );
    }

    /// The action is a TOGGLE reflected in the T-668 checkmark gutter, and the overlay is mounted
    /// from this file (which is what puts it behind `mission_editor`'s `chrome_hidden` gate — the
    /// structural half is pinned in `eden_help`).
    #[test]
    fn the_toggle_is_checked_in_the_gutter_and_mounted_here() {
        let code = crate::arsenal::class_r_scrub::live_code(include_str!("eden_top_strip.rs"));
        assert!(
            code.contains("ControlsHint open=hint_open"),
            "the Controls Hint must be mounted inside the strip's own subtree"
        );
        // The gutter glyph is reactive on the open latch, so both menus agree on the state.
        assert!(
            code.contains("hint_open.get()"),
            "the checkmark gutter must read the LIVE open state, not a constant"
        );
    }
}

/// T-633 — the top strip's two native controls are gone.
///
/// The defect was narrow and visible: the time scrubber was a raw `<input type="range">` whose only
/// styling was `accent-[--color-primary]` — which tints the UA widget and nothing else, so it still
/// drew a browser-blue rail and a browser-shaped thumb against Aegis `#adc6ff` — and the weather
/// picker was a raw `<select>` wearing the platform's native arrow. Both are now `crate::ui`
/// primitives (created by this ticket; see the pins in `ui.rs` for what they guarantee).
///
/// Source pins, on scrubbed source: this is a Leptos view a native test cannot render. Absence
/// needles are assembled from fragments so this module's own prose cannot satisfy them.
#[cfg(test)]
mod t633_aegis_controls {
    use super::WEATHER_OPTIONS;
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

    /// THE FIX, stated as an absence. No raw range input and no raw select may remain in the strip.
    /// Checked on the string-KEPT source, because `type="range"` is a literal and that is exactly
    /// where the defect lived.
    #[test]
    fn no_raw_browser_control_remains_in_the_strip() {
        let src = live_source(include_str!("eden_top_strip.rs"));
        let raw_range = [r#"type=""#, r#"range""#].concat();
        assert!(
            !src.contains(&raw_range),
            "T-633: the time scrubber must be the ui::Slider primitive, not a raw range input"
        );
        let raw_select = ["<sel", "ect"].concat();
        assert!(
            !src.contains(&raw_select),
            "T-633: the weather picker must be the ui::Select primitive, not a raw select element"
        );
        // The accent-colour escape hatch is the thing that LOOKED like a fix and was not: it tints
        // the UA widget and leaves its geometry alone. It must be gone, not merely supplemented.
        let accent = ["accent-[--color-", "primary]"].concat();
        assert!(
            !src.contains(&accent),
            "T-633: `accent-color` only tints browser chrome — the control must paint its own parts"
        );
    }

    /// …and the primitives are actually WIRED, in the strip's own subtree. An absence pin alone is
    /// satisfied by deleting the controls, which is not the fix.
    #[test]
    fn the_strip_renders_the_aegis_primitives() {
        let code = live_code(include_str!("eden_top_strip.rs"));
        let body = only_body(&code, "pub fn TopCommandStrip(");
        for needle in ["<Slider", "<Select", "options=WEATHER_OPTIONS"] {
            assert!(
                body.contains(needle),
                "T-633: the top strip must render `{needle}` — the raw controls were replaced, not \
                 removed"
            );
        }
        // The import is by name, so a stale `ui::` glob cannot make the pin above pass on nothing.
        assert!(
            code.contains("use crate::ui::{cn, MaterialIcon, Select, Slider}"),
            "T-633: the strip must import the two primitives by name from the shared ui module"
        );
    }

    /// **The debounce is not regressed.** T-192's whole point is that a held scrubber emits ~30
    /// values/second and the `missions` row gets ONE PATCH per settle. The commit path is unchanged
    /// by this ticket — `author_env` then `row_mirror.set_time` — and it hangs off the primitive's
    /// `on_change` (the native `change`), never an input/drag event. `ui.rs`'s own pins prove the
    /// primitive has no `on:input`; this proves the STRIP did not grow one around it.
    #[test]
    fn the_scrubber_still_commits_only_on_settle() {
        let code = live_code(include_str!("eden_top_strip.rs"));
        let body = only_body(&code, "pub fn TopCommandStrip(");
        assert!(
            body.contains("on_change=Callback::new(move |mins: i32|"),
            "T-633: the scrubber commits through the primitive's settle callback"
        );
        for step in ["author_env(", "row_mirror.set_time(&hhmm)"] {
            assert!(
                body.contains(step),
                "T-633: the T-192 mirror path (`{step}`) must survive the control swap"
            );
        }
        // The live readout is what makes a settle-only callback sufficient: the operator sees the
        // time move during the drag because the label re-reads the same `env` memo.
        assert!(
            body.contains("env.get().time"),
            "T-633: the HH:MM readout is the drag feedback — without it, settle-only would feel dead"
        );
    }

    /// The option table is data, and it is the wire enum. A picker whose values drifted from the
    /// schema's weather strings would author a document the mod cannot read, and `MIRROR_WEATHER`
    /// would mirror the drift onto the `missions` row.
    #[test]
    fn the_weather_options_are_the_wire_enum() {
        let values: Vec<&str> = WEATHER_OPTIONS.iter().map(|(v, _)| *v).collect();
        assert_eq!(
            values,
            vec!["clear", "overcast", "heavy_rain", "dense_fog"],
            "T-633: the picker's values are the schema's snake_case weather enum, in order"
        );
        assert!(
            WEATHER_OPTIONS.iter().all(|(_, label)| !label.is_empty()),
            "every option needs a human label — a blank row is an unpickable option"
        );
    }
}

/// T-634 — two rows, and one action hierarchy.
///
/// Two defects, one cause. The strip held five menus, a title, a scrubber, a weather picker, three
/// history glyphs, three buttons, a status readout and a gear in ONE 48 px row, so (a) it read as
/// crowded and (b) the only signal left to separate `Save Version` from `Export JSON` and
/// `Export Compiled` — visual weight — was spent evenly across all three. Eden fits eight menus at
/// `y 0–22` and twenty-five tool icons at `y 22–40`, in forty pixels, by giving each kind of thing
/// its own row. That is what these pin: the split, its cost in pixels (zero), and the hierarchy the
/// space bought.
///
/// Source pins on scrubbed source — this is a Leptos view a native test cannot render — plus data
/// pins over `MENUS`, which is a `const` table and can be read directly.
#[cfg(test)]
mod t634_two_rows_and_a_hierarchy {
    use super::MENUS;
    use crate::arsenal::class_r_scrub::{live_source, only_body};
    // T-637 — the row recipes moved to `eden_layout` (the T-634 fold-back) and the dead one-row
    // `STRIP` is deleted. `DOCK_L` stands in for it below: "the strip is the same glass as the docks
    // it sits above" is what comparing the two shells always meant.
    use crate::eden_layout::{
        BTN_ICON, DOCK_L, ROW_MENUS, ROW_MENUS_PX, ROW_TOOLS, ROW_TOOLS_PX, STRIP_ROWS,
        STRIP_TOP_PX,
    };

    /// The strip's own view body, with comments blanked and class/aria literals kept — the literals
    /// ARE the structure these pins read.
    fn body() -> String {
        let src = live_source(include_str!("eden_top_strip.rs"));
        only_body(&src, "pub fn TopCommandStrip(").to_string()
    }

    /// Where a needle first appears in the body. Panics rather than returning an `Option`: a missing
    /// landmark is a renamed control, which is new information, not a silently-skipped ordering.
    fn at(body: &str, needle: &str) -> usize {
        body.find(needle)
            .unwrap_or_else(|| panic!("T-634: the strip no longer contains `{needle}`"))
    }

    /// **THE HEIGHT CONTRACT.** `STRIP_TOP_PX` (48) is the top inset the four `eden_layout`
    /// accessors and `mission_editor`'s `top-12`/`h-12` are written from. Two rows must therefore
    /// SPLIT it, never add to it. The menu row states a fixed `h-6`; the tool row states no height at
    /// all and takes the remainder (`flex-1`), so the sum is 48 by construction and no consumer of
    /// `STRIP_TOP_PX` moves. This checks both halves: the arithmetic, and that the classes really
    /// are "one fixed, one elastic" rather than two fixed heights that could drift apart.
    #[test]
    fn two_rows_split_the_strip_height_they_do_not_add_to_it() {
        assert!(
            (ROW_MENUS_PX + ROW_TOOLS_PX - STRIP_TOP_PX).abs() < f64::EPSILON,
            "T-634: the two rows must sum to eden_layout::STRIP_TOP_PX ({STRIP_TOP_PX}), not \
             {}. A taller strip is a layout change every inset reader downstream would have to \
             follow.",
            ROW_MENUS_PX + ROW_TOOLS_PX
        );
        assert!(
            ROW_MENUS.contains("h-6"),
            "T-634: the menu row is the FIXED one — `h-6` (= {ROW_MENUS_PX} px)"
        );
        assert!(
            ROW_TOOLS.contains("flex-1") && !ROW_TOOLS.contains("h-["),
            "T-634: the tool row must take the REMAINDER (`flex-1`) and state no height of its own \
             — two stated heights can drift apart from STRIP_TOP_PX; a remainder cannot"
        );
    }

    /// The shell is two stacked rows, and its MATERIAL is the docks', verbatim. Only the flow
    /// changed: `items-center` (one centred row) → `flex-col` (two rows). If the surface halves ever
    /// diverge the strip stops matching the docks it sits above, which is the "disjointed" complaint
    /// `editor_chrome_direction.md` exists to answer.
    ///
    /// T-637 — this used to compare `STRIP_ROWS` against `eden_layout::STRIP`, the one-row shell it
    /// replaced. That shell had no consumer left after T-634 and is now deleted, so the comparison
    /// runs against `DOCK_L` instead. This is not a weakening: "the same glass as the docks" was
    /// always the property; `STRIP` was only ever a proxy for it.
    #[test]
    fn the_shell_is_two_rows_in_the_same_glass() {
        let surface =
            "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl";
        assert!(
            DOCK_L.starts_with(surface) && STRIP_ROWS.starts_with(surface),
            "T-634/T-637: the two-row shell must wear the same docked-overlay glass as the docks"
        );
        assert!(
            STRIP_ROWS.contains("border-b") && STRIP_ROWS.contains("border-white/10"),
            "T-634: the strip's bottom edge is unchanged by the row split"
        );
        assert!(
            STRIP_ROWS.contains("flex-col") && STRIP_ROWS.contains("h-full"),
            "T-634: the shell stacks its two rows and still fills the 48 px it is given"
        );
        assert!(
            !STRIP_ROWS.contains("items-center"),
            "T-634: `items-center` in a column would centre the rows horizontally — the shell must \
             let each row stretch to full width"
        );
    }

    /// **Menus on row 1, the icon toolbar on row 2** — Eden's `y 0–22` / `y 22–40`. Proven as an
    /// ORDERING over the rendered body: the menu table is iterated inside the menu row, and every
    /// tool glyph appears only after the tool row opens. A re-layout that put a tool back in the
    /// menu row would move one of these indices past another.
    #[test]
    fn the_menus_own_row_one_and_the_toolbar_owns_row_two() {
        let b = body();
        let row_menus = at(&b, "class=ROW_MENUS");
        let row_tools = at(&b, "class=ROW_TOOLS");
        assert!(
            row_menus < row_tools,
            "T-634: the menu row renders first — Eden puts the menus at y 0–22, above the tools"
        );
        // The menu bar is inside row 1.
        let menu_table = at(&b, "{MENUS");
        assert!(
            row_menus < menu_table && menu_table < row_tools,
            "T-634: the menu bar must render inside the MENU row, not the tool row"
        );
        // …and every tool-row citizen is inside row 2. `History` is the disabled version-list glyph,
        // `Undo`/`Redo` the two live ones, `Mission settings` the gear, `Export` the demoted menu.
        for tool in [
            r#"aria-label="History""#,
            r#"aria-label="Undo""#,
            r#"aria-label="Redo""#,
            r#"aria-label="ORBAT Manager""#,
            r#"aria-label="Mission settings""#,
            r#"aria-label="Export""#,
        ] {
            assert!(
                at(&b, tool) > row_tools,
                "T-634: `{tool}` belongs to the TOOL row — a command in the menu row is the mixing \
                 that made one 48 px strip unreadable"
            );
        }
        // The title stays with the menus: it is identity, not a tool.
        assert!(
            at(&b, r#"aria-label="Mission title""#) < row_tools,
            "T-634: the editable title is row-1 identity, beside the menus"
        );
    }

    /// **ONE primary.** `Save Version` is the only filled button in the strip; the exports are one
    /// secondary trigger. Checked on the CONSTS (a filled recipe vs an outlined one) and on the body
    /// (the primary recipe is used exactly once). Before this ticket three sibling buttons carried
    /// `px-3 py-1 text-xs font-medium` and only their fill differed, so the routine save and the two
    /// consequential exports read at near-equal weight.
    #[test]
    fn exactly_one_action_is_primary() {
        assert!(
            super::ACTION_PRIMARY.contains("bg-primary"),
            "T-634: the primary action is FILLED — that is what makes it the loudest thing"
        );
        assert!(
            !super::ACTION_SECONDARY.contains("bg-primary")
                && super::ACTION_SECONDARY.contains("border"),
            "T-634: the demoted tier is outlined and unfilled — a second fill is a second primary"
        );
        let b = body();
        assert_eq!(
            b.matches("class=ACTION_PRIMARY").count(),
            1,
            "T-634: exactly ONE control in the strip may wear the primary recipe"
        );
        assert!(
            b.contains("class=ACTION_SECONDARY") || b.contains("ACTION_SECONDARY, HOVER_FILL"),
            "T-634: the Export trigger must wear the demoted recipe"
        );
    }

    /// **…and the two exports are demoted BEHIND it, not merely shrunk.** Each export now has
    /// exactly ONE dispatch site in the strip — the shared `run_action` — where before it had two
    /// (the menu row and a top-level button of its own). The second copy was the near-equal weight.
    /// The rows themselves reuse the T-668 menu vocabulary rather than inventing a second dropdown
    /// language: `MENU_PANEL` / `MENU_ROW` / the unconditional `MENU_GUTTER`.
    #[test]
    fn the_exports_live_behind_one_secondary_trigger() {
        let b = body();
        for dispatch in [
            "crate::mission_commands::export_now(",
            "crate::mission_commands::export_compiled_now(",
        ] {
            assert_eq!(
                b.matches(dispatch).count(),
                1,
                "T-634: `{dispatch}` must have exactly one dispatch site in the strip (the shared \
                 `run_action`); a second call site is the top-level button this ticket demoted"
            );
        }
        let gate = at(&b, "export_open");
        for row in [
            "run_action(MenuAction::Export)",
            "run_action(MenuAction::ExportCompiled)",
        ] {
            assert!(
                at(&b, row) > gate,
                "T-634: `{row}` must render inside the export dropdown, behind `export_open`"
            );
        }
        assert!(
            b.contains("MENU_PANEL, ") && b.matches("MENU_ROW, HOVER_FILL").count() >= 2,
            "T-634: the export menu reuses the T-668 menu recipes — the demotion lands inside the \
             vocabulary, not beside it"
        );
    }

    /// **The history glyphs are no longer too dim to find** — and, since T-637, neither is anything
    /// else that wears the shared icon recipe.
    ///
    /// **THIS PIN WENT RED ON PURPOSE.** T-634 wrote it with the premise `BTN_ICON.contains(
    /// "text-on-surface-variant")` — an assertion that the SHARED recipe was still the muted, 36 px
    /// one — because T-634 could not fix `eden_layout` (another slice owned it that wave) and had to
    /// route around it with a local `TOOL_ICON` copy. The premise was a tripwire: it fires the moment
    /// someone fixes the recipe at its source, which is exactly what T-637 did.
    ///
    /// Resolved by INVERTING it, not by weakening it and not by leaving the recipe broken to keep
    /// the pin green. The property never changed — "a live glyph rests bright, and dimming means
    /// disabled" — only where it is enforced: `BTN_ICON` itself now carries it, so it holds for the
    /// help panel's close button and every dock/toolbelt caller too, not just for these four.
    #[test]
    fn a_live_tool_glyph_rests_bright_and_only_a_dead_one_dims() {
        assert!(
            BTN_ICON.contains("text-on-surface") && !BTN_ICON.contains("text-on-surface-variant"),
            "T-637: the SHARED icon recipe rests at full strength — being hard to find was the \
             defect, and T-634's local TOOL_ICON copy only hid it from this one file"
        );
        assert!(
            !BTN_ICON.contains("hover:") && !BTN_ICON.contains("disabled:"),
            "T-637: the recipe carries geometry + rest weight only; its states come from the T-668 \
             vocabulary at the call site, so a disabled glyph dims, refuses the hover fill, and \
             keeps its title (rule 3)"
        );
        let b = body();
        assert_eq!(
            b.matches("cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])").count(),
            4,
            "T-634: all four tool glyphs (History · Undo · Redo · the settings gear) take the same \
             recipe and the same T-668 state pair"
        );
        assert!(
            !b.contains("TOOL_ICON"),
            "T-637: the local copy is gone — a second source of truth for the same geometry is what \
             let the defect survive everywhere except this file"
        );
        // Rule (3) survives the swap: the permanently-disabled History glyph still explains itself.
        assert!(
            b.contains("Version history (soon)"),
            "T-668 rule (3): a disabled glyph keeps its tooltip — it must not go silent"
        );
    }

    /// **The gear is not stranded any more.** It used to render LAST, past both export buttons, at
    /// the far right of the strip, adjacent to nothing it had anything to do with. It now sits
    /// immediately after the weather picker: the scrubber and the picker are two Mission Settings
    /// fields rendered inline, and the gear opens the rest of them. Pinned as an ordering — gear
    /// after the `Select`, and before the actions rather than after them.
    #[test]
    fn the_gear_sits_with_the_environment_it_opens() {
        let b = body();
        let weather = at(&b, "<Select");
        let gear = at(&b, r#"aria-label="Mission settings""#);
        let primary = at(&b, "class=ACTION_PRIMARY");
        assert!(
            weather < gear && gear < primary,
            "T-634: the gear belongs to the environment cluster (after the weather picker, before \
             the actions), not alone at the far right past every button"
        );
    }

    /// **T-668's `…` rule, applied to this ticket's menu.** `…` means "opens a dialog", everywhere.
    /// `Save Version…` and the two `Mission Settings…` rows earn it. `Export Compiled Mission` did
    /// not: `export_compiled_now` composes bytes, starts a browser download and reports through a
    /// toast — no dialog ever appears. A suffix that promises one and delivers a download is the
    /// convention leaking, and a leaking convention teaches the operator to ignore it.
    ///
    /// T-690 gave that row a second effect (it publishes the compile's findings to the already-open
    /// T-655 panel) and the verdict did not move: updating a surface the operator is already looking
    /// at is not opening one. See the `Export Compiled Mission` row's own comment in `MENUS`.
    #[test]
    fn an_ellipsis_is_a_promise_of_a_dialog() {
        for (menu, items) in MENUS {
            for it in items {
                let promises_dialog = it.label.ends_with('…');
                let opens_dialog = matches!(
                    it.action,
                    Some(super::MenuAction::Save) | Some(super::MenuAction::Settings)
                );
                assert_eq!(
                    promises_dialog,
                    opens_dialog,
                    "T-668/T-634: `{}` (menu `{menu}`) — the `…` suffix and \"a dialog follows\" \
                     must agree exactly. Save Version and Mission Settings put a dialog in front of \
                     the operator; every other row acts, downloads or toggles.",
                    it.label
                );
            }
        }
    }
}
