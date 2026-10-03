//! The names a caller imports with `use repository_checks::prelude::*;`: each check's entry point.

pub use crate::architecture::editor_orbat_coherency::verify_editor_orbat_coherency;
pub use crate::architecture::engine_layer_boundaries::verify_engine_layers;
pub use crate::architecture::route_tags::verify_route_tags;
pub use crate::architecture::workspace_laws::{
    WorkspaceLaw, verify_crate_anatomy, verify_crate_tiers, verify_frontend_layering,
    verify_strangler, verify_tailwind_sources, verify_workspace_law, workspace_law_report,
};
pub use crate::language_bans::node_and_file_limits::{verify_file_length, verify_no_node};
pub use crate::language_bans::python_scripts::verify_no_python;
pub use crate::language_bans::shell_scripts::verify_no_shell;
pub use crate::licensing::upstream_code_leaks::verify_crf_leak;
pub use crate::registry::object_registry_aliases::verify_object_registry_aliases;
pub use crate::{Error, Result};
