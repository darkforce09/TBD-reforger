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
use crate::v2::apps::editor::ui::inspector::env::author_env;
// T-637 — `STRIP_ROWS` / `ROW_MENUS` / `ROW_TOOLS` and the icon recipe are `eden_layout`'s again
// (the T-634 fold-back); this file renders them rather than redefining them.
use crate::v2::apps::editor::shell::layout::{
    BTN_ICON, DISABLED_GLYPH, DIVIDER, HOVER_FILL, MENU_GUTTER, ROW_MENUS, ROW_TOOLS, STRIP_ROWS,
    TOGGLED_PLATE,
};
// T-633 — the scrubber and the weather picker are the shared Aegis primitives now, not raw
// `<input type="range">` / `<select>`.
use crate::v2::core::ui::{cn, MaterialIcon, Select, Slider};

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
const VALIDATION_CHIP: &str = "flex shrink-0 items-center gap-1 rounded px-2 py-0.5 text-on-surface-variant transition-colors";

/// T-634 — one dropdown-row recipe, shared by the menu-bar dropdowns and the demoted-export menu, so
/// the demotion lands INSIDE the T-668 menu vocabulary instead of inventing a second dropdown
/// language beside it. Compose with [`HOVER_FILL`] + [`DISABLED_GLYPH`]; every row that uses it
/// leads with the unconditional [`MENU_GUTTER`] cell.
const MENU_ROW: &str = "flex w-full items-center gap-1.5 px-3 py-1.5 text-left text-label-sm text-on-surface disabled:cursor-default disabled:text-outline";

/// T-939.4 — the right-aligned chord cell on a menu row that has one. `ml-auto` inside `MENU_ROW`'s
/// flex box pushes it to the trailing edge; dimmer and smaller than the label because it is a hint,
/// not the command. Same job as `context_menu`'s own shortcut cell, and deliberately the same look.
const MENU_CHORD: &str = "ml-auto pl-4 text-label-sm text-outline";

/// T-634 — the dropdown surface itself (menu bar and export menu alike).
const MENU_PANEL: &str =
    "glass animate-menu-in absolute top-full z-50 mt-1 rounded-lg py-1 shadow-lg";

// Top Command Strip (T-172 B9) — menu bar · editable title · time scrubber + weather ·
// History (disabled) · Undo/Redo · Save dialog · Export · Settings.

/// One top-strip menu (T-172 B9). React rendered File/Edit/View/Mission/Environment as dead
/// "(soon)" stubs; these open real dropdowns with the commands that exist. No DOM while closed.
///
/// T-939.4 — `Copy`, so [`ARRANGE_ITEMS`] can be BUILT from [`ARRANGE`] in a `const fn` instead of
/// being a second hand-written copy of the same twenty rows.
#[derive(Clone, Copy)]
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
    // ops moving > 10 entities confirm (`undo_grouped_gestures::confirm_bulk`). The dispatch bodies are
    // wasm-gated in `run_action` (like Undo/Redo); the enum + descriptor compile natively.
    /// Apply a placement pattern (Circular / Line / Grid / Fill Area).
    Pattern(PatternKind),
    /// Align the selection to a box edge / centre axis.
    Align(AlignEdge),
    /// Space the selection equally along an axis.
    Space(SpaceAxis),
    /// Orient the selection (N/E/S/W / face-centre / face-away).
    Orient(Orient),
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

use website_map_engine::editing::tools::placement::{AlignEdge, Orient, PatternKind, SpaceAxis};

/* ═══════════ T-939.4 — the Arrange rows, as ONE list three surfaces read ══════════════════════
 *
 * T-645 built the Placement Tools as a top-strip dropdown and stopped there. T-939.4 adds two more
 * doors to the same nineteen commands — the right-click menu (`ui/docks/context_menu.rs`) and the
 * keyboard (the `mission_editor` chord listener) — and the way three doors stay honest is that
 * there is only ever one list behind them.
 *
 * [`ARRANGE`] is that list: identity, label, chord. Everything else is derived from it —
 * [`ARRANGE_ITEMS`] (what the menu bar renders) is BUILT from it in a `const fn`, the context
 * menu's submenu maps over it, and [`arrange_for_code`] is what the keydown looks a keypress up in.
 * A twentieth tool is one row here and appears in all three places; that is the property, and
 * the `t939_4_one_arrange_list` pins at the bottom of this file are what keep it.
 */

