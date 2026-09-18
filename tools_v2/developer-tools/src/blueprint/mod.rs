//! `cargo xtask map blueprint-from-voxels` — the offline half of the blueprint split.
//!
//! Reads raw voxel dumps (`prefabs/dumps/<slug>_voxels.jsonl[.gz]`, written by the Workbench
//! `dump` action) and runs ALL interpretation here: slab detection, per-band wall extraction,
//! floor plates, masses, furniture — emitting schema-1.0.0 `BuildingBlueprint` JSON straight
//! into `assets_v2/terrains/everon/prefabs/buildings/`. A heuristic tune is a `cargo run`,
//! not a compile gate + Workbench restart + operator click.
//!
//! Usage: `--filter <substr> [--src <dir>] [--out <dir>] [--algo segments|grid]
//!         [--params <file.json>] [--debug-dir <dir>]`
//! Exit: 0 = every matched dump interpreted and validated · 1 = no match or any failure.
//!
//! **Subcommand `archive`** (T-935.8) — `cargo xtask map blueprint-from-voxels archive
//! [--terrain everon] [--out <prefabs dir>] [--dry-run]` folds the emitted library
//! (`prefabs/blas-manifest.json` + `descriptors/*.json` + `buildings/*.json`) into one
//! `prefabs/building_blueprints.rkyv` — see `archive_emit`. It takes the first positional token,
//! so it does not collide with `blueprint-from-voxels`' own flags; the archive is *not* reachable
//! as `cargo xtask map-blueprint archive` because that top-level command does not exist (the
//! `map_blueprint` module hangs off `map blueprint-from-voxels`, `tools_v2/xtask/src/main.rs:496`).

#[path = "archive_emission/archive_writer.rs"]
mod archive_emit;
#[path = "bvh/batch_processing.rs"]
mod batch;
#[path = "bvh/construction.rs"]
mod bvh;
#[path = "archive_emission/blueprint_assembly.rs"]
mod emit;
#[path = "architectural_analysis/convex_hulls.rs"]
mod hull;
#[path = "mesh_decoding/archive_inspection.rs"]
mod inspect;
#[path = "archive_emission/library_reader.rs"]
mod library;
#[path = "archive_emission/archive_command.rs"]
mod library_cli;
#[path = "architectural_analysis/contour_tracing.rs"]
mod march;
#[path = "voxel_processing/mesh_voxelization.rs"]
mod mesh;
#[path = "bvh/instance_pairs.rs"]
mod pair;
#[path = "voxel_processing/analysis_parameters.rs"]
mod params;
#[path = "voxel_processing/dump_parser.rs"]
mod parse;
#[path = "architectural_analysis/floor_plates.rs"]
mod plate;
#[path = "bvh/prefab_catalog.rs"]
mod prefab;
#[path = "architectural_analysis/polygon_rings.rs"]
mod rings;
#[path = "architectural_analysis/roof_profiles.rs"]
mod roof;
#[path = "bvh/rotation_validation.rs"]
mod rotation_pin;
#[path = "architectural_analysis/vertical_slabs.rs"]
mod slabs;
#[path = "architectural_analysis/surface_classification.rs"]
mod surface_kind;
#[cfg(test)]
#[path = "voxel_processing/synthetic_fixtures.rs"]
mod synth;
#[path = "voxel_processing/voxel_types.rs"]
mod types;
#[path = "bvh/instance_verification.rs"]
mod verify;
#[path = "architectural_analysis/wall_extraction.rs"]
mod walls;
#[path = "bvh/world_instances.rs"]
mod world_row;
#[path = "mesh_decoding/mesh_format.rs"]
mod xob;
#[path = "mesh_decoding/node_records.rs"]
mod xob_nodes;

pub use batch::run_bvh_batch;
pub use bvh::{run_bvh_emit, run_bvh_parity};
pub use inspect::{run_pak_cat, run_xob_inspect};
pub use mesh::run_voxels_from_mesh;
pub use rotation_pin::run_rotation_pin;
pub use verify::run_instances_verify;

use std::path::PathBuf;

use anyhow::Result;

use crate::blueprint::ingest::find_profile_subdirs;
use crate::repository_layout::{definition_path, terrain_dir};
use emit::BandProducts;
use params::Params;
use walls::Algo;

