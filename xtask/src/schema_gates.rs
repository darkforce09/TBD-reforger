//! T-165.1 — the text/JSON schema gates, ported from `packages/tbd-schema/scripts/*.mjs`
//! (verify-contract-citations, verify-t090-spec-consistency, verify-n6-sentence,
//! verify-n10-tile-budget, verify-map-object-enums, verify-type-inventory,
//! verify-terrain-manifest, flatten-orbat-slots). Behavior parity with the Node originals:
//! same gate semantics, same OK/FAIL verdict lines, same exit codes; stdout formatting is
//! near-identical but the acceptance contract is verdict-set + exit code (T-165 plan).
//!
//! Retirements carried over from the Node era (printed, so the surface change is visible):
//! - TS-6 front-end export tags — the React contract layer was deleted at T-159.29.3; the
//!   Leptos contract layer is Rust (`dto.rs`) gated by R-api golden tests.
//! - GO-7 @route match — the Go handlers were retired at the T-145 Rust cutover; axum wires
//!   routes through typed fns, so a rename is a compile error, not doc rot.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::root::find_repo_root as repo_root;

fn read_json(p: &Path) -> Result<Value> {
    let raw = fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}

fn schema_root(root: &Path) -> PathBuf {
    root.join("packages/tbd-schema")
}

fn spec_dir(root: &Path) -> PathBuf {
    root.join("docs/specs/Mission_Creator_Architecture")
}

/// Print a FAIL header + errors and return exit code 1; or the OK line and 0.
fn verdict(name: &str, ok_line: &str, errors: &[String]) -> u8 {
    if errors.is_empty() {
        if ok_line.is_empty() {
            println!("{name}: OK");
        } else {
            println!("{name}: OK {ok_line}");
        }
        0
    } else {
        eprintln!("{name}: FAIL ({})", errors.len());
        for e in errors {
            eprintln!("  {e}");
        }
        1
    }
}

/* ─────────────────────────── citations ─────────────────────────── */

/// RFC-6901 pointer resolution ("", "#", "#/" = root) — mirror of `pointerResolves`.
fn pointer_resolves(doc: &Value, pointer: &str) -> bool {
    if pointer.is_empty() || pointer == "#" || pointer == "#/" {
        return true;
    }
    let path = pointer.strip_prefix('#').unwrap_or(pointer);
    if !path.starts_with('/') {
        return false;
    }
    let mut cur = doc;
    for raw in path.split('/').skip(1) {
        let key = raw.replace("~1", "/").replace("~0", "~");
        match cur {
            Value::Object(m) => match m.get(&key) {
                Some(v) => cur = v,
                None => return false,
            },
            Value::Array(a) => match key.parse::<usize>().ok().and_then(|i| a.get(i)) {
                Some(v) => cur = v,
                None => return false,
            },
            _ => return false,
        }
    }
    true
}

const CODE_EXTS: [&str; 6] = ["go", "ts", "tsx", "c", "mjs", "js"];
const IGNORE_DIRS: [&str; 6] = [
    "node_modules",
    "dist",
    ".git",
    "build",
    "coverage",
    "vendor",
];