/// One Arrange command's IDENTITY — an id enum, never a label string.
///
/// This is the payload `context_menu`'s `ArrangeRun` row carries and the value the keydown chord
/// resolves to, so both cross the module boundary without either side re-parsing menu copy. The
/// mapping to the internal [`MenuAction`] lives in exactly one place ([`Self::action`]), which is
/// why a click and a chord cannot come to mean different things.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrangeKind {
    PatternCircular,
    PatternLine,
    PatternGrid,
    PatternFillArea,
    AlignLeft,
    AlignRight,
    AlignTop,
    AlignBottom,
    AlignCentreH,
    AlignCentreV,
    SpaceHorizontal,
    SpaceVertical,
    SpaceAlongLine,
    OrientNorth,
    OrientEast,
    OrientSouth,
    OrientWest,
    OrientFaceCentre,
    OrientFaceAway,
}

impl ArrangeKind {
    /// The strip's own dispatch value. Private on purpose: [`MenuAction`] is this module's business
    /// and no other surface should learn it — they carry [`ArrangeKind`] and call [`run_arrange`].
    const fn action(self) -> MenuAction {
        match self {
            Self::PatternCircular => MenuAction::Pattern(PatternKind::Circular),
            Self::PatternLine => MenuAction::Pattern(PatternKind::Line),
            Self::PatternGrid => MenuAction::Pattern(PatternKind::Grid),
            Self::PatternFillArea => MenuAction::Pattern(PatternKind::FillArea),
            Self::AlignLeft => MenuAction::Align(AlignEdge::Left),
            Self::AlignRight => MenuAction::Align(AlignEdge::Right),
            Self::AlignTop => MenuAction::Align(AlignEdge::Top),
            Self::AlignBottom => MenuAction::Align(AlignEdge::Bottom),
            Self::AlignCentreH => MenuAction::Align(AlignEdge::CentreH),
            Self::AlignCentreV => MenuAction::Align(AlignEdge::CentreV),
            Self::SpaceHorizontal => MenuAction::Space(SpaceAxis::Horizontal),
            Self::SpaceVertical => MenuAction::Space(SpaceAxis::Vertical),
            Self::SpaceAlongLine => MenuAction::Space(SpaceAxis::AlongLine),
            Self::OrientNorth => MenuAction::Orient(Orient::North),
            Self::OrientEast => MenuAction::Orient(Orient::East),
            Self::OrientSouth => MenuAction::Orient(Orient::South),
            Self::OrientWest => MenuAction::Orient(Orient::West),
            Self::OrientFaceCentre => MenuAction::Orient(Orient::FaceCentre),
            Self::OrientFaceAway => MenuAction::Orient(Orient::FaceAway),
        }
    }
}

/// One Arrange row: what it is, what it is called, and (for the six that have one) its chord.
pub struct ArrangeEntry {
    /// The id every surface dispatches on.
    pub kind: ArrangeKind,
    /// The row label, shared verbatim by the menu bar and the context submenu — so an operator who
    /// learned a command in one place recognises it in the other.
    pub label: &'static str,
    /// The `KeyboardEvent.code` that runs this row, or `""` when it has no chord. **Only the six
    /// the ticket names are keyed**: aligning to four edges and distributing on two axes are the
    /// acts an author repeats; patterns and orient are deliberate one-off choices made from a menu,
    /// and giving all nineteen a chord would spend most of the keyboard on rows nobody repeats.
    pub code: &'static str,
    /// The chord as an operator reads it (`"Alt + L"`), or `""`. Rendered on both menu surfaces and
    /// documented in the Controls Hint — one spelling, three places.
    pub chord: &'static str,
}

/// How many entities the Arrange chords and the context-menu submenu require.
///
/// **Two, and that is an acceptance line rather than a nicety.** Aligning one object to itself
/// moves nothing and distributing one has no gaps to equalise, so a submenu offered on a single
/// selection is a menu of no-ops — the dead-control shape T-668 exists to remove. The top-strip
/// menu keeps its own looser `>= 1` gate (patterns and orient DO act on one entity); this constant
/// governs only the two surfaces this slice adds.
pub const ARRANGE_MIN_SELECTION: usize = 2;

