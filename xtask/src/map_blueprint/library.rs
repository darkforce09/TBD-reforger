//! `cargo xtask map bvh-batch --all-prefabs` — T-090.12.2: the prefab BLAS library.
//!
//! Walks EVERY prefab of `objects/prefabs.json.gz` through the T-090.11.2 [`Walker`] straight out
//! of the game paks and emits, under `packages/map-assets/<terrain>/prefabs/`:
//!
//! - `blas/<stem>.bvh` — one sidecar per distinct XOB (dedup by stem via the walker's asset
//!   cache), shared across every prefab that uses the model;
//! - `descriptors/<pid>.json` — one [`PrefabDescriptor`] per catalogue prefab: the root mesh
//!   as an instance record at identity (kind `Shell` for buildings, the walker's own kind for
//!   everything else) plus every collision-bearing child (doors, frames, panes, furniture), or
//!   `blocks: false` + a reason when nothing in the closure collides;
//! - `blas-manifest.json` — the [`BlasManifest`]: every BLAS with its bytes / tris / kinds, every
//!   descriptor, the per-kind census and the `hot` prefetch set (the most-placed blocking pids).
//!
//! Trees: the trunk and the foliage colliders come from the COLL records exactly as `bvh-batch`
//! reads them (`kind_for_gamemat` / `kind_for_layer`). A tree whose COLL carries no Foliage
//! triangle gets a canopy from the convex hull of its visual LOD0 (`hull_triangles`, the T-090.11
//! `TreeCanopy` fallback) as an all-Foliage sidecar `blas/<stem>_canopy.bvh`; the census says how
//! many needed it.
//!
//! Deterministic by construction: descriptors and the manifest are sorted, timestamp-free,
//! pretty-printed with a trailing newline, and written through `write_if_changed`, so a re-emit
//! that changes nothing writes nothing.
//!
//! Usage: `--all-prefabs [--terrain everon] [--only-kind tree]… [--limit N] [--hot 100] [--dry-run]
//!         [--paks <dir>] [--extract <dir>] [--out <dir>]`
//! A filtered run (`--only-kind` / `--limit`) writes descriptors + BLAS only — never a partial
//! manifest over a full one.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use map_engine_core::building_compound::{
    CoverTier, InstanceKind, InstanceRecord, LocalTransform, PlacementSource,
};
use map_engine_core::bvh::{Bvh, BvhSidecar, SurfaceKind, emit_bytes, lift_verts, quantize_verts};
use map_engine_core::geometry::rigid::Rigid;
use map_engine_core::world::occluder::{
    BlasEntry, BlasManifest, Bounds3, DESCRIPTOR_SCHEMA_VERSION, DescEntry,
    MANIFEST_SCHEMA_VERSION, PrefabDescriptor, Totals,
};
use serde_json::Value;

use super::batch::{Asset, Walker, classify_prefab, cover_for_prefab, slug_of, write_if_changed};
use super::hull::hull_triangles;
use super::pak::AssetSource;
use super::prefab::strip_guid;
use super::world_row::load_rows;
use super::xob;

/// Default size of the `hot` prefetch set.
pub const DEFAULT_HOT: usize = 100;

/// One catalogue row of `objects/prefabs.json.gz`.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefabRow {
    pub pid: u32,
    /// As the catalogue carries it (`{GUID}Prefabs/…/X.et`).
    pub resource_name: String,
    pub kind: String,
}

fn gunzip_json(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).with_context(|| path.display().to_string())?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut out)
        .with_context(|| format!("gunzip {}", path.display()))?;
    serde_json::from_slice(&out).with_context(|| format!("parse {}", path.display()))
}

/// The catalogue rows, sorted by pid.
pub fn load_prefab_rows(prefabs_gz: &Path) -> Result<Vec<PrefabRow>> {
    let doc = gunzip_json(prefabs_gz)?;
    let rows = doc["prefabs"]
        .as_array()
        .context("prefabs.json.gz: no prefabs array")?;
    let mut out = Vec::with_capacity(rows.len());
    for p in rows {
        let (Some(pid), Some(rn), Some(kind)) = (
            p["prefabId"].as_u64(),
            p["resourceName"].as_str(),
            p["kind"].as_str(),
        ) else {
            continue;
        };
        out.push(PrefabRow {
            pid: u32::try_from(pid).context("prefabId exceeds u32")?,
            resource_name: rn.to_string(),
            kind: kind.to_string(),
        });
    }
    if out.is_empty() {
        bail!("prefabs.json.gz: no prefab rows");
    }
    out.sort_by_key(|r| r.pid);
    Ok(out)
}

