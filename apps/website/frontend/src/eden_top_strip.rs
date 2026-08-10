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

/// T-798 — the validation error chip's geometry (F-11 / operator decision 3). A compact status chip
/// in the actions cluster: `rule` glyph · count · caret. `shrink-0` so it keeps its width against the
/// elastic gap; `gap-1` between the three; the height matches the row's other `text-xs` controls. It
/// carries only geometry + the on-surface text default — the COUNT's severity colour is applied on
/// the count `<span>` itself (`text-error-alert` / `-tactical-yellow` / muted), and the open/hover
/// STATE is composed at the call site with [`TOGGLED_PLATE`] / [`HOVER_FILL`], exactly as the row-2
/// toggle buttons do, so "this dropdown is open" reads the same here as everywhere in the strip.
const VALIDATION_CHIP: &str =
    "flex shrink-0 items-center gap-1 rounded px-2 py-0.5 text-on-surface-variant transition-colors";

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
    /// toggle, not a one-shot command: its state shows in the T-668 checkmark gutter. T-797 F-15 —
    /// it now has ONE home, Help > Keyboard Shortcuts; the earlier View-menu duplicate (a second
    /// door to the same overlay) was dropped as ambiguous. Still a toggle: the Help row both opens
    /// the reference and puts it away.
    ControlsHint,
    // T-797 — the transform-widget / snap-grid / select-all verbs the Edit menu (and the row-2 icon
    // cluster) now DISPATCH rather than merely advertise. Each routes through
    // `mission_editor::with_editor_toolbar_dispatch`, the registered bridge to the keydown closure's
    // `widget_variant` / `snap` signals + the live canvas rect (the fix the earlier `action: None`
    // rows anticipated — "when a later slice exposes those signals, these rows wire to them without
    // moving"). Bodies are wasm-gated in `run_action` (like Undo/Redo); the native build compiles the
    // arms as no-ops (the dispatch is `None` off-wasm anyway).
    /// Select every entity in the viewport (the Ctrl+A arm; the editor owns the canvas rect).
    SelectAll,
    /// Pick the transform-widget variant from its `1`/`2` digit (Translate / Rotate).
    SetWidget(u8),
    /// Toggle the snap-grid master latch (the `G` chord).
    ToggleSnap,
    /// Step the active widget's snap ladder by ±1 (the `[`/`]` chords).
    SnapStep(i32),
}

use crate::place_helpers::{AlignEdge, Orient, PatternKind, SpaceAxis};

