//! Role: the left-button gesture model: threshold, promotion guard, and what a press can become.
//! Position: `editing/tools/selection` in the map engine.
//! Signals & state: a frozen camera snapshot and the app-side selected-id set; never the document.
//! Invariants: a pending gesture promotes only while a button is still held, and every gesture unprojects against the camera FROZEN at press time — a live unproject would feed pan and zoom back into the gesture mid-drag.

use std::cell::RefCell;
use std::rc::Rc;

use crate::camera::ortho::state::OrthoCamera;

/// Motion (CSS px) separating a click from a drag — the React `useSelectTool` `DRAG_THRESHOLD`.
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
pub(super) const GRID_CELL_M: f64 = crate::data::store::MissionDocCore::GRID_CELL_M;
/// Everon bounds, for the frozen-camera target clamp.
pub(super) const TERRAIN_W: f64 = 12_800.0;
pub(super) const TERRAIN_H: f64 = 12_800.0;

/// The app-side selected-slot set. Selection is not document state: a mission is the same mission
/// whatever is highlighted. A shared handle, so a host's long-lived closures never reach into
/// reactive state a route change could dispose.
pub type SelectionHandle = Rc<RefCell<Vec<String>>>;

/// The shared `Option<RenderEngine>` handle a host owns. Named here so every consumer of the
/// gesture model spells it one way instead of redeclaring a twin.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use crate::frame::EngineHandle;

/// The pending LMB gesture: the press point (CSS px, container-local) + a **frozen** ortho camera
/// copied at pointer-down. A sub-threshold release unprojects against `cam` (never the live engine).
#[derive(Clone)]
pub struct PendingLeft {
    pub start_x: f64,
    pub start_y: f64,
    pub cam: OrthoCamera,
}

/// T-159.19 — the in-flight LMB gesture, mirroring the React `useSelectTool` union
/// (`pending-left` → `move` | `marquee`). A `pointerdown` opens `Pending`; the first `pointermove`
/// past [`DRAG_THRESHOLD_PX`] promotes it to `Move` (a pick hit under the press) or `Marquee` (an
/// empty press) **only when [`may_promote_pending`] is true** (T-723 — buttons still held); a
/// `pointerup` commits on button 0. While a place is armed the host must not open this gesture at
/// all, and the armed pointerup must `take()` any stranded value (Pending/Ruler) — see
/// `mission_editor::armed_place`. Every world unproject in the gesture uses the **frozen**
/// `cam` copied at the press (M2/X-05 — the live `RenderEngine::unproject_xy` is deleted; a live
/// one would feedback-loop as pan/zoom mutate mid-gesture). `Move.dx/dy` is the last coalesced
/// world delta (fed to `engine.set_drag` for the GPU preview + `move_entities` on release).
///
/// T-642 — the RULER is the THIRD mode a left gesture can be in, and the ticket's core "how does a
/// new mode enter `LeftGesture`" answer. When the Ruler tool is active (`ruler_tool::EditorTool`),
/// an LMB `pointerdown` opens [`LeftGesture::Ruler`] INSTEAD of [`LeftGesture::Pending`] — a
/// separate arm that the pointermove/up branches match by name, so the ruler NEVER promotes to a
/// pick/marquee/move and never reaches those commits. Critically it also does NOT route through the
/// armed-placement pointerup branch (the T-723 defect zone): that branch is gated on
/// `editor_ops::has_pending()` (a palette place), which a ruler click never sets, so the ruler
/// pointerup falls straight through to its own `LG::Ruler` arm. `Ruler.cam` is the frozen press
/// camera so the rubber-band preview (in the overlay) and the eventual commit unproject alike.
pub enum LeftGesture {
    Pending(PendingLeft),
    Move {
        ids: Vec<String>,
        start_wx: f64,
        start_wy: f64,
        cam: OrthoCamera,
        dx: f64,
        dy: f64,
    },
    Marquee {
        start_x: f64,
        start_y: f64,
        start_wx: f64,
        start_wy: f64,
        cam: OrthoCamera,
    },
    /// T-642 — an in-flight ruler press: the frozen press camera + press pixel. A sub-threshold
    /// release commits ONE ruler vertex (unprojected against `cam`); the tool stays armed for the
    /// next click. Carries no pick/move/marquee payload — a ruler gesture measures, it never edits
    /// the document, so it deliberately shares nothing with the three commit arms above.
    Ruler {
        start_x: f64,
        start_y: f64,
        cam: OrthoCamera,
    },
    /// T-648 XFORM-SHIFT-001 — an in-flight **Shift-rotate**: a Shift+LMB press that landed on a
    /// selected entity. The whole live selection rotates to FACE the cursor (each entity about its
    /// own position); the release px is unprojected against the frozen `cam` to the aim point that
    /// [`crate::editing::hosted_commands::selection_transform::rotate_selection_to_face`]
    /// rotates toward, quantised to the active
    /// rotation ladder rung. It is a SEPARATE arm from [`LeftGesture::Move`] on purpose: a rotate
    /// commits rotation (through the existing `attrs_update_position` / `set_vehicle_position` field
    /// writes), never the atomic `move_entities_and_vehicles` translate — so the `mission_editor`
    /// move-commit pin (which requires exactly one `LG::Move` arm calling that API) is unaffected,
    /// and Shift+drag can never be mistaken for a positional move. Carries no `ids`: the commit reads
    /// the live selection at release, so a selection edited mid-gesture cannot desync a stale copy.
    Rotate {
        start_x: f64,
        start_y: f64,
        cam: OrthoCamera,
    },
}
