//! The names a gate imports with `use repository_laws::prelude::*;`: each law's entry point and
//! the member reader they share.

pub use crate::exemption_mechanisms::scan_exemption_mechanisms;
pub use crate::file_length::scan_file_lengths;
pub use crate::sibling_test_placement::scan_inline_test_modules;
pub use crate::workspace_laws::WorkspaceLawReport;
pub use crate::workspace_laws::crate_anatomy::check_crate_anatomy;
pub use crate::workspace_laws::crate_tiers::check_crate_tiers;
pub use crate::workspace_laws::frontend_layering::check_frontend_layering;
pub use crate::workspace_laws::tailwind_sources::check_tailwind_sources;
pub use crate::workspace_laws::test_file_reachability::check_test_file_reachability;
pub use crate::workspace_members::{WorkspaceMember, read_workspace_members};