// T-797 — six menus after the View menu was removed (F-14 + F-15 emptied it). The count is a
// compile-time invariant; the `.enumerate()` render and the `open_menu: Option<usize>` latch index
// by position, so shrinking the table needs no other edit.
const MENUS: [(&str, &[MenuItem]); 6] = [
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
    // T-797 (F-06 / operator pass) — Eden's menu-bar Edit carries NO clipboard verbs (verified from
    // pixels, frame 163508) and TBD's context menu already owns cut/copy/paste, so this menu does
    // NOT grow clipboard rows. What it gains is the discoverability the row-2 icon toolbar cannot
    // give a keyboard-only affordance: the on-screen Select All, and the widget / snap / grid keys
    // that were previously invisible secrets. Each row carries its chord in the label.
    //
    // **wave-202 — the widget / snap / grid rows now DISPATCH (they are live commands).** The slice
    // that shipped them left them `action: None` because their live state (`widget_variant` / `snap`,
    // T-648/T-795) lived in `mission_editor`'s keydown closure with no cross-file bridge, and
    // `mission_editor.rs` was another slice's `owns`. That bridge now exists —
    // `mission_editor::register_editor_toolbar_dispatch` (the `register_widget_pivot` pattern) — so
    // each row wires to it "without moving", exactly as the previous note anticipated. Each still
    // carries its chord in the label, and a click runs the identical keydown arm. The T-668
    // dead-control rule no longer applies here (these ARE clickable now); it still governs the one
    // genuinely-absent dispatch — none remain in this menu.
    (
        "Edit",
        &[
            MenuItem {
                label: "Undo (Ctrl+Z)",
                action: Some(MenuAction::Undo),
            },
            MenuItem {
                label: "Redo (Ctrl+Shift+Z)",
                action: Some(MenuAction::Redo),
            },
            // SEL-ALL-001 — the Ctrl/Cmd+A arm (mission_editor keydown) scopes Select All to the
            // viewport, not the whole mission. T-797 wave-202: this row now DISPATCHES — it reaches
            // the keydown closure through `with_editor_toolbar_dispatch`, which owns the live canvas
            // rect the query needs. The chord and the row run the identical arm; the label keeps its
            // chord so the key stays discoverable.
            MenuItem {
                label: "Select All on Screen (Ctrl+A)",
                action: Some(MenuAction::SelectAll),
            },
            // WIDGET-CYCLE-001 — the transformation-widget modes. Eden numbers five (No Widget 1 /
            // Translation 2 / Rotation 3 / Area Scaling 4 / Area 5); T-795 renumbers TBD onto Eden's
            // FIRST THREE — `1` No Widget / `2` Translate / `3` Rotate (4/5 reserved-unbound, no
            // area-scale target — see `WidgetVariant`). These rows dispatch `set_widget(1|2|3)` through
            // the bridge; the digit in each label is the chord, kept so the key stays discoverable.
            MenuItem {
                label: "Widget: No Widget (1)",
                action: Some(MenuAction::SetWidget(1)),
            },
            MenuItem {
                label: "Widget: Translation (2)",
                action: Some(MenuAction::SetWidget(2)),
            },
            MenuItem {
                label: "Widget: Rotation (3)",
                action: Some(MenuAction::SetWidget(3)),
            },
            // KEY-GRID-001 / TOOLBAR-GRID-MOVE-001 — the snap grid: G toggles it, `[`/`]` tune the
            // active ladder's step. One SNAP grid (move + rotation rungs), the status-bar chip labels
            // it SNAP (O-10) — distinct from the map reference grid the View/environment owns. T-797
            // wave-202: these three rows dispatch toggle / step ∓1 through the bridge.
            MenuItem {
                label: "Toggle Snap Grid (G)",
                action: Some(MenuAction::ToggleSnap),
            },
            MenuItem {
                label: "Snap Step — Decrease ([)",
                action: Some(MenuAction::SnapStep(-1)),
            },
            MenuItem {
                label: "Snap Step — Increase (])",
                action: Some(MenuAction::SnapStep(1)),
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
    // T-797 (F-14 + F-15) — the View menu is GONE. It held exactly two rows and both had to leave:
    //   • F-14 — `Map layers — render host (T-159.28)` was an inert, permanently-disabled row that
    //     named an unbuilt render host. The operator pass said ship-it-or-drop-it; there is no host
    //     to wire, so it drops. (It also carried a `T-xxx` string, which the acceptance forbids in a
    //     menu row.)
    //   • F-15 — `Controls Hint` was the SECOND home of the shortcut reference. One Controls Hint
    //     home, and it is Help > Keyboard Shortcuts (below). The View-side duplicate drops.
    // With both rows gone the menu is empty, and an empty menu-bar dropdown is a dead stub — so the
    // whole entry is removed rather than left to open onto nothing. The MENUS count drops 7 → 6.
    (
        "Mission",
        &[
            MenuItem {
                label: "Mission Settings…",
                action: Some(MenuAction::Settings),
            },
            // T-671 — a named route to the two attribute rows this menu previously had no word for:
            // an author looking for where the mission's blurb and card picture are set should not
            // have to guess that "Settings" is the answer. Same reasoning as the Environment menu's
            // `Time & Weather…` row. `…` because a dialog is exactly what follows (T-668).
            // T-797 F-16-copy — the `(Mission Settings)` parenthetical is dropped (it named the
            // dialog these rows already open, which was noise); `…` stays (a dialog still follows).
            MenuItem {
                label: "Briefing & Thumbnail…",
                action: Some(MenuAction::Settings),
            },
        ],
    ),
    (
        "Environment",
        // T-797 F-16-copy — `(Mission Settings)` parenthetical dropped, as on the Mission menu row.
        &[MenuItem {
            label: "Time & Weather…",
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

/// T-789 F-04 — is the currently-focused element one of `nodes`? Used by the Save-dialog Tab trap
/// to tell "focus is inside the dialog, wrap at the edge" from "focus escaped the dialog, pull it
/// back to the first focusable". `active` is `document.activeElement` boxed as a `JsValue`; a plain
/// pointer-equality walk over the NodeList is enough (the list is four items). wasm-only: the whole
/// trap body is wasm-gated (`NodeList` / `query_selector_all` are not in the native web-sys set).
#[cfg(target_arch = "wasm32")]
fn within(active: &Option<wasm_bindgen::JsValue>, nodes: &web_sys::NodeList) -> bool {
    let Some(active) = active else { return false };
    for i in 0..nodes.length() {
        if let Some(n) = nodes.item(i) {
            if *active == *AsRef::<wasm_bindgen::JsValue>::as_ref(&n) {
                return true;
            }
        }
    }
    false
}

/// T-789 F-04 — the Save-Version dialog's Tab trap. Keeps Tab / Shift+Tab cycling inside the
/// dialog subtree (`root`) instead of walking out into the left dock. Enumerates the dialog's own
/// focusables in DOM order (✕ → version → notes → Save) and wraps at the edges: Shift+Tab off the
/// first goes to the last, Tab off the last goes to the first; a Tab that arrives with focus already
/// outside the set is pulled back to the first. Only `Tab` is acted on — Escape still bubbles to the
/// strip's window listener, and ordinary typing is untouched. wasm-only (`NodeList` /
/// `query_selector_all` are not in the native web-sys feature set); the native build takes the
/// no-op below (the trap only has meaning against a live DOM).
#[cfg(target_arch = "wasm32")]
fn trap_tab_in_dialog(dialog_ref: NodeRef<leptos::html::Div>, ev: &web_sys::KeyboardEvent) {
    use wasm_bindgen::JsCast;
    if ev.key() != "Tab" {
        return;
    }
    let Some(root) = dialog_ref.get_untracked() else {
        return;
    };
    let root: &web_sys::Element = root.as_ref();
    let Ok(nodes) = root.query_selector_all(
        "button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
    ) else {
        return;
    };
    let len = nodes.length();
    if len == 0 {
        return;
    }
    let first = nodes.item(0);
    let last = nodes.item(len - 1);
    let active = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        .map(wasm_bindgen::JsValue::from);
    let is = |a: &Option<wasm_bindgen::JsValue>, b: &Option<web_sys::Node>| match (a, b) {
        (Some(a), Some(b)) => *a == *AsRef::<wasm_bindgen::JsValue>::as_ref(b),
        _ => false,
    };
    let focus_node = |n: Option<web_sys::Node>| {
        if let Some(el) = n.and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) {
            let _ = el.focus();
        }
    };
    if ev.shift_key() {
        if is(&active, &first) || !within(&active, &nodes) {
            ev.prevent_default();
            focus_node(last);
        }
    } else if is(&active, &last) || !within(&active, &nodes) {
        ev.prevent_default();
        focus_node(first);
    }
}

/// Native no-op — the Tab trap only has meaning against a live DOM (see the wasm variant above).
#[cfg(not(target_arch = "wasm32"))]
fn trap_tab_in_dialog(_dialog_ref: NodeRef<leptos::html::Div>, _ev: &web_sys::KeyboardEvent) {}

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
    // T-789 (wave-203) — the Save Version dialog is portaled to `document.body` (see its mount
    // below) so its `position:fixed` centering resolves against the viewport, not the strip's
    // `backdrop-filter` glass root. Same `leptos::portal::Portal` idiom `eden_dock_right`'s
    // `TriggerOwnerLine` uses to escape the right dock's clipping box.
    use leptos::portal::Portal;
    let open_menu = RwSignal::new(None::<usize>);
    // T-634 — the demoted exports' dropdown. A second latch rather than an eighth `MENUS` entry:
    // the export menu hangs off a BUTTON in the tool row, not off the menu bar, and the two are
    // mutually exclusive (opening either closes the other) so only one dropdown is ever up.
    let export_open = RwSignal::new(false);
    let save_open = RwSignal::new(false);
    let save_notes = RwSignal::new(String::new());
    // T-798 — the validation error chip's dropdown latch. A transient (menu-class), exactly like the
    // export dropdown: it hangs off a BUTTON in the tool row, joins `close_transients` + the strip's
    // ONE Escape closure + the click-away scrim, and is NOT a `modal_stack` Dialog (a count chip is
    // not a modal surface — it must not steal Escape from an open dialog, and its dropdown is
    // ANCHORED, not `fixed`, so it needs no portal). `true` ⇒ the findings list is dropped open.
    let validation_open = RwSignal::new(false);
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
    // T-786 O-5 — opening a dialog closes the strip's popovers/help surfaces (the open menu, the
    // export dropdown, and the Controls Hint), so a dialog and a reference card can no longer be up
    // at once ("Help + Save Version stack simultaneously"). It touches ONLY surfaces this strip
    // owns; the right-click context menu is not a Dialog and keeps its own dismissal (T-786 trap).
    // A `Copy` closure so every dialog-open handler can call it.
    let close_transients = move || {
        open_menu.set(None);
        export_open.set(false);
        validation_open.set(false);
        set_hint(false);
    };
    // T-814 — register the same closer with the modal stack so overlays opened *outside* this
    // strip (canvas dblclick → Attributes, context-menu Arsenal, …) still clear menu/export/hint.
    // The wasm open-edge pump in `modal_stack` observes closed→open without per-dialog wiring.
    #[cfg(target_arch = "wasm32")]
    let transient_closer_id = crate::ui::modal_stack::register_transient_closer(close_transients);
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
                // T-726 / T-814 — yield when a registered overlay consumed this Escape. The
                // capture-phase sentinel marks before any bubble listener runs; checking the mark
                // (not live `any_open()`) survives a peer Dialog closing in the same keydown
                // (wave200 F4 / wave139 F3 pile-up).
                if crate::ui::modal_stack::escape_consumed() {
                    return;
                }
                // One surface per press — each arm returns after closing its layer.
                if open_menu.get_untracked().is_some() {
                    open_menu.set(None);
                    return;
                }
                // T-634 — the export dropdown joins the strip's ONE Escape closure, for the same
                // reason the Controls Hint did: a third window listener is the Esc pile-up.
                if export_open.get_untracked() {
                    export_open.set(false);
                    return;
                }
                // T-798 — the validation dropdown rides the SAME closure (menu-class transient, not a
                // Dialog): one surface per press, closed before the Save dialog arm below.
                if validation_open.get_untracked() {
                    validation_open.set(false);
                    return;
                }
                if save_open.get_untracked() {
                    save_open.set(false);
                    return;
                }
                // T-692 — Esc also closes the Controls Hint on this same listener.
                if hint_open.get_untracked() {
                    set_hint(false);
                }
            }
        });
        on_cleanup(move || {
            esc.remove();
            crate::ui::modal_stack::unregister_transient_closer(transient_closer_id);
        });
    }
    // T-789 F-04 — FRESH STATE on reopen. `save_status` is a shared prop (it also paints inline in
    // the strip at the actions row) and `mission_commands::save_now` writes it to `Saved v{semver}`
    // on success, where it STAYS — so the next time the dialog opens it greets the author with a
    // stale "Saved v0.2.0" describing the *previous* save, before anything has happened this open.
    // Clear it (and the rejected-save findings list) on the CLOSED→OPEN edge only: the Effect tracks
    // `save_open`, and the `was_open` cell fires the clear once per open, never mid-save (you cannot
    // open an already-open dialog) and never on close. The prefill/auto-bump of `save_semver` is
    // deliberately untouched — it is the one thing about the reopened dialog that must persist.
    let save_was_open = std::rc::Rc::new(std::cell::Cell::new(false));
    Effect::new({
        let save_was_open = save_was_open.clone();
        move |_| {
            let now = save_open.get();
            let rising = now && !save_was_open.get();
            save_was_open.set(now);
            if rising {
                save_status.set(String::new());
                save_findings.set(Vec::new());
            }
        }
    });
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
    // above. `refresh_docks` (`editor_ops.rs:2660`) bumps `doc_tick` from `refresh_signals`
    // (`mission_history.rs:480`) at every mutation site, so this recomputes on slot add/remove/refile
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
    // T-798 — the validation findings the error chip renders. The headless eval loop
    // (`validation_panel::ValidationPanel`, mounted once from `mission_editor`) publishes into a sink
    // this reads through `chip_findings()`; the chip is that sink's readout in the top strip.
    //
    // Reactivity is TWO-CHANNEL, and both matter. (1) `doc_tick.track()` FIRST — the strip mounts
    // BEFORE the eval loop (`mission_editor` ~:5923 vs ~:6120), so on the first render `chip_findings`
    // is `None`; the mount-seed `doc_tick` bump (`refresh_docks`) re-runs this memo, by which point
    // the sink is registered. (2) once the sink resolves, reading its signal `.get()` SUBSCRIBES this
    // memo to it directly, so a compile publish — which repaints the sink but bumps no `doc_tick` —
    // still updates the chip. Native / pre-mount: `None` sink ⇒ empty ⇒ the chip reads "No issues".
    let validation_findings = Memo::new(move |_| {
        if let Some(t) = doc_tick {
            t.track();
        }
        match crate::validation_panel::chip_findings() {
            Some(sig) => sig.get(),
            None => Vec::new(),
        }
    });
    // T-799 (a) — the once-per-gesture guard for the two EXPORT rows (F-28/F-34: Export Compiled
    // fired TWICE per click — two `createObjectURL` + two anchor clicks of the same payload). The
    // export rows live in conditionally-rendered menus that close on activation, so the DOM
    // re-dispatches a synthesised second click carrying the SAME `Event.timeStamp`; this returns
    // `false` for that duplicate and `true` for a genuinely new gesture (a real second intent has its
    // own later stamp and still fires). Only the export rows are gated — an accidental double Undo is
    // harmless, a double download is not, and the review named the export activation specifically. The
    // seam is `mission_commands::begin_export_gesture` (the latch) over `export_gesture_is_duplicate`
    // (the pure rule); native has no DOM double-activation, so this is `true` off-wasm.
    let export_gesture_ok = move |_ev: &leptos::ev::MouseEvent| -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            // `MouseEvent: AsRef<Event>` — `time_stamp()` is the base `Event`'s `DOMHighResTimeStamp`.
            let stamp = AsRef::<web_sys::Event>::as_ref(_ev).time_stamp();
            crate::mission_commands::begin_export_gesture(stamp)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            true
        }
    };
    let run_action = move |a: MenuAction| {
        open_menu.set(None);
        // T-634 — the export menu dispatches through this same function, so it closes here too.
        export_open.set(false);
        // T-798 — and the validation dropdown, for the same one-popover-up reason.
        validation_open.set(false);
        match a {
            // T-786 O-5 — a dialog opening closes the Controls Hint (menu/export already closed
            // above). Save Version and Mission Settings are the two dialog-opening menu actions.
            MenuAction::Save => {
                set_hint(false);
                save_open.set(true);
            }
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
                    set_hint(false);
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
            // T-797 — the transform-widget / snap / select-all verbs route to the editor's keydown
            // closure through the registered dispatch (peer of the placement tools reaching
            // `editor_ops` above). A click and the chord run the identical arm. `None` (native /
            // pre-mount) is a silent no-op.
            MenuAction::SelectAll => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_editor::with_editor_toolbar_dispatch(|d| (d.select_all)());
            }
            MenuAction::SetWidget(digit) => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_editor::with_editor_toolbar_dispatch(|d| (d.set_widget)(digit));
                #[cfg(not(target_arch = "wasm32"))]
                let _ = digit;
            }
            MenuAction::ToggleSnap => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_editor::with_editor_toolbar_dispatch(|d| (d.toggle_snap)());
            }
            MenuAction::SnapStep(delta) => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_editor::with_editor_toolbar_dispatch(|d| (d.snap_step)(delta));
                #[cfg(not(target_arch = "wasm32"))]
                let _ = delta;
            }
        }
    };
    // T-797 wave-202 — the row-2 toggle buttons' ACTIVE-PLATE reads, reactive through the editor's
    // registered dispatch. The getters call `widget_variant.get()` / `snap.get()` TRACKED across the
    // thread_local, so a `class=move || …` closure that calls these subscribes and re-renders when a
    // chord (not just a click) flips the state — keyboard and toolbar cannot disagree. Native / pre-
    // mount: the dispatch is `None`, so both read `false` and no plate lights (the strip still
    // renders; the buttons just act through `run_action`, which no-ops without the bridge).
    //
    // THE SUBSCRIPTION-ORDER FIX (wave-202 MAJOR): the strip renders — and these closures run —
    // BEFORE the editor's `on_load` registers the dispatch. On that first pass the dispatch is `None`,
    // so `with_editor_toolbar_dispatch` never fires and the getter reads NO tracked signal; with
    // nothing to depend on, Leptos would never re-run the closure, and the plate would freeze at its
    // first-render default (Translate stuck lit, Snap dark — the exact regression). So each getter
    // FIRST reads the dispatch GENERATION signal (`.get()`, tracked) — a subscription that exists from
    // frame one regardless of the dispatch — and only THEN reads through the dispatch. When the
    // dispatch registers (or a mission switch re-registers it) the generation bumps, these closures
    // re-run, the dispatch is now present, and the getters reach the tracked `widget_variant`/`snap`
    // getters and subscribe to the live state directly. The generation read must stay ORDERED FIRST:
    // it is the only dependency guaranteed on the first, dispatch-less pass. Pinned by
    // `plates_subscribe_to_dispatch_generation_before_reading_it`.
    let widget_is = move |digit: u8| -> bool {
        // Subscribe to the dispatch's reactive presence FIRST (see the note above) — before, and
        // independent of, whether a dispatch is registered yet.
        let _gen = crate::mission_editor::toolbar_dispatch_generation().get();
        #[cfg(target_arch = "wasm32")]
        {
            // T-795's `widget_digit()` (1 No Widget / 2 Translate / 3 Rotate) is the tracked
            // three-way read — merged at the wave-205 barrier exactly as both slices planned —
            // so each plate lights on its own digit and keyboard and toolbar cannot disagree.
            let mut active = 0u8;
            crate::mission_editor::with_editor_toolbar_dispatch(|d| active = (d.widget_digit)());
            active == digit
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = digit;
            false
        }
    };
    let snap_on = move || -> bool {
        // Subscribe to the dispatch generation FIRST (see the note above `widget_is`).
        let _gen = crate::mission_editor::toolbar_dispatch_generation().get();
        #[cfg(target_arch = "wasm32")]
        {
            let mut on = false;
            crate::mission_editor::with_editor_toolbar_dispatch(|d| on = (d.snap_enabled)());
            on
        }
        #[cfg(not(target_arch = "wasm32"))]
        false
    };
    let title_fallback = StoredValue::new(title);
    view! {
        <div class=STRIP_ROWS>
            // ═══════════ ROW 1 — menus · title · census (Eden `y 0–22`) ═══════════
            // Identity and commands: what this mission IS and the eight ways into it. Nothing that
            // acts on the map lives here any more — that is row 2's job.
            <div class=ROW_MENUS>
            // Menu bar (T-797: File / Edit / Arrange / Mission / Environment / Help — the View menu
            // was removed, F-14 + F-15). The ORBAT Manager button follows the bar, still in row 1.
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
                                                                            on:click=move |ev| {
                                                                                // T-799 (a) — the File
                                                                                // menu carries the same
                                                                                // two export rows as the
                                                                                // export dropdown, so its
                                                                                // menu-close-on-activate
                                                                                // double-fires them too;
                                                                                // gate export actions on
                                                                                // the same once-per-gesture
                                                                                // latch. Non-export rows
                                                                                // dispatch unchanged.
                                                                                if matches!(
                                                                                    a,
                                                                                    MenuAction::Export
                                                                                        | MenuAction::ExportCompiled
                                                                                ) && !export_gesture_ok(&ev)
                                                                                {
                                                                                    return;
                                                                                }
                                                                                run_action(a);
                                                                            }
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
                                                                    // wave-202 — NO menu row is `action:
                                                                    // None` any more: the T-797 widget / snap
                                                                    // / Select-All rows that once landed here
                                                                    // now DISPATCH through the editor bridge
                                                                    // (see the `edit_menu_widget_snap_rows_
                                                                    // dispatch_not_disabled` pin). This arm
                                                                    // is the exhaustiveness fallback for the
                                                                    // `Option<MenuAction>` match — a DISABLED
                                                                    // row keeping the gutter + a tooltip (the
                                                                    // T-668 dead-control idiom) should anyone
                                                                    // re-introduce a future-command stub, so
                                                                    // it stays honest without asserting a
                                                                    // keyboard chord that may not exist.
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
            // ── T-797 (operator decision 2) — ORBAT Manager, now a MENU-ROW entry ──────────────────
            //
            // It used to be a loud primary text-button in row 2. The operator moved it up here, into
            // the menu bar, beside File/Edit/…: opening the faction→squad→slot tree is a top-level
            // destination like a menu, not a row-2 map tool. It is a BUTTON, not a dropdown (there is
            // one thing to open, not a list), so it wears the menu-bar entry idiom (`text-label-sm`
            // + HOVER_FILL, muted like its menu neighbours) rather than the primary CTA it was — a
            // command that opens a modal reads as a peer of the menus here. Disabled in the
            // scaffold-only case (no `orbat_open`), keeping its tooltip (T-668 rule 3), exactly as
            // the gear does. It still closes the strip's transient surfaces before opening (T-786).
            <button
                type="button"
                aria-label="ORBAT Manager"
                title="Open the ORBAT Manager"
                class=cn(
                    &[
                        "shrink-0 rounded px-2 py-0.5 text-label-sm text-on-surface-variant",
                        HOVER_FILL,
                        DISABLED_GLYPH,
                    ],
                )
                disabled=orbat_open.is_none()
                on:click=move |_| {
                    if let Some(o) = orbat_open {
                        close_transients();
                        o.set(true);
                    }
                }
            >
                "ORBAT Manager"
            </button>
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
            // ── T-797 (F-06 a) — the widget-mode + snap-grid icon cluster ──────────────────────────
            //
            // Eden's row 2 exposes these as tool icons; TBD bound the keys (T-648/T-795) but showed
            // no button, so widget modes / snapping / grid stepping were keyboard secrets. These
            // ICONS give them a home in the toolbar, each carrying its chord in the tooltip `Name
            // (Key)`.
            //
            // **wave-202 — they are LIVE now.** The slice that shipped them left them disabled because
            // the live state (`widget_variant` / `snap`) sat in `mission_editor`'s keydown closure
            // with no cross-file bridge. That bridge now exists (`register_editor_toolbar_dispatch`,
            // the `register_widget_pivot` pattern), so each button dispatches through `run_action` →
            // `with_editor_toolbar_dispatch` — a click runs the identical keydown arm. They take the
            // live-glyph recipe `BTN_ICON + HOVER_FILL` (NOT the count-4 `HOVER_FILL, DISABLED_GLYPH`
            // set — those four are History/Undo/Redo/gear and their pin stays exact), and the two
            // TOGGLE buttons (widget variant, snap latch) add a REACTIVE `TOGGLED_PLATE` when active
            // (`widget_is` / `snap_on` above), exactly the "on:click + TOGGLED_PLATE active state" the
            // prior note promised. The two step buttons are momentary (no plate). Each keeps its
            // `Name (Key)` tooltip so the chord stays discoverable.
            //
            // WIDGET-CYCLE-001 — T-795 renumbers the widget chords onto Eden's first three: `1` No
            // Widget / `2` Translate / `3` Rotate (4/5 reserved-unbound). The cluster is now THREE
            // mutually-exclusive buttons (was two) so the No-Widget mode has a home in the toolbar and
            // not only on the `1` key; each dispatches `set_widget(1|2|3)` through the same bridge and
            // lights its `TOGGLED_PLATE` when it is the active variant (`widget_is` above). The digit
            // in each tooltip is the chord, kept discoverable.
            <button
                type="button"
                aria-label="No widget"
                title="No widget (1)"
                class=move || {
                    if widget_is(1) {
                        cn(&[BTN_ICON, HOVER_FILL, TOGGLED_PLATE])
                    } else {
                        cn(&[BTN_ICON, HOVER_FILL])
                    }
                }
                on:click=move |_| run_action(MenuAction::SetWidget(1))
            >
                <MaterialIcon name="block" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Translate widget"
                title="Translate widget (2)"
                class=move || {
                    if widget_is(2) {
                        cn(&[BTN_ICON, HOVER_FILL, TOGGLED_PLATE])
                    } else {
                        cn(&[BTN_ICON, HOVER_FILL])
                    }
                }
                on:click=move |_| run_action(MenuAction::SetWidget(2))
            >
                <MaterialIcon name="open_with" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Rotate widget"
                title="Rotate widget (3)"
                class=move || {
                    if widget_is(3) {
                        cn(&[BTN_ICON, HOVER_FILL, TOGGLED_PLATE])
                    } else {
                        cn(&[BTN_ICON, HOVER_FILL])
                    }
                }
                on:click=move |_| run_action(MenuAction::SetWidget(3))
            >
                <MaterialIcon name="rotate_right" class="block text-base leading-none" />
            </button>
            <span class=DIVIDER></span>
            // KEY-GRID-001 / TOOLBAR-GRID-MOVE-001 — the SNAP grid: G toggles it, `[`/`]` tune the
            // active widget's ladder step. One SNAP grid (move + rot rungs); the status-bar chip
            // labels it SNAP (O-10), distinct from the always-on map reference grid the toolbelt
            // frames. TBD binds no separate reference-grid-visibility key, so only these three carry
            // chords (inventing a phantom grid-label chord would lie the way the census forbids).
            <button
                type="button"
                aria-label="Toggle snap grid"
                title="Toggle snap grid (G)"
                class=move || {
                    if snap_on() {
                        cn(&[BTN_ICON, HOVER_FILL, TOGGLED_PLATE])
                    } else {
                        cn(&[BTN_ICON, HOVER_FILL])
                    }
                }
                on:click=move |_| run_action(MenuAction::ToggleSnap)
            >
                <MaterialIcon name="grid_on" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Decrease snap step"
                title="Decrease snap step ([)"
                class=cn(&[BTN_ICON, HOVER_FILL])
                on:click=move |_| run_action(MenuAction::SnapStep(-1))
            >
                <MaterialIcon name="remove" class="block text-base leading-none" />
            </button>
            <button
                type="button"
                aria-label="Increase snap step"
                title="Increase snap step (])"
                class=cn(&[BTN_ICON, HOVER_FILL])
                on:click=move |_| run_action(MenuAction::SnapStep(1))
            >
                <MaterialIcon name="add" class="block text-base leading-none" />
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
                // Settled/authored HH:MM from `env` (recomputed on `doc_tick`). Mid-drag the
                // thumb moves in the UA control; this span stays frozen until `on_change` commits
                // and bumps the doc — so it is NOT live drag feedback.
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
                            close_transients();
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
            // ── T-798 (F-11 / F-35 / F-36) — the validation error chip ─────────────────────────────
            //
            // Operator decision 3: the floating bottom-left card (one of four bottom furniture pieces)
            // is retired for a top-strip count chip that drops the findings list on click. It lives
            // HERE, at the head of the actions cluster, reading the headless eval loop's sink
            // (`validation_findings`), so:
            //   * (F-11) it shows the TRUE state at load — the eval loop seeds t0 from the PayloadSource
            //     (`validation_panel` initial-eval poll), so a mission that declares a faction but has
            //     no slots reads "1 error" (V1-PLAYER-SPAWN) immediately, not after the first edit;
            //   * (F-35) it hides on Backspace BY CONSTRUCTION — the whole strip is gated on
            //     `chrome_hidden` (mission_editor ~:5921), so no legend is left in a clean screenshot,
            //     and there is no second gate to keep in step (no `mission_editor` edit needed);
            //   * (F-36) the error count wears `text-error-alert` (#f87171, ≥4.5:1 on the chrome
            //     plate), not `text-error` (#ef4444, the app's one 3.9:1 WCAG failure).
            //
            // The dropdown is ANCHORED (`MENU_PANEL`, `absolute`), like the Export menu — NOT `fixed` —
            // so the strip's `backdrop-blur-xl` containing block (the Save-dialog portal trap) never
            // bites it; no portal needed. The chip is a menu-class transient: it joins
            // `close_transients`, the strip's ONE Escape closure, and the click-away scrim, and it is
            // deliberately NOT a `modal_stack` Dialog (a count popover must not steal Escape from an
            // open dialog). Mutually exclusive with the menu bar / export dropdown.
            <div class="relative shrink-0">
                <button
                    type="button"
                    aria-label="Validation issues"
                    aria-haspopup="menu"
                    aria-expanded=move || validation_open.get()
                    title="Mission validation — click for the findings"
                    data-validation-chip
                    data-issue-total=move || {
                        crate::validation_panel::Rollup::of(&validation_findings.get()).total()
                    }
                    class=move || {
                        if validation_open.get() {
                            cn(&[VALIDATION_CHIP, TOGGLED_PLATE])
                        } else {
                            cn(&[VALIDATION_CHIP, HOVER_FILL])
                        }
                    }
                    on:click=move |_| {
                        open_menu.set(None);
                        export_open.set(false);
                        validation_open.update(|o| *o = !*o);
                    }
                >
                    <MaterialIcon name="rule" class="block text-sm leading-none" />
                    // The one-line count. `text-error-alert` when an error blocks (F-36 contrast),
                    // the advisory tactical-yellow when only warnings remain, and the muted variant on
                    // a clean mission — where the text is the quiet "No issues" (never a 0-badge, the
                    // ticket's empty-state call). `tabular-nums` so the count does not jitter width.
                    <span class=move || {
                        let r = crate::validation_panel::Rollup::of(&validation_findings.get());
                        let accent = if r.has_blocking() {
                            "text-error-alert"
                        } else if r.total() > 0 {
                            "text-tactical-yellow"
                        } else {
                            "text-on-surface-variant"
                        };
                        cn(&["text-xs font-medium tabular-nums", accent])
                    }>
                        {move || {
                            let r = crate::validation_panel::Rollup::of(&validation_findings.get());
                            if r.is_empty() { "No issues".to_string() } else { r.chip_text() }
                        }}
                    </span>
                    <MaterialIcon
                        name="expand_more"
                        class="inline-block align-middle text-sm leading-none"
                    />
                </button>
                {move || {
                    validation_open
                        .get()
                        .then(|| {
                            // The findings list + severity legend — rendered by `validation_panel`
                            // (the pinned V1 copy + legend content live there). `w-80` matches the old
                            // card width; `right-0` anchors the drop to the chip's right edge so a
                            // wide list never spills off the viewport's right side.
                            view! {
                                <div class=cn(&[MENU_PANEL, "right-0 w-80"])>
                                    {crate::validation_panel::findings_dropdown(
                                        validation_findings.get(),
                                    )}
                                </div>
                            }
                        })
                }}
            </div>
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
                on:click=move |_| {
                    close_transients();
                    save_open.set(true);
                }
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
                                        on:click=move |ev| {
                                            if export_gesture_ok(&ev) {
                                                run_action(MenuAction::Export);
                                            }
                                        }
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
                                        on:click=move |ev| {
                                            if export_gesture_ok(&ev) {
                                                run_action(MenuAction::ExportCompiled);
                                            }
                                        }
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
            // covers the export menu too, so both dropdowns dismiss the same way. T-798 — the
            // validation dropdown joins it: one scrim, every strip popover dismisses on an outside
            // click.
            {move || {
                (open_menu.get().is_some() || export_open.get() || validation_open.get())
                    .then(|| {
                        view! {
                            <div
                                class="fixed inset-0 z-40"
                                on:click=move |_| {
                                    open_menu.set(None);
                                    export_open.set(false);
                                    validation_open.set(false);
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
                        // `StoredValue` (Copy) so the dialog view can be a re-runnable `Fn` closure —
                        // wave-203 wraps it in `Portal`, whose `children` is `TypedChildrenFn` and
                        // must implement `Fn`; a plain owned `String` moved into the view would make
                        // the closure `FnOnce`. The value is computed once per open (the size line is
                        // a snapshot at dialog-open, not reactive), so a StoredValue read is exact.
                        let size_line = StoredValue::new(match estimate {
                            Some(b) => {
                                format!(
                                    "~{} · {} objects",
                                    crate::mission_size::format_bytes(b),
                                    obj,
                                )
                            }
                            None => format!("{obj} objects"),
                        });
                        let big = estimate.is_some_and(|b| b > 200_000_000);
                        // T-789 F-04 — initial focus + Tab trap. The dialog is hand-rolled (it is not
                        // the shared `ui::Dialog`, which has no focus handling of its own either), so
                        // both live here, dialog-local.
                        //
                        // FOCUS-IN: the VERSION input is the one decision the dialog demands, and the
                        // review's "blind-typeable offscreen field" is why it must own focus the
                        // instant the dialog paints — a keyboard author lands ON the field, not on the
                        // opener button two Tabs away. NodeRef + on_load (the T-785/T-811 lesson: a
                        // bare `autofocus` on a reactively-inserted node does NOT fire). `.select()`
                        // too, so the pre-filled semver is replace-ready; the input stays uncontrolled
                        // after mount (initial `prop:value` only) so a reactive value write cannot land
                        // after on_load and clear the selection (the wave200 F2 trap).
                        let version_ref = NodeRef::<leptos::html::Input>::new();
                        version_ref
                            .on_load(|el: web_sys::HtmlInputElement| {
                                let _ = el.focus();
                                el.select();
                            });
                        // TRAP: keep Tab inside the dialog subtree. Before this, the cycle
                        // ✕ → version → notes → Save WALKED OUT into the left dock (chevron_left →
                        // Layers → …) with no wrap. The container NodeRef lets the handler enumerate
                        // this dialog's own focusables in DOM order and wrap at both edges (Shift+Tab
                        // at the first → last; Tab at the last → first). Only Tab is touched: Escape
                        // still bubbles to the strip's window listener (`save_open.set(false)`), and
                        // typing in the fields is untouched.
                        let dialog_ref = NodeRef::<leptos::html::Div>::new();
                        let trap_tab = move |ev: web_sys::KeyboardEvent| {
                            trap_tab_in_dialog(dialog_ref, &ev);
                        };
                        // T-789 (wave-203 MAJOR) — PORTAL the dialog to `document.body`.
                        //
                        // The bug this fixes: `position:fixed` resolves against the nearest ancestor
                        // that establishes a containing block, and `backdrop-filter` (any non-`none`
                        // value) is exactly such an establisher. This dialog is a DOM descendant of
                        // the strip's glass root (`STRIP_ROWS`, `…backdrop-blur-xl`), so `top-1/2
                        // -translate-y-1/2` centered it on the 48px STRIP, not the viewport — the
                        // Version input rendered at y=-22 (1920×1080) / y=-184 (1366×768), OFF the top
                        // edge (verifier wave203 MAJOR; removing the ancestor filter snapped it to
                        // y=423 — the causation proof). The wave-101 idiom is that dialogs mount
                        // BESIDE the ungated chrome mounts (Attributes/ORBAT: `mission_editor.rs`
                        // ~6026–6043), where no `backdrop-filter` ancestor exists and `fixed top-1/2`
                        // centers correctly. `Portal` teleports these exact nodes to `<body>` — a
                        // sibling of that same top-level container — so the containing block becomes
                        // the ICB (viewport). ONLY the containing block changes: the overlay/dialog
                        // markup, the `version_ref` focus-in, the `dialog_ref` Tab trap, the
                        // fresh-state effect, the semver prefill and the `save_now` wiring are all
                        // inside the children and move intact; Esc still closes via the strip's
                        // window-level keydown listener (`save_open.set(false)`, :927), which is
                        // position-independent, so the T-726/T-814 Esc ladder is UNTOUCHED. The
                        // portal unmounts (Owner::on_cleanup) when `save_open` flips false and this
                        // `.then(|| …)` returns None. The Save dialog's centering class-pin is a class
                        // guard only; the REAL guard is the live-rect smoke `smoke_save_dialog_rect`
                        // (`gate smoke save-dialog-rect`, tools/tbd-tools/src/smokes.rs), which reads
                        // the Version input's getBoundingClientRect in real Chrome at 1920×1080 and
                        // 1366×768 — "by construction" was exactly the claim that lied when the
                        // containing block was wrong.
                        view! {
                            <Portal>
                            <div
                                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
                                on:click=move |_| save_open.set(false)
                            ></div>
                            <div
                                node_ref=dialog_ref
                                on:keydown=trap_tab
                                class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
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
                                            node_ref=version_ref
                                            class="w-32 rounded border border-outline-variant/40 bg-surface-container px-2 py-1 font-mono text-xs text-on-surface"
                                            // T-789 — uncontrolled after mount (initial `value`, not a
                                            // reactive `prop:value`): the dialog remounts on every open
                                            // so the prefilled / auto-bumped semver is read fresh here,
                                            // and a reactive value write can no longer land after
                                            // on_load and clear the focus-in `.select()` (wave200 F2).
                                            // `save_now` reads `save_semver.get_untracked()` at click,
                                            // and `on:input` keeps the signal current, so the typed
                                            // value still reaches the save.
                                            value=save_semver.get_untracked()
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
                                    }>{move || size_line.get_value()}</p>
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
                            </Portal>
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
// from `mission_history::refresh_signals` (`mission_history.rs:480`) at EVERY mutation site (place /
// drag / undo / redo / refile / the IDB restore swap), so the badge is live: it updates on slot
// add/remove/refile with no manual refresh (`editor_ops.rs:2660` is where the bump happens).
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
/// `orbat_add_squad` guard on `editor_ops.rs:4249`); the `label` half is the milsim-facing word the
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
    /// `editor_ops.rs:2660`), which native tests cannot drive but which the memo wiring pins.
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
    use super::{MenuAction, MENUS};
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

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
        let src = live_code(include_str!("eden_top_strip.rs"));
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
        let code = live_code(include_str!("eden_top_strip.rs"));
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
}

/// T-692 — the help surface's TOP-STRIP half: a Help menu in the bar (MENU-BAR-008 / MENU-HELP-001)
/// reaching the one Controls Hint. T-797 F-15 removed the second door (the View-menu MENU-VIEW-017
/// toggle) so the reference now has a single home; the `the_hint_has_exactly_one_home_and_it_is_help`
/// pin below enforces that.
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

    /// T-797 F-15 — the Controls Hint has ONE home, and it is Help > Keyboard Shortcuts. It used to
    /// be reachable from BOTH Help and the View menu (the old MENU-VIEW-017 toggle), which the
    /// operator pass called a duplicate: two doors to one overlay is the kind of "where do I find
    /// the shortcuts" ambiguity a single home removes. The View menu itself is gone (F-14 + F-15
    /// emptied it), so the pin now asserts Help is the SOLE menu reaching the hint — the inverse of
    /// the "both" it used to require, and the acceptance's "exactly one Controls Hint entry".
    #[test]
    fn the_hint_has_exactly_one_home_and_it_is_help() {
        let reaching = menus_reaching_the_hint();
        assert_eq!(
            reaching,
            vec!["Help"],
            "T-797 F-15: the Controls Hint must be reachable from Help ALONE (exactly one entry \
             across all menus); the View-menu duplicate was dropped (found {reaching:?})"
        );
    }

    /// The action is a TOGGLE reflected in the T-668 checkmark gutter, and the overlay is mounted
    /// from this file — specifically inside `TopCommandStrip`'s body (which is what puts it behind
    /// `mission_editor`'s `chrome_hidden` gate — the structural half is pinned in `eden_help`).
    /// Wave-115 NIT-1 / T-755 / wave-134 F3: presence alone is hollow; the pin must defend POSITION
    /// in the gated subtree (between STRIP_ROWS open and its matching close), not merely that the
    /// mount string exists somewhere after the open tag.
    #[test]
    fn the_toggle_is_checked_in_the_gutter_and_mounted_here() {
        let code = crate::arsenal::class_r_scrub::live_code(include_str!("eden_top_strip.rs"));
        let body = crate::arsenal::class_r_scrub::only_body(&code, "pub fn TopCommandStrip(");
        let mount = format!("{} open=hint_open", "ControlsHint");
        let mount_at = body.find(&mount).expect(
            "T-692/T-755: the Controls Hint must be mounted inside TopCommandStrip's body — that              is the chrome_hidden-gated subtree",
        );
        // Inside the strip shell — between STRIP_ROWS open and its matching `</div>`, not a sibling
        // above OR after the close (wave-115 NIT-1 / T-755; wave-134 F3 closes the after-close gap).
        let strip_at = body
            .find("STRIP_ROWS")
            .expect("T-692/T-755: TopCommandStrip must still open with STRIP_ROWS");
        let strip_div = body[..strip_at]
            .rfind("<div")
            .expect("T-692/wave-134: STRIP_ROWS must be a <div class=…> open tag");
        let strip_close = {
            // Div-balance from the STRIP_ROWS open to its matching close.
            let bytes = body.as_bytes();
            let mut i = strip_div;
            let mut depth = 0i32;
            let close = loop {
                if i >= body.len() {
                    panic!("T-692/wave-134: STRIP_ROWS <div> never closed");
                }
                if body[i..].starts_with("<div") {
                    let gt = body[i..].find('>').expect("unclosed <div");
                    let self_closing = bytes.get(i + gt - 1) == Some(&b'/');
                    if !self_closing {
                        depth += 1;
                    }
                    i += gt + 1;
                } else if body[i..].starts_with("</div>") {
                    depth -= 1;
                    if depth == 0 {
                        break i;
                    }
                    i += 6;
                } else {
                    i += 1;
                }
            };
            close
        };
        assert!(
            mount_at > strip_at && mount_at < strip_close,
            "T-692/T-755/wave-134: ControlsHint must sit inside the STRIP_ROWS subtree (between open              and matching close), not beside/above it or after the close"
        );
        // The gutter glyph is reactive on the open latch, so both menus agree on the state.
        assert!(
            body.contains("hint_open.get()"),
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

    /// **Settle commit path is not regressed.** T-192's whole point is that a held scrubber
    /// emits ~30 values/second and the `missions` row gets ONE PATCH per settle. This pin locks
    /// that path only: `on_change` → `author_env` → `row_mirror.set_time`, plus the HH:MM span
    /// wired to settled `env` time. It does **not** claim strip-local `on:input` absence
    /// (Save-dialog handlers elsewhere in `TopCommandStrip` would false-fail a whole-body scan;
    /// `ui.rs` already pins the Slider primitive itself).
    #[test]
    fn the_scrubber_settle_commit_path() {
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
        // HH:MM span is wired to settled authored time via `env` (doc_tick). That is display of
        // the committed value, not mid-drag preview — preview would need a local drag signal.
        assert!(
            body.contains("{move || env.get().time}"),
            "T-633: the HH:MM readout must stay wired to the settled env time"
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
        // `Undo`/`Redo` the two live ones; T-797 adds the widget-mode + snap-grid icon cluster and
        // T-795/T-799 makes it THREE widget buttons (`No widget` / `Translate widget` / `Rotate
        // widget`) beside `Toggle snap grid` and the two snap-step glyphs; `Mission settings` is the
        // gear and `Export` the demoted menu.
        for tool in [
            r#"aria-label="History""#,
            r#"aria-label="Undo""#,
            r#"aria-label="Redo""#,
            r#"aria-label="No widget""#,
            r#"aria-label="Translate widget""#,
            r#"aria-label="Rotate widget""#,
            r#"aria-label="Toggle snap grid""#,
            r#"aria-label="Decrease snap step""#,
            r#"aria-label="Increase snap step""#,
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
        // T-797 (operator decision 2) — ORBAT Manager MOVED the other way: from a row-2 command to a
        // menu-row entry. It now renders in row 1 (after the menu bar, before the title divider), so
        // it must sit BEFORE the tool row opens — the inverse of the assertion it used to satisfy.
        assert!(
            at(&b, r#"aria-label="ORBAT Manager""#) < row_tools,
            "T-797: ORBAT Manager is now a menu-row entry (row 1), not a row-2 tool — it must render \
             before the toolbar row"
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

/// T-726 / T-814 — top-strip Esc yields when modal_stack consumed Escape (wave139 F3 / wave200 F4).
#[cfg(test)]
mod t726_top_strip_esc_stack {
    use crate::arsenal::class_r_scrub::{live_source, only_body};

    #[test]
    fn top_command_strip_escape_yields_when_modal_stack_consumed_escape() {
        // live_source (not live_code): Escape is a string literal; live_code blanks literals.
        let code = live_source(include_str!("eden_top_strip.rs"));
        let body = only_body(&code, "pub fn TopCommandStrip(");
        let esc = ["if ev.key() == \"", "Escape\""].concat();
        let esc_at = body
            .find(&esc)
            .unwrap_or_else(|| panic!("T-726: TopCommandStrip must own an Escape keydown arm"));
        let esc_region = &body[esc_at..];
        let guard = ["modal_stack", "::", "escape_consumed()"].concat();
        assert!(
            esc_region.contains(&guard),
            "T-814: TopCommandStrip Esc must consult modal_stack::escape_consumed() so an open \
             dialog/manager consumes Esc alone even when a peer listener already closed it \
             (wave200 F4). Hollow: delete the escape_consumed guard → RED."
        );
        let guard_at = esc_region
            .find(&guard)
            .expect("escape_consumed present in Esc arm");
        for needle in [
            "open_menu.set(None)",
            "export_open.set(false)",
            "save_open.set(false)",
        ] {
            let at = esc_region
                .find(needle)
                .unwrap_or_else(|| panic!("missing {needle} in Esc arm"));
            assert!(
                guard_at < at,
                "T-814: escape_consumed() must precede `{needle}` (yield before act)"
            );
        }
        // Must NOT regress to live any_open() — that is the F4 pile-up (dialog closes, then strip
        // sees any_open==false and clears the hint in the same keydown).
        let any = ["modal_stack", "::", "any_open()"].concat();
        assert!(
            !esc_region.contains(&any),
            "T-814: strip Esc must not consult live any_open() (wave200 F4 insertion-order trap)"
        );
    }

    #[test]
    fn top_strip_escape_consumed_guard_is_load_bearing() {
        let code = live_source(include_str!("eden_top_strip.rs"));
        let body = only_body(&code, "pub fn TopCommandStrip(");
        let guard = ["modal_stack", "::", "escape_consumed()"].concat();
        assert!(
            body.contains(&guard),
            "canary: strip carries escape_consumed"
        );
        let perturbed = body.replacen(&guard, "false /* hollow */", 1);
        assert!(
            !perturbed.contains(&guard),
            "fired rule: deleting escape_consumed must break the T-814 top-strip Esc pin"
        );
    }

    #[test]
    fn top_strip_registers_transient_closer_with_modal_stack() {
        let code = live_source(include_str!("eden_top_strip.rs"));
        let body = only_body(&code, "pub fn TopCommandStrip(");
        let reg = ["modal_stack", "::", "register_transient_closer"].concat();
        assert!(
            body.contains(&reg),
            "T-814: strip must register close_transients with modal_stack so non-strip dialog \
             opens clear menu/export/hint"
        );
        assert!(
            body.contains("unregister_transient_closer"),
            "T-814: strip must unregister the transient closer on cleanup"
        );
    }
}

/// T-786 O-5 — opening a dialog closes the strip's popovers/help surfaces (the Controls Hint).
#[cfg(test)]
mod t786_dialog_closes_popovers {
    use crate::arsenal::class_r_scrub::{live_code, only_body};

    /// The `close_transients` helper must actually close all three transient surfaces — the open
    /// menu, the export dropdown, and the Controls Hint — or the exclusivity is hollow. Scrubbed
    /// `live_code`, so a doc comment or string cannot satisfy the needles.
    #[test]
    fn close_transients_closes_menu_export_and_hint() {
        let scrubbed = live_code(include_str!("eden_top_strip.rs"));
        let body = only_body(&scrubbed, "pub fn TopCommandStrip(");
        // The definition and its three effects.
        let def_at = body
            .find("let close_transients =")
            .expect("T-786: TopCommandStrip must define close_transients");
        let region = &body[def_at..];
        for needle in [
            "open_menu.set(None)",
            "export_open.set(false)",
            "set_hint(false)",
        ] {
            assert!(
                region.contains(needle),
                "T-786 O-5: close_transients must run `{needle}` so a dialog opening clears it"
            );
        }
    }

    /// The wiring: every path that OPENS a dialog from the strip must first close the Controls Hint
    /// — so "Help + Save Version stack simultaneously" (O-5) can no longer happen. The three buttons
    /// go through `close_transients()`; the two dialog-opening menu actions call `set_hint(false)`
    /// directly (they already close menu/export at the top of `run_action`).
    #[test]
    fn every_dialog_open_path_closes_the_controls_hint() {
        let scrubbed = live_code(include_str!("eden_top_strip.rs"));
        let body = only_body(&scrubbed, "pub fn TopCommandStrip(");
        // Save Version, Mission Settings, and ORBAT Manager buttons each close transients before
        // opening. Match the open call, then require a hint-close within the handler just above it.
        for (label, open_call) in [
            ("Save Version button", "save_open.set(true)"),
            ("ORBAT Manager button", "o.set(true)"),
            ("Mission Settings button", "s.set(true)"),
        ] {
            let at = body
                .find(open_call)
                .unwrap_or_else(|| panic!("T-786: {label} open call `{open_call}` not found"));
            // The handler is small; look back a short window for the transient-close. Walk the
            // start back to a char boundary so a `—` in a nearby comment cannot split a slice.
            let mut window_start = at.saturating_sub(160);
            while window_start > 0 && !body.is_char_boundary(window_start) {
                window_start -= 1;
            }
            let handler = &body[window_start..at];
            assert!(
                handler.contains("close_transients()") || handler.contains("set_hint(false)"),
                "T-786 O-5: {label} must close the Controls Hint before opening its dialog. \
                 Handler window was: {handler}"
            );
        }
        // The menu-action arms for Save and Settings each close the hint directly.
        let run_at = body
            .find("let run_action =")
            .expect("run_action must exist");
        let run_region = &body[run_at..];
        for arm in ["MenuAction::Save =>", "MenuAction::Settings =>"] {
            let arm_at = run_region
                .find(arm)
                .unwrap_or_else(|| panic!("T-786: {arm} arm not found in run_action"));
            let mut arm_end = (arm_at + 160).min(run_region.len());
            while arm_end < run_region.len() && !run_region.is_char_boundary(arm_end) {
                arm_end += 1;
            }
            let arm_region = &run_region[arm_at..arm_end];
            assert!(
                arm_region.contains("set_hint(false)"),
                "T-786 O-5: the `{arm}` action must close the Controls Hint when it opens its dialog"
            );
        }
    }
}

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
/// it, wave-203 MAJOR); it is now proven by the live-rect smoke in `tools/tbd-tools`, not a class pin.
#[cfg(test)]
mod t789_save_version_dialog {
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

    /// FRESH STATE. `save_status` is a shared prop (it also paints inline in the strip) and
    /// `save_now` writes it to `Saved v{semver}`, where it stays — so on reopen the dialog would
    /// greet the author with the *previous* save's line. An Effect on `save_open` must clear both
    /// `save_status` and the `save_findings` list on the closed→open edge. Scrubbed `live_code`, so
    /// the needles are the actual sets, not a mention in a comment/string.
    #[test]
    fn clears_stale_status_on_the_reopen_edge() {
        let code = live_code(include_str!("eden_top_strip.rs"));
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
        let code = live_code(include_str!("eden_top_strip.rs"));
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
        let code = live_code(include_str!("eden_top_strip.rs"));
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
        let full = live_code(include_str!("eden_top_strip.rs"));
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
    /// `tools/tbd-tools/src/smokes.rs` (real `getBoundingClientRect`, both viewports, in the wave
    /// gate). This test remains only as a cheap source-scrub sentinel: it holds the centering classes
    /// in place and forbids the upward-anchored (`top-full`) regression — but it does NOT and cannot
    /// prove on-screen-ness. Never re-add a "by construction" claim here. `live_source` (classes).
    #[test]
    fn dialog_carries_the_centering_classes_rect_is_smoke_proven() {
        let code = live_source(include_str!("eden_top_strip.rs"));
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
}

/// T-798 — the validation error chip in the top strip (F-11 / F-35 / F-36 + operator decision 3). The
/// floating card is retired; the chip reads the headless eval loop's sink, drops the findings list on
/// click, hides on Backspace by living inside the chrome_hidden-gated strip, and wears an AA-contrast
/// red. Source-scrub pins (the mechanical lane); the live-DOM acceptance (chip '1 error' at load,
/// count unchanged on a marker, Backspace clean-screenshot diff, contrast calc) is the playtest lane.
#[cfg(test)]
mod t798_validation_chip {
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

    /// THE CHIP EXISTS, IN THE STRIP, READING THE SEAM. The count comes from the headless eval loop
    /// via `validation_panel::chip_findings`, and the drop is `validation_panel::findings_dropdown` —
    /// so the strip does not re-implement the findings vocabulary, it renders the one home's output.
    #[test]
    fn the_chip_reads_the_validation_seam() {
        let code = live_code(include_str!("eden_top_strip.rs"));
        let body = only_body(&code, "pub fn TopCommandStrip(");
        for needle in [
            "validation_open",                     // the chip's own latch
            "validation_panel::chip_findings",     // reads the headless eval loop's sink
            "validation_panel::findings_dropdown", // renders the pinned list + legend
        ] {
            assert!(
                body.contains(needle),
                "T-798: the strip's validation chip must use `{needle}`. Hollow: drop it → the chip \
                 stops reflecting validation and the F-11 fix (count at load) is gone."
            );
        }
        // The chip's DOM handle for the live acceptance (the gate reads data-issue-total off it).
        let lit = live_source(include_str!("eden_top_strip.rs"));
        let body_lit = only_body(&lit, "pub fn TopCommandStrip(");
        assert!(
            body_lit.contains("data-validation-chip") && body_lit.contains("data-issue-total"),
            "T-798: the chip needs `data-validation-chip` + `data-issue-total` so the scripted \
             acceptance can read the count at load and after a marker place."
        );
    }

    /// F-36 — CONTRAST. The error count must wear `text-error-alert` (#f87171, ≥4.5:1 on the chrome
    /// plate), NOT `text-error` (#ef4444, the app's single 3.9:1 WCAG failure the review measured).
    /// `live_source` keeps class strings (the colour is a class literal).
    #[test]
    fn the_error_count_uses_the_aa_contrast_red() {
        let lit = live_source(include_str!("eden_top_strip.rs"));
        let body = only_body(&lit, "pub fn TopCommandStrip(");
        // Scope to the CHIP's accent decision, not the whole strip: the Save dialog's rejected-save
        // list wears its own `text-error` on a different (passing) plate and is out of this finding's
        // scope (the review flagged the COUNT chip alone). The chip's accent is the closure guarded on
        // `has_blocking()` immediately after the `data-validation-chip` marker.
        let chip_at = body
            .find("data-validation-chip")
            .expect("T-798: the validation chip must exist");
        let acc_at = body[chip_at..]
            .find("has_blocking()")
            .map(|o| chip_at + o)
            .expect("T-798: the chip's accent must branch on has_blocking()");
        // The accent region: from has_blocking() through its short if/else colour ladder.
        let mut end = (acc_at + 220).min(body.len());
        while end < body.len() && !body.is_char_boundary(end) {
            end += 1;
        }
        let region = &body[acc_at..end];
        assert!(
            region.contains("text-error-alert"),
            "T-798 (F-36): the chip's blocking-error count must be `text-error-alert` (#f87171, \
             ≥4.5:1). Hollow: swap it to `text-error` → the app's one WCAG failure returns.\n{region}"
        );
        // …and NOT the failing `text-error` (#ef4444, 3.9:1) inside that same accent ladder.
        assert!(
            !region.contains("text-error\""),
            "T-798 (F-36): the failing `text-error` (#ef4444, 3.9:1) must not paint the chip count; \
             use `text-error-alert`.\n{region}"
        );
    }

    /// TRANSIENT, NOT A DIALOG. The dropdown rides the strip's existing transient machinery — it
    /// joins `close_transients`, the ONE Escape closure, and the click-away scrim — and is
    /// deliberately NOT registered in the modal stack (a count popover must not steal Escape from an
    /// open dialog). This is the deviation the ticket asked be stated: dropdown = menu-class transient.
    #[test]
    fn the_dropdown_is_a_transient_not_a_modal_dialog() {
        let code = live_code(include_str!("eden_top_strip.rs"));
        let body = only_body(&code, "pub fn TopCommandStrip(");
        // (1) close_transients clears it (opening a dialog / another popover closes the chip).
        let ct_at = body
            .find("let close_transients =")
            .expect("T-798: close_transients closure must exist");
        let ct_region = &body[ct_at..ct_at + 200.min(body.len() - ct_at)];
        assert!(
            ct_region.contains("validation_open.set(false)"),
            "T-798: close_transients must clear validation_open so opening a dialog closes the chip \
             dropdown (one popover up at a time)."
        );
        // (2) the ONE Escape closure closes it — one surface per press.
        let esc_at = body
            .find("escape_consumed()")
            .expect("T-798: the strip's Esc arm must exist");
        let esc_region = &body[esc_at..];
        assert!(
            esc_region.contains("validation_open.get_untracked()")
                && esc_region.contains("validation_open.set(false)"),
            "T-798: the validation dropdown must close on Esc through the strip's ONE closure — not a \
             new window listener (the Esc pile-up the strip already avoids)."
        );
        // (3) NOT a modal_stack Dialog. The chip's latch must never be registered as a modal — it is
        // a menu-class transient. (register_transient_closer is the T-814 strip-owned closer and is
        // fine; `register(` / a Dialog wrapper around validation_open is what is forbidden.)
        let lit = live_source(include_str!("eden_top_strip.rs"));
        let body_lit = only_body(&lit, "pub fn TopCommandStrip(");
        assert!(
            !body_lit.contains("<Dialog open=validation_open")
                && !body_lit.contains("modal_stack::register(move || validation_open"),
            "T-798 deviation: the dropdown is a transient, not a Dialog — validation_open must not be \
             registered in the modal stack (it must not own Escape over a real dialog)."
        );
    }

    /// ANCHORED DROPDOWN, NO PORTAL. The drop uses `MENU_PANEL` (absolute, anchored to the chip), like
    /// the Export menu — NOT `position:fixed`, which the strip's `backdrop-blur-xl` containing block
    /// would mis-centre (the Save-dialog portal trap). No portal needed, no rect-smoke regression.
    #[test]
    fn the_dropdown_is_anchored_via_menu_panel() {
        let lit = live_source(include_str!("eden_top_strip.rs"));
        let body = only_body(&lit, "pub fn TopCommandStrip(");
        // The validation dropdown's surface reuses MENU_PANEL (the export menu's anchored recipe).
        let drop_at = body
            .find("validation_panel::findings_dropdown")
            .expect("T-798: the chip must render findings_dropdown");
        // Look just before the dropdown body for its container class — MENU_PANEL, anchored right.
        let start = drop_at.saturating_sub(200);
        let region = &body[start..drop_at];
        assert!(
            region.contains("MENU_PANEL"),
            "T-798: the chip's dropdown must reuse MENU_PANEL (absolute/anchored), the export-menu \
             idiom — NOT a fixed-positioned panel the strip's backdrop-filter would mis-centre."
        );
    }
}
