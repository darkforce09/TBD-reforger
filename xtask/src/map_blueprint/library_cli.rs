//! The CLI arm of `cargo xtask map bvh-batch --all-prefabs` (T-090.12.2): argument parsing, the
//! census report and the write. The library itself is [`super::library`].

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use map_engine_core::world::occluder::BlasEntry;

use super::batch::open_sources;
use super::library::{
    DEFAULT_HOT, Library, LibraryOptions, build_library, load_prefab_rows, world_census,
    write_library,
};

pub(super) fn print_report(lib: &Library, census: &HashMap<u32, u64>, terrain: &str) {
    let t = &lib.manifest.totals;
    let world_total: u64 = census.values().sum();
    let hot_cover: u64 = lib
        .manifest
        .hot
        .iter()
        .map(|p| census.get(p).copied().unwrap_or(0))
        .sum();
    println!(
        "bvh-batch --all-prefabs {terrain}: {} prefabs · {} block · {} blocks:false · canopy {} (hull {}) · {} BLAS files {:.1} MB · hot {} pids cover {:.1} % of {} placed instances",
        t.prefabs,
        t.blocks,
        t.prefabs - t.blocks,
        t.canopy,
        t.canopy_hull,
        t.blas_files,
        t.blas_bytes as f64 / 1_048_576.0,
        lib.manifest.hot.len(),
        if world_total == 0 {
            0.0
        } else {
            hot_cover as f64 * 100.0 / world_total as f64
        },
        world_total
    );
    println!(
        "  {:<10} {:>7} {:>6} {:>7} {:>10} {:>7} {:>6} {:>10} {:>10}",
        "kind",
        "prefabs",
        "block",
        "no-mesh",
        "unreadable",
        "no-coll",
        "empty",
        "unresolved",
        "bytes"
    );
    for (k, kt) in &t.by_kind {
        println!(
            "  {:<10} {:>7} {:>6} {:>7} {:>10} {:>7} {:>6} {:>10} {:>10}",
            k,
            kt.prefabs,
            kt.blocks,
            kt.no_mesh,
            kt.model_unreadable,
            kt.no_coll,
            kt.empty_coll,
            kt.unresolved,
            kt.bytes
        );
    }
    let mut largest: Vec<&BlasEntry> = lib.manifest.blas.iter().collect();
    largest.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.path.cmp(&b.path)));
    println!("  20 largest BLAS:");
    for b in largest.iter().take(20) {
        println!(
            "    {:>9} B  {:>7} tris  [o {} g {} f {}]  {}",
            b.bytes, b.tris, b.kinds[0], b.kinds[1], b.kinds[2], b.path
        );
    }
    let mut noted = 0usize;
    for d in &lib.descriptors {
        if !d.blocks {
            if noted < 20 {
                println!(
                    "  blocks:false  pid {:>5}  {:<9} {:<16} {}",
                    d.prefab_id,
                    d.kind,
                    d.reason.as_deref().unwrap_or("?"),
                    d.resource_name
                );
            }
            noted += 1;
        }
    }
    if noted > 20 {
        println!("  … {} more blocks:false descriptors", noted - 20);
    }
}

/// The `--all-prefabs` arm of `map bvh-batch`.
pub fn run(args: &[String]) -> Result<u8> {
    let mut terrain = "everon".to_string();
    let mut only_kinds: Vec<String> = Vec::new();
    let mut limit: Option<usize> = None;
    let mut hot = DEFAULT_HOT;
    let mut dry_run = false;
    let mut paks: Option<PathBuf> = None;
    let mut extract: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--all-prefabs" => i += 1,
            "--terrain" if i + 1 < args.len() => {
                terrain = args[i + 1].clone();
                i += 2;
            }
            "--only-kind" if i + 1 < args.len() => {
                only_kinds.push(args[i + 1].clone());
                i += 2;
            }
            "--limit" if i + 1 < args.len() => {
                limit = Some(args[i + 1].parse().context("--limit <N>")?);
                i += 2;
            }
            "--hot" if i + 1 < args.len() => {
                hot = args[i + 1].parse().context("--hot <N>")?;
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--paks" if i + 1 < args.len() => {
                paks = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--extract" if i + 1 < args.len() => {
                extract = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                eprintln!(
                    "bvh-batch --all-prefabs: unknown arg {other} (usage: --all-prefabs [--terrain everon] [--only-kind K]… [--limit N] [--hot N] [--dry-run] [--paks <dir>] [--extract <dir>] [--out <dir>])"
                );
                return Ok(1);
            }
        }
    }
    let root = crate::root::find_repo_root()?;
    let assets = root.join("packages/map-assets").join(&terrain);
    let out_dir = out.unwrap_or_else(|| assets.join("prefabs"));
    let schema_dir = root.join("packages/tbd-schema/schema");
    let rows = load_prefab_rows(&assets.join("objects/prefabs.json.gz"))?;
    let census = world_census(&assets.join("objects/chunks"))?;
    let source = open_sources(paks.as_deref(), extract.as_deref())?;
    let started = std::time::Instant::now();
    let opts = LibraryOptions {
        terrain: terrain.clone(),
        only_kinds,
        limit,
        hot,
    };
    let lib = build_library(&source, &rows, &census, &opts)?;
    print_report(&lib, &census, &terrain);
    let partial = !opts.only_kinds.is_empty() || opts.limit.is_some();
    if dry_run {
        println!(
            "  dry run — nothing written ({} descriptors, {} BLAS in memory, {:.1} s)",
            lib.descriptors.len(),
            lib.blas.len(),
            started.elapsed().as_secs_f64()
        );
        return Ok(0);
    }
    let written = write_library(&out_dir, &lib, &schema_dir, partial)?;
    println!(
        "  wrote {written} file(s) under {}{} ({:.1} s)",
        out_dir.display(),
        if partial {
            " — filtered run: manifest NOT written"
        } else {
            ""
        },
        started.elapsed().as_secs_f64()
    );
    Ok(0)
}
