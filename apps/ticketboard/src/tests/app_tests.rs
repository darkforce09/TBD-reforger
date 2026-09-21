use super::*;
use crate::testutil::{corpus_of, work};

fn board_state(selected: Option<usize>) -> BoardState {
    let corpus = corpus_of(vec![
        work("T-1", "status = \"idea\"", ""),
        work("T-2", "status = \"idea\"", ""),
    ]);
    let mut b = BoardState::new(
        corpus,
        LockState::Missing {
            message: "no lock".to_owned(),
        },
        MetricsState::NoReceipts,
        estimates::RawEstimates::default(),
        None,
        Carried::default(),
    );
    b.selected = selected;
    b
}

/// Comfortably two-column vs clearly narrow window widths for the gate.
const WIDE: f32 = 1500.0;
const NARROW: f32 = 900.0;

/// T-918.4 Back contract under the T-920.2 beside-layout, pinned on the
/// SAME gate the paint path consumes: opening a document adds the viewer
/// COLUMN beside the detail column (both visible); Back (close) touches
/// only the viewer machine, so ONLY the column collapses and the same
/// selection's detail panel stays.
#[test]
fn back_collapses_the_viewer_column_only() {
    let b = board_state(Some(1));
    let mut viewer = ViewerState::Closed;
    assert_eq!(
        right_pane(viewer.is_open(), b.selected, WIDE),
        RightPane::Detail(1)
    );

    // A plan click: the viewer column opens BESIDE the detail column —
    // the ticket stays visible while its plan is read (decision log #5).
    viewer.open("docs/plans/t-920_2_plan.md");
    assert_eq!(
        right_pane(viewer.is_open(), b.selected, WIDE),
        RightPane::Both(1),
        "detail and viewer are both visible while a document is open"
    );

    // Back — the CloseViewer action calls exactly this and nothing else
    // on the board: selection intact, viewer column gone, detail stays.
    viewer.close();
    assert_eq!(b.selected, Some(1), "close touches no board state");
    assert_eq!(
        right_pane(viewer.is_open(), b.selected, WIDE),
        RightPane::Detail(1)
    );
}

/// The gate is total, and the narrow-window degrade never clips two
/// columns: with the selection gone (a reload may drop it under an open
/// document) the viewer stands alone at any width; below
/// [`TWO_COLUMN_MIN_WINDOW_W`] the viewer takes the region alone; neither
/// open ⇒ no right pane.
#[test]
fn right_pane_gate_is_total_and_degrades_honestly() {
    for width in [WIDE, NARROW] {
        assert_eq!(right_pane(false, None, width), RightPane::None);
        assert_eq!(right_pane(true, None, width), RightPane::Viewer);
        assert_eq!(right_pane(false, Some(0), width), RightPane::Detail(0));
    }
    // The degrade boundary: side-by-side AT the minimum, viewer-alone
    // below it (replace, the pre-T-920.2 behavior — never two clipped
    // columns).
    assert_eq!(right_pane(true, Some(0), WIDE), RightPane::Both(0));
    assert_eq!(
        right_pane(true, Some(0), TWO_COLUMN_MIN_WINDOW_W),
        RightPane::Both(0)
    );
    assert_eq!(
        right_pane(true, Some(0), TWO_COLUMN_MIN_WINDOW_W - 1.0),
        RightPane::Viewer,
        "narrow windows degrade to the viewer alone"
    );
    // The smallest legal window (720 min inner size) always degrades.
    assert_eq!(right_pane(true, Some(0), 720.0), RightPane::Viewer);
}

/// The persisted viewer width is revalidated on load (T-920.2): parseable
/// finite values clamp to the drag bounds, everything else falls back to
/// the default — a corrupt preference can never yield an invisible or
/// board-swallowing column — and the `save` form round-trips.
#[test]
fn viewer_width_persistence_model() {
    assert_eq!(parse_viewer_width(None), VIEWER_W);
    assert_eq!(parse_viewer_width(Some("700".into())), 700.0);
    assert_eq!(parse_viewer_width(Some(" 640.5 ".into())), 640.5);
    for junk in ["", "wide", "NaN", "inf", "-inf"] {
        assert_eq!(parse_viewer_width(Some(junk.into())), VIEWER_W, "{junk}");
    }
    assert_eq!(parse_viewer_width(Some("10".into())), VIEWER_W_MIN);
    assert_eq!(parse_viewer_width(Some("-500".into())), VIEWER_W_MIN);
    assert_eq!(parse_viewer_width(Some("99999".into())), VIEWER_W_MAX);
    // The exact string `save` writes comes back as the same width.
    assert_eq!(parse_viewer_width(Some(612.5_f32.to_string())), 612.5);
}
