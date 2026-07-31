//! T-165.8 — the world-export pipeline (ports of scripts/map-assets/{decode-topo, decode-edds,
//! build-world-objects, build-roads-from-topo, verify-phase, validate-export-artifacts, …}.mjs).

use anyhow::{Result, bail};

pub mod aux;
pub mod build;
pub mod classify;
pub mod edds;
pub mod gates;
pub mod jsval;
pub mod pak;
pub mod reclassify;
pub mod topo;

/// T-278 — the instance kinds a census bucket exists for, in emitted key order.
///
/// WHY THIS IS ONE CONST AND NOT THREE ARRAYS: it was three. `build.rs` had `kind_order`,
/// `aux.rs` had `ALL_KINDS`, and `xtask/src/schema_gates.rs` has `INSTANCE_KINDS` — all frozen at
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
mod refuse_empty_tests {
    use super::refuse_empty_write;

    #[test]
    fn refuse_empty_write_reds_on_empty() {
        let err = refuse_empty_write("probe", true, "structurally empty").expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(msg.contains("refusing empty write (probe)"), "{msg}");
        assert!(msg.contains("structurally empty"), "{msg}");
    }

    #[test]
    fn refuse_empty_write_ok_when_nonempty() {
        refuse_empty_write("probe", false, "unused").expect("non-empty must pass");
    }
}

#[cfg(test)]
mod instance_kind_tests {
    use std::collections::BTreeSet;

    use super::INSTANCE_KINDS;
    use crate::serve::repo_root;

    /// T-278 — the guard that would have caught T-244 the day it landed.
    ///
    /// T-244 added `vehicle` to the enums schema and to the classify rules, and every census
    /// kind list stayed at eight. Nothing compared them, so the only signal would have been a
    /// panic during an export nobody could run. This compares them.
    #[test]
    fn instance_kinds_match_enums_schema() {
        let p = repo_root().join("packages/tbd-schema/schema/map-object-enums.schema.json");
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&p).expect("enums schema")).unwrap();
        let names = |k: &str| -> BTreeSet<String> {
            doc["$defs"][k]["enum"]
                .as_array()
                .unwrap_or_else(|| panic!("$defs.{k}.enum missing"))
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        };
        let all = names("kind");
        let regions = names("regionKind");
        assert!(!all.is_empty() && !regions.is_empty(), "empty enums");
        let expected: BTreeSet<String> = all.difference(&regions).cloned().collect();
        let actual: BTreeSet<String> = INSTANCE_KINDS.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            actual, expected,
            "INSTANCE_KINDS drifted from map-object-enums.schema.json $defs.kind \
             minus $defs.regionKind — a census bucket is missing or spurious, which panics \
             build-world-objects at `by_kind.get_mut(kind).expect(\"kind bucket\")`"
        );
    }
}
