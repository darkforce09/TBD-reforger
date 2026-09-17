//! The Smart-Arsenal domain core — every decision the loadout editor makes, and no rendering.
//!
//! **Role:** owns the 14 loadout rows (including the compatibility **edge** rows, optic and
//! magazine), the compatibility edge graph behind `items_for`, per-row option building (abstract
//! and variant rows filtered out, a stranded pick preserved so it can still be seen and
//! cleared), loadout validation, the paper-doll region model and the weight readout.
//! **Position:** the pure floor of `v2::apps::editor::arsenal`. The panels and the persisted
//! `SlotLoadoutV2` serialization sit on top of it; it depends on neither, so it is framework-free
//! and tested on the native shell.
//! **Signals & state:** none. Every entry point is a function of the registry rows, the
//! compatibility edges and the current picks.
//! **Invariants:** the blanket `allow(dead_code)` below is earned by exactly three items in the
//! shipping wasm32 build — `PRIMARY_SUB_REGIONS`, `DollRegion::kind` and `DOLL_REGIONS`, the
//! paper-doll region model that currently has no consumer. A native `cargo check` lists more,
//! but those are consumers behind `cfg(target_arch = "wasm32")` rather than real deadness.
//! While the `allow` is on, the compiler cannot report an unwired rule, so remove it the moment
//! those three find a caller or go — and do not delete them to get there.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap, HashSet};
pub use website_map_engine::data::store::operations::cargo_rules::cargo_from_loadout;
pub use website_map_engine::data::store::operations::cargo_rules::cargo_rows_json;

pub use website_map_engine::data::store::operations::cargo_rules::CargoRow;

use crate::v2::core::api::dto::{RegistryCompatEdge, RegistryItem};

/// How a row sources its options.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RowSource {
    /// Flat registry catalog filtered by `kind` — never compat-constrained (clothing mix-and-match
    /// is deliberate). Weapon rows carry the engine slot mapping.
    Kind {
        kind: &'static str,
        weapon: Option<(i64, &'static str)>,
    },
    /// Compat-graph fed: options come from `items_for(picks[depends_on], edge)`; empty until the
    /// dependency is picked.
    Edge {
        edge: &'static str,
        depends_on: &'static str,
    },
}

/// One row of the loadout editor: the pick slot's stable key, the caption beside it, and where
/// its options come from.
pub struct LoadoutRow {
    pub key: &'static str,
    pub label: &'static str,
    pub source: RowSource,
}

/// `LOADOUT_ROWS` — render/compile order. optic + magazine are **edge** rows sitting immediately
/// after `primary` (this ordering is the load-bearing fact from the React ref).
pub const LOADOUT_ROWS: &[LoadoutRow] = &[
    LoadoutRow {
        key: "primary",
        label: "Primary",
        source: RowSource::Kind {
            kind: "gear_primary",
            weapon: Some((0, "primary")),
        },
    },
    LoadoutRow {
        key: "optic",
        label: "Optic",
        source: RowSource::Edge {
            edge: "optic_on_weapon",
            depends_on: "primary",
        },
    },
    LoadoutRow {
        key: "magazine",
        label: "Magazine",
        source: RowSource::Edge {
            edge: "mag_in_weapon",
            depends_on: "primary",
        },
    },
    LoadoutRow {
        key: "launcher",
        label: "Launcher / 2nd rifle",
        source: RowSource::Kind {
            kind: "gear_launcher",
            weapon: Some((1, "primary")),
        },
    },
    LoadoutRow {
        key: "handgun",
        label: "Handgun",
        source: RowSource::Kind {
            kind: "gear_handgun",
            weapon: Some((2, "secondary")),
        },
    },
    LoadoutRow {
        key: "throwable",
        label: "Throwable",
        source: RowSource::Kind {
            kind: "gear_throwable",
            weapon: Some((3, "grenade")),
        },
    },
    LoadoutRow {
        key: "headCover",
        label: "Helmet",
        source: RowSource::Kind {
            kind: "gear_helmet",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "jacket",
        label: "Jacket",
        source: RowSource::Kind {
            kind: "gear_jacket",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "pants",
        label: "Pants",
        source: RowSource::Kind {
            kind: "gear_pants",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "boots",
        label: "Boots",
        source: RowSource::Kind {
            kind: "gear_boots",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "vest",
        label: "Vest (chest rig)",
        source: RowSource::Kind {
            kind: "gear_vest",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "armoredVest",
        label: "Armored vest",
        source: RowSource::Kind {
            kind: "gear_armored_vest",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "backpack",
        label: "Backpack",
        source: RowSource::Kind {
            kind: "gear_backpack",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "handwear",
        label: "Gloves",
        source: RowSource::Kind {
            kind: "gear_gloves",
            weapon: None,
        },
    },
];

/// The 4 weapon keys → their engine `(slotIndex, slotType)`.
pub const WEAPON_SLOTS: &[(&str, i64, &str)] = &[
    ("primary", 0, "primary"),
    ("launcher", 1, "primary"),
    ("handgun", 2, "secondary"),
    ("throwable", 3, "grenade"),
];

/// The two primary sub-slots the doll folds onto the rifle rather than showing as body regions.
pub const PRIMARY_SUB_REGIONS: &[&str] = &["optic", "magazine"];

/// Look up a row by key.
pub fn row(key: &str) -> Option<&'static LoadoutRow> {
    LOADOUT_ROWS.iter().find(|r| r.key == key)
}

mod cargo_capacity_and_delivery;
mod compatibility_and_row_options;
mod export_schema_contract;
mod export_schema_validation;
mod paper_doll_and_weight;

pub use cargo_capacity_and_delivery::*;
pub use compatibility_and_row_options::*;
pub use export_schema_contract::*;
pub use export_schema_validation::*;
pub use paper_doll_and_weight::*;

#[cfg(test)]
use export_schema_contract::{validate_against_schema, SchemaFaults, SUPPORTED_SCHEMA_KEYWORDS};
#[cfg(test)]
use export_schema_validation::{check_schema_node, MAX_PATTERN_INPUT};

#[cfg(test)]
#[path = "tests/rules/cargo_rules.rs"]
mod cargo_rules_tests;
#[cfg(test)]
#[path = "tests/rules/export_schema_rules.rs"]
mod export_schema_rules_tests;
#[cfg(test)]
#[path = "tests/rules/fixtures.rs"]
mod fixtures;
#[cfg(test)]
#[path = "tests/rules/loadout_row_rules.rs"]
mod loadout_row_rules_tests;