/// The nineteen Arrange commands, in menu order: patterns, align, space, orient.
pub const ARRANGE: [ArrangeEntry; 19] = [
    ArrangeEntry {
        kind: ArrangeKind::PatternCircular,
        label: "Pattern: Circular",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::PatternLine,
        label: "Pattern: Line",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::PatternGrid,
        label: "Pattern: Grid",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::PatternFillArea,
        label: "Pattern: Fill Area",
        code: "",
        chord: "",
    },
    // The four edge aligns + the two distributes carry the chords. `Alt` + a mnemonic letter:
    // L/R/T/B for the four edges, H/V for the two distribute axes. Every one of the six is free of
    // the rest of the editor's keymap under `Alt` — `KeyL`/`KeyT`/`KeyB`/`KeyH` are bound by
    // nothing at all, and `KeyR` (bare, the right dock) / `KeyV` (Ctrl+V, paste) are claimed only
    // under modifier predicates that `Alt`-without-Ctrl cannot satisfy. `keymap_census`'s
    // `no_two_listeners_claim_the_same_chord` is what proves that rather than this comment.
    ArrangeEntry {
        kind: ArrangeKind::AlignLeft,
        label: "Align Left",
        code: "KeyL",
        chord: "Alt + L",
    },
    ArrangeEntry {
        kind: ArrangeKind::AlignRight,
        label: "Align Right",
        code: "KeyR",
        chord: "Alt + R",
    },
    ArrangeEntry {
        kind: ArrangeKind::AlignTop,
        label: "Align Top",
        code: "KeyT",
        chord: "Alt + T",
    },
    ArrangeEntry {
        kind: ArrangeKind::AlignBottom,
        label: "Align Bottom",
        code: "KeyB",
        chord: "Alt + B",
    },
    ArrangeEntry {
        kind: ArrangeKind::AlignCentreH,
        label: "Align Centres (horizontal)",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::AlignCentreV,
        label: "Align Centres (vertical)",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::SpaceHorizontal,
        label: "Space Equally (horizontal)",
        code: "KeyH",
        chord: "Alt + H",
    },
    ArrangeEntry {
        kind: ArrangeKind::SpaceVertical,
        label: "Space Equally (vertical)",
        code: "KeyV",
        chord: "Alt + V",
    },
    ArrangeEntry {
        kind: ArrangeKind::SpaceAlongLine,
        label: "Space Equally (along line)",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::OrientNorth,
        label: "Orient North",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::OrientEast,
        label: "Orient East",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::OrientSouth,
        label: "Orient South",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::OrientWest,
        label: "Orient West",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::OrientFaceCentre,
        label: "Orient: Face Centre",
        code: "",
        chord: "",
    },
    ArrangeEntry {
        kind: ArrangeKind::OrientFaceAway,
        label: "Orient: Face Away",
        code: "",
        chord: "",
    },
];

/// The Arrange dropdown's rows, DERIVED from [`ARRANGE`] rather than retyped beside it. A `const fn`
/// because [`MENUS`] is a `const` and must be able to name this at compile time.
const ARRANGE_ITEMS: [MenuItem; ARRANGE.len()] = arrange_items();

const fn arrange_items() -> [MenuItem; ARRANGE.len()] {
    let mut out = [MenuItem {
        label: "",
        action: None,
    }; ARRANGE.len()];
    let mut i = 0;
    while i < ARRANGE.len() {
        out[i] = MenuItem {
            label: ARRANGE[i].label,
            action: Some(ARRANGE[i].kind.action()),
        };
        i += 1;
    }
    out
}

/// The chord printed on the menu row `label`, or `""` when that row has none. The menu bar's own
/// read of [`ARRANGE`] — the context menu reads the same field through [`ARRANGE`] directly.
#[must_use]
pub fn arrange_chord_for_label(label: &str) -> &'static str {
    ARRANGE
        .iter()
        .find(|e| e.label == label)
        .map_or("", |e| e.chord)
}

