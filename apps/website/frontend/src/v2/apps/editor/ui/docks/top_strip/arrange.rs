//! Arrange for the top command strip.

use super::*;

/// "(soon)" stubs; these open real dropdowns with the commands that exist. No DOM while closed.
///
/// being a second hand-written copy of the same twenty rows.
#[derive(Clone, Copy)]
pub(super) struct MenuItem {
    pub(super) label: &'static str,
    /// None = disabled row (rendered, not clickable — parity with genuinely-future features).
    pub(super) action: Option<MenuAction>,
}

#[derive(Clone, Copy)]
/// Commands dispatched from top-strip menus and toolbar actions.
pub(super) enum MenuAction {
    Save,
    /// The editor SUPERSET envelope (`MissionExport`) — re-importable, not loadable by the mod.
    Export,
    /// server. A separate action rather than a replacement for [`MenuAction::Export`]: the two
    /// files answer different questions and both have a caller.
    ExportCompiled,
    Undo,
    Redo,
    Settings,
    /// Apply a placement pattern (Circular / Line / Grid / Fill Area).
    Pattern(PatternKind),
    /// Align the selection to a box edge / centre axis.
    Align(AlignEdge),
    /// Space the selection equally along an axis.
    Space(SpaceAxis),
    /// Orient the selection (N/E/S/W / face-centre / face-away).
    Orient(Orient),
    /// it now has ONE home, Help > Keyboard Shortcuts; the earlier View-menu duplicate (a second
    /// door to the same overlay) was dropped as ambiguous. Still a toggle: the Help row both opens
    /// the reference and puts it away.
    ControlsHint,
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
pub(super) const ARRANGE_ITEMS: [MenuItem; ARRANGE.len()] = arrange_items();

/// Build menu entries from the shared Arrange command list.
pub(super) const fn arrange_items() -> [MenuItem; ARRANGE.len()] {
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
pub(super) fn run_arrange_action(action: MenuAction) {
    #[cfg(target_arch = "wasm32")]
    {
        use crate::v2::apps::editor::bridge::host_state::undo_grouped_gestures;
        use website_map_engine::editing::hosted_commands::selection_transform;
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
            _ => {}
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = action;
}
