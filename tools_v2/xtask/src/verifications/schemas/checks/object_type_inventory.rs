use super::*;
use developer_tools::repository_layout::{
    map_scratch_dir, terrain_assets_dir, terrain_dir, terrain_registry_path,
};

/// The lockstep invariant for `INSTANCE_KINDS`, as a list of failure strings (empty = OK).
///
/// Shared by the runtime gate and the unit test so the two can never disagree about what "in
/// lockstep" means. `enums` is a parsed `map-object-enums.schema.json`.
///
/// Two comparisons, because they fail differently:
///   1. against `$defs.kind` minus `$defs.regionKind` — the single source of truth. This is what
///      catches the NEXT kind addition on the day it lands.
///   2. against `developer_tools::world_export_pipeline::INSTANCE_KINDS`, order included — the two copies exist because
///      `xtask` stays dependency-light and `tbd-tools` owns the export pipeline, and a divergence
///      between them is precisely the T-244 defect. Order matters: it is the emitted `byKind` key
///      order, so a reordering here would silently change the artifact on the next rebuild.
///
/// Missing enum `$defs` are a FAILURE, not a skip: a schema that could not be read must not let
/// this report "in lockstep" over a comparison it never made.
pub(super) fn instance_kinds_lockstep_failures(enums: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let names = |k: &str| -> Option<HashSet<String>> {
        enums["$defs"][k]["enum"].as_array().map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
    };
    match (names("kind"), names("regionKind")) {
        (Some(all), Some(regions)) if !all.is_empty() && !regions.is_empty() => {
            let expected: BTreeSet<&String> = all.difference(&regions).collect();
            let actual: BTreeSet<String> =
                INSTANCE_KINDS.iter().map(|s| (*s).to_string()).collect();
            let actual_ref: BTreeSet<&String> = actual.iter().collect();
            if actual_ref != expected {
                let missing: Vec<&str> = expected
                    .difference(&actual_ref)
                    .map(|s| s.as_str())
                    .collect();
                let spurious: Vec<&str> = actual_ref
                    .difference(&expected)
                    .map(|s| s.as_str())
                    .collect();
                out.push(format!(
                    "INSTANCE_KINDS (tools_v2/xtask/src/verifications/schemas/checks.rs) drifted from \
                     map-object-enums.schema.json $defs.kind minus $defs.regionKind — \
                     missing {missing:?}, spurious {spurious:?}. I1 sums only the kinds named \
                     there, so a missing bucket makes the sum come up short by that bucket's \
                     instances and reads as a bad artifact instead of a stale gate (T-244/T-594)"
                ));
            }
        }
        _ => out.push(
            "INSTANCE_KINDS lockstep: map-object-enums.schema.json $defs.kind / $defs.regionKind \
             missing or empty — refusing to report lockstep over a comparison never made"
                .to_string(),
        ),
    }
    // Compared as SLICES, not arrays, and that is not a style choice. `[&str; N] == [&str; M]` for
    // N != M is a hard type error (E0277), so an array-to-array comparison here turns the most
    // likely drift — someone adds or drops a kind in one copy — into a raw "can't compare
    // [&str; 8] with [&str; 9]" instead of the explanation below. Measured while perturbing this
    // very check: the length-changing case never reached the assertion at all. Slices compare
    // across lengths, so every drift shape lands on one legible message.
    if INSTANCE_KINDS[..] != developer_tools::world_export_pipeline::INSTANCE_KINDS[..] {
        out.push(format!(
            "INSTANCE_KINDS (tools_v2/xtask/src/verifications/schemas/checks.rs) {:?} != \
             developer_tools::world_export_pipeline::INSTANCE_KINDS {:?} — the two census kind lists must stay \
             identical INCLUDING ORDER (it is the emitted byKind key order)",
            INSTANCE_KINDS,
            developer_tools::world_export_pipeline::INSTANCE_KINDS
        ));
    }
    out
}

