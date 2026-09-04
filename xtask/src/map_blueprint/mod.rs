//! `cargo xtask map blueprint-from-voxels` — the offline half of the blueprint split.
//!
//! Reads raw voxel dumps (`prefabs/dumps/<slug>_voxels.jsonl[.gz]`, written by the Workbench
//! `dump` action) and runs ALL interpretation here: slab detection, per-band wall extraction,
//! floor plates, masses, furniture — emitting schema-1.0.0 `BuildingBlueprint` JSON straight
//! into `packages/map-assets/everon/prefabs/buildings/`. A heuristic tune is a `cargo run`,
//! not a compile gate + Workbench restart + operator click.
//!
//! Usage: `--filter <substr> [--src <dir>] [--out <dir>] [--algo segments|grid]
//!         [--params <file.json>] [--debug-dir <dir>]`
//! Exit: 0 = every matched dump interpreted and validated · 1 = no match or any failure.

mod batch;
mod bvh;
mod emit;
mod hull;
mod inspect;
mod march;
mod mesh;
mod pair;
mod pak;
mod params;
mod parse;
mod plate;
mod prefab;
mod rings;
mod roof;
mod rotation_pin;
mod slabs;
mod surface_kind;
#[cfg(test)]
mod synth;
mod types;
mod verify;
mod walls;
mod world_row;
mod xob;
mod xob_nodes;

pub use batch::run_bvh_batch;
pub use bvh::{run_bvh_emit, run_bvh_parity};
pub use inspect::{run_pak_cat, run_xob_inspect};
pub use mesh::run_voxels_from_mesh;
pub use rotation_pin::run_rotation_pin;
pub use verify::run_instances_verify;

use std::path::PathBuf;

use anyhow::Result;

use crate::map_ingest_blueprints::find_profile_subdirs;
use crate::root::find_repo_root;
use emit::BandProducts;
use params::Params;
use walls::Algo;