/// How many chunk rows place each pid (`objects/chunks/*.json.gz`).
pub fn world_census(chunks_dir: &Path) -> Result<HashMap<u32, u64>> {
    let mut files: Vec<PathBuf> = fs::read_dir(chunks_dir)
        .with_context(|| chunks_dir.display().to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".json.gz"))
        .collect();
    files.sort();
    let mut census = HashMap::new();
    for f in &files {
        for row in load_rows(f)? {
            *census
                .entry(u32::try_from(row.pid).unwrap_or(u32::MAX))
                .or_insert(0) += 1;
        }
    }
    Ok(census)
}

#[derive(Clone, Debug)]
pub struct LibraryOptions {
    pub terrain: String,
    pub only_kinds: Vec<String>,
    pub limit: Option<usize>,
    pub hot: usize,
}

/// The emitted library, in memory.
pub struct Library {
    pub descriptors: Vec<PrefabDescriptor>,
    /// `blas/<stem>.bvh` → sidecar bytes.
    pub blas: BTreeMap<String, Vec<u8>>,
    pub manifest: BlasManifest,
}

/// The 26 axis / face-diagonal / corner directions the canopy hull samples.
fn hull_directions() -> Vec<[f64; 3]> {
    let mut dirs = Vec::with_capacity(26);
    for x in -1..=1_i32 {
        for y in -1..=1_i32 {
            for z in -1..=1_i32 {
                if (x, y, z) == (0, 0, 0) {
                    continue;
                }
                let v = [f64::from(x), f64::from(y), f64::from(z)];
                let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                dirs.push([v[0] / n, v[1] / n, v[2] / n]);
            }
        }
    }
    dirs
}

