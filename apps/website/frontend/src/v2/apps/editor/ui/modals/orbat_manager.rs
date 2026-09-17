//! ORBAT manager dialog and faction-template operations.

#![allow(dead_code, unused_variables)]

use std::collections::{HashMap, HashSet};

use leptos::prelude::*;
use website_map_engine::data::scenario::slot_line::format_slot_line;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::entity_selection;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::outliner::outliner;
use crate::v2::apps::editor::ui::outliner::outliner::{
    filter_orbat_squads_by_side_key, flatten_visible, FlatRow, NodeKind, OutlinerNode,
    ORBAT_MANAGER_DIALOG_CLASS, ORBAT_MANAGER_EMPTY, VIRTUAL_SLOT_THRESHOLD,
};
use crate::v2::core::api::dto::{FactionDoc, RegistryItem, UserFaction};
use crate::v2::core::ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::hosted_commands as engine_ops;

/// Near-fullscreen dialog class shared with the outliner.
pub const DIALOG_CLASS: &str = ORBAT_MANAGER_DIALOG_CLASS;

const SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];
const ROW_H: f64 = 32.0;
const CONTAINER_H: f64 = 480.0;
const OVERSCAN: usize = 8;

mod dialog;
mod dialog_lifecycle;
mod faction_templates;
mod slot_inspector;
mod snapshot;
mod stats;
mod tree_panel;
mod tree_rows;

pub use dialog::OrbatManagerDialog;
use dialog::*;
use dialog_lifecycle::*;
use faction_templates::*;
pub use faction_templates::{
    apply_confirm_allows, faction_doc_role_count, merge_faction_doc_from_side,
    registry_vehicle_options, save_from_side_shrink_warning, template_options_for_side,
    SaveFromSideRefusal,
};
use slot_inspector::*;
use snapshot::*;
use stats::*;
use tree_panel::*;
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
