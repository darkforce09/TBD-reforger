//! T-165.8 — the world-export pipeline (ports of scripts/map-assets/{decode-topo, decode-edds,
//! build-world-objects, build-roads-from-topo, verify-phase, validate-export-artifacts, …}.mjs).

use anyhow::{Result, bail};

/// T-935.2 — the `TBDC` binary twin written beside every chunk `.json.gz`.
pub mod binary_emit;
/// T-935.11 — the rkyv twins of the three catalogue JSONs (`objects/prefabs.rkyv`,
/// `objects/type-inventory.rkyv`, `objects/forest-regions.rkyv`), written beside them by
/// `build_world_objects`.
pub mod catalog_emit;
pub mod chunk_partitioner;
pub mod classify;
pub mod enfusion_texture_decoder;
pub mod export_preparation;
/// T-149 — Chaikin smoothing of the Path B forest rings, between `forest::trace_rings` and the
/// `forest-regions.json.gz` emit.
pub mod forest_smoothing;
pub mod json_number_formatting;
pub mod mathematical_verification;
pub mod reclassify;
/// T-935.6 — the `roads/road_network.rkyv` twin written beside `objects/roads.json.gz`.
pub mod roads_emit;
pub mod topo;

/// T-278 — the instance kinds a census bucket exists for, in emitted key order.
///
/// WHY THIS IS ONE CONST AND NOT THREE ARRAYS: it was three. `build.rs` had `kind_order`,
/// `aux.rs` had `ALL_KINDS`, and `tools_v2/xtask/src/verifications/schemas/checks.rs` has `INSTANCE_KINDS` — all frozen at
/// the eight kinds that existed before T-244 added `vehicle` to `map-object-enums.schema.json`.
/// A `vehicle`-classified prefab therefore did not land in a missing bucket, it **panicked the
/// builder** (`by_kind.get_mut(kind).expect("kind bucket")`, build.rs), which is why re-running
/// the export could never have made T-244's rule change live. The two copies inside this module
/// are now this const; the `xtask` copy is outside this slice's files and is reported, not edited.
///
/// The order is the emitted `type-inventory.json` `byKind` key order (serde_json is built with
/// `preserve_order`), so `vehicle` is inserted after `water` rather than appended — `road` stays
/// last, as every committed inventory has it.
///
/// INVARIANT, asserted by `instance_kinds_match_enums_schema`: this set is exactly
/// `map-object-enums.schema.json` `$defs.kind.enum` minus `$defs.regionKind.enum`. That test is
/// what makes the next kind addition fail loudly here instead of going latent the way `vehicle`
/// did for a month.
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

/// T-537 / T-383 — refuse structurally empty / vacuous overwrites of committed map-assets.
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