pub fn run(root: &std::path::Path, args: &[String]) -> Result<u8> {
    // T-935.8 — `… blueprint-from-voxels archive [--terrain everon]`: the rkyv fold of the emitted
    // library. A leading positional token, so every existing flag parse below is untouched.
    if args.first().is_some_and(|a| a == "archive") {
        return library_cli::run_archive(root, &args[1..]);
    }
    let mut src_override: Option<PathBuf> = None;
    let mut out_override: Option<PathBuf> = None;
    let mut params_path: Option<PathBuf> = None;
    let mut debug_dir: Option<PathBuf> = None;
    let mut filter = String::new();
    let mut algo = Algo::Segments;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--src" if i + 1 < args.len() => {
                src_override = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out_override = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--filter" if i + 1 < args.len() => {
                filter = args[i + 1].clone();
                i += 2;
            }
            "--params" if i + 1 < args.len() => {
                params_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--debug-dir" if i + 1 < args.len() => {
                debug_dir = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--algo" if i + 1 < args.len() => {
                algo = match args[i + 1].as_str() {
                    "segments" => Algo::Segments,
                    "grid" => Algo::Grid,
                    other => {
                        eprintln!("blueprint-from-voxels: unknown --algo {other}");
                        return Ok(1);
                    }
                };
                i += 2;
            }
            other => {
                eprintln!(
                    "blueprint-from-voxels: unknown arg {other} (usage: [--src <dir>] [--out <dir>] \
                     [--filter <substr>] [--algo segments|grid] [--params <file.json>] [--debug-dir <dir>])"
                );
                return Ok(1);
            }
        }
    }
    let params = Params::load(params_path.as_deref())?;

    let sources = find_profile_subdirs(src_override.as_deref(), "prefabs/dumps");
    if sources.is_empty() {
        eprintln!("blueprint-from-voxels: no prefabs/dumps dir found under any profile candidate");
        return Ok(1);
    }
    let out_dir =
        out_override.unwrap_or_else(|| terrain_dir(root, "everon").join("prefabs/buildings"));
    let schema = definition_path(root, "building-blueprint.schema.json");

    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut matched = 0usize;
    let mut seen: Vec<String> = Vec::new();
    for src_dir in &sources {
        let Ok(entries) = std::fs::read_dir(src_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(slug) = name
                .strip_suffix("_voxels.jsonl")
                .or_else(|| name.strip_suffix("_voxels.jsonl.gz"))
            else {
                continue;
            };
            if slug.is_empty()
                || (!filter.is_empty() && !name.contains(&filter))
                || seen.contains(&slug.to_string())
            {
                continue;
            }
            seen.push(slug.to_string());
            matched += 1;
            match interpret_one(&path, algo, &params, debug_dir.as_deref()) {
                Ok(bp) => {
                    let out_path = out_dir.join(format!("{slug}.json"));
                    match emit::validate_and_write(&bp, &schema, &out_path) {
                        Ok(()) => {
                            let walls: usize = bp.levels.iter().map(|l| l.walls.len()).sum();
                            println!(
                                "  OK  {slug}: {} levels · {walls} walls -> {}",
                                bp.levels.len(),
                                out_path.display()
                            );
                            ok += 1;
                        }
                        Err(e) => {
                            eprintln!("  FAIL {slug}: {e:#}");
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  FAIL {slug}: {e:#}");
                    failed += 1;
                }
            }
        }
    }

    println!(
        "blueprint-from-voxels: {ok} interpreted, {failed} failed, {matched} matched (algo: {algo:?}, filter: {})",
        if filter.is_empty() { "<none>" } else { &filter }
    );
    if failed > 0 || matched == 0 {
        return Ok(1);
    }
    Ok(0)
}

fn interpret_one(
    path: &std::path::Path,
    algo: Algo,
    base_params: &Params,
    debug_dir: Option<&std::path::Path>,
) -> Result<website_map_engine::world::architecture::blueprint::structure::BuildingBlueprint> {
    let dump = parse::parse_dump(path)?;
    let m = dump.meta().clone();
    println!(
        "    {}: dims {:?} cell {} · excluded {} doors / {} glass / {} furniture · tick {}",
        m.slug, m.dims, m.cell, m.excluded.doors, m.excluded.glass, m.excluded.furniture, m.tick
    );
    if dump.truncated > 0 {
        eprintln!(
            "  WARN {}: {} scanlines truncated at the dumper's 48-hit cap",
            m.slug, dump.truncated
        );
    }

    // The live floor filter is expressed in LOCAL y (> -0.5); shift it into normalized space.
    let mut p = base_params.clone();
    p.min_floor_y -= m.origin[1];

    let vert = slabs::analyze(&dump.y_down, m.dims, m.cell, m.span[1], &p);
    let mut band_debugs: Vec<walls::BandDebug> = debug_dir.map(|_| Vec::new()).unwrap_or_default();
    let bands = build_bands(&dump, &vert, algo, &p, debug_dir.map(|_| &mut band_debugs));

    if let Some(dir) = debug_dir {
        std::fs::create_dir_all(dir)?;
        let dbg = serde_json::json!({
            "slug": m.slug,
            "floors_local": vert.floors.iter().map(|f| f + m.origin[1]).collect::<Vec<_>>(),
            "slabs_local": vert.slabs.iter().map(|s| s + m.origin[1]).collect::<Vec<_>>(),
            "eave_local": vert.eave + m.origin[1],
            "ridge_local": vert.ridge + m.origin[1],
            "truncated_scanlines": dump.truncated,
            "furniture_records": dump.furniture.len(),
            // Per-band cluster attribution: every viewer-visible wall gap has a verdict here
            // (persistence / drift), with the roof-clipped denominator it was judged against.
            "bands": band_debugs,
        });
        std::fs::write(
            dir.join(format!("{}_stages.json", m.slug)),
            serde_json::to_string_pretty(&dbg)? + "\n",
        )?;
    }

    Ok(emit::assemble(&dump, &vert, bands, &p))
}

/// Band construction: one band per detected floor slab, then — when the ridge rises at least
/// `attic_min_rise_m` above the last band — a synthesized attic band up to the ridge (gable
/// ends and the upper walls of double-height rooms live there). The attic gets NO floor plate:
/// it has no slab, and probing the y_down window there would ingest sloped-roof smear as floor.
fn build_bands(
    dump: &types::VoxelDump,
    vert: &types::VerticalScan,
    algo: Algo,
    p: &Params,
    mut debugs: Option<&mut Vec<walls::BandDebug>>,
) -> Vec<BandProducts> {
    let m = dump.meta();
    let cell = m.cell;
    let mut bands = Vec::new();
    for (li, &lo) in vert.floors.iter().enumerate() {
        let hi = match vert.floors.get(li + 1) {
            Some(&next) => next,
            None => vert.eave.max(lo + p.top_band_min_m),
        };
        let mut dbg = debugs.as_deref_mut().map(|_| walls::BandDebug::default());
        let bw = walls::extract_band(dump, vert, lo, hi, algo, p, dbg.as_mut());
        if let (Some(sink), Some(d)) = (debugs.as_deref_mut(), dbg) {
            sink.push(d);
        }
        let (plate_grid, plate_heights) = plate::floor_plate(&dump.y_down, vert.nx, vert.nz, lo, p);
        let plate_cells = plate_grid.count();
        let traced = rings::trace(&plate_grid, cell, p.plate_min_ring_area_m2);
        let (footprint, floor_polygons) = traced.contract();
        println!(
            "    band {li} [{:.2}..{:.2}]: plate={plate_cells} raw={} walls={} masses={} rings={}(+{} dropped)",
            lo + m.origin[1],
            hi + m.origin[1],
            bw.raw_count,
            bw.walls.len(),
            bw.masses.len(),
            traced.pieces.len(),
            traced.dropped,
        );
        bands.push(BandProducts {
            band_lo: lo,
            band_hi: hi,
            walls: bw,
            footprint,
            floor_polygons,
            plate_heights,
            plate_cells,
            is_attic: false,
        });
    }

    let last_hi = bands.last().map_or(0.0, |b| b.band_hi);
    if vert.ridge - last_hi >= p.attic_min_rise_m {
        let mut dbg = debugs.as_deref_mut().map(|_| walls::BandDebug::default());
        let bw = walls::extract_band(dump, vert, last_hi, vert.ridge, algo, p, dbg.as_mut());
        if let (Some(sink), Some(d)) = (debugs, dbg) {
            sink.push(d);
        }
        println!(
            "    attic [{:.2}..{:.2}]: raw={} walls={} masses={}",
            last_hi + m.origin[1],
            vert.ridge + m.origin[1],
            bw.raw_count,
            bw.walls.len(),
            bw.masses.len(),
        );
        bands.push(BandProducts {
            band_lo: last_hi,
            band_hi: vert.ridge,
            walls: bw,
            footprint: Vec::new(),
            floor_polygons: Vec::new(),
            plate_heights: Vec::new(),
            plate_cells: 0,
            is_attic: true,
        });
    }
    bands
}

#[cfg(test)]
#[path = "tests/module/tests.rs"]
pub(crate) mod tests;

pub mod ingest;
pub mod parity_report;