/// The extreme vertex of `verts` along each of 26 directions (a k-DOP sample of the convex
/// hull, ≤ 26 points). `hull_triangles` is O(n⁴) — meant for colliders of a few dozen
/// vertices — so a visual LOD0 (thousands of vertices) is never fed to it directly.
#[must_use]
pub fn hull_sample(verts: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mut out: Vec<[f64; 3]> = Vec::with_capacity(26);
    for d in hull_directions() {
        let mut best: Option<(f64, [f64; 3])> = None;
        for v in verts {
            let s = v[0] * d[0] + v[1] * d[1] + v[2] * d[2];
            if best.is_none_or(|(bs, _)| s > bs) {
                best = Some((s, *v));
            }
        }
        if let Some((_, p)) = best {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// An all-Foliage sidecar over the convex hull of the 26-direction extreme points of `verts`
/// (the visual LOD0 canopy fallback).
fn canopy_sidecar(all_verts: &[[f64; 3]]) -> Option<Vec<u8>> {
    let verts = hull_sample(all_verts);
    let tris = hull_triangles(&verts);
    if tris.is_empty() {
        return None;
    }
    let q = quantize_verts(&verts);
    if q.iter().flatten().any(|c| !c.is_finite()) {
        return None;
    }
    let lifted = lift_verts(&q);
    let bvh = Bvh::build(&lifted, &tris);
    let kinds = vec![SurfaceKind::Foliage; tris.len()];
    let bytes = emit_bytes(&q, &tris, &kinds, &bvh);
    BvhSidecar::parse(&bytes).ok().map(|_| bytes)
}

/// Why a closure with no collision-bearing instance blocks nothing.
fn no_block_reason(walker: &mut Walker, path: &str, root: Option<&Asset>) -> &'static str {
    let mesh = walker
        .resolver
        .resolve(path)
        .ok()
        .and_then(|p| p.mesh.clone());
    match (mesh, root) {
        (None, _) => "no-mesh",
        (Some(_), None) => "model-unreadable",
        (Some(_), Some(a)) => match &a.coll {
            Some(m) if m.tris.is_empty() => "empty-coll",
            _ => "no-coll",
        },
    }
}

/// Walk every selected catalogue prefab and assemble descriptors, BLAS bytes and the manifest.
pub fn build_library(
    source: &dyn AssetSource,
    rows: &[PrefabRow],
    census: &HashMap<u32, u64>,
    opts: &LibraryOptions,
) -> Result<Library> {
    let mut walker = Walker::new(source);
    let mut descriptors: Vec<PrefabDescriptor> = Vec::new();
    let mut blas: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut canopy_hull = 0u32;
    let selected = rows
        .iter()
        .filter(|r| opts.only_kinds.is_empty() || opts.only_kinds.iter().any(|k| k == &r.kind))
        .take(opts.limit.unwrap_or(usize::MAX));
    let started = std::time::Instant::now();
    for (done, row) in selected.enumerate() {
        if done > 0 && done % 100 == 0 {
            eprintln!(
                "  … {done} prefabs walked, {} BLAS, {:.0} s",
                blas.len(),
                started.elapsed().as_secs_f64()
            );
        }
        let path = strip_guid(&row.resource_name).to_string();
        let slug = slug_of(&path);
        walker.instances.clear();
        walker.notes.clear();
        // Walked exactly as `bvh-batch` walks a building (the root as the shell), so the child
        // ids and parents are the T-090.11 instances file's; the root record is synthesised.
        let walked = walker.walk(
            &path,
            &slug,
            None,
            Rigid::identity(),
            PlacementSource::PrefabCoords,
            true,
            0,
        );
        let mut instances = std::mem::take(&mut walker.instances);
        let mut notes = std::mem::take(&mut walker.notes);
        let mut reason: Option<String> = None;
        let mut root_asset = None;
        match walked {
            Err(e) => {
                reason = Some("unresolved".into());
                notes.push(format!("{slug}: {path} unresolved ({e:#})"));
            }
            Ok(asset) => {
                root_asset = asset;
                // The root record: a building's shell reads as `Shell`; every other kind keeps
                // the walker's classification (tree / prop / furniture / glass) so a hit names
                // what it is.
                if let Some(a) = root_asset.as_ref().filter(|a| a.has_collision()) {
                    let kind = if row.kind == "building" {
                        InstanceKind::Shell
                    } else {
                        walker
                            .resolver
                            .resolve(&path)
                            .map_or(InstanceKind::Prop, |p| classify_prefab(&p, a))
                    };
                    let cover = match kind {
                        InstanceKind::Furniture | InstanceKind::Prop => cover_for_prefab(&path).1,
                        InstanceKind::Tree => CoverTier::Full,
                        _ => CoverTier::None,
                    };
                    instances.insert(
                        0,
                        InstanceRecord {
                            id: slug.clone(),
                            kind,
                            prefab: path.clone(),
                            blas: format!("blas/{}.bvh", a.stem),
                            xob: Some(a.path.clone()),
                            local: LocalTransform::identity(),
                            door: None,
                            cover,
                            source: PlacementSource::PrefabCoords,
                            parent: None,
                        },
                    );
                }
                if instances.is_empty() {
                    reason =
                        Some(no_block_reason(&mut walker, &path, root_asset.as_deref()).into());
                }
            }
        }
        let shell_bvh = instances
            .iter()
            .find(|i| i.id == slug && i.parent.is_none())
            .map(|i| i.blas.clone())
            .unwrap_or_default();
        // Canopy: Foliage triangles in the COLL, else the visual-LOD0 hull.
        let mut canopy = false;
        let mut canopy_bounds: Option<Bounds3> = None;
        if row.kind == "tree" {
            let foliage = root_asset.as_ref().map_or(0, |a| a.kind_counts().2);
            if foliage > 0 {
                canopy = true;
            } else if let Some(mesh) = walker
                .resolver
                .resolve(&path)
                .ok()
                .and_then(|p| p.mesh.clone())
            {
                match source.read(&mesh).and_then(|d| xob::parse_xob(&d, None)) {
                    Ok(vis) => match canopy_sidecar(&vis.verts) {
                        Some(bytes) => {
                            let stem = root_asset
                                .as_ref()
                                .map_or_else(|| slug_of(&mesh), |a| a.stem.clone());
                            let rel = format!("blas/{stem}_canopy.bvh");
                            let (lo, hi) = xob::aabb(&vis.verts);
                            canopy_bounds = Some(Bounds3 { min: lo, max: hi });
                            blas.insert(rel.clone(), bytes);
                            instances.push(InstanceRecord {
                                id: format!("{slug}/canopy"),
                                kind: InstanceKind::TreeCanopy,
                                prefab: path.clone(),
                                blas: rel,
                                xob: Some(mesh.clone()),
                                local: LocalTransform::identity(),
                                door: None,
                                cover: CoverTier::None,
                                source: PlacementSource::PrefabCoords,
                                parent: Some(slug.clone()),
                            });
                            canopy = true;
                            canopy_hull += 1;
                            reason = None;
                            notes.push(format!(
                                "{slug}/canopy: convex hull of the visual LOD0 ({} verts) tagged Foliage — the COLL carries no Foliage triangle",
                                vis.verts.len()
                            ));
                        }
                        None => notes.push(format!(
                            "{slug}: visual LOD0 hull degenerate — no canopy (COLL carries no Foliage triangle)"
                        )),
                    },
                    Err(e) => notes.push(format!(
                        "{slug}: visual LOD0 unreadable for the canopy hull ({e:#}) — no canopy"
                    )),
                }
            }
        }
        let blocks = !instances.is_empty();
        // Placed bounds, object frame.
        let mut bounds: Option<Bounds3> = None;
        for inst in &instances {
            let b = if inst.kind == InstanceKind::TreeCanopy {
                canopy_bounds
            } else {
                inst.xob
                    .as_deref()
                    .and_then(|x| walker.assets.load(x).ok())
                    .and_then(|a| a.bounds())
                    .map(|(min, max)| Bounds3 { min, max })
            };
            if let Some(b) = b {
                let (min, max) = inst.local.rigid().aabb_of(b.min, b.max);
                let placed = Bounds3 { min, max };
                bounds = Some(bounds.map_or(placed, |u| u.union(placed)));
            }
        }
        // Sidecar bytes for every referenced BLAS (canopies were inserted above).
        for inst in &instances {
            if blas.contains_key(&inst.blas) {
                continue;
            }
            if let Some(a) = inst.xob.as_deref().and_then(|x| walker.assets.load(x).ok()) {
                if let Some(bytes) = &a.sidecar_bytes {
                    blas.insert(inst.blas.clone(), bytes.clone());
                }
            }
        }
        descriptors.push(PrefabDescriptor {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
            prefab_id: row.pid,
            slug,
            resource_name: path,
            kind: row.kind.clone(),
            blocks,
            reason: if blocks { None } else { reason },
            canopy,
            local_bounds: bounds,
            shell_bvh,
            instances,
            notes,
        });
    }
    let manifest = assemble_manifest(
        &opts.terrain,
        &descriptors,
        &blas,
        census,
        opts.hot,
        canopy_hull,
    )?;
    Ok(Library {
        descriptors,
        blas,
        manifest,
    })
}

fn assemble_manifest(
    terrain: &str,
    descriptors: &[PrefabDescriptor],
    blas: &BTreeMap<String, Vec<u8>>,
    census: &HashMap<u32, u64>,
    hot_n: usize,
    canopy_hull: u32,
) -> Result<BlasManifest> {
    let mut blas_entries = Vec::with_capacity(blas.len());
    for (path, bytes) in blas {
        let sc = BvhSidecar::parse(bytes).with_context(|| format!("parse {path}"))?;
        let (o, g, f) = sc.kind_counts();
        blas_entries.push(BlasEntry {
            path: path.clone(),
            bytes: bytes.len() as u64,
            tris: u32::try_from(sc.tris.len()).unwrap_or(u32::MAX),
            kinds: [o as u32, g as u32, f as u32],
        });
    }
    let mut totals = Totals::default();
    let mut desc_entries = Vec::with_capacity(descriptors.len());
    let mut attributed: HashSet<&str> = HashSet::new();
    for d in descriptors {
        let kt = totals.by_kind.entry(d.kind.clone()).or_default();
        kt.prefabs += 1;
        totals.prefabs += 1;
        if d.blocks {
            kt.blocks += 1;
            totals.blocks += 1;
        } else {
            match d.reason.as_deref() {
                Some("no-mesh") => kt.no_mesh += 1,
                Some("model-unreadable") => kt.model_unreadable += 1,
                Some("no-coll") => kt.no_coll += 1,
                Some("empty-coll") => kt.empty_coll += 1,
                _ => kt.unresolved += 1,
            }
        }
        if d.canopy {
            totals.canopy += 1;
        }
        let paths: Vec<String> = d.blas_paths().iter().map(ToString::to_string).collect();
        for p in &d.blas_paths() {
            if attributed.insert(p) {
                kt.bytes += blas.get(*p).map_or(0, |b| b.len() as u64);
            }
        }
        desc_entries.push(DescEntry {
            pid: d.prefab_id,
            path: format!("descriptors/{}.json", d.prefab_id),
            kind: d.kind.clone(),
            blocks: d.blocks,
            canopy: d.canopy,
            blas: paths,
            instance_count: u32::try_from(d.instances.len()).unwrap_or(u32::MAX),
            instances_in_world: census.get(&d.prefab_id).copied().unwrap_or(0),
        });
    }
    totals.canopy_hull = canopy_hull;
    totals.blas_files = u32::try_from(blas.len()).unwrap_or(u32::MAX);
    totals.blas_bytes = blas.values().map(|b| b.len() as u64).sum();
    let mut hot: Vec<(u64, u32)> = desc_entries
        .iter()
        .filter(|d| d.blocks)
        .map(|d| (d.instances_in_world, d.pid))
        .collect();
    hot.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let hot = hot.into_iter().take(hot_n).map(|(_, p)| p).collect();
    Ok(BlasManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.into(),
        terrain_id: terrain.to_string(),
        blas: blas_entries,
        descriptors: desc_entries,
        hot,
        totals,
    })
}

/// Validate `value` against the JSON schema at `schema_path`.
pub fn validate_against(value: &Value, schema_path: &Path, label: &str) -> Result<()> {
    let schema: Value = serde_json::from_str(
        &fs::read_to_string(schema_path).with_context(|| schema_path.display().to_string())?,
    )?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| format!("{} @ {}", e, e.instance_path()))
        .take(8)
        .collect();
    if !errors.is_empty() {
        bail!(
            "{label} fails {}:\n  {}",
            schema_path.display(),
            errors.join("\n  ")
        );
    }
    Ok(())
}

