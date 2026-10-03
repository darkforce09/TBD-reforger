//! ORBAT manager dialog and faction-template operations.

use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::collections::HashSet;

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use mission_model::slot_line::format_slot_line;

#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::RegistryItem;
use crate::foundation::transport::dto::{FactionDoc, UserFaction};
#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::bridge::host_state::entity_selection;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::ui::outliner::node_model;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::ui::outliner::node_model::OutlinerNode;
use crate::workspaces::editor::ui::outliner::node_model::ORBAT_MANAGER_DIALOG_CLASS;
#[cfg(target_arch = "wasm32")]
use crate::workspaces::editor::ui::outliner::node_model::{
    filter_orbat_squads_by_side_key, flatten_visible, FlatRow, NodeKind, ORBAT_MANAGER_EMPTY,
    VIRTUAL_SLOT_THRESHOLD,
};
#[cfg(target_arch = "wasm32")]
use mission_editing_commands::hosted_commands as engine_ops;

/// Near-fullscreen dialog class shared with the outliner.
pub const DIALOG_CLASS: &str = ORBAT_MANAGER_DIALOG_CLASS;

#[cfg(target_arch = "wasm32")]
const SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];
#[cfg(target_arch = "wasm32")]
const ROW_H: f64 = 32.0;
#[cfg(target_arch = "wasm32")]
const CONTAINER_H: f64 = 480.0;
#[cfg(target_arch = "wasm32")]
const OVERSCAN: usize = 8;

mod dialog;
mod dialog_lifecycle;
mod faction_templates;
mod slot_inspector;
mod snapshot;
mod stats;
mod tree_panel;
mod tree_rows;

#[cfg(target_arch = "wasm32")]
pub use dialog::OrbatManagerDialog;
#[cfg(target_arch = "wasm32")]
use dialog_lifecycle::*;
#[cfg(target_arch = "wasm32")]
pub use faction_templates::registry_vehicle_options;
pub use faction_templates::{
    apply_confirm_allows, merge_faction_doc_from_side, save_from_side_shrink_warning,
    template_options_for_side,
};
#[cfg(test)]
pub use faction_templates::{faction_doc_role_count, SaveFromSideRefusal};
#[cfg(target_arch = "wasm32")]
use slot_inspector::*;
#[cfg(target_arch = "wasm32")]
use snapshot::*;
#[cfg(target_arch = "wasm32")]
use stats::*;
#[cfg(target_arch = "wasm32")]
use tree_panel::*;
#[cfg(target_arch = "wasm32")]
use tree_rows::*;

#[cfg(test)]
#[path = "tests/orbat_manager/roster_and_virtualization.rs"]
mod orbat_manager_roster_and_virtualization;

#[cfg(test)]
#[path = "tests/orbat_manager/mounted_refile.rs"]
mod orbat_manager_mounted_refile;

#[cfg(test)]
#[path = "tests/orbat_manager/source.rs"]
mod source;
