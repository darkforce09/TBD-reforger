//! `cargo xtask verify blas-manifest` — T-090.12.2: the prefab BLAS library under
//! `packages/map-assets/everon/prefabs/` is complete and self-consistent.
//!
//! - `blas-manifest.json` validates against `blas-manifest.schema.json` and parses as
//!   `BlasManifest`; its `blas` and `descriptors` lists are sorted (the SPA binary-searches them);
//! - every catalogue pid (`objects/prefabs.json.gz`) has a descriptor entry whose file exists,
//!   validates against `prefab-descriptor.schema.json`, parses, and whose BLAS paths are all in
//!   the manifest and on disk;
//! - every BLAS entry's file exists with exactly the manifest's byte size and parses as a v2
//!   sidecar with the manifest's triangle and kind counts;
//! - the hot set lists blocking pids only, most-placed first;
//! - the farmhouse descriptor's root BLAS is byte-identical to the T-090.11 shell sidecar.
//!
//! Exit 0 = every check passed; 1 = any failure (each printed). No pak access, no Workbench.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use map_engine_core::bvh::BvhSidecar;
use map_engine_core::world::occluder::{BlasManifest, PrefabDescriptor};
use serde_json::Value;

const TERRAIN: &str = "everon";
const FARMHOUSE_SLUG: &str = "FarmHouse_E_1L01_Wood";

fn validator(path: &Path) -> Result<jsonschema::Validator> {
    let schema: Value = serde_json::from_str(
        &fs::read_to_string(path).with_context(|| path.display().to_string())?,
    )?;
    jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
}

fn catalogue_pids(prefabs_gz: &Path) -> Result<Vec<u32>> {
    let bytes = fs::read(prefabs_gz).with_context(|| prefabs_gz.display().to_string())?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice()).read_to_end(&mut out)?;
    let doc: Value = serde_json::from_slice(&out)?;
    let mut pids: Vec<u32> = doc["prefabs"]
        .as_array()
        .context("prefabs.json.gz: no prefabs array")?
        .iter()
        .filter_map(|p| p["prefabId"].as_u64().and_then(|v| u32::try_from(v).ok()))
        .collect();
    pids.sort_unstable();
    Ok(pids)
}

