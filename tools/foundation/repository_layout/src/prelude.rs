//! The names a caller imports with `use repository_layout::prelude::*;`: the checkout-root finder
//! the locations are joined onto, and the top-level trees the other locations lie under.

pub use crate::agent_artifacts::ARTIFACTS_DIR;
pub use crate::build_output::BUILD_OUTPUT_FOLDER;
pub use crate::contracts::CONTRACTS_DIR;
pub use crate::deployment::DEPLOY_DIR;
pub use crate::map_assets::TERRAIN_ASSETS_DIR;
pub use crate::ticket_registry::TICKETS_DIR;
pub use crate::upstream_references::REFERENCES_DIR;

// The finder's names, so a tool binary finds the checkout root through this crate alone.
pub use repository_root::prelude::{
    ROOT_MARKER, find_repository_root, find_repository_root_from, is_repository_root,
};
