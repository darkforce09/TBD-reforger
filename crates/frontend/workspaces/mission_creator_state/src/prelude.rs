//! The Mission Creator state items most callers name, for `use mission_creator_state::prelude::*;`.
//!
//! **Role:** re-exports the catalog, loadout, outliner, node-key, review-mode and seam items the
//! editor's upper layers read most.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::arsenal_rules::CompatFeed;
pub use crate::asset_catalog::{CatalogNode, CatalogPalette, CatalogState};
pub use crate::ids::{CatalogNodeId, OutlinerNodeId};
pub use crate::outliner_model::{NodeKind, OutlinerNode};
pub use crate::review_mode::{ReviewedVersion, writes_mission};
pub use crate::seam_registration::{SeamRegistration, install_seam};
pub use crate::transform::{SnapState, WidgetVariant};
