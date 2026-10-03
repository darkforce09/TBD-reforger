//! The borrowed view the Waves tab paints.
//!
//! **Role:** `WavePlanView`: the lock outcome, the projected lanes, the filter verdicts, the
//! selection and the wave 0 toggle for one frame.
//! **Position:** lent by the desktop application to its Waves tab.
//! **Signals & state:** none; the view only borrows.
//! **Invariants:** painting cannot change the lock or the lanes.

use super::wave_projection::WavesModel;
use crate::wave_plan::services::lock_file::LockState;
/// What the Waves tab reads each frame, borrowed from the workspace.
pub struct WavePlanView<'a> {
    /// The lock outcome.
    pub lock: &'a LockState,
    /// The projected lanes, when the lock loaded.
    pub waves: Option<&'a WavesModel>,
    /// True when any filter is active.
    pub filters_active: bool,
    /// Per corpus index, whether the filters let the ticket through.
    pub matches: &'a [bool],
    /// The selected ticket's corpus index.
    pub selected: Option<usize>,
    /// The comparison ticket's corpus index.
    pub compare: Option<usize>,
    /// True when the wave 0 id list is shown.
    pub wave0_expanded: bool,
}
