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

mod emit;
mod march;
mod mesh;
mod pair;
mod params;
mod parse;
mod plate;
mod slabs;
#[cfg(test)]
mod synth;
mod types;
mod walls;
mod xob;

pub use mesh::run_voxels_from_mesh;

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
    let mut bands = Vec::new();
    for (li, &lo) in vert.floors.iter().enumerate() {
        let hi = match vert.floors.get(li + 1) {
            Some(&next) => next,
            None => vert.eave.max(lo + p.top_band_min_m),
        };
        let bw = walls::extract_band(&dump, &vert, lo, hi, algo, &p);
        let plate_grid = plate::floor_plate(&dump.y_down, vert.nx, vert.nz, lo, &p);
        let plate_cells = plate_grid.count();
        let footprint = plate::outline(&plate_grid, m.cell, &p);
        println!(
            "    band {li} [{:.2}..{:.2}]: plate={plate_cells} raw={} walls={} masses={}",
            lo + m.origin[1],
            hi + m.origin[1],
            bw.raw_count,
            bw.walls.len(),
            bw.masses.len()
        );
        bands.push(BandProducts {
            band_lo: lo,
            band_hi: hi,
            walls: bw,
            footprint,
            plate_cells,
        });
    }

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
        });
        std::fs::write(
            dir.join(format!("{}_stages.json", m.slug)),
            serde_json::to_string_pretty(&dbg)? + "\n",
        )?;
    }

    Ok(emit::assemble(&dump, &vert, bands, &p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::root::find_repo_root;

    fn fixture(name: &str) -> std::path::PathBuf {
        find_repo_root()
            .expect("repo root")
            .join("xtask/tests/fixtures")
            .join(name)
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
    /// the golden blueprint. 260/400 was measured 2026-08-29 (segments 65.0% vs live v6 64.0%,
    /// grid port 63.2%); every miss is model-clear/engine-blocked — the unmodeled roof.
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
        let oracle: ParityFile = serde_json::from_str(
            &std::fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_parity.json"))
                .expect("read parity"),
        )
        .expect("parse parity");
        assert_eq!(oracle.pairs.len(), 400);
        let mut agree = 0usize;
        let mut model_blocked_engine_clear = 0usize;
        for &(ox, oy, oz, tx, ty, tz, engine_clear) in &oracle.pairs {
            let model_clear = bp.evaluate_los([ox, oy, oz], [tx, ty, tz]).is_clear;
            if model_clear == engine_clear {
                agree += 1;
            } else if !model_clear && engine_clear {
                model_blocked_engine_clear += 1;
            }
        }
        assert_eq!(agree, 260, "parity drifted (was 65.0%)");
        assert_eq!(
            model_blocked_engine_clear, 0,
            "phantom geometry blocks rays the engine clears"
        );
    }
}
