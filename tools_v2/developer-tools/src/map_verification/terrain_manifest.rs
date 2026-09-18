//! Terrain manifest validation against schema, spatial dimensions, and binary contracts.
#[cfg(test)]
use crate::repository_paths::find_repo_root as repo_root;
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use website_map_engine::io::containers::header::CONTAINER_VERSION;
use website_map_engine::io::pod::instance::POD_BYTES;
use website_map_engine::io::pod::instance::POD_NAME;
use website_map_engine::streaming::loaders::chunk_bin::chunk_bin_path;
use website_map_engine::streaming::loaders::manifest::parse_manifest_binary;
fn read_json(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}
fn schema_root(root: &Path) -> PathBuf {
    root.join("packages/tbd-schema")
}
/* ─────────────────────────── terrain manifest ─────────────────────────── */

struct TerrainContract {
    width: f64,
    height: f64,
    min_m: f64,
    max_m: f64,
}

/* ───────────────── T-935.12 — the manifest's binary blocks (spec §5) ───────────────── */

/// `map-object-instance.schema.json` `$defs/objectInstancePodRow` against the Rust POD.
///
/// The layout block is documentation, because JSON Schema cannot describe bytes — and a wire
/// contract nobody checks is how a format rots. So the arithmetic is checked here: it must cover
/// exactly [`POD_BYTES`] bytes with no gap and no overlap, in ascending offset, with each field's
/// width matching its declared type. Offsets are proved against real bytes elsewhere: `golden_gate`
/// S15 decodes the committed golden `.bin` at these very offsets and compares to the JSON decode.
fn pod_row_doc_failures(instance_schema: &Value) -> Vec<String> {
    let doc = &instance_schema["$defs"]["objectInstancePodRow"];
    let mut errs = Vec::new();
    if doc.is_null() {
        return vec![
            "map-object-instance.schema.json: $defs/objectInstancePodRow is missing — the \
                     32-byte binary row must stay documented beside the JSON rows it encodes"
                .to_string(),
        ];
    }
    if doc["podBytes"].as_u64() != Some(POD_BYTES as u64) {
        errs.push(format!(
            "objectInstancePodRow.podBytes {} != POD_BYTES {POD_BYTES}",
            doc["podBytes"]
        ));
    }
    // The seven f32s then u16/u8/u8, spec §2. This order is what makes the struct padding-free, so
    // it is pinned by name here and by fixed byte offset in golden_gate S15.
    let expect: [(&str, &str); 10] = [
        ("x", "f32"),
        ("y", "f32"),
        ("z", "f32"),
        ("yaw", "f32"),
        ("pitch", "f32"),
        ("roll", "f32"),
        ("scale", "f32"),
        ("prefab_id", "u16"),
        ("class_code", "u8"),
        ("_pad", "u8"),
    ];
    let fields = doc["fields"].as_array().cloned().unwrap_or_default();
    if fields.len() != expect.len() {
        errs.push(format!(
            "objectInstancePodRow has {} fields, want {}",
            fields.len(),
            expect.len()
        ));
        return errs;
    }
    let mut at = 0_u64;
    for (i, f) in fields.iter().enumerate() {
        let (want_name, want_ty) = expect[i];
        let (name, ty) = (
            f["name"].as_str().unwrap_or(""),
            f["type"].as_str().unwrap_or(""),
        );
        let (off, bytes) = (f["offset"].as_u64(), f["bytes"].as_u64());
        if name != want_name || ty != want_ty {
            errs.push(format!(
                "field {i} is {name}:{ty}, want {want_name}:{want_ty}"
            ));
        }
        let want_bytes = match want_ty {
            "f32" => 4,
            "u16" => 2,
            _ => 1,
        };
        if bytes != Some(want_bytes) {
            errs.push(format!(
                "field {want_name} declares {bytes:?} bytes, want {want_bytes}"
            ));
        }
        if off != Some(at) {
            errs.push(format!(
                "field {want_name} declares offset {off:?}, want {at}"
            ));
        }
        at += want_bytes;
    }
    if at != POD_BYTES as u64 {
        errs.push(format!(
            "documented row covers {at} bytes, want {POD_BYTES}"
        ));
    }
    errs
}

/// Does every path a binary block names exist under the terrain's asset directory, and does the
/// chunk container it declares match the row shape THIS build implements?
///
/// Both failures are invisible at runtime, which is why they are gated here. A **dangling** path
/// costs nothing loud: the loader's fallback for a missing binary is the JSON path that still
/// works, so a manifest naming an archive nobody emitted just silently gives up the whole point of
/// the migration. A **shape** disagreement is worse — `pod`/`podBytes`/`containerVersion` exist on
/// the wire precisely so a loader can refuse a 24-byte-row file rather than read it at a 32-byte
/// stride and draw a map made of garbage that never once errors
/// ([`ObjectsBinaryBlock::matches_this_build`] is the loader's own predicate, reused here).
///
/// Existence only, never content: in a slice worktree these files are git-LFS pointers.
/// Is `kind` (e.g. `objects.binary.prefabs`) actually PRESENT in the manifest, as opposed to absent?
///
/// T-946.24 — the difference between "not claimed" and "claimed as nothing".
fn manifest_names_key(manifest: &Value, kind: &str) -> bool {
    let mut cur = manifest;
    for seg in kind.split('.') {
        match cur.get(seg) {
            Some(v) => cur = v,
            None => return false,
        }
    }
    true
}

