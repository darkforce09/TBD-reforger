use super::wave_projection::WavesModel;
use crate::wave_plan::services::lock_file::LockState;
pub(crate) struct WavePlanView<'a> {
    pub lock: &'a LockState,
    pub waves: Option<&'a WavesModel>,
    pub filters_active: bool,
    pub matches: &'a [bool],
    pub selected: Option<usize>,
    pub compare: Option<usize>,
    pub wave0_expanded: bool,
}