pub fn citations() -> Result<u8> {
    let root = repo_root()?;
    let schema_dir = schema_root(&root).join("schema");
    let tag_re = regex::Regex::new(r#"@contract\s+([A-Za-z0-9_.\-]+\.schema\.json)(#[^\s)"']*)?"#)?;

    let mut schema_cache: HashMap<String, Option<Value>> = HashMap::new();
    let mut citations = 0usize;
    let mut problems: Vec<String> = Vec::new();

    for scan in ["apps", "packages"] {
        let base = root.join(scan);
        if !base.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&base)
            .into_iter()
            .filter_entry(|e| {
                !(e.file_type().is_dir()
                    && IGNORE_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let ext = entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            if !CODE_EXTS.contains(&ext.as_str()) {
                continue;
            }
            let Ok(text) = fs::read_to_string(entry.path()) else {
                continue;
            };
            for cap in tag_re.captures_iter(&text) {
                citations += 1;
                let name = cap.get(1).unwrap().as_str();
                let pointer = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                let rel = entry
                    .path()
                    .strip_prefix(&root)
                    .unwrap_or(entry.path())
                    .display();
                let doc = schema_cache
                    .entry(name.to_string())
                    .or_insert_with(|| read_json(&schema_dir.join(name)).ok());
                match doc {
                    None => problems.push(format!(
                        "{rel}: @contract {name}{pointer} -> schema/{name} not found"
                    )),
                    Some(doc) => {
                        if !pointer_resolves(doc, pointer) {
                            problems.push(format!(
                                "{rel}: @contract {name}{pointer} -> JSON pointer not found in schema"
                            ));
                        }
                    }
                }
            }
        }
    }

    println!("Checked {citations} @contract citation(s) across apps, packages.");
    if problems.is_empty() {
        println!("All @contract citations resolve.");
    } else {
        eprintln!("\n{} dangling citation(s):", problems.len());
        for p in &problems {
            eprintln!("  {p}");
        }
    }
    println!(
        "TS-6 retired: the React contract layer was deleted at T-159.29.3 (Leptos dto.rs is R-api-golden gated)."
    );
    println!(
        "GO-7 retired: Go handlers removed at the T-145 Rust cutover (axum routes are compile-checked)."
    );
    Ok(if problems.is_empty() { 0 } else { 1 })
}

/* ─────────────────────────── n6 / n10 ─────────────────────────── */

pub fn n6_sentence() -> Result<u8> {
    let root = repo_root()?;
    let norm = |s: &str| -> String {
        let stripped: String = s.chars().filter(|c| *c != '`' && *c != '*').collect();
        stripped.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let core = norm(
        "oriented bounding rectangle from spatial.halfExtentsM + rotationDeg. Real footprint polygon rings \
         are populated only when T-090.3.0 proves Enfusion footprint export; when present, polygons \
         supersede OBB rectangles for render.",
    );
    let spec = spec_dir(&root);
    let files = [
        spec.join("t090_2_map_object_taxonomy.md"),
        spec.join("t090_5_map_object_render_layer.md"),
        spec.join("t090_6_geometry_placement_audit.md"),
        spec.join("t090_world_object_glyphs.md"),
        schema_root(&root).join("schema/map-object-prefab.schema.json"),
    ];
    let mut missing = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).with_context(|| format!("read {}", f.display()))?;
        if !norm(&text).contains(&core) {
            missing.push(f.strip_prefix(&root).unwrap_or(f).display().to_string());
        }
    }
    if missing.is_empty() {
        println!(
            "verify-n6-sentence: OK (N6 sentence identical across {} locations)",
            files.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-n6-sentence: FAIL — N6 building-geometry sentence missing/drifted in:");
        for m in &missing {
            eprintln!("  {m}");
        }
        Ok(1)
    }
}

pub fn n10_tile_budget() -> Result<u8> {
    let root = repo_root()?;
    let spec = spec_dir(&root);
    // Dash-agnostic (figure/en/em → hyphen), mirroring the Node normalizer.
    let norm = |name: &str| -> Result<String> {
        let raw = fs::read_to_string(spec.join(name)).with_context(|| name.to_string())?;
        Ok(raw
            .chars()
            .map(|c| match c {
                '\u{2012}'..='\u{2015}' => '-',
                other => other,
            })
            .collect())
    };
    let canonical = [
        "200-400 MB",
        "400-800 MB",
        "512 tiles",
        "Max concurrent tile fetches",
        "one basemap pyramid",
    ];
    let forbidden = ["1.6 GB", "200-800 MB"];
    let mut errors = Vec::new();
    for f in [
        "t090_basemap_dual_view.md",
        "t090_terrain_export_pipeline.md",
    ] {
        let text = norm(f)?;
        for row in canonical {
            if !text.contains(row) {
                errors.push(format!("{f}: N10 row missing \"{row}\""));
            }
        }
    }
    for entry in fs::read_dir(&spec)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if !(name.starts_with("t090") && name.ends_with(".md")) {
            continue;
        }
        let text = norm(&name)?;
        for bad in forbidden {
            if text.contains(bad) {
                errors.push(format!(
                    "{name}: restates conflicting tile budget \"{bad}\" (N10 is single source)"
                ));
            }
        }
    }
    if errors.is_empty() {
        println!(
            "verify-n10-tile-budget: OK (N10 tile-budget single-source across basemap + pipeline)"
        );
        Ok(0)
    } else {
        eprintln!("verify-n10-tile-budget: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}

/* ─────────────────────────── map-object enums ─────────────────────────── */

pub fn map_object_enums() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let enums = read_json(&sroot.join("schema/map-object-enums.schema.json"))?;
    let defs = &enums["$defs"];
    let set = |name: &str| -> HashSet<String> {
        defs[name]["enum"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let sets: BTreeMap<&str, HashSet<String>> = BTreeMap::from([
        ("kind", set("kind")),
        ("buildingClass", set("buildingClass")),
        ("roadClass", set("roadClass")),
        ("speciesClass", set("speciesClass")),
        ("forestClass", set("forestClass")),
        ("rockClass", set("rockClass")),
        ("propClass", set("propClass")),
        ("utilityClass", set("utilityClass")),
        ("waterClass", set("waterClass")),
    ]);
    let class_enum_for_kind: BTreeMap<&str, &str> = BTreeMap::from([
        ("building", "buildingClass"),
        ("road", "roadClass"),
        ("tree", "speciesClass"),
        ("vegetation", "speciesClass"),
        ("rock", "rockClass"),
        ("prop", "propClass"),
        ("utility", "utilityClass"),
        ("water", "waterClass"),
    ]);

    let mut errors: Vec<String> = Vec::new();
    let mut check_row = |src: String, kind: Option<&str>, class: Option<&str>| {
        let Some(kind) = kind else {
            return;
        };
        if !sets["kind"].contains(kind) {
            errors.push(format!(
                "{src}: kind '{kind}' not in map-object-enums#/$defs/kind"
            ));
            return;
        }
        let Some(enum_name) = class_enum_for_kind.get(kind) else {
            errors.push(format!(
                "{src}: kind '{kind}' has no class-enum mapping (regions carry no prefab class)"
            ));
            return;
        };
        if let Some(klass) = class {
            if !sets[enum_name].contains(klass) {
                errors.push(format!(
                    "{src}: class '{klass}' not in {enum_name} (kind={kind})"
                ));
            }
        }
    };

    let prefabs = read_json(&sroot.join("golden/map-objects/map-object-prefabs-sample.json"))?;
    let prefab_count = prefabs.as_array().map(Vec::len).unwrap_or(0);
    for p in prefabs.as_array().into_iter().flatten() {
        check_row(
            format!("golden prefab {}", p["prefabId"]),
            p["kind"].as_str(),
            p["class"].as_str(),
        );
    }

    let classify = read_json(&sroot.join("rules/prefab-classify.json"))?;
    for (i, r) in classify["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check_row(
            format!("prefab-classify rule[{i}]"),
            r["kind"].as_str(),
            r["class"].as_str(),
        );
    }
    if classify["fallback"].is_object() {
        check_row(
            "prefab-classify fallback".to_string(),
            classify["fallback"]["kind"].as_str(),
            classify["fallback"]["class"].as_str(),
        );
    }

    let regions =
        read_json(&sroot.join("golden/map-objects/map-object-regions-everon-sample.json"))?;
    for reg in regions.as_array().into_iter().flatten() {
        let id = &reg["id"];
        if let Some(kind) = reg["kind"].as_str() {
            if !sets["kind"].contains(kind) {
                errors.push(format!("region {id}: kind '{kind}' not in kind enum"));
            }
        }
        if let Some(d) = reg["dominantSpeciesClass"].as_str() {
            if !sets["forestClass"].contains(d) {
                errors.push(format!(
                    "region {id}: dominantSpeciesClass '{d}' not in forestClass"
                ));
            }
        }
    }

    let glyphs_doc = read_json(&root.join("packages/map-assets/glyphs/manifest.json"))?;
    let glyphs = glyphs_doc["glyphs"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for key in glyphs.keys() {
        let kind_tok = key.split('-').next().unwrap_or("");
        if !sets["kind"].contains(kind_tok) {
            errors.push(format!(
                "glyph '{key}': kind prefix '{kind_tok}' not in kind enum"
            ));
        }
    }

    if errors.is_empty() {
        println!(
            "verify-map-object-enums: OK ({prefab_count} prefabs, {} glyphs, enums single-source)",
            glyphs.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-map-object-enums: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}

/* ─────────────────────────── type inventory (I1–I7) ─────────────────────────── */

const INSTANCE_KINDS: [&str; 8] = [
    "building",
    "tree",
    "vegetation",
    "rock",
    "prop",
    "utility",
    "water",
    "road",
];

pub fn type_inventory() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let schema = read_json(&sroot.join("schema/map-object-type-inventory.schema.json"))?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let enums = read_json(&sroot.join("schema/map-object-enums.schema.json"))?;

    let mut failures: Vec<String> = Vec::new();

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
        if let Some(by_building) = inv["byBuildingClass"].as_object() {
            if !by_building.is_empty() {
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
        }

        // Forest region tree assignment — exact.
        if inv["byRegionKind"]["forest"].is_object() {
            if let Some(tree_total) = inv["byKind"]["tree"]["instances"].as_i64() {
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
        if let Some(m) = manifest {
            if let Some(prefab_count) = m["objects"]["prefabCount"].as_i64() {
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
        }
    };

    let registry_path = root.join("packages/map-assets/terrain-registry.json");
    if registry_path.exists() {
        let reg = read_json(&registry_path)?;
        for t in reg["terrains"].as_array().into_iter().flatten() {
            let terrain_id = t["terrainId"].as_str().unwrap_or_default();
            let inv_path = root
                .join("packages/map-assets")
                .join(terrain_id)
                .join("objects/type-inventory.json");
            if !inv_path.exists() {
                continue;
            }
            let manifest_path = root
                .join("packages/map-assets")
                .join(t["manifestPath"].as_str().unwrap_or_default());
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

    let golden = sroot.join("golden/map-objects/type-inventory-pending-everon.json");
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
        let spike = root
            .join("packages/map-assets")
            .join(t)
            .join("staging/spike/type-inventory-spike.json");
        if spike.exists() {
            let inv = read_json(&spike)?;
            check(
                &format!("{t}/staging/spike/type-inventory-spike.json"),
                &inv,
                None,
                &mut failures,
            );
        }
    }

    Ok(verdict("verify-type-inventory", "", &failures))
}

/* ─────────────────────────── terrain manifest ─────────────────────────── */

struct TerrainContract {
    width: f64,
    height: f64,
    min_m: f64,
    max_m: f64,
}

pub fn terrain_manifest(terrain: &str) -> Result<u8> {
    let contract = match terrain {
        "everon" => TerrainContract {
            width: 12800.0,
            height: 12800.0,
            min_m: -204.78,
            max_m: 375.53,
        },
        "arland" => TerrainContract {
            width: 4096.0,
            height: 4096.0,
            min_m: -163.0,
            max_m: 148.38,
        },
        other => {
            eprintln!("Unknown terrain \"{other}\". Use: everon | arland");
            return Ok(2);
        }
    };
    let root = repo_root()?;
    let manifest_path = root.join(format!("packages/map-assets/{terrain}/manifest.json"));
    let manifest = match read_json(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("FAIL  Cannot read manifest: {}", manifest_path.display());
            eprintln!("{e}");
            return Ok(1);
        }
    };

    let schema = read_json(&schema_root(&root).join("schema/terrain-manifest.schema.json"))?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let schema_errs: Vec<String> = validator
        .iter_errors(&manifest)
        .map(|e| {
            let p = e.instance_path().to_string();
            format!(
                "      {} {e}",
                if p.is_empty() { "/".to_string() } else { p }
            )
        })
        .collect();
    if !schema_errs.is_empty() {
        eprintln!("FAIL  Manifest schema validation:");
        for e in schema_errs {
            eprintln!("{e}");
        }
        return Ok(1);
    }
    println!("PASS  Manifest validates against terrain-manifest.schema.json");

    let bounds: Vec<f64> = manifest["worldBounds"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let mut errors = Vec::new();
    if manifest["terrainId"] != terrain {
        errors.push("terrainId mismatch".to_string());
    }
    if bounds.len() != 4
        || bounds[0] != 0.0
        || bounds[1] != 0.0
        || bounds[2] != contract.width
        || bounds[3] != contract.height
    {
        errors.push(format!(
            "worldBounds !== [0,0,{},{}]",
            contract.width, contract.height
        ));
    }
    let min_m = manifest["dem"]["heightRangeMinM"]
        .as_f64()
        .unwrap_or(f64::NAN);
    let max_m = manifest["dem"]["heightRangeMaxM"]
        .as_f64()
        .unwrap_or(f64::NAN);
    if (min_m - contract.min_m).abs() > 0.01 {
        errors.push("dem.heightRangeMinM !== terrains.ts".to_string());
    }
    if (max_m - contract.max_m).abs() > 0.01 {
        errors.push("dem.heightRangeMaxM !== terrains.ts".to_string());
    }
    if manifest["precision"]["storageDecimals"] != 3 {
        errors.push("storageDecimals must be 3".to_string());
    }
    if manifest["precision"]["spawnAuthority"] != "mod-get-surface-y" {
        errors.push("spawnAuthority must be mod-get-surface-y".to_string());
    }
    let wpx = manifest["dem"]["widthPx"].as_f64().unwrap_or(0.0);
    let hpx = manifest["dem"]["heightPx"].as_f64().unwrap_or(0.0);
    if wpx == 0.0 || hpx == 0.0 {
        println!("WARN  Stub manifest (widthPx/heightPx=0) — OK for T-090.0");
    } else if manifest["dem"]["exportedAt"]
        .as_str()
        .unwrap_or("")
        .is_empty()
        || manifest["dem"]["workbenchVersion"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    {
        errors.push("exportedAt/workbenchVersion required when DEM dims set".to_string());
    }

    if !errors.is_empty() {
        eprintln!("FAIL  terrains.ts cross-check:");
        for e in &errors {
            eprintln!("      {e}");
        }
        return Ok(1);
    }
    println!("PASS  Manifest matches terrains.ts for {terrain}");
    println!("\nverify-terrain-manifest: OK");
    Ok(0)
}

/* ─────────────────────────── t090 spec consistency (12 gates) ─────────────────────────── */

pub fn t090_specs() -> Result<u8> {
    let root = repo_root()?;
    let spec = spec_dir(&root);
    let read = |p: PathBuf| -> Result<String> {
        fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))
    };

    let mut t090_files: Vec<String> = fs::read_dir(&spec)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("t090") && n.ends_with(".md"))
        .collect();
    t090_files.sort();
    let corpus: Vec<(String, String)> = t090_files
        .iter()
        .map(|n| Ok((n.clone(), read(spec.join(n))?)))
        .collect::<Result<_>>()?;

    let mut failures: Vec<String> = Vec::new();
    let mut fail = |gate: &str, msg: String| failures.push(format!("[{gate}] {msg}"));

    let window_has = |text: &str, i: usize, radius: usize, re: &regex::Regex| -> bool {
        let lo = i.saturating_sub(radius);
        let hi = (i + radius).min(text.len());
        // Snap to char boundaries.
        let lo = (lo..=i).find(|&b| text.is_char_boundary(b)).unwrap_or(i);
        let hi = (hi..text.len())
            .find(|&b| text.is_char_boundary(b))
            .unwrap_or(text.len());
        re.is_match(&text[lo..hi])
    };

    // Gate 1.
    let g1 = regex::RegexBuilder::new(r"Pick/select world objects \(future")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        if g1.is_match(text) {
            fail(
                "1",
                format!("{name}: contains forbidden \"Pick/select world objects (future...\""),
            );
        }
    }

    // Gate 2.
    let g2a = regex::RegexBuilder::new(r"reuse\s+slotClusterIndex")
        .case_insensitive(true)
        .build()?;
    let g2b = regex::RegexBuilder::new(r"separate\s+world")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        if g2a.is_match(text) && !g2b.is_match(text) {
            fail(
                "2",
                format!(
                    "{name}: \"reuse slotClusterIndex\" without \"separate world\" clarification"
                ),
            );
        }
    }

    // Gate 3 — tile-zoom LOD tokens need deckZoom context within 800 chars.
    let lod = regex::Regex::new(r"z\s*[≤≥<>]\s*[0-5]|\bz[0-5]\s*[-–]\s*z?[0-5]\b|\bz[0-5]\+")?;
    let zoom_ctx = regex::RegexBuilder::new(r"deckZoom|Deck orthographic")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        for m in lod.find_iter(text) {
            if !window_has(text, m.start(), 800, &zoom_ctx) {
                fail(
                    "3",
                    format!(
                        "{name}: tile-zoom LOD token \"{}\" without deckZoom/Deck-orthographic context within 800 chars",
                        m.as_str().trim()
                    ),
                );
            }
        }
    }

    // Gate 4 — "Deck pick"/"onHover" need forbidden-context within 220 chars.
    let pick_ctx =
        regex::RegexBuilder::new(r"forbidden|removed|never|no\s+deck|not\s+re-?enable|do\s+not")
            .case_insensitive(true)
            .build()?;
    let deck_pick = regex::RegexBuilder::new(r"Deck\s+pick")
        .case_insensitive(true)
        .build()?;
    let on_hover = regex::Regex::new(r"onHover")?;
    for (name, text) in &corpus {
        for re in [&deck_pick, &on_hover] {
            for m in re.find_iter(text) {
                if !window_has(text, m.start(), 220, &pick_ctx) {
                    fail(
                        "4",
                        format!(
                            "{name}: \"{}\" without forbidden/removed/never context within 220 chars",
                            m.as_str()
                        ),
                    );
                }
            }
        }
    }

    // Gate 5.
    let eng: String = read(spec.join("engineering_plan.md"))?
        .chars()
        .filter(|c| *c != '`' && *c != '*')
        .collect();
    let g5 = regex::RegexBuilder::new(r"Picking via Deck's onClick/onHover")
        .case_insensitive(true)
        .build()?;
    if g5.is_match(&eng) {
        fail(
            "5",
            "engineering_plan.md: still contains \"Picking via Deck's onClick/onHover\""
                .to_string(),
        );
    }

    // Gate 6.
    let hub = read(spec.join("t090_091_map_terrain_program.md"))?;
    let gap_ids = [
        "GAP-001", "GAP-002", "GAP-003", "GAP-004", "GAP-005", "GAP-H1", "GAP-H2", "GAP-H3",
        "GAP-H4", "GAP-H5", "GAP-H6", "GAP-H7", "GAP-H8", "GAP-M1", "GAP-M2", "GAP-M3", "GAP-M4",
        "GAP-M5", "GAP-M6", "GAP-M7",
    ];
    for id in gap_ids {
        if !hub.contains(id) {
            fail(
                "6",
                format!("t090_091_map_terrain_program.md: audit closure missing {id}"),
            );
        }
    }
    for low in ["L1", "L2", "L3", "L4", "L5"] {
        let re = regex::Regex::new(&format!(r"\b{low}\b"))?;
        if !re.is_match(&hub) {
            fail(
                "6",
                format!("t090_091_map_terrain_program.md: audit closure missing {low}"),
            );
        }
    }

    // Gate 7 — every referenced make target / npm script exists (frozen React allowlist).
    let makefile = read(root.join("Makefile"))?;
    let target_re = regex::Regex::new(r"(?m)^([A-Za-z0-9_-]+):")?;
    let mut make_targets: HashSet<String> = target_re
        .captures_iter(&makefile)
        .map(|c| c[1].to_string())
        .collect();
    for t in [
        "map-assets-link",
        "web",
        "wasm",
        "verify-wgpu-gpu",
        "ci-local-frontend",
        "verify-migration",
    ] {
        make_targets.insert(t.to_string());
    }
    // T-165.9: the tbd-schema npm package is deleted (the Node eradication endpoint) — any
    // npm-script citation in the spec corpus is archival by definition, so the live-scripts
    // set is empty and the allowlist below carries every historically-cited name.
    let pkg_path = schema_root(&root).join("package.json");
    let mut npm_scripts: HashSet<String> = if pkg_path.exists() {
        read_json(&pkg_path)?["scripts"]
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    } else {
        HashSet::new()
    };
    for s in [
        "dev",
        "build",
        "lint",
        "preview",
        "test",
        "format",
        "format:check",
    ] {
        npm_scripts.insert(s.to_string());
    }
    // Gate scripts retired to `cargo xtask schema …` at T-165.1/.2 — historical specs may still
    // quote the npm form (archival, not executable).
    for s in [
        "validate",
        "codegen",
        "verify-map-object-golden",
        "verify-map-glyphs",
        "verify-citations",
        "verify-map-object-enums",
        "verify-type-inventory",
        "verify-t090-specs",
        "verify-n6",
        "verify-n10",
        "verify-terrain-manifest",
        // retired with the T-165.4/.9 terrain + image lanes (package deleted at .9)
        "verify-terrain-alignment",
        "verify-terrain",
    ] {
        npm_scripts.insert(s.to_string());
    }
    let make_re = regex::Regex::new(r"\bmake\s+([a-z0-9]+(?:-[a-z0-9]+)+)")?;
    let npm_re = regex::Regex::new(r"\bnpm run ([a-z0-9:_-]+)")?;
    for (name, text) in &corpus {
        for c in make_re.captures_iter(text) {
            if !make_targets.contains(&c[1]) {
                fail(
                    "7",
                    format!(
                        "{name}: referenced `make {}` not defined in root Makefile",
                        &c[1]
                    ),
                );
            }
        }
        for c in npm_re.captures_iter(text) {
            if !npm_scripts.contains(&c[1]) {
                fail(
                    "7",
                    format!(
                        "{name}: referenced `npm run {}` not in the historically-cited npm-script allowlist (Node was eradicated at T-165)",
                        &c[1]
                    ),
                );
            }
        }
    }

    // Gate 8 — no doc claims T-090.1 active.
    let authority = [
        root.join("CLAUDE.md"),
        spec.join("ROADMAP.md"),
        spec.join("agent_execution.md"),
        spec.join("engineering_plan.md"),
        root.join("docs/website/frontend/ROADMAP.md"),
        root.join("docs/website/frontend/INDEX.md"),
        root.join("docs/website/frontend/pages/mission-editor.md"),
        root.join("docs/mod/CLAUDE-CODE-START.md"),
    ];
    let mut gate8: Vec<(String, String)> = corpus.clone();
    for p in authority {
        let name = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
        let text = if p.exists() { read(p)? } else { String::new() };
        gate8.push((name, text));
    }
    let t0901 = regex::Regex::new(r"T-090\.1([^\d.]|\.\D|$)")?;
    let active = regex::RegexBuilder::new(r"\bactive\b")
        .case_insensitive(true)
        .build()?;
    let ok_ctx = regex::RegexBuilder::new(r"T-090\.3\.0|\bqueued\b|active\s+basemap")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &gate8 {
        for line in text.lines() {
            if !t0901.is_match(line) || !active.is_match(line) {
                continue;
            }
            if ok_ctx.is_match(line) {
                continue;
            }
            let trimmed: String = line.trim().chars().take(90).collect();
            fail(
                "8",
                format!("{name}: claims T-090.1 active — \"{trimmed}\""),
            );
        }
    }

    // Gate 9.
    let eden = read(spec.join("t090_eden_ai_world_object_schema.md"))?;
    let g9 = regex::RegexBuilder::new(r"move/delete this object")
        .case_insensitive(true)
        .build()?;
    if g9.is_match(&eden) {
        fail("9", "t090_eden_ai_world_object_schema.md: still says \"move/delete this object\" (mutation is Workbench-only)".to_string());
    }

    // Gate 10 — hub header names the registry active slice.
    let mut active_slice = "T-090.1.2.5".to_string();
    if let Ok(reg) = read_json(&root.join(".ai/tickets/registry.json")) {
        if let Some(t090) = reg["tickets"]
            .as_array()
            .and_then(|a| a.iter().find(|t| t["id"] == "T-090"))
        {
            if let Some(s) = t090["active_slice"].as_str() {
                active_slice = s.to_string();
            }
        }
    }
    let header: String = hub.chars().take(800).collect();
    if !header.contains(&active_slice) {
        fail(
            "10",
            format!(
                "t090_091_map_terrain_program.md: header does not name {active_slice} as the active slice"
            ),
        );
    }

    // Gate 11.
    let inv_spec = read(spec.join("t090_world_object_type_inventory.md"))?;
    let range_re =
        regex::RegexBuilder::new(r"800k|900k|1\.2M|2k–20k|400k–900k|order-of-magnitude \(Everon")
            .case_insensitive(true)
            .build()?;
    let ok11 = regex::RegexBuilder::new(
        r"\bnever\b|forbidden|not a substitute|PENDING|hard-coded|no hard-",
    )
    .case_insensitive(true)
    .build()?;
    for line in inv_spec.lines() {
        if range_re.is_match(line) && !ok11.is_match(line) {
            let trimmed: String = line.trim().chars().take(90).collect();
            fail(
                "11",
                format!(
                    "t090_world_object_type_inventory.md: Everon estimate range — \"{trimmed}\""
                ),
            );
        }
    }
    if !inv_spec.contains("censusStatus") || !inv_spec.contains("pending_export") {
        fail("11", "t090_world_object_type_inventory.md: must document censusStatus pending_export baseline".to_string());
    }

    // Gate 12 — phase-budget rows must cite inventory tokens, not hard-coded counts.
    let budget = regex::Regex::new(r"~?\d+(\.\d+)?\s*[kM]\b|\d{1,3},\d{3}")?;
    let inv_tok = regex::Regex::new(r"byKind|levels\.|inventory|derived")?;
    let p_row = regex::Regex::new(r"^\|\s*P\d+")?;
    for (name, text) in &corpus {
        for line in text.lines() {
            if !p_row.is_match(line) {
                continue;
            }
            if budget.is_match(line) && !inv_tok.is_match(line) {
                let trimmed: String = line.trim().chars().take(90).collect();
                fail(
                    "12",
                    format!("{name}: phase-budget row hard-codes a count — \"{trimmed}\""),
                );
            }
        }
    }

    if failures.is_empty() {
        println!(
            "verify-t090-specs: OK ({} spec files + authority docs, all 12 gates pass)",
            t090_files.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-t090-specs: FAIL ({})", failures.len());
        for f in &failures {
            eprintln!("  {f}");
        }
        Ok(1)
    }
}

/* ─────────────────────────── flatten-orbat-slots ─────────────────────────── */

pub fn flatten_orbat_slots(path: &str, in_place: bool) -> Result<u8> {
    let file = PathBuf::from(path);
    let mut mission = read_json(&file)?;

    let mut anchors: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for zone in mission["zones"].as_array().into_iter().flatten() {
        if zone["type"] == "spawn" {
            if let (Some(faction), Some(x), Some(z)) = (
                zone["faction"].as_str(),
                zone["shape"]["circle"]["x"].as_f64(),
                zone["shape"]["circle"]["z"].as_f64(),
            ) {
                anchors.insert(faction.to_string(), (x, z));
            }
        }
    }
    anchors.entry("blufor".into()).or_insert((4831.2, 6620.8));
    anchors.entry("opfor".into()).or_insert((6010.0, 7211.5));

    let mut slots: Vec<Value> = Vec::new();
    let mut slot_index = 0usize;
    let orbat = mission["orbat"].as_object().cloned().unwrap_or_default();
    for (faction_key, faction_orbat) in &orbat {
        let anchor = anchors
            .get(faction_key)
            .copied()
            .unwrap_or((6400.0, 6400.0));
        for group in faction_orbat["groups"].as_array().into_iter().flatten() {
            let callsign = group["callsign"].as_str().unwrap_or_default();
            for role in group["roles"].as_array().into_iter().flatten() {
                let count = role["count"].as_i64().unwrap_or(0);
                for i in 0..count {
                    let ring = (slot_index / 8) as f64;
                    let pos_in_ring = (slot_index % 8) as f64;
                    let angle = pos_in_ring / 8.0 * std::f64::consts::PI * 2.0;
                    let radius = 8.0 + ring * 6.0;
                    let x = anchor.0 + angle.cos() * radius;
                    let z = anchor.1 + angle.sin() * radius;
                    let heading =
                        (((anchor.0 - x).atan2(anchor.1 - z).to_degrees()) + 360.0) % 360.0;
                    slots.push(serde_json::json!({
                        "id": format!("{faction_key}:{callsign}:{}:{i}", role["slot"].as_str().unwrap_or_default()),
                        "faction": faction_key,
                        "groupCallsign": callsign,
                        "role": role["slot"],
                        "kit": role["kit"],
                        "x": (x * 10.0).round() / 10.0,
                        "z": (z * 10.0).round() / 10.0,
                        "headingDeg": heading.round(),
                    }));
                    slot_index += 1;
                }
            }
        }
    }

    mission["schemaVersion"] = Value::String("1.1".into());
    let n = slots.len();
    mission["slots"] = Value::Array(slots);
    let out = serde_json::to_string_pretty(&mission)? + "\n";
    if in_place {
        fs::write(&file, out)?;
        println!("Wrote {n} slots to {}", file.display());
    } else {
        print!("{out}");
    }
    Ok(0)
}

/* ─────────────────────────── validate (T-165.2 — the validate.mjs core) ─────────────────────────── */

/// The full contract-validation suite (port of `packages/tbd-schema/scripts/validate.mjs`):
/// golden missions + registries + compat FK walkers + addon/variant provenance + bridge samples +
/// terrain manifests/anchors + ENF-4 Enfusion DTO fixtures + the T-090.2 map-object goldens.
/// Cross-file `$ref`s resolve through a `referencing::Registry` keyed by each schema's `$id`
/// (the ajv `addSchema` equivalent); ENF-4 pointer validators are built as `{"$ref": "<id>#/$defs/<n>"}`.
pub fn validate_all() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let schema = |name: &str| read_json(&sroot.join("schema").join(name));
    let reg_file = |name: &str| sroot.join("registry").join(name);

    // Register every map-object schema (plus mission for the ENF-4 pointers) by $id.
    let mut registered: Vec<(String, Value)> = Vec::new();
    for f in [
        "map-object-enums.schema.json",
        "map-object-prefab.schema.json",
        "map-object-instance.schema.json",
        "map-object-region.schema.json",
        "map-object-roads.schema.json",
        "map-object-catalog.schema.json",
        "map-object-resolved.schema.json",
        "map-object-type-inventory.schema.json",
        "terrain-registry.schema.json",
        "mission.schema.json",
    ] {
        let doc = schema(f)?;
        let id = doc["$id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("{f}: missing $id"))?
            .to_string();
        registered.push((id, doc));
    }
    let registry = jsonschema::Registry::new()
        .extend(registered.iter().map(|(id, doc)| {
            (
                id.as_str(),
                jsonschema::Resource::from_contents(doc.clone()),
            )
        }))
        .map_err(|e| anyhow::anyhow!("registry: {e}"))?
        .prepare()
        .map_err(|e| anyhow::anyhow!("registry prepare: {e}"))?;
    let compile = |doc: &Value| -> Result<jsonschema::Validator> {
        jsonschema::options()
            .with_registry(&registry)
            .build(doc)
            .map_err(|e| anyhow::anyhow!("schema compile: {e}"))
    };
    let by_id = |name: &str| -> Result<jsonschema::Validator> {
        compile(&serde_json::json!({
            "$ref": format!("https://schema.tbdevent.eu/{name}/v1.json")
        }))
    };

    let failures = std::cell::Cell::new(0usize);
    let check = |label: &str, v: &jsonschema::Validator, data: &Value| {
        let errs: Vec<String> = v
            .iter_errors(data)
            .map(|e| {
                let p = e.instance_path().to_string();
                format!(
                    "        {} {e}",
                    if p.is_empty() { "/".to_string() } else { p }
                )
            })
            .collect();
        if errs.is_empty() {
            println!("  PASS  {label}");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {label}");
            for e in errs {
                println!("{e}");
            }
        }
    };

    let v_mission = compile(&schema("mission.schema.json")?)?;
    let v_registry = compile(&schema("registry.schema.json")?)?;
    let v_items = compile(&schema("registry-items.schema.json")?)?;
    let v_compat = compile(&schema("registry-compat.schema.json")?)?;
    let v_loadout = compile(&schema("loadout-export.schema.json")?)?;
    let v_bridge = compile(&read_json(
        &sroot.join("bridge/bridge-messages.schema.json"),
    )?)?;
    let v_tmanifest = compile(&schema("terrain-manifest.schema.json")?)?;
    let v_anchors = compile(&schema("terrain-anchors.schema.json")?)?;
    let v_editor = compile(&schema("mission-editor-payload.schema.json")?)?;
    let v_locations = compile(&schema("locations.schema.json")?)?;
    let v_hlabels = compile(&schema("height-labels.schema.json")?)?;
    let v_faction = compile(&schema("faction-library.schema.json")?)?;
    let v_mo_prefab = by_id("map-object-prefab")?;
    let v_mo_instance = by_id("map-object-instance")?;
    let v_mo_region = by_id("map-object-region")?;
    let v_mo_roads = by_id("map-object-roads")?;
    let v_mo_catalog = by_id("map-object-catalog")?;
    let v_mo_resolved = by_id("map-object-resolved")?;
    let v_mo_inventory = by_id("map-object-type-inventory")?;
    let v_tregistry = by_id("terrain-registry")?;

    let sorted_json_files = |dir: &Path| -> Result<Vec<String>> {
        let mut v: Vec<String> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".json"))
            .collect();
        v.sort();
        Ok(v)
    };

    println!("Golden missions:");
    let missions_dir = sroot.join("golden-missions");
    for f in sorted_json_files(&missions_dir)? {
        check(&f, &v_mission, &read_json(&missions_dir.join(&f))?);
    }

    println!("Registry:");
    check(
        "registry.example.json",
        &v_registry,
        &read_json(&reg_file("registry.example.json"))?,
    );
    check(
        "registry.vanilla-poc.json",
        &v_registry,
        &read_json(&reg_file("registry.vanilla-poc.json"))?,
    );

    println!("Registry items:");
    let items_sample = read_json(&reg_file("registry-items.sample.json"))?;
    let items_wb = read_json(&reg_file("registry-items.workbench.json"))?;
    check("registry-items.sample.json", &v_items, &items_sample);
    check("registry-items.workbench.json", &v_items, &items_wb);

    // Addon provenance + variant_of integrity (FK walkers).
    let fk = |label: String, ok: bool, pass_note: String, bad: Vec<String>| {
        if ok {
            println!("  PASS  {label} ({pass_note})");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {label}");
            for b in bad.iter().take(10) {
                println!("        {b}");
            }
            if bad.len() > 10 {
                println!("        ... {} more", bad.len() - 10);
            }
        }
    };
    let addon_refs = |items: &Value| -> (usize, usize, Vec<String>) {
        let known: HashSet<&str> = items["addons"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect())
            .unwrap_or_default();
        let mut with_addon = 0;
        let mut bad = Vec::new();
        let total = items["items"].as_array().map(Vec::len).unwrap_or(0);
        for it in items["items"].as_array().into_iter().flatten() {
            let Some(addon) = it.get("addon").and_then(Value::as_str) else {
                continue;
            };
            with_addon += 1;
            if !known.contains(addon) {
                bad.push(format!(
                    "dangling {} addon {addon}",
                    it["resource_name"].as_str().unwrap_or("?")
                ));
            }
        }
        (with_addon, total, bad)
    };
    for (label, items) in [
        ("registry-items.sample.json", &items_sample),
        ("registry-items.workbench.json", &items_wb),
    ] {
        let (with_addon, total, bad) = addon_refs(items);
        fk(
            format!("{label} (addon provenance"),
            bad.is_empty(),
            format!("addon provenance, {with_addon}/{total} items carry addon"),
            bad,
        );
    }
    let variant_refs = |items: &Value| -> (usize, Vec<String>) {
        let known: HashSet<&str> = items["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x["resource_name"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut variants = 0;
        let mut bad = Vec::new();
        for it in items["items"].as_array().into_iter().flatten() {
            let Some(vof) = it.get("variant_of").and_then(Value::as_str) else {
                continue;
            };
            variants += 1;
            let rn = it["resource_name"].as_str().unwrap_or("?");
            if !known.contains(vof) {
                bad.push(format!("{rn} variant_of {vof}"));
            }
            if vof == rn {
                bad.push(format!("{rn} is its own variant"));
            }
        }
        (variants, bad)
    };
    for (label, items) in [
        ("registry-items.sample.json", &items_sample),
        ("registry-items.workbench.json", &items_wb),
    ] {
        let (variants, bad) = variant_refs(items);
        fk(
            format!("{label} (variant_of integrity"),
            bad.is_empty(),
            format!("variant_of integrity, {variants} variants"),
            bad,
        );
    }

    println!("Registry compat:");
    let edge_refs = |items: &Value, compat: &Value| -> (usize, Vec<String>) {
        let known: HashSet<&str> = items["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x["resource_name"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut bad = Vec::new();
        let edges = compat["edges"].as_array().map(Vec::len).unwrap_or(0);
        for e in compat["edges"].as_array().into_iter().flatten() {
            let et = e["edge_type"].as_str().unwrap_or("?");
            for endpoint in ["from_node", "to_node"] {
                if let Some(n) = e[endpoint].as_str() {
                    if !known.contains(n) {
                        bad.push(format!("dangling {et} {endpoint} {n}"));
                    }
                }
            }
        }
        (edges, bad)
    };
    let compat_sample = read_json(&reg_file("registry-compat.sample.json"))?;
    check("registry-compat.sample.json", &v_compat, &compat_sample);
    let (edges, bad) = edge_refs(&items_sample, &compat_sample);
    fk(
        "registry-compat.sample.json vs registry-items.sample.json (referential integrity"
            .to_string(),
        bad.is_empty(),
        format!("referential integrity, {edges} edges"),
        bad,
    );
    let compat_wb = read_json(&reg_file("registry-compat.workbench.json"))?;
    check("registry-compat.workbench.json", &v_compat, &compat_wb);
    let (edges, bad) = edge_refs(&items_wb, &compat_wb);
    fk(
        "registry-compat.workbench.json vs registry-items.workbench.json (referential integrity"
            .to_string(),
        bad.is_empty(),
        format!("referential integrity, {edges} edges"),
        bad,
    );

    println!("Faction library:");
    check(
        "faction-library.sample.json",
        &v_faction,
        &read_json(&reg_file("faction-library.sample.json"))?,
    );

    println!("Loadout export:");
    check(
        "loadout-export.sample.json",
        &v_loadout,
        &read_json(&reg_file("loadout-export.sample.json"))?,
    );
    check(
        "loadout-export.v2.sample.json",
        &v_loadout,
        &read_json(&reg_file("loadout-export.v2.sample.json"))?,
    );

    println!("Mission editor payload:");
    check(
        "mission-editor-payload.sample.json",
        &v_editor,
        &read_json(&reg_file("mission-editor-payload.sample.json"))?,
    );

    println!("Bridge message samples:");
    let samples = sroot.join("bridge/samples");
    for f in sorted_json_files(&samples)? {
        check(&f, &v_bridge, &read_json(&samples.join(&f))?);
    }

    println!("Terrain manifest:");
    check(
        "everon/manifest.json",
        &v_tmanifest,
        &read_json(&root.join("packages/map-assets/everon/manifest.json"))?,
    );

    println!("Locations (T-152.6):");
    check(
        "locations-everon-sample.json",
        &v_locations,
        &read_json(&sroot.join("golden/locations-everon-sample.json"))?,
    );
    let everon_loc = root.join("packages/map-assets/everon/locations.json");
    if everon_loc.exists() {
        check(
            "map-assets/everon/locations.json",
            &v_locations,
            &read_json(&everon_loc)?,
        );
    }

    println!("Height labels (T-152.16):");
    let hl = root.join("packages/map-assets/everon/height-labels.json");
    if hl.exists() {
        check(
            "map-assets/everon/height-labels.json",
            &v_hlabels,
            &read_json(&hl)?,
        );
    }

    println!("Terrain anchors example:");
    check(
        "everon/anchors/verification.example.json",
        &v_anchors,
        &read_json(&root.join("packages/map-assets/everon/anchors/verification.example.json"))?,
    );

    println!("Enfusion DTO fixtures (ENF-4):");
    let mission_id = registered
        .iter()
        .find(|(_, d)| {
            d["$id"]
                .as_str()
                .map(|s| s.contains("mission"))
                .unwrap_or(false)
        })
        .map(|(id, _)| id.clone())
        .unwrap_or_default();
    let enf = sroot.join("enfusion");
    for f in sorted_json_files(&enf)? {
        if !f.ends_with(".sample.json") {
            continue;
        }
        let base = f.trim_end_matches(".sample.json");
        let data = read_json(&enf.join(&f))?;
        if base == "root" {
            check(&f, &v_mission, &data);
        } else {
            match compile(&serde_json::json!({ "$ref": format!("{mission_id}#/$defs/{base}") })) {
                Ok(v) => check(&f, &v, &data),
                Err(_) => {
                    failures.set(failures.get() + 1);
                    println!("  FAIL  {f} (no schema for #/$defs/{base})");
                }
            }
        }
    }

    let mo = sroot.join("golden/map-objects");
    println!("Map object prefabs (S9 — one row per buildingClass):");
    for (i, row) in read_json(&mo.join("map-object-prefabs-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!(
                "prefab[{i}] {}/{}",
                row["kind"].as_str().unwrap_or("?"),
                row["class"].as_str().unwrap_or("?")
            ),
            &v_mo_prefab,
            row,
        );
    }

    println!("Map object instances:");
    for (i, row) in read_json(&mo.join("map-object-instances-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(&format!("instance[{i}]"), &v_mo_instance, row);
    }

    println!("Map object chunk sample (T-090.3.1 — all-number 5-tuples):");
    let chunk = read_json(&mo.join("map-object-chunk-sample.json"))?;
    for (i, row) in chunk["chunk"]["instances"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(&format!("chunk-instance[{i}]"), &v_mo_instance, row);
    }

    println!("Map object regions (forest / field):");
    for (i, row) in read_json(&mo.join("map-object-regions-everon-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!("region[{i}] {}", row["kind"].as_str().unwrap_or("?")),
            &v_mo_region,
            row,
        );
    }

    println!("Map object roads:");
    check(
        "map-object-roads-sample.json",
        &v_mo_roads,
        &read_json(&mo.join("map-object-roads-sample.json"))?,
    );

    println!("Map object catalog bundle (validation-only, N12):");
    check(
        "map-object-catalog-everon-sample.json",
        &v_mo_catalog,
        &read_json(&mo.join("map-object-catalog-everon-sample.json"))?,
    );
    check(
        "phased/P1-buildings.json",
        &v_mo_catalog,
        &read_json(&mo.join("phased/P1-buildings.json"))?,
    );

    println!("ResolvedWorldObject (Eden AI + T-090.7):");
    for (i, row) in read_json(&mo.join("map-object-resolved-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!("resolved[{i}] {}", row["kind"].as_str().unwrap_or("?")),
            &v_mo_resolved,
            row,
        );
    }

    println!("Terrain registry:");
    check(
        "golden terrain-registry.sample.json",
        &v_tregistry,
        &read_json(&mo.join("terrain-registry.sample.json"))?,
    );
    check(
        "map-assets/terrain-registry.json",
        &v_tregistry,
        &read_json(&root.join("packages/map-assets/terrain-registry.json"))?,
    );

    println!("Dual + legacy terrain manifests (T-090.1/.1.1):");
    check(
        "everon-dual-tiles",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-dual-tiles.json"))?,
    );
    check(
        "everon-legacy-tiles",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-legacy-tiles.json"))?,
    );
    check(
        "everon-unified-satellite",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-unified-satellite.json"))?,
    );

    println!("Map object type inventory (exact counts — pending until export):");
    check(
        "type-inventory-pending-everon.json",
        &v_mo_inventory,
        &read_json(&mo.join("type-inventory-pending-everon.json"))?,
    );
    check(
        "map-assets/everon/objects/type-inventory.json",
        &v_mo_inventory,
        &read_json(&root.join("packages/map-assets/everon/objects/type-inventory.json"))?,
    );

    if failures.get() > 0 {
        eprintln!("\n{} validation failure(s).", failures.get());
        Ok(1)
    } else {
        println!("\nAll contracts valid.");
        Ok(0)
    }
}

/// Validate one mission JSON file (or stdin with `-`) — port of `validate-file.mjs`
/// (schema + the 1.1 ORBAT-count/slot-id checks; the deploy-staging V1 gate).
pub fn validate_file(target: &str) -> Result<u8> {
    let raw = if target == "-" {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        s
    } else {
        fs::read_to_string(target).with_context(|| target.to_string())?
    };
    let Ok(data) = serde_json::from_str::<Value>(&raw) else {
        eprintln!("invalid JSON");
        return Ok(1);
    };

    let root = repo_root()?;
    let schema = read_json(&schema_root(&root).join("schema/mission.schema.json"))?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let errs: Vec<String> = validator
        .iter_errors(&data)
        .map(|e| {
            let p = e.instance_path().to_string();
            format!("{} {e}", if p.is_empty() { "/".to_string() } else { p })
        })
        .collect();
    if !errs.is_empty() {
        for e in errs {
            eprintln!("{e}");
        }
        return Ok(1);
    }

    if data["schemaVersion"] == "1.1" {
        let mut expected: i64 = 0;
        for faction in data["orbat"]
            .as_object()
            .map(|m| m.values())
            .into_iter()
            .flatten()
        {
            for group in faction["groups"].as_array().into_iter().flatten() {
                for role in group["roles"].as_array().into_iter().flatten() {
                    expected += role["count"].as_i64().unwrap_or(0);
                }
            }
        }
        let slots = data["slots"].as_array().cloned().unwrap_or_default();
        if slots.len() as i64 != expected {
            eprintln!(
                "/slots ORBAT instance count mismatch: orbat expects {expected}, slots has {}",
                slots.len()
            );
            return Ok(1);
        }
        let mut ids = HashSet::new();
        for slot in &slots {
            let id = slot["id"].as_str().unwrap_or_default().to_string();
            if !ids.insert(id.clone()) {
                eprintln!("/slots duplicate slot id '{id}'");
                return Ok(1);
            }
        }
    }
    println!("ok");
    Ok(0)
}

/* ─────────────────────────── map glyphs manifest (GL-G1…G6) ─────────────────────────── */

/// Glyph coverage gate (port of `verify-map-glyphs-manifest.mjs`) — golden + committed-catalog
/// iconKey coverage, SVG existence/viewBox, sane render fields, and the built-atlas rect/RIFF
/// checks when present.
pub fn map_glyphs() -> Result<u8> {
    use std::io::Read as _;
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let glyph_dir = root.join("packages/map-assets/glyphs");
    let manifest = read_json(&glyph_dir.join("manifest.json"))?;
    let glyphs = manifest["glyphs"].as_object().cloned().unwrap_or_default();
    let prefabs = read_json(&sroot.join("golden/map-objects/map-object-prefabs-sample.json"))?;

    let mut errors: Vec<String> = Vec::new();

    // 1. Golden coverage.
    for p in prefabs.as_array().into_iter().flatten() {
        if let Some(key) = p["render"]["iconKey"].as_str() {
            if !glyphs.contains_key(key) {
                errors.push(format!(
                    "prefab {}: render.iconKey '{key}' missing from glyph manifest",
                    p["prefabId"]
                ));
            }
        }
    }

    // 1b. Committed terrain catalogs.
    let catalog = root.join("packages/map-assets/everon/objects/prefabs.json.gz");
    if catalog.exists() {
        let bytes = fs::read(&catalog)?;
        let mut inflated = Vec::new();
        let parsed: Result<Value> = (|| {
            flate2::read::GzDecoder::new(bytes.as_slice()).read_to_end(&mut inflated)?;
            Ok(serde_json::from_slice(&inflated)?)
        })();
        match parsed {
            Ok(doc) => {
                let mut missing: BTreeMap<String, usize> = BTreeMap::new();
                for p in doc["prefabs"].as_array().into_iter().flatten() {
                    if let Some(key) = p["render"]["iconKey"].as_str() {
                        if !glyphs.contains_key(key) {
                            *missing.entry(key.to_string()).or_insert(0) += 1;
                        }
                    }
                }
                for (key, n) in missing {
                    errors.push(format!(
                        "catalog everon: render.iconKey '{key}' ({n} prefabs) missing from glyph manifest"
                    ));
                }
            }
            Err(e) => errors.push(format!("catalog {}: unreadable ({e})", catalog.display())),
        }
    }

    // 2. SVG + render-field sanity.
    for (key, g) in &glyphs {
        let Some(svg_rel) = g["svg"].as_str() else {
            errors.push(format!("glyph '{key}': no svg path"));
            continue;
        };
        let svg_path = glyph_dir.join(svg_rel);
        if !svg_path.exists() {
            errors.push(format!("glyph '{key}': svg file not found ({svg_rel})"));
            continue;
        }
        let svg = fs::read_to_string(&svg_path)?;
        if !svg.contains("viewBox") {
            errors.push(format!("glyph '{key}': {svg_rel} has no viewBox"));
        }
        let has_svg_tag = svg.contains("<svg ") || svg.contains("<svg>") || svg.contains("<svg\n");
        if !has_svg_tag {
            errors.push(format!("glyph '{key}': {svg_rel} is not a valid <svg>"));
        }
        if !g["baseSizePx"].as_f64().map(|v| v > 0.0).unwrap_or(false) {
            errors.push(format!(
                "glyph '{key}': baseSizePx must be > 0 (got {})",
                g["baseSizePx"]
            ));
        }
        let anchor_ok = g["anchor"].as_array().map(|a| {
            a.len() == 2
                && a.iter().all(|v| {
                    v.as_f64()
                        .map(|x| (0.0..=1.0).contains(&x))
                        .unwrap_or(false)
                })
        }) == Some(true);
        if !anchor_ok {
            errors.push(format!(
                "glyph '{key}': anchor must be [x,y] with components in [0,1] (got {})",
                g["anchor"]
            ));
        }
    }

    // 3. Atlas gate (when built).
    let atlas_json = glyph_dir.join(
        manifest["atlas"]["rects"]
            .as_str()
            .unwrap_or("atlas/world-glyphs.json"),
    );
    let atlas_webp = glyph_dir.join(
        manifest["atlas"]["image"]
            .as_str()
            .unwrap_or("atlas/world-glyphs.webp"),
    );
    let atlas_built = atlas_json.exists();
    if atlas_built {
        let atlas = read_json(&atlas_json)?;
        let width = atlas["meta"]["width"].as_i64().unwrap_or(-1);
        let height = atlas["meta"]["height"].as_i64().unwrap_or(-1);
        let is_pow2 = |n: i64| n > 0 && (n & (n - 1)) == 0;
        if !is_pow2(width) || !is_pow2(height) || width > 4096 || height > 4096 {
            errors.push(format!(
                "atlas: dims {width}×{height} not power-of-two ≤ 4096²"
            ));
        }
        for key in glyphs.keys() {
            let r = &atlas["icons"][key];
            if r.is_null() {
                errors.push(format!(
                    "atlas: glyph '{key}' has no rect in world-glyphs.json (rebuild: make map-glyphs-build)"
                ));
                continue;
            }
            let (x, y, w, h) = (
                r["x"].as_f64().unwrap_or(-1.0),
                r["y"].as_f64().unwrap_or(-1.0),
                r["width"].as_f64().unwrap_or(0.0),
                r["height"].as_f64().unwrap_or(0.0),
            );
            if x < 0.0 || y < 0.0 || x + w > width as f64 || y + h > height as f64 {
                errors.push(format!(
                    "atlas: glyph '{key}' rect exceeds {width}×{height} bounds"
                ));
            }
            let (ax, ay) = (
                r["anchorX"].as_f64().unwrap_or(-1.0),
                r["anchorY"].as_f64().unwrap_or(-1.0),
            );
            if !(ax >= 0.0 && ax <= w && ay >= 0.0 && ay <= h) {
                errors.push(format!("atlas: glyph '{key}' anchor outside its rect"));
            }
        }
        if !atlas_webp.exists() {
            errors.push("atlas: world-glyphs.json present but world-glyphs.webp missing".into());
        } else {
            let head = fs::read(&atlas_webp)?;
            if head.len() < 12 || &head[0..4] != b"RIFF" || &head[8..12] != b"WEBP" {
                errors.push("atlas: world-glyphs.webp is not a RIFF/WEBP file".into());
            }
        }
    }

    if errors.is_empty() {
        let atlas_note = if atlas_built {
            ", atlas rects verified"
        } else {
            ", no atlas built"
        };
        println!(
            "verify-map-glyphs: OK ({} glyphs, golden + everon iconKeys covered{atlas_note})",
            glyphs.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-map-glyphs: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}