pub fn verify_blas_manifest(root: &Path) -> Result<u8> {
    let assets = root.join("packages/map-assets").join(TERRAIN);
    let prefabs = assets.join("prefabs");
    let schemas = root.join("packages/tbd-schema/schema");
    let mut errs: Vec<String> = Vec::new();

    let manifest_path = prefabs.join("blas-manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "{} (run `cargo xtask map bvh-batch --all-prefabs`)",
            manifest_path.display()
        )
    })?;
    let manifest_value: Value = serde_json::from_str(&manifest_text)?;
    let v_manifest = validator(&schemas.join("blas-manifest.schema.json"))?;
    for e in v_manifest.iter_errors(&manifest_value).take(8) {
        errs.push(format!("blas-manifest.json: {e} @ {}", e.instance_path()));
    }
    let manifest: BlasManifest = serde_json::from_value(manifest_value)?;
    if manifest.terrain_id != TERRAIN {
        errs.push(format!("terrainId {} != {TERRAIN}", manifest.terrain_id));
    }
    if !manifest.blas.windows(2).all(|w| w[0].path < w[1].path) {
        errs.push("blas entries not strictly sorted by path".into());
    }
    if !manifest.descriptors.windows(2).all(|w| w[0].pid < w[1].pid) {
        errs.push("descriptor entries not strictly sorted by pid".into());
    }

    // Every BLAS file: present, exact bytes, parses, counts agree.
    let mut blas_ok: HashSet<&str> = HashSet::new();
    for b in &manifest.blas {
        let path = prefabs.join(&b.path);
        let Ok(bytes) = fs::read(&path) else {
            errs.push(format!("{}: missing", b.path));
            continue;
        };
        if bytes.len() as u64 != b.bytes {
            errs.push(format!(
                "{}: {} bytes on disk, manifest says {}",
                b.path,
                bytes.len(),
                b.bytes
            ));
        }
        match BvhSidecar::parse(&bytes) {
            Ok(sc) => {
                let (o, g, f) = sc.kind_counts();
                if sc.tris.len() as u32 != b.tris || [o as u32, g as u32, f as u32] != b.kinds {
                    errs.push(format!(
                        "{}: {} tris {:?} kinds on disk, manifest says {} / {:?}",
                        b.path,
                        sc.tris.len(),
                        (o, g, f),
                        b.tris,
                        b.kinds
                    ));
                }
                blas_ok.insert(b.path.as_str());
            }
            Err(e) => errs.push(format!("{}: does not parse ({e})", b.path)),
        }
    }

    // Every catalogue pid: a descriptor that validates, parses, and references known BLAS.
    let pids = catalogue_pids(&assets.join("objects/prefabs.json.gz"))?;
    let v_desc = validator(&schemas.join("prefab-descriptor.schema.json"))?;
    let entries: HashMap<u32, _> = manifest.descriptors.iter().map(|d| (d.pid, d)).collect();
    let mut descriptors: HashMap<u32, PrefabDescriptor> = HashMap::new();
    let mut blocks_by_pid: HashMap<u32, bool> = HashMap::new();
    for pid in &pids {
        let Some(entry) = entries.get(pid) else {
            errs.push(format!("pid {pid}: no descriptor entry in the manifest"));
            continue;
        };
        let path = prefabs.join(&entry.path);
        let Ok(text) = fs::read_to_string(&path) else {
            errs.push(format!("pid {pid}: {} missing", entry.path));
            continue;
        };
        let value: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                errs.push(format!("pid {pid}: {} is not JSON ({e})", entry.path));
                continue;
            }
        };
        if let Some(e) = v_desc.iter_errors(&value).next() {
            errs.push(format!(
                "pid {pid}: {} schema: {e} @ {}",
                entry.path,
                e.instance_path()
            ));
        }
        let d: PrefabDescriptor = match serde_json::from_value(value) {
            Ok(d) => d,
            Err(e) => {
                errs.push(format!("pid {pid}: {} does not parse ({e})", entry.path));
                continue;
            }
        };
        if d.prefab_id != *pid || d.blocks != entry.blocks || d.kind != entry.kind {
            errs.push(format!(
                "pid {pid}: descriptor / manifest entry disagree (pid, blocks or kind)"
            ));
        }
        if d.blocks == d.instances.is_empty() {
            errs.push(format!(
                "pid {pid}: blocks {} with {} instances",
                d.blocks,
                d.instances.len()
            ));
        }
        if !d.blocks && d.reason.is_none() {
            errs.push(format!("pid {pid}: blocks:false without a reason"));
        }
        let paths: Vec<String> = d.blas_paths().iter().map(ToString::to_string).collect();
        if paths != entry.blas {
            errs.push(format!(
                "pid {pid}: descriptor BLAS {paths:?} != manifest {:?}",
                entry.blas
            ));
        }
        for p in &paths {
            if !blas_ok.contains(p.as_str()) {
                errs.push(format!(
                    "pid {pid}: references {p}, not a verified manifest BLAS"
                ));
            }
        }
        blocks_by_pid.insert(*pid, d.blocks);
        descriptors.insert(*pid, d);
    }
    for d in &manifest.descriptors {
        if pids.binary_search(&d.pid).is_err() {
            errs.push(format!(
                "manifest lists pid {} which the catalogue does not have",
                d.pid
            ));
        }
    }

    // Hot set: blocking pids, most-placed first.
    let mut prev: Option<u64> = None;
    for pid in &manifest.hot {
        match entries.get(pid) {
            Some(e) if e.blocks => {
                if let Some(p) = prev {
                    if e.instances_in_world > p {
                        errs.push(format!("hot set not ordered by placements at pid {pid}"));
                    }
                }
                prev = Some(e.instances_in_world);
            }
            Some(_) => errs.push(format!("hot set lists non-blocking pid {pid}")),
            None => errs.push(format!("hot set lists unknown pid {pid}")),
        }
    }

    // The T-090.11 pin: the farmhouse root BLAS is the shell sidecar, byte for byte.
    match descriptors.values().find(|d| d.slug == FARMHOUSE_SLUG) {
        Some(d) => {
            let shell = prefabs.join(&d.shell_bvh);
            let committed = prefabs
                .join("buildings")
                .join(format!("{FARMHOUSE_SLUG}.bvh"));
            match (fs::read(&shell), fs::read(&committed)) {
                (Ok(a), Ok(b)) if a == b => {}
                (Ok(_), Ok(_)) => errs.push(format!(
                    "{FARMHOUSE_SLUG}: root BLAS differs from the committed shell sidecar"
                )),
                _ => errs.push(format!("{FARMHOUSE_SLUG}: shell sidecar unreadable")),
            }
        }
        None => errs.push(format!("{FARMHOUSE_SLUG}: no descriptor")),
    }

    let blocks = blocks_by_pid.values().filter(|b| **b).count();
    println!(
        "verify-blas-manifest: {} catalogue pids · {} descriptors ({blocks} block) · {} BLAS files {:.1} MB · hot {} · {} error(s)",
        pids.len(),
        descriptors.len(),
        manifest.blas.len(),
        manifest.totals.blas_bytes as f64 / 1_048_576.0,
        manifest.hot.len(),
        errs.len()
    );
    for e in errs.iter().take(40) {
        println!("  FAIL {e}");
    }
    if errs.len() > 40 {
        println!("  … {} more", errs.len() - 40);
    }
    Ok(u8::from(!errs.is_empty()))
}