/// The Arrange row a `KeyboardEvent.code` runs, or `None` when the key is not an Arrange chord.
///
/// The empty `code` on the thirteen chord-less rows can never match: a real `KeyboardEvent.code` is
/// never the empty string, and the guard below refuses it outright rather than relying on that.
#[must_use]
pub fn arrange_for_code(code: &str) -> Option<&'static ArrangeEntry> {
    if code.is_empty() {
        return None;
    }
    ARRANGE.iter().find(|e| e.code == code)
}

/// **THE Arrange invoker.** The top-strip row, the context-menu row and the keyboard chord all end
/// here, which is what makes "the chord does the same thing as the menu entry" a fact about the
/// code rather than a claim in a comment.
///
/// Ungated at the door and wasm-gated inside, like the rest of the strip's `editor_ops` calls: the
/// native view shell has no document, so there is nothing for a placement to act on.
pub fn run_arrange(kind: ArrangeKind) {
    run_arrange_action(kind.action());
}

/// The invoker's body, in [`MenuAction`] terms — so [`run_action`]'s four placement arms and
/// [`run_arrange`] are literally the same code path rather than two copies that agree today.
fn run_arrange_action(action: MenuAction) {
    #[cfg(target_arch = "wasm32")]
    {
        use crate::v2::apps::editor::bridge::host_state::undo_grouped_gestures;
        use website_map_engine::editing::hosted_commands::selection_transform;
        // Align is the one that must undo as a single step, so it goes through the host's undo
        // grouping; the other three commit as the engine already batches them. All four are
        // handed the host's bulk confirmation, which the engine calls before it commits.
        match action {
            MenuAction::Pattern(kind) => {
                selection_transform::apply_pattern_to_selection(
                    kind,
                    undo_grouped_gestures::confirm_bulk,
                );
            }
            MenuAction::Align(edge) => {
                undo_grouped_gestures::align_selection(edge);
            }
            MenuAction::Space(axis) => {
                selection_transform::space_selection(axis, undo_grouped_gestures::confirm_bulk);
            }
            MenuAction::Orient(cmd) => {
                selection_transform::orient_selection(cmd, undo_grouped_gestures::confirm_bulk);
            }
            // Not a placement action — `run_action` only routes the four here, and `ArrangeKind`
            // cannot name anything else, so this arm is unreachable in practice.
            _ => {}
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = action;
}

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
    //
    // T-939.4 — the rows are no longer written here. They are DERIVED from `ARRANGE`, the one
    // list the context menu and the keyboard chords read too, so a twentieth tool is a single
    // row above and appears in all three surfaces at once.
    ("Arrange", &ARRANGE_ITEMS),
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
        website_map_engine::editing::host::selection_len()
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

/* ───────────────── T-804 (F-24) — the "draft saved Ns ago" recency label ───────────────── */

/// T-804 — the full chip text for a completed draft flush that happened `elapsed_ms` ago.
///
/// The instant comes from [`crate::v2::apps::editor::shell::persist::last_flush_ms`] (a REAL completed flush — the T-779
/// ack discipline), and this turns "now − then" into the F-24 pre-approved copy: **"Draft saved"**
/// plus a coarse recency. The recency is deliberately whole-unit and monotone-degrading — seconds,
/// then minutes, then hours — because it is refreshed off a 1 s coarse tick (never a per-frame
/// timer), so anything finer than a second would claim a precision the tick cannot honour.
///
/// A negative or sub-`RECENCY_JUST_NOW_MS` gap reads "just now": clock skew (the flush stamp and
/// `Date.now()` are both wall-clock and can cross a resync) must degrade to the harmless reading, not
/// a "-3s ago". Pure and native-testable on purpose — the acceptance's "recency <=6s after an edit"
/// and "counts up over 30s idle" are properties of THIS function, checked without a browser.
#[must_use]
pub fn format_draft_recency(elapsed_ms: f64) -> String {
    format!("Draft saved {}", draft_recency_phrase(elapsed_ms))
}

/// Under this gap the chip says "just now" rather than "0s ago" / "1s ago". Also the floor that
/// absorbs backwards clock skew (a resync between the flush stamp and the render tick).
const RECENCY_JUST_NOW_MS: f64 = 5_000.0;

/// The recency phrase alone (no "Draft saved " prefix) — split out so the pin can assert the ladder
/// without threading the prefix through every case.
fn draft_recency_phrase(elapsed_ms: f64) -> String {
    // The negative/near-zero band — and a NaN from a degenerate clock read — degrade to "just now".
    // Past that floor `elapsed_ms` is finite and >= 5000, so `as u64` is the saturating cast we want
    // (no `f64→int` UB).
    if elapsed_ms.is_nan() || elapsed_ms < RECENCY_JUST_NOW_MS {
        return "just now".to_string();
    }
    let secs = (elapsed_ms / 1_000.0) as u64;
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3_600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3_600)
    }
}