pub fn run(args: &[String]) -> Result<u8> {
    let root = find_repo_root()?;
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
        out_override.unwrap_or_else(|| root.join("packages/map-assets/everon/prefabs/buildings"));
    let schema = root.join("packages/tbd-schema/schema/building-blueprint.schema.json");

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
) -> Result<map_engine_core::building_blueprint::BuildingBlueprint> {
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
        if let (Some(sink), Some(d)) = (debugs.as_deref_mut(), dbg) {
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
pub(crate) mod tests {
    use super::*;
    use crate::root::find_repo_root;

    pub(crate) fn fixture(name: &str) -> std::path::PathBuf {
        find_repo_root()
            .expect("repo root")
            .join("xtask/tests/fixtures")
            .join(name)
    }

    /// The floors-and-walls fixture end to end: partial mezzanine plate (void stays void),
    /// the under-roof knee wall surviving via the roof-clipped persistence denominator, and
    /// the attic band self-synthesizing with gable ends and no plate.
    #[test]
    fn gable_mezzanine_bands_plate_and_knee_wall() {
        let d = synth::gable_mezzanine();
        let m = d.meta().clone();
        let mut p = Params::default();
        p.min_floor_y = -0.5 - m.origin[1];
        let vert = slabs::analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
        assert_eq!(
            vert.floors.len(),
            2,
            "ground + mezzanine: {:?}",
            vert.floors
        );

        let mut dbg: Vec<walls::BandDebug> = Vec::new();
        let bands = build_bands(&d, &vert, Algo::Segments, &p, Some(&mut dbg));
        assert_eq!(bands.len(), 3, "two floors + attic");
        assert_eq!(dbg.len(), 3);

        // Attic: [band1_hi .. ridge], no plate products, gable ends on both x extremes.
        let attic = &bands[2];
        assert!(attic.is_attic);
        assert!((attic.band_hi - vert.ridge).abs() < 1e-9);
        assert!(attic.plate_heights.is_empty() && attic.footprint.is_empty());
        let gable_ends = attic
            .walls
            .walls
            .iter()
            .filter(|w| (w.start[0] - w.end[0]).abs() < 1e-9)
            .count();
        assert!(
            gable_ends >= 2,
            "gable ends missing: {:?}",
            attic.walls.walls
        );

        // Mezzanine plate covers roughly the west half — and only that.
        assert!(bands[1].plate_cells > 0);
        assert!(
            bands[1].plate_cells * 3 < bands[0].plate_cells * 2,
            "mezzanine plate must stay partial: {} vs ground {}",
            bands[1].plate_cells,
            bands[0].plate_cells
        );

        // The knee wall (x-running; solid z = 0.325 → normalized 0.925 after the 0.6 m dump
        // PAD) survives in band 1: only ~6/16 whole-window rows, but ≥ need against its
        // roof-clipped denominator.
        let knee = bands[1]
            .walls
            .walls
            .iter()
            .find(|w| {
                (w.start[1] - w.end[1]).abs() < 1e-9
                    && (w.start[1] - 0.925).abs() < 0.15
                    && w.start[0].min(w.end[0]) > 3.4
            })
            .unwrap_or_else(|| panic!("knee wall missing: {:?}", bands[1].walls.walls));
        assert!(
            (w_len(knee) - 1.8).abs() < 0.4,
            "knee wall run length: {:?}",
            knee
        );
        let knee_cluster = dbg[1]
            .clusters
            .iter()
            .find(|c| c.axis == "x-running" && (c.center - 0.925).abs() < 0.15 && c.fixed > 34)
            .expect("knee cluster recorded");
        assert_eq!(knee_cluster.verdict, "accepted");
        assert!(
            knee_cluster.rows_avail < 16 && knee_cluster.rows_seen >= knee_cluster.need,
            "roof clipping engaged: {knee_cluster:?}"
        );
    }

    fn w_len(w: &types::WallSeg) -> f64 {
        ((w.end[0] - w.start[0]).powi(2) + (w.end[1] - w.start[1]).powi(2)).sqrt()
    }

    /// Full pipeline on the real FarmHouse dump == the committed golden. Any heuristic change
    /// that shifts the output must re-bless the golden deliberately.
    #[test]
    fn farmhouse_dump_matches_golden_blueprint() {
        let bp = interpret_one(
            &fixture("FarmHouse_E_1L01_Wood_voxels.jsonl.gz"),
            Algo::Segments,
            &Params::default(),
            None,
        )
        .expect("interpret fixture dump");
        let got = serde_json::to_value(&bp).expect("serialize");
        let golden: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_blueprint.golden.json"))
                .expect("read golden"),
        )
        .expect("parse golden");
        // The writer strips null optionals; mirror that for the comparison.
        let mut got = got;
        if got.get("modelMesh").is_some_and(serde_json::Value::is_null) {
            got.as_object_mut().expect("obj").remove("modelMesh");
        }
        assert_eq!(
            got, golden,
            "pipeline drifted from the blessed golden — re-bless on purpose"
        );
    }

    /// The acceptance instrument, pinned: replay the committed 400-pair engine oracle through
    /// the golden blueprint + the golden `.bvh` sidecar. **400/400** since `evaluate_los` moved
    /// onto the BVH raycaster (T-090.6 step 3, 2026-09-01). 2.5D history: 260/400 pre-roof
    /// (every miss the unmodeled roof) → 384 roof heightfield → 387 attic band + above-roof
    /// wall cap, where the 13 misses were all model-clear/engine-blocked roof-margin leans.
    /// Same instrument as `bvh::tests::farmhouse_bvh_sidecar_parity_is_pinned` by construction
    /// (`Bvh::first_hit` ⇔ `Bvh::any_hit` on existence) — kept as the blueprint-lane pin so an
    /// attribution or annotation bug that flips `is_clear` fails HERE, on the shipping path.
    #[test]
    fn farmhouse_golden_parity_is_pinned() {
        #[derive(serde::Deserialize)]
        struct ParityFile {
            pairs: Vec<(f64, f64, f64, f64, f64, f64, bool)>,
        }
        let bp: map_engine_core::building_blueprint::BuildingBlueprint = serde_json::from_str(
            &std::fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_blueprint.golden.json"))
                .expect("read golden"),
        )
        .expect("parse golden");
        let sidecar = map_engine_core::bvh::BvhSidecar::parse(
            &std::fs::read(fixture("FarmHouse_E_1L01_Wood.bvh.golden")).expect("read sidecar"),
        )
        .expect("parse golden sidecar");
        let oracle: ParityFile = serde_json::from_str(
            &std::fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_parity.json"))
                .expect("read parity"),
        )
        .expect("parse parity");
        assert_eq!(oracle.pairs.len(), 400);
        let mut agree = 0usize;
        let mut model_blocked_engine_clear = 0usize;
        for &(ox, oy, oz, tx, ty, tz, engine_clear) in &oracle.pairs {
            let model_clear = bp
                .evaluate_los(&sidecar, [ox, oy, oz], [tx, ty, tz])
                .is_clear;
            if model_clear == engine_clear {
                agree += 1;
            } else if !model_clear && engine_clear {
                model_blocked_engine_clear += 1;
            }
        }
        assert_eq!(agree, 400, "parity drifted (was 100%)");
        assert_eq!(
            model_blocked_engine_clear, 0,
            "phantom geometry blocks rays the engine clears"
        );
    }
}
