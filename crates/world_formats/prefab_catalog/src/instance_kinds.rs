//! The instance kinds a terrain's object census counts, in emitted order.
//!
//! **Role:** [`INSTANCE_KINDS`], the one list of census buckets: the world export mints one bucket
//! per kind and the type-inventory gate sums exactly these.
//! **Position:** the census half of the prefab catalogue; the world-export pipeline's builders,
//! census and reclassification and the contract schema gates of `schema_tooling` read it.
//! **Signals & state:** none; a constant.
//! **Invariants:** the set is `map-object-enums.schema.json` `$defs.kind.enum` minus
//! `$defs.regionKind.enum`, and the order is the emitted `byKind` key order.

/// The instance kinds a census bucket exists for, in emitted key order.
///
/// One list, not one per call site: every census bucket the world export writes is minted from
/// this array, so a classified prefab whose kind has no bucket is a hard failure at the census
/// rather than a missing row in the emitted inventory, and the type-inventory gate sums the same
/// buckets.
///
/// The order is the emitted `type-inventory.json` `byKind` key order (`serde_json` is built with
/// `preserve_order`): `vehicle` sits after `water` rather than at the end, and `road` stays last,
/// as every committed inventory has it.
///
/// Invariant, asserted by `instance_kinds_match_enums_schema`: this set is exactly
/// `map-object-enums.schema.json` `$defs.kind.enum` minus `$defs.regionKind.enum`, so a kind added
/// to the schema and not to this array fails loudly.
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

#[cfg(test)]
#[path = "tests/instance_kinds_tests.rs"]
mod tests;