/// T-804 — the chip's hover copy: the two-layer save model in one sentence (F-24 pre-approved
/// wording). `local draft` (this, automatic, in this browser) versus `Save Version` (the durable
/// library entry). Author-facing, so it names neither IndexedDB nor the debounce — it answers the
/// only question the chip raises: "is this the same as saving?"
const DRAFT_CHIP_TOOLTIP: &str = "Your work is auto-saved as a local draft in this browser, so \
     closing the tab will not cost you the session — use Save Version to publish a durable version \
     to the mission library.";

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
/// `<option>` tags in the view, because [`crate::v2::core::ui::Select`] takes its options as data: the wire
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
fn mirror_failure_message(
    field: MirroredField,
    err: &crate::v2::core::api::client::ApiErr,
) -> String {
    let label = field.label;
    if err.0 == 403 {
        return format!(
            "{label} was not saved — you are not this mission's author. It will revert when the editor reloads."
        );
    }
    format!(
        "Could not save {}: {}. It will revert when the editor reloads — try again.",
        label.to_lowercase(),
        crate::v2::core::api::client::api_error_message(err, "the server did not respond")
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
    auth: crate::v2::core::auth::AuthStore,
    mission_id: StoredValue<String>,
    /// Where a failed mirror is reported. Resolved at setup for the same reason `auth` is —
    /// `use_toasts()` is an `expect_context` and would panic from a DOM handler or a timer.
    toasts: crate::v2::core::ui::toast::Toasts,
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
    /// Build the mirror at component setup, resolving the auth store, the toast sink and the
    /// mission id from the route.
    ///
    /// Setup is the only place these can be resolved: each one reaches through the reactive owner,
    /// which a plain DOM event handler or a timer callback does not have.
    pub(crate) fn from_route() -> Self {
        use leptos_router::hooks::use_params_map;
        let id = use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self {
            auth: expect_context::<crate::v2::core::auth::AuthStore>(),
            mission_id: StoredValue::new(id),
            toasts: crate::v2::core::ui::toast::use_toasts(),
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
            let res = crate::v2::core::api::client::api_patch::<serde_json::Value>(
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
                    crate::v2::core::api::client::api_error_message(
                        e,
                        "PATCH /missions/:id failed"
                    )
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

/// The editor's top command strip: the mission identity row above, and the tool and command
/// clusters below it.
///
/// **Signals & state:** binds the undo and redo availability, the version and export controls and
/// the environment row to the page's signals, and routes every button through the page's toolbar
/// dispatch so a click takes the same path as its keyboard chord.
///
/// **Invariants:** the strip's two rows together occupy the fixed strip height the layout module
/// states, so adding a control can never grow the strip and shrink the canvas.
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
    let hint_open = RwSignal::new(crate::v2::apps::editor::ui::modals::help_modal::hint_shown());
    let set_hint = move |v: bool| {
        hint_open.set(v);
        crate::v2::apps::editor::ui::modals::help_modal::set_hint_shown(v);
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
    let transient_closer_id =
        crate::v2::core::ui::modal_stack::register_transient_closer(close_transients);
    // T-192 — row mirror for the inline scrubber / weather select. Setup-time, not handler-time.
    #[cfg(target_arch = "wasm32")]
    let row_mirror = RowMirror::from_route();
    // T-243 — where the server-truth Export reports its outcome. Resolved at setup for the same
    // reason `row_mirror` is: `use_toasts()` is an `expect_context` and a DOM click handler has no
    // reactive owner to resolve it through.
    #[cfg(target_arch = "wasm32")]
    let toasts = crate::v2::core::ui::toast::use_toasts();
    // T-181.44 — per-problem lines from a rejected Save (`details` from the 400). Local to the
    // strip because the Save dialog is the only place they are read; `save_status` stays a
    // one-liner because it also renders in the strip itself.
    let save_findings = RwSignal::new(Vec::<String>::new());
    // T-804 (F-24) — the draft-safety chip's two inputs, both created HERE where the component has a
    // reactive owner (`yrs_persist` runs in detached timers and has none, so it cannot create these):
    //   * `last_flush` — the epoch-ms of the last COMPLETED draft flush, `None` until one lands.
    //     Handed to `yrs_persist::set_last_flush_signal`, which seeds it from the module's ack cell
    //     and thereafter `.set()`s it on every real flush. `None` ⇒ no flush yet ⇒ never-edited ⇒ no
    //     chip; a value ⇒ the chip renders "Draft saved Ns ago". This is not a second dirtiness
    //     source — the flush is downstream of the same edit that arms the dirty dot.
    //   * `recency_tick` — a COARSE 1 s heartbeat the "Ns ago" text reads so it counts up while the
    //     author sits idle. A 1 s interval, not a per-frame RAF (the strip must not grow a per-frame
    //     timer): the recency is whole-seconds, so 1 Hz is exactly enough and no finer.
    let last_flush = RwSignal::new(None::<f64>);
    let recency_tick = RwSignal::new(0u32);
    // `yrs_persist` is wasm32-only (IndexedDB), so the install — and thus the chip's data — exists
    // only in the browser. On the native view shell `last_flush` stays `None`, so the chip's `move ||`
    // renders nothing, which is the correct native behaviour (there is no draft store to report on).
    #[cfg(target_arch = "wasm32")]
    crate::v2::apps::editor::shell::persist::set_last_flush_signal(last_flush);
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        // The coarse heartbeat. Bumps `recency_tick` once a second so the chip's `move ||` re-runs and
        // re-derives "Ns ago" from `Date.now()`. The closure is `.forget()`ed (like the strip's other
        // window handlers — `on_cleanup` is `Send + Sync`-bound and a `Closure` is `!Send`, the
        // `mission_history` T-189 trap), and the interval is CLEARED on route-leave via its handle:
        // `handle` is a plain `i32` (Copy, Send), so the cleanup captures no `!Send` state. After the
        // clear the leaked closure is inert — it never fires against a disposed `recency_tick` again.
        if let Some(win) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                recency_tick.update(|n| *n = n.wrapping_add(1));
            });
            let handle = win
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    1_000,
                )
                .unwrap_or(0);
            cb.forget();
            on_cleanup(move || {
                if let Some(w) = web_sys::window() {
                    w.clear_interval_with_handle(handle);
                }
            });
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if ev.key() == "Escape" {
                // T-726 / T-814 — yield when a registered overlay consumed this Escape. The
                // capture-phase sentinel marks before any bubble listener runs; checking the mark
                // (not live `any_open()`) survives a peer Dialog closing in the same keydown
                // (wave200 F4 / wave139 F3 pile-up).
                if crate::v2::core::ui::modal_stack::escape_consumed() {
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
            crate::v2::core::ui::modal_stack::unregister_transient_closer(transient_closer_id);
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
            crate::v2::apps::editor::bridge::host_state::editor_context::read_env()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::v2::core::api::dto::MissionEnv::default()
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
            let (factions, squads, slot_squad_ids) =
                website_map_engine::editing::hosted_commands::census_input();
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
        let mode =
            crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value("mode")
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
        match crate::v2::apps::editor::ui::inspector::validation_panel::chip_findings() {
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
            crate::v2::apps::editor::shell::document_commands::begin_export_gesture(stamp)
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
                crate::v2::apps::editor::shell::document_commands::export_now(
                    &save_semver.get_untracked(),
                );
            }
            MenuAction::ExportCompiled => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::shell::document_commands::export_compiled_now(toasts);
            }
            MenuAction::Undo => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::bridge::document_host::history::undo();
            }
            MenuAction::Redo => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::bridge::document_host::history::redo();
            }
            MenuAction::Settings => {
                if let Some(s) = settings_open {
                    set_hint(false);
                    s.set(true);
                }
            }
            // T-645 — the Placement Tools act on the live selection, read from the installed
            // `EDITOR_CONTEXT` (like Undo/Redo reach the undo stack). The confirm (> 10 entities)
            // lives inside each placement fn. Wasm-gated bodies; the native
            // build compiles the match arms but does nothing (no doc).
            //
            // T-939.4 — all four now hand off to [`run_arrange_action`], which is also what
            // [`run_arrange`] (the context-menu row and the keyboard chord) calls. Not a tidy-up: it
            // is what makes "the chord performs the same operation as its menu entry" true by
            // construction rather than by two copies happening to agree.
            MenuAction::Pattern(_)
            | MenuAction::Align(_)
            | MenuAction::Space(_)
            | MenuAction::Orient(_) => run_arrange_action(a),
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
                crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                    (d.select_all)()
                });
            }
            MenuAction::SetWidget(digit) => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                    (d.set_widget)(digit)
                });
                #[cfg(not(target_arch = "wasm32"))]
                let _ = digit;
            }
            MenuAction::ToggleSnap => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                    (d.toggle_snap)()
                });
            }
            MenuAction::SnapStep(delta) => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                    (d.snap_step)(delta)
                });
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
        let _gen = crate::v2::apps::editor::mission_editor::toolbar_dispatch_generation().get();
        #[cfg(target_arch = "wasm32")]
        {
            // T-795's `widget_digit()` (1 No Widget / 2 Translate / 3 Rotate) is the tracked
            // three-way read — merged at the wave-205 barrier exactly as both slices planned —
            // so each plate lights on its own digit and keyboard and toolbar cannot disagree.
            let mut active = 0u8;
            crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                active = (d.widget_digit)()
            });
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
        let _gen = crate::v2::apps::editor::mission_editor::toolbar_dispatch_generation().get();
        #[cfg(target_arch = "wasm32")]
        {
            let mut on = false;
            crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                on = (d.snap_enabled)()
            });
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
            // Editable mission title (React setTitle) + the dirty dot (anchored on far left).
            <div class="flex shrink-0 items-center">
                {move || {
                    if let Some(t) = doc_tick {
                        t.track();
                    }
                    #[cfg(target_arch = "wasm32")]
                    let doc_title = {
                        let t = crate::v2::apps::editor::bridge::host_state::editor_context::read_title();
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
                            class="w-36 sm:w-48 max-w-[16rem] truncate rounded border border-transparent bg-transparent px-1.5 py-0 text-label-md font-semibold text-on-surface outline-none transition-colors focus:border-outline-variant/40 focus:bg-surface-container"
                            prop:value=doc_title
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let v = event_target_value(&ev);
                                    if !v.trim().is_empty() {
                                        crate::v2::apps::editor::bridge::host_state::editor_context::set_title(v.trim());
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
            <span class=DIVIDER></span>
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
                                                            // T-939.4 — the row's chord, or "" for
                                                            // the rows that have none. Looked up
                                                            // in `ARRANGE` by label rather than
                                                            // stored on `MenuItem`, so the chord
                                                            // has ONE home and the other 30-odd
                                                            // menu rows need no new field.
                                                            let chord = arrange_chord_for_label(label);
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
                                                                            // T-939.4 — the chord,
                                                                            // right-aligned, read
                                                                            // off the SAME
                                                                            // `ARRANGE` row the key
                                                                            // listener resolves
                                                                            // against. A menu that
                                                                            // teaches its own
                                                                            // keyboard: the only
                                                                            // way an author
                                                                            // discovers `Alt + L`
                                                                            // is by reading it
                                                                            // beside the row it
                                                                            // runs. Empty for the
                                                                            // rows with no chord —
                                                                            // no DOM at all there.
                                                                            {(!chord.is_empty())
                                                                                .then(|| {
                                                                                    view! {
                                                                                        <span class=MENU_CHORD>{chord}</span>
                                                                                    }
                                                                                })}
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
            // T-804 (F-24) — the draft-safety chip. Anchored at the trailing edge of the menu row.
            <div class="flex min-w-0 flex-1 items-center justify-end">
                {move || {
                    recency_tick.track();
                    last_flush
                        .get()
                        .map(|ts| {
                            #[cfg(target_arch = "wasm32")]
                            let elapsed = js_sys::Date::now() - ts;
                            #[cfg(not(target_arch = "wasm32"))]
                            let elapsed = {
                                let _ = ts;
                                0.0
                            };
                            let label = format_draft_recency(elapsed);
                            let aria = label.clone();
                            view! {
                                <span
                                    class="ml-2 shrink-0 whitespace-nowrap text-label-sm text-on-surface-variant"
                                    title=DRAFT_CHIP_TOOLTIP
                                    aria-label=aria
                                    data-draft-chip
                                >
                                    {label}
                                </span>
                            }
                        })
                }}
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
            <div class="hidden">
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
                        crate::v2::apps::editor::bridge::document_host::history::undo();
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
                        crate::v2::apps::editor::bridge::document_host::history::redo();
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
                        crate::v2::apps::editor::ui::inspector::validation_panel::Rollup::of(&validation_findings.get()).total()
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
                        let r = crate::v2::apps::editor::ui::inspector::validation_panel::Rollup::of(&validation_findings.get());
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
                            let r = crate::v2::apps::editor::ui::inspector::validation_panel::Rollup::of(&validation_findings.get());
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
                                    {crate::v2::apps::editor::ui::inspector::validation_panel::findings_dropdown(
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
            <crate::v2::apps::editor::ui::modals::help_modal::ControlsHint open=hint_open />
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
                                crate::v2::apps::editor::bridge::host_state::editor_context::slots_json()
                                    .as_deref()
                                    .and_then(crate::v2::apps::editor::shell::mission_size::estimate_compiled_bytes)
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
                                    crate::v2::apps::editor::shell::mission_size::format_bytes(b),
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
                                            crate::v2::apps::editor::shell::document_commands::save_now(
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
// `env` `Memo` above re-reads on every `doc_tick`, and `editor_context::refresh_docks` bumps `doc_tick`
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
// the engine's `census_input` (which reuses `orbat_manager_snapshot`, not a second doc read).

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
/// Reuses the snapshot's own rows (fed by the engine's `census_input`, which reads them once via
/// `orbat_manager_snapshot`); it never re-parses the document. Each `(slot, squadId)` walks
/// squad → faction → `key`; an id that dangles at any hop (deleted squad, faction with no side key)
/// falls through to `unassigned`. `slot_squad_ids` is one entry per slot — its length IS `total`, so
/// the buckets can never disagree with the slot set.
///
/// Pure + total (no panics, no I/O): the whole reason it lives here and not behind the wasm gate.
#[must_use]
pub fn census_from_rows(
    factions: &[crate::v2::apps::editor::ui::outliner::outliner::FactionRow],
    squads: &[crate::v2::apps::editor::ui::outliner::outliner::SquadRow],
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
///   (`editor_context::read_env_value("mode")`); it is omitted entirely otherwise. Game mode is not a
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
#[path = "tests/top_strip/mission_summary_and_mirror.rs"]
mod mission_summary_and_mirror;

#[cfg(test)]
#[path = "tests/top_strip/menu_state_vocabulary.rs"]
mod menu_state_vocabulary;

#[cfg(test)]
#[path = "tests/top_strip/controls_hint_menu.rs"]
mod controls_hint_menu;

#[cfg(test)]
#[path = "tests/top_strip/form_controls.rs"]
mod form_controls;

#[cfg(test)]
#[path = "tests/top_strip/menu_row_layout.rs"]
mod menu_row_layout;

#[cfg(test)]
#[path = "tests/top_strip/escape_modal_stack.rs"]
mod escape_modal_stack;

#[cfg(test)]
#[path = "tests/top_strip/dialog_transient_exclusivity.rs"]
mod dialog_transient_exclusivity;

#[cfg(test)]
#[path = "tests/top_strip/save_version_dialog.rs"]
mod save_version_dialog;

#[cfg(test)]
#[path = "tests/top_strip/validation_chip.rs"]
mod validation_chip;

#[cfg(test)]
#[path = "tests/top_strip/arrange_actions.rs"]
mod arrange_actions;
