//! Role: the left-button gesture model: threshold, promotion guard, and what a press can become.
//! Position: `selection` in `map_editing_tools`.
//! Signals & state: a frozen camera snapshot and the app-side selected-id set; never the document.
//! Invariants: a pending gesture promotes only while a button is still held, and every gesture
//! unprojects against the camera FROZEN at press time — a live unproject would feed pan and zoom
//! back into the gesture mid-drag.

use std::cell::RefCell;
use std::rc::Rc;

use camera_math::ortho::state::OrthoCamera;

/// Motion (CSS px) separating a click from a drag.
pub const DRAG_THRESHOLD_PX: f64 = 4.0;

/// A pending gesture must not promote into Move or Marquee unless at least one button is still
/// held. A button-less promote is the phantom-drag class: a pending gesture stranded by an
/// armed-place release survives disarm, the next bare move past [`DRAG_THRESHOLD_PX`] turns it into
/// Move, and the next release of any button commits a teleport. A host calls this before it
/// captures or promotes.
#[inline]
pub fn may_promote_pending(buttons: u16) -> bool {
    buttons != 0
}
/// The point index grid cell, world metres. One source of truth: the document's own constant.
pub(super) const GRID_CELL_M: f64 = mission_document::MissionDocCore::GRID_CELL_M;
/// Everon bounds, for the frozen-camera target clamp: width, metres.
pub(super) const TERRAIN_W: f64 = 12_800.0;
/// Everon bounds: height, metres.
pub(super) const TERRAIN_H: f64 = 12_800.0;

/// The app-side selected-slot set. Selection is not document state: a mission is the same mission
/// whatever is highlighted. A shared handle, so a host's long-lived closures never reach into
/// reactive state a route change could dispose.
pub type SelectionHandle = Rc<RefCell<Vec<String>>>;

/// The pending left-button gesture: the press point (CSS px, container-local) and the orthographic
/// camera frozen at pointer-down. A sub-threshold release unprojects against `cam`, never against
/// the live view.
#[derive(Clone)]
pub struct PendingLeft {
    /// Press point, container-local CSS px, x.
    pub start_x: f64,
    /// Press point, container-local CSS px, y.
    pub start_y: f64,
    /// The camera frozen at the press.
    pub cam: OrthoCamera,
}

/// The in-flight left-button gesture.
///
/// A `pointerdown` opens [`LeftGesture::Pending`]; the first `pointermove` past
/// [`DRAG_THRESHOLD_PX`] promotes it to [`LeftGesture::Move`] (a pick hit under the press) or
/// [`LeftGesture::Marquee`] (an empty press), and only while [`may_promote_pending`] holds; a
/// `pointerup` of button 0 commits. While a placement is armed the host opens no gesture, and the
/// armed release takes any stranded value. Every world unproject in a gesture uses the camera
/// frozen at the press: a live unproject would feed pan and zoom back into the gesture mid-drag.
///
/// The ruler and the line-of-sight tool open [`LeftGesture::Ruler`] instead of `Pending`: a
/// separate arm the move and release handlers match by name, so a measuring press never promotes
/// to a pick, a marquee or a move, and never reaches the armed-placement release, which is gated
/// on a palette placement a measuring click never sets. A Shift press on a selected entity opens
/// [`LeftGesture::Rotate`].
pub enum LeftGesture {
    /// A press that has not yet crossed the drag threshold.
    Pending(PendingLeft),
    /// A drag that moves the pressed entity, or the whole selection when it was selected.
    Move {
        /// The ids the drag moves.
        ids: Vec<String>,
        /// The press point unprojected to the world, x.
        start_wx: f64,
        /// The press point unprojected to the world, y.
        start_wy: f64,
        /// The camera frozen at the press.
        cam: OrthoCamera,
        /// The last coalesced world delta, x: the drag preview's offset and the commit's move.
        dx: f64,
        /// The last coalesced world delta, y.
        dy: f64,
    },
    /// A rectangle selection drag from an empty press.
    Marquee {
        /// Press point, container-local CSS px, x.
        start_x: f64,
        /// Press point, container-local CSS px, y.
        start_y: f64,
        /// The press point unprojected to the world, x.
        start_wx: f64,
        /// The press point unprojected to the world, y.
        start_wy: f64,
        /// The camera frozen at the press.
        cam: OrthoCamera,
    },
    /// An in-flight measuring press of the ruler or the line-of-sight tool. A sub-threshold release
    /// commits one point, unprojected against `cam`, and the tool stays armed for the next click.
    /// It carries no pick, move or marquee payload: a measurement never edits the document.
    Ruler {
        /// Press point, container-local CSS px, x.
        start_x: f64,
        /// Press point, container-local CSS px, y.
        start_y: f64,
        /// The camera frozen at the press.
        cam: OrthoCamera,
    },
    /// An in-flight Shift-rotate: a Shift press that landed on a selected entity. The whole live
    /// selection rotates to face the cursor, each entity about its own position; the release pixel
    /// is unprojected against `cam` to the aim point the hosted `rotate_selection_to_face` command
    /// turns toward, quantised to the active rotation step. It commits rotation through the
    /// position field writes, never the atomic move translate, so Shift-drag is never a positional
    /// move. It carries no ids: the commit reads the live selection at release, so a selection
    /// edited mid-gesture cannot desynchronise a stale copy.
    Rotate {
        /// Press point, container-local CSS px, x.
        start_x: f64,
        /// Press point, container-local CSS px, y.
        start_y: f64,
        /// The camera frozen at the press.
        cam: OrthoCamera,
    },
}