pub fn type_inventory() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let schema = read_json(&sroot.join("definitions/map-object-type-inventory.schema.json"))?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let enums = read_json(&sroot.join("definitions/map-object-enums.schema.json"))?;

    let mut failures: Vec<String> = Vec::new();

    // T-594. The lockstep pin for INSTANCE_KINDS, RUN rather than merely written down. It is here
    // and not only in a #[test] because nothing runs xtask's tests: the wave gate tests
    // website-api / map-engine-* / website-frontend, and CI mirrors that. `xtask schema
    // type-inventory` is in GATE_SCHEMA_VALIDATE_GATES, so this executes in both gate halves.
    // First, before any inventory is examined — if the kind list is wrong then every I1 verdict
    // below it is computed over the wrong set of buckets and must not be believed.
    failures.extend(instance_kinds_lockstep_failures(&enums));

    let check = |label: &str, inv: &Value, manifest: Option<&Value>, failures: &mut Vec<String>| {
        let errs: Vec<String> = validator
            .iter_errors(inv)
            .map(|e| {
                let p = e.instance_path().to_string();
                format!(
                    "{label}: schema {} {e}",
                    if p.is_empty() { "/".to_string() } else { p }
                )
            })
            .collect();
        if !errs.is_empty() {
            failures.extend(errs);
            return;
        }

        if inv["censusStatus"] == "pending_export" {
            if !inv["levels"]["totalInstances"].is_null()
                || !inv["levels"]["uniquePrefabs"].is_null()
            {
                failures.push(format!(
                    "{label}: pending_export requires null levels.* counts"
                ));
            }
            for k in INSTANCE_KINDS {
                let bucket = &inv["byKind"][k];
                if !bucket["prefabTypes"].is_null() || !bucket["instances"].is_null() {
                    failures.push(format!(
                        "{label}: pending_export requires null byKind.{k} counts"
                    ));
                }
                if k == "road" && !bucket["segments"].is_null() {
                    failures.push(format!(
                        "{label}: pending_export requires null byKind.road.segments"
                    ));
                }
            }
            return;
        }

        // I1 — Σ byKind.instances = levels.totalInstances.
        let kind_sum: i64 = INSTANCE_KINDS
            .iter()
            .filter_map(|k| inv["byKind"][*k]["instances"].as_i64())
            .sum();
        let total = inv["levels"]["totalInstances"].as_i64().unwrap_or(-1);
        if kind_sum != total {
            failures.push(format!(
                "{label}: I1 kind sum {kind_sum} !== levels.totalInstances {total}"
            ));
        }

        // I2 — building class sum when populated.
        if let Some(by_building) = inv["byBuildingClass"].as_object()
            && !by_building.is_empty()
        {
            let class_sum: i64 = by_building
                .values()
                .filter_map(|row| row["instances"].as_i64())
                .sum();
            let b = inv["byKind"]["building"]["instances"]
                .as_i64()
                .unwrap_or(-1);
            if class_sum != b {
                failures.push(format!(
                    "{label}: I2 byBuildingClass sum {class_sum} !== byKind.building.instances {b}"
                ));
            }
        }

        // Forest region tree assignment — exact.
        if inv["byRegionKind"]["forest"].is_object()
            && let Some(tree_total) = inv["byKind"]["tree"]["instances"].as_i64()
        {
            let region_trees = inv["byRegionKind"]["forest"]["treeCount"]
                .as_i64()
                .unwrap_or(0);
            let unassigned = inv["unassignedTrees"].as_i64().unwrap_or(0);
            if region_trees + unassigned != tree_total {
                failures.push(format!(
                        "{label}: F-count forest.treeCount ({region_trees}) + unassignedTrees ({unassigned}) !== byKind.tree.instances ({tree_total})"
                    ));
            }
        }

        // I3 — per-class keys ∈ closed enums.
        for (bucket, enum_name) in [
            ("byBuildingClass", "buildingClass"),
            ("byRoadClass", "roadClass"),
            ("bySpeciesClass", "speciesClass"),
        ] {
            let allowed: HashSet<&str> = enums["$defs"][enum_name]["enum"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            for cls in inv[bucket]
                .as_object()
                .map(|m| m.keys())
                .into_iter()
                .flatten()
            {
                if !allowed.contains(cls.as_str()) {
                    failures.push(format!(
                        "{label}: I3 {bucket} key '{cls}' not in {enum_name} enum"
                    ));
                }
            }
        }

        // I4 — complete census requires needsReview.prefabTypes = 0.
        if inv["censusStatus"] == "complete" && inv["needsReview"]["prefabTypes"] != 0 {
            failures.push(format!(
                "{label}: I4 complete census requires needsReview.prefabTypes = 0 (got {})",
                inv["needsReview"]["prefabTypes"]
            ));
        }

        // I5 / I7 — manifest.objects cross-check.
        if let Some(m) = manifest
            && let Some(prefab_count) = m["objects"]["prefabCount"].as_i64()
        {
            let unique = inv["levels"]["uniquePrefabs"].as_i64().unwrap_or(-1);
            if prefab_count != unique {
                failures.push(format!(
                        "{label}: I5 manifest.objects.prefabCount {prefab_count} !== levels.uniquePrefabs {unique}"
                    ));
            }
            let mi = m["objects"]["instanceCount"].as_i64().unwrap_or(-1);
            if mi != total {
                failures.push(format!(
                        "{label}: I7 manifest.objects.instanceCount {mi} !== levels.totalInstances {total}"
                    ));
            }
        }
    };

    let registry_path = terrain_registry_path(&root);
    if registry_path.exists() {
        let reg = read_json(&registry_path)?;
        for t in reg["terrains"].as_array().into_iter().flatten() {
            let terrain_id = t["terrainId"].as_str().unwrap_or_default();
            let inv_path = terrain_dir(&root, terrain_id).join("objects/type-inventory.json");
            if !inv_path.exists() {
                continue;
            }
            let manifest_path =
                terrain_assets_dir(&root).join(t["manifestPath"].as_str().unwrap_or_default());
            let manifest = manifest_path
                .exists()
                .then(|| read_json(&manifest_path))
                .transpose()?;
            let inv = read_json(&inv_path)?;
            check(
                &format!("{terrain_id}/objects/type-inventory.json"),
                &inv,
                manifest.as_ref(),
                &mut failures,
            );
        }
    }

    let golden = sroot.join("fixtures/map/type-inventory-pending-everon.json");
    if golden.exists() {
        let inv = read_json(&golden)?;
        check(
            "golden/type-inventory-pending-everon.json",
            &inv,
            None,
            &mut failures,
        );
    }

    for t in ["everon", "arland", "custom"] {
        let spike = map_scratch_dir(&root, t).join("spike/type-inventory-spike.json");
        if spike.exists() {
            let inv = read_json(&spike)?;
            check(
                &format!("assets_v2/scratch/{t}/spike/type-inventory-spike.json"),
                &inv,
                None,
                &mut failures,
            );
        }
    }

    Ok(verdict("verify-type-inventory", "", &failures))
}
