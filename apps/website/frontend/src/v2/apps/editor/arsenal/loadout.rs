//! The pure loadout core — `SlotLoadoutV2` serialization ([`loadout_to_picks`] /
//! [`picks_to_loadout`]), the export/import gates ([`try_export`] / [`try_import`]), the
//! buffered loadout planning, commit receipts, and refusal vocabulary.
//! The module exports the domain operations consumed by the Arsenal UI.

use std::collections::{HashMap, HashSet};
pub use website_map_engine::data::store::operations::cargo::commit_writes;
pub use website_map_engine::data::store::operations::cargo::BufferedLoadout;
pub use website_map_engine::data::store::operations::cargo::LoadoutWrite;

use crate::v2::apps::editor::arsenal::rules::{self, index_by_name, validate_loadout, CompatFeed};
use crate::v2::core::api::dto::RegistryItem;

/// A loadout row: the pick key (matches `arsenalRules` `LoadoutKey`), its label, the registry kind
/// it sources from, and whether it is a weapon slot (→ `weapons[]`) or wear (→ `wear{}`).
struct Row {
    key: &'static str,
    label: &'static str,
    kind: &'static str,
    /// `Some((slot_index, slot_type))` for weapon rows; `None` for wear rows.
    weapon: Option<(i64, &'static str)>,
}

/// `LOADOUT_ROWS` minus the two compat `edge` rows (optic / magazine) — the kind-sourced set.
/// Order mirrors the React ACE layout.
const ROWS: &[Row] = &[
    Row {
        key: "primary",
        label: "Primary",
        kind: "gear_primary",
        weapon: Some((0, "primary")),
    },
    Row {
        key: "launcher",
        label: "Launcher",
        kind: "gear_launcher",
        weapon: Some((1, "primary")),
    },
    Row {
        key: "handgun",
        label: "Handgun",
        kind: "gear_handgun",
        weapon: Some((2, "secondary")),
    },
    Row {
        key: "throwable",
        label: "Throwable",
        kind: "gear_throwable",
        weapon: Some((3, "grenade")),
    },
    Row {
        key: "headCover",
        label: "Helmet",
        kind: "gear_helmet",
        weapon: None,
    },
    Row {
        key: "jacket",
        label: "Jacket",
        kind: "gear_jacket",
        weapon: None,
    },
    Row {
        key: "pants",
        label: "Pants",
        kind: "gear_pants",
        weapon: None,
    },
    Row {
        key: "boots",
        label: "Boots",
        kind: "gear_boots",
        weapon: None,
    },
    Row {
        key: "vest",
        label: "Vest (chest rig)",
        kind: "gear_vest",
        weapon: None,
    },
    Row {
        key: "armoredVest",
        label: "Armored Vest",
        kind: "gear_armored_vest",
        weapon: None,
    },
    Row {
        key: "backpack",
        label: "Backpack",
        kind: "gear_backpack",
        weapon: None,
    },
    Row {
        key: "handwear",
        label: "Gloves",
        kind: "gear_gloves",
        weapon: None,
    },
];

mod attachments_and_faults;
mod buffered_loadout_operations;
mod loadout_export;
mod loadout_import;
mod slot_loadout_serialization;

use attachments_and_faults::{attachment_errors, ATTACHMENT_SEP};
pub(crate) use attachments_and_faults::{
    attachments_key, attachments_of, pack_attachments, ATTACHMENT_EDGE,
};
pub(super) use attachments_and_faults::{kit_default_items, loadout_faults, slot_asset_id};
pub use buffered_loadout_operations::*;
pub(super) use loadout_export::export_modpack_id;
pub use loadout_export::*;
pub(super) use loadout_import::import_summary;
#[cfg(test)]
use loadout_import::IMPORT_DOC_KEY;
pub use loadout_import::*;
pub use slot_loadout_serialization::*;

#[cfg(test)]
#[path = "tests/loadout/buffer_operations.rs"]
mod buffer_operations_tests;
#[cfg(test)]
#[path = "tests/loadout/import_round_trip.rs"]
mod import_round_trip_tests;
#[cfg(test)]
#[path = "tests/loadout/refusal_messages.rs"]
mod refusal_messages_tests;
#[cfg(test)]
#[path = "tests/loadout/serialization_and_export.rs"]
mod serialization_and_export_tests;
#[cfg(test)]
#[path = "tests/loadout/write_acknowledgment.rs"]
mod write_acknowledgment_tests;
