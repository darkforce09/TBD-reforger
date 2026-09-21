//! The world-export pipeline (ports of scripts/map-assets/{decode-topo, decode-edds,
//! build-world-objects, build-roads-from-topo, verify-phase, validate-export-artifacts, …}.mjs).

use anyhow::{Result, bail};

/// The `TBDC` binary twin written beside every chunk `.json.gz`.
pub mod binary_emit;
/// The rkyv twins of the three catalogue JSONs (`objects/prefabs.rkyv`,
/// `objects/type-inventory.rkyv`, `objects/forest-regions.rkyv`), written beside them by
/// `build_world_objects`.
pub mod catalog_emit;
pub mod chunk_partitioner;
pub mod classify;
pub mod enfusion_texture_decoder;
pub mod export_preparation;
/// Chaikin smoothing of the Path B forest rings, between `forest::trace_rings` and the
/// `forest-regions.json.gz` emit.
pub mod forest_smoothing;
pub mod json_number_formatting;
pub mod mathematical_verification;
pub mod reclassify;
/// The `roads/road_network.rkyv` twin written beside `objects/roads.json.gz`.
pub mod roads_emit;
pub mod topo;

/// The instance kinds a census bucket exists for, in emitted key order.
///
/// ONE CONST, NOT ONE PER CALL SITE: every census bucket in this module is minted from this
/// array, so a classified prefab whose kind has no bucket is a hard failure at the census
/// rather than a missing row in the emitted inventory. A second copy lives in
/// `tools_v2/xtask/src/verifications/schemas/checks.rs`, which verifies the emitted document
/// from outside this crate and therefore may not read this one.
///
/// The order is the emitted `type-inventory.json` `byKind` key order (serde_json is built with
/// `preserve_order`): `vehicle` sits after `water` rather than at the end, and `road` stays
/// last, as every committed inventory has it.
///
/// INVARIANT, asserted by `instance_kinds_match_enums_schema`: this set is exactly
/// `map-object-enums.schema.json` `$defs.kind.enum` minus `$defs.regionKind.enum`, so a kind
/// added to the schema and not to this array fails loudly here.
pub const INSTANCE_KINDS: [&str; 9] = [
    "building",
    "tree",
    "vegetation",
    "rock",
    "prop",
    "utility",
    "water",
    "vehicle",
    "road",
];

/// Refuse structurally empty / vacuous overwrites of committed map-assets.
pub(crate) fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        bail!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/module/refuse_empty_tests.rs"]
mod refuse_empty_tests;

#[cfg(test)]
#[path = "tests/module/instance_kind_tests.rs"]
mod instance_kind_tests;

pub mod vegetation_density;

pub mod forest_contours;

pub mod polygon_geometry;

pub mod cli;