fn pretty_nl(v: &Value) -> Vec<u8> {
    let mut s = serde_json::to_string_pretty(v).expect("serialize");
    s.push('\n');
    s.into_bytes()
}

/// Validate every document against the schemas and write the library under `out_dir`
/// (descriptors, BLAS, and the manifest unless `partial`). Returns the number of files written.
pub fn write_library(
    out_dir: &Path,
    lib: &Library,
    schema_dir: &Path,
    partial: bool,
) -> Result<usize> {
    let desc_schema = schema_dir.join("prefab-descriptor.schema.json");
    let manifest_schema = schema_dir.join("blas-manifest.schema.json");
    let mut written = 0usize;
    for d in &lib.descriptors {
        let v = serde_json::to_value(d)?;
        validate_against(&v, &desc_schema, &format!("descriptor {}", d.prefab_id))?;
        let path = out_dir
            .join("descriptors")
            .join(format!("{}.json", d.prefab_id));
        written += usize::from(write_if_changed(&path, &pretty_nl(&v))?);
    }
    for (rel, bytes) in &lib.blas {
        written += usize::from(write_if_changed(&out_dir.join(rel), bytes)?);
    }
    if !partial {
        let v = serde_json::to_value(&lib.manifest)?;
        validate_against(&v, &manifest_schema, "blas-manifest")?;
        written += usize::from(write_if_changed(
            &out_dir.join("blas-manifest.json"),
            &pretty_nl(&v),
        )?);
    }
    Ok(written)
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod tests;