fn manifest_binary_failures(manifest: &Value, asset_dir: &Path) -> (usize, Vec<String>) {
    let bin = parse_manifest_binary(manifest);
    let mut errs = Vec::new();
    let mut declared = 0_usize;
    let want = |kind: &str, rel: &str, dir: bool, errs: &mut Vec<String>| {
        // T-946.24 — an ABSENT key and a key set to "" are different statements. Absent means the
        // manifest does not claim this artefact exists; empty means it claims one and names
        // nothing, which resolves to the asset directory itself and used to pass both this gate and
        // the schema. Found by the wave-242 verifier.
        if rel.is_empty() {
            if manifest_names_key(manifest, kind) {
                errs.push(format!(
                    "{kind} is present but empty — name the path or drop the key; an empty path                      resolves to the asset directory and claims an artefact that is not there"
                ));
            }
            return;
        }
        let p = asset_dir.join(rel);
        let ok = if dir { p.is_dir() } else { p.is_file() };
        if !ok {
            let what = if dir { "directory" } else { "file" };
            errs.push(format!("{kind} names {rel}, but no such {what} exists"));
        }
    };
    if let Some(o) = &bin.objects {
        declared += 1;
        if !o.matches_this_build() {
            errs.push(format!(
                "objects.binary declares {}/v{} rows of {} x {} B; this build reads TBDC/v{} {POD_NAME} x {POD_BYTES} B",
                o.container, o.container_version, o.pod, o.pod_bytes, CONTAINER_VERSION
            ));
        }
        // The chunk path is a TEMPLATE. `chunk_bin_path` is the loader's own filler: it returns
        // None unless BOTH placeholders are present, because a template missing one resolves every
        // chunk in the world to a single URL and the map fills with copies of one tile.
        match chunk_bin_path(&o.chunks, "0_0") {
            None => errs.push(format!(
                "objects.binary.chunks '{}' does not carry both {{cx}} and {{cy}}",
                o.chunks
            )),
            Some(filled) => {
                let dir = Path::new(&filled)
                    .parent()
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let abs = asset_dir.join(&dir);
                let any_bin = fs::read_dir(&abs).is_ok_and(|rd| {
                    rd.filter_map(std::result::Result::ok)
                        .any(|e| e.path().extension().is_some_and(|x| x == "bin"))
                });
                if !any_bin {
                    errs.push(format!(
                        "objects.binary.chunks resolves to {}, which holds no .bin",
                        dir.display()
                    ));
                }
            }
        }
        want("objects.binary.prefabs", &o.prefabs, false, &mut errs);
        want("objects.binary.roads", &o.roads, false, &mut errs);
        want("objects.binary.regions", &o.regions, false, &mut errs);
        want(
            "objects.binary.typeInventory",
            &o.type_inventory,
            false,
            &mut errs,
        );
    }
    if let Some(d) = &bin.dem_raw {
        declared += 1;
        want("dem.raw.path", &d.path, false, &mut errs);
    }
    if let Some(l) = &bin.labels {
        declared += 1;
        want("labels.path", &l.path, false, &mut errs);
    }
    if let Some(w) = &bin.water {
        declared += 1;
        want("water.vectors", &w.vectors, false, &mut errs);
        want("water.bathymetry", &w.bathymetry, false, &mut errs);
    }
    if let Some(b) = &bin.buildings {
        declared += 1;
        want("buildings.archive", &b.archive, false, &mut errs);
        want("buildings.blas", &b.blas, true, &mut errs);
    }
    (declared, errs)
}

pub fn terrain_manifest(root: &Path, terrain: &str) -> Result<u8> {
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
    let manifest_path = root.join(format!("packages/map-assets/{terrain}/manifest.json"));
    let manifest = match read_json(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("FAIL  Cannot read manifest: {}", manifest_path.display());
            eprintln!("{e}");
            return Ok(1);
        }
    };

    let schema = read_json(&schema_root(root).join("schema/terrain-manifest.schema.json"))?;
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

    // T-935.12. Runs on every manifest, with or without binary blocks — a manifest that declares
    // none is the shipped state and says so out loud, because "PASS" over zero examined blocks is
    // this program's signature defect. The POD row doc is checked unconditionally: it describes the
    // format whether or not this terrain has migrated yet.
    let instance_schema =
        read_json(&schema_root(root).join("schema/map-object-instance.schema.json"))?;
    let mut bin_errors = pod_row_doc_failures(&instance_schema);
    let (declared, path_errors) = manifest_binary_failures(
        &manifest,
        &root.join(format!("packages/map-assets/{terrain}")),
    );
    bin_errors.extend(path_errors);
    if !bin_errors.is_empty() {
        eprintln!("FAIL  T-935 binary blocks (spec §5):");
        for e in &bin_errors {
            eprintln!("      {e}");
        }
        return Ok(1);
    }
    if declared == 0 {
        println!(
            "PASS  ObjectInstancePod row doc; {terrain} declares no T-935 binary block (JSON paths)"
        );
    } else {
        println!(
            "PASS  ObjectInstancePod row doc + {declared} T-935 binary block(s), every path resolved"
        );
    }

    println!("\nverify-terrain-manifest: OK");
    Ok(0)
}

#[cfg(test)]
#[path = "tests/terrain_manifest.rs"]
mod tests;
