use super::{check_status::CheckModel, git_status::GitChip};
use crate::core::process::BoundedLog;
pub(crate) struct StatusView<'a> {
    pub check: &'a CheckModel,
    pub check_running: bool,
    pub check_log: &'a BoundedLog,
    pub show_output: bool,
    pub watch_error: Option<&'a str>,
    pub degraded_watches: &'a [String],
    pub git_chip: &'a GitChip,
    pub git_expanded: bool,
}
