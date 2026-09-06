//! T-935.8 — `cargo xtask map blueprint-from-voxels archive [--terrain everon]`: the whole prefab
//! occluder library as ONE `prefabs/building_blueprints.rkyv`.
//!
//! Reads what `bvh-batch --all-prefabs` already wrote — `prefabs/blas-manifest.json`,
//! `prefabs/descriptors/<pid>.json` (1623 files, 19 MB) — plus the extracted blueprints under
//! `prefabs/buildings/`, and folds them into the T-935.1
//! [`BuildingBlueprintArchive`](map_engine_core::world::binary::archives::BuildingBlueprintArchive):
//! the descriptor census, the shared BLAS index the descriptors point into, and the tactical
//! blueprint levels. The loader side is `world::occluder::descriptor` (`BuildingArchiveBytes`,
//! `PrefabDescriptor::from_archived`) and `building_blueprint::BuildingBlueprint::from_archived`.
//!
//! **This emitter refuses rather than approximates.** Every descriptor must project (a BLAS it
//! names must be in the library; `blocks` and `localBounds` must agree), the descriptor files and
//! the manifest's descriptor list must be the same set, and the bytes are read back through
//! `access_checked` before they are written. A descriptor silently dropped or written with a zero
//! bounding box is a prefab that stops occluding — a wrong sightline, not a crash.
//!
//! Blueprint levels exist for a handful of prefabs; the rest come from a Workbench pass the
//! operator runs, and the command prints that exact invocation for what is still missing.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use map_engine_core::building_blueprint::BuildingBlueprint as JsonBlueprint;
use map_engine_core::world::binary::archives::{
    ARCHIVE_SCHEMA_VERSION, BuildingBlueprintArchive, BuildingLevel as WireLevel,
    DoorRec as WireDoor, FurnitureRec as WireFurniture, StairsRec as WireStairs,
    VerticalProfile as WireProfile, WallRec as WireWall, WindowRec as WireWindow,
};
use map_engine_core::world::binary::{access_checked, to_bytes};
use map_engine_core::world::occluder::{BlasEntry, BlasManifest, PrefabDescriptor};

use super::batch::write_if_changed;

/// Where the archive is written, relative to `packages/map-assets/<terrain>/`.
pub const ARCHIVE_REL: &str = "prefabs/building_blueprints.rkyv";

/// The built archive plus what the report needs to say about it.
pub struct Built {
    pub archive: BuildingBlueprintArchive,
    /// Slugs of `blocks: true` building prefabs the archive has no blueprint levels for, sorted.
    /// The Workbench `dump` pass is what fills these in.
    pub without_levels: Vec<String>,
    /// Blueprint files read, in slug order.
    pub blueprint_slugs: Vec<String>,
}

/// The `.json` files under `prefabs/buildings/` that are NOT blueprints. Skipped by name rather
/// than by "did serde fail?": a suffix list cannot quietly swallow a blueprint that is genuinely
/// malformed, which is the failure a `from_str(..).ok()` filter would hide.
const NON_BLUEPRINT_SUFFIXES: &[&str] = &[".instances.json", ".scene.json"];

/// Read `prefabs/` and fold it into one archive.
///
/// # Errors
/// Any unreadable/unparseable input, a descriptor set that disagrees with the manifest, a
/// descriptor that will not project, or a blueprint whose slug is not in the catalogue.
pub fn build(prefabs: &Path) -> Result<Built> {
    let manifest: BlasManifest = read_json(&prefabs.join("blas-manifest.json"))?;
    let blas_index: Vec<_> = manifest.blas.iter().map(BlasEntry::to_archived).collect();
    // `manifest.blas` is emitted sorted by path (library.rs), so the index is a binary search and
    // the archive's `blas` indices are stable across re-emits.
    let index_of = |p: &str| {
        manifest
            .blas
            .binary_search_by(|b| b.path.as_str().cmp(p))
            .ok()
            .and_then(|i| u32::try_from(i).ok())
    };

    let mut descriptors = Vec::with_capacity(manifest.descriptors.len());
    let mut slug_of_pid: BTreeMap<u32, String> = BTreeMap::new();
    let mut pid_of_slug: BTreeMap<String, u32> = BTreeMap::new();
    let mut json_pids: Vec<u32> = Vec::new();
    for path in sorted_json_files(&prefabs.join("descriptors"))? {
        let d: PrefabDescriptor = read_json(&path)?;
        json_pids.push(d.prefab_id);
        slug_of_pid.insert(d.prefab_id, d.slug.clone());
        pid_of_slug.insert(d.slug.clone(), d.prefab_id);
        descriptors.push(
            d.to_archived(&index_of)
                .with_context(|| path.display().to_string())?,
        );
    }
    json_pids.sort_unstable();
    let manifest_pids: Vec<u32> = manifest.descriptors.iter().map(|d| d.pid).collect();
    if json_pids != manifest_pids {
        bail!(
            "descriptors/ and blas-manifest.json disagree: {} files vs {} manifest rows — the \
             archive would be a census of a library that does not exist. Re-run \
             `map bvh-batch --all-prefabs` first.",
            json_pids.len(),
            manifest_pids.len()
        );
    }
    descriptors.sort_by_key(|d| d.prefab_id);

    let mut blueprints = Vec::new();
    let mut blueprint_slugs = Vec::new();
    for path in sorted_json_files(&prefabs.join("buildings"))? {
        let name = file_name(&path);
        if NON_BLUEPRINT_SUFFIXES.iter().any(|s| name.ends_with(s)) {
            continue;
        }
        let b: JsonBlueprint = read_json(&path)?;
        let pid = *pid_of_slug.get(&b.prefab_id).with_context(|| {
            format!(
                "{}: prefabId {:?} is not in the prefab catalogue — a blueprint the occluder can \
                 never key on",
                path.display(),
                b.prefab_id
            )
        })?;
        blueprint_slugs.push(b.prefab_id.clone());
        blueprints.push(wire_blueprint(&b, pid).with_context(|| path.display().to_string())?);
    }
    blueprints.sort_by_key(|b| b.prefab_id);

    let have: Vec<u32> = blueprints.iter().map(|b| b.prefab_id).collect();
    let without_levels = descriptors
        .iter()
        .filter(|d| d.blocks && d.kind == "building" && !have.contains(&d.prefab_id))
        .filter_map(|d| slug_of_pid.get(&d.prefab_id).cloned())
        .collect();

    Ok(Built {
        archive: BuildingBlueprintArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            descriptors,
            blas_index,
            blueprints,
        },
        without_levels,
        blueprint_slugs,
    })
}

/// One extracted blueprint as the archive's tactical row.
///
/// The counterpart of [`JsonBlueprint::from_archived`]; the fields dropped here are the ones that
/// method documents. `usize`/`u32` level and step counts are narrowed with `try_from` so an
/// impossible building fails the emit instead of wrapping into a plausible small number.
fn wire_blueprint(
    b: &JsonBlueprint,
    prefab_id: u32,
) -> Result<map_engine_core::world::binary::archives::BuildingBlueprint> {
    let mut levels = Vec::with_capacity(b.levels.len());
    for l in &b.levels {
        levels.push(WireLevel {
            level_index: u8::try_from(l.level_index)
                .with_context(|| format!("level_index {} does not fit u8", l.level_index))?,
            elevation_range: pair(l.elevation_range),
            footprint_polygon: l.footprint_polygon.iter().map(|p| pair(*p)).collect(),
            walls: l
                .walls
                .iter()
                .map(|w| WireWall {
                    id: w.id.clone(),
                    start: pair(w.start),
                    end: pair(w.end),
                    thickness_m: w.thickness as f32,
                    is_exterior: w.is_exterior,
                    material: w.material.clone(),
                })
                .collect(),
            doors: l
                .doors
                .iter()
                .map(|d| WireDoor {
                    id: d.id.clone(),
                    wall_id: d.wall_id.clone(),
                    position: pair(d.pos2_d),
                    width_m: d.width_m as f32,
                    height_m: d.height_m as f32,
                    is_exterior: d.is_exterior,
                    has_glass: d.has_glass,
                })
                .collect(),
            windows: l
                .windows
                .iter()
                .map(|w| WireWindow {
                    id: w.id.clone(),
                    wall_id: w.wall_id.clone(),
                    position: pair(w.pos2_d),
                    width_m: w.width_m as f32,
                    sill_height_m: w.sill_height_m as f32,
                    window_height_m: w.window_height_m as f32,
                    normal: pair(w.normal),
                    fov_deg: w.fov_deg as f32,
                    has_glass: w.has_glass,
                })
                .collect(),
            stairs: l
                .stairs
                .iter()
                .map(|s| {
                    Ok(WireStairs {
                        id: s.id.clone(),
                        bounds: [pair(s.bounds[0]), pair(s.bounds[1])],
                        connects_to_level: u8::try_from(s.connects_to_level).with_context(
                            || format!("connects_to_level {} does not fit u8", s.connects_to_level),
                        )?,
                        direction_deg: s.direction_deg as f32,
                        step_count: u16::try_from(s.step_count).with_context(|| {
                            format!("step_count {} does not fit u16", s.step_count)
                        })?,
                        transparent_steps: s.transparent_steps,
                        los_concealment: s.los_concealment as f32,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            furniture: l
                .furniture
                .iter()
                .map(|f| WireFurniture {
                    id: f.id.clone(),
                    name: f.name.clone(),
                    category: f.category.clone(),
                    position: pair(f.pos2_d),
                    rotation_deg: f.rotation_deg as f32,
                    height_m: f.height_m as f32,
                    blocks_movement: f.blocks_movement,
                    los_cover: f.los_cover.clone(),
                })
                .collect(),
        });
    }
    Ok(
        map_engine_core::world::binary::archives::BuildingBlueprint {
            prefab_id,
            slug: b.prefab_id.clone(),
            vertical_profile: WireProfile {
                pivot_elevation_offset_m: b.vertical_profile.pivot_elevation_offset_m as f32,
                foundation_skirt_depth_m: b.vertical_profile.foundation_skirt_depth_m as f32,
                total_height_m: b.vertical_profile.total_height_m as f32,
                eave_height_m: b.vertical_profile.eave_height_m as f32,
                ridge_height_m: b.vertical_profile.ridge_height_m as f32,
                roof_type: b.vertical_profile.roof_type.clone(),
            },
            levels,
        },
    )
}

fn pair(p: [f64; 2]) -> [f32; 2] {
    [p[0] as f32, p[1] as f32]
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let s = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    serde_json::from_str(&s).with_context(|| path.display().to_string())
}

/// Every `*.json` directly in `dir`, sorted by name — a deterministic read order, so the archive
/// is byte-identical across machines and `write_if_changed` means what it says.
fn sorted_json_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for ent in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let p = ent?.path();
        if p.extension().is_some_and(|e| e == "json") {
            out.push(p);
        }
    }
    if out.is_empty() {
        bail!(
            "{}: no .json files — refusing to write an empty archive over a real library",
            dir.display()
        );
    }
    out.sort();
    Ok(out)
}

/// The Workbench pass that extracts the blueprint levels this archive is still missing.
///
/// Printed, not run: it needs a live Workbench with the terrain loaded, which is an operator step.
/// The whole slug list goes into the string on purpose — a truncated "271 …" is a description of a
/// command rather than a command, and the operator would have to reconstruct it by hand.
fn workbench_batch_command(slugs: &[String]) -> String {
    if slugs.is_empty() {
        return "# nothing to do — every building prefab already has blueprint levels".to_string();
    }
    let list = slugs
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "for slug in {list}; do \\\n    \
           cargo xtask mcp wbcall EMCP_WB_TbdBlueprint '{{\"action\":\"dump\",\"filter\":\"'\"$slug\"'\"}}' && \\\n    \
           cargo xtask map blueprint-from-voxels --filter \"$slug\"; \\\n  \
         done\n  \
         # then re-run `cargo xtask map blueprint-from-voxels archive` to fold them in"
    )
}

/// `map blueprint-from-voxels archive [--terrain everon] [--out <dir>] [--dry-run]`.
pub fn run(args: &[String]) -> Result<u8> {
    let mut terrain = "everon".to_string();
    let mut out: Option<PathBuf> = None;
    let mut dry_run = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--terrain" if i + 1 < args.len() => {
                terrain = args[i + 1].clone();
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            other => {
                eprintln!(
                    "archive: unknown arg {other} (usage: archive [--terrain everon] \
                     [--out <prefabs dir>] [--dry-run])"
                );
                return Ok(1);
            }
        }
    }
    let root = crate::root::find_repo_root()?;
    let assets = root.join("packages/map-assets").join(&terrain);
    let prefabs = out.unwrap_or_else(|| assets.join("prefabs"));
    let started = std::time::Instant::now();

    let built = build(&prefabs)?;
    let bytes = to_bytes(&built.archive).map_err(|e| anyhow::anyhow!("{e}"))?;
    // Read back what we are about to write, through the loader's own entry point. An archive that
    // does not validate is worse than no archive: the occluder would fall back silently.
    access_checked::<BuildingBlueprintArchive>(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;

    let levels: usize = built
        .archive
        .blueprints
        .iter()
        .map(|b| b.levels.len())
        .sum();
    println!(
        "map-blueprint archive {terrain}: {} descriptors · {} BLAS index rows · {} blueprints \
         ({levels} levels) · {:.2} MB rkyv ({:.1} s)",
        built.archive.descriptors.len(),
        built.archive.blas_index.len(),
        built.archive.blueprints.len(),
        bytes.len() as f64 / 1_048_576.0,
        started.elapsed().as_secs_f64()
    );
    println!("  blueprints: {}", built.blueprint_slugs.join(", "));
    println!(
        "  {} building prefab(s) still have no blueprint levels. Workbench pass (operator, live \
         Workbench with {terrain} loaded):\n  {}",
        built.without_levels.len(),
        workbench_batch_command(&built.without_levels)
    );
    let dest = assets.join(ARCHIVE_REL);
    if dry_run {
        println!("  dry run — nothing written ({})", dest.display());
        return Ok(0);
    }
    let wrote = write_if_changed(&dest, &bytes)?;
    println!(
        "  {} {}",
        if wrote { "wrote" } else { "unchanged" },
        dest.display()
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_engine_core::world::occluder::descriptor::{ArchiveBoot, BuildingArchiveBytes};

    fn prefabs_dir() -> PathBuf {
        crate::root::test_repo_root().join("packages/map-assets/everon/prefabs")
    }

    /// The archive read back through the loader's own entry point, plus the JSON it came from.
    fn built_and_read() -> (Built, BuildingArchiveBytes) {
        let built = build(&prefabs_dir()).expect("build the archive");
        let bytes = to_bytes(&built.archive).expect("serialise");
        let held = BuildingArchiveBytes::new(&bytes);
        held.archive().expect("access_checked validates the emit");
        (built, held)
    }

    /// EVERY committed descriptor, not a sample: the archive is the occluder's boot census, and a
    /// row that is quietly wrong is a sightline that is quietly wrong.
    ///
    /// The oracle is an independent re-read of `descriptors/<pid>.json` — the archive is compared
    /// against the JSON on disk, not against the value the emitter happened to build.
    #[test]
    fn archive_round_trips_every_committed_descriptor() {
        let (built, held) = built_and_read();
        let a = held.archive().expect("access");
        let files = sorted_json_files(&prefabs_dir().join("descriptors")).expect("descriptors");
        assert_eq!(files.len(), 1623, "the committed descriptor corpus");
        assert_eq!(
            a.descriptors.len(),
            files.len(),
            "every descriptor archived"
        );
        assert_eq!(
            a.blas_index.len(),
            built.archive.blas_index.len(),
            "the whole BLAS library index"
        );

        let by_pid: BTreeMap<u32, &_> = a
            .descriptors
            .iter()
            .map(|d| (d.prefab_id.to_native(), d))
            .collect();
        let mut blocking = 0usize;
        for path in &files {
            let json: PrefabDescriptor = read_json(path).expect("parse descriptor");
            let row = by_pid
                .get(&json.prefab_id)
                .unwrap_or_else(|| panic!("pid {} missing from the archive", json.prefab_id));

            assert_eq!(row.slug.as_str(), json.slug, "pid {}", json.prefab_id);
            assert_eq!(row.kind.as_str(), json.kind, "pid {}", json.prefab_id);
            assert_eq!(row.blocks, json.blocks, "pid {}", json.prefab_id);
            assert_eq!(row.canopy, json.canopy, "pid {}", json.prefab_id);
            // The .bvh fetch list, resolved through the index — the thing the loader uses and the
            // one field a lost or misordered index would corrupt.
            assert_eq!(
                PrefabDescriptor::archived_blas_paths(row, &a.blas_index),
                Some(
                    json.blas_paths()
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect::<Vec<_>>()
                ),
                "pid {} BLAS list",
                json.prefab_id
            );
            assert_eq!(
                PrefabDescriptor::from_archived(row),
                json.archive_census(),
                "pid {} census",
                json.prefab_id
            );
            blocking += usize::from(json.blocks);
        }
        assert_eq!(blocking, 1322, "blocking descriptors in the corpus");
    }

    /// Every extracted blueprint on disk, level by level and record by record, against an
    /// independent re-read of its JSON (through `f32`, which is what the wire is).
    #[test]
    fn archive_carries_every_committed_blueprint_level() {
        let (built, held) = built_and_read();
        let a = held.archive().expect("access");
        let dir = prefabs_dir().join("buildings");
        let jsons: Vec<PathBuf> = sorted_json_files(&dir)
            .expect("buildings")
            .into_iter()
            .filter(|p| {
                let n = file_name(p);
                !NON_BLUEPRINT_SUFFIXES.iter().any(|s| n.ends_with(s))
            })
            .collect();
        assert!(!jsons.is_empty(), "committed blueprints");
        assert_eq!(a.blueprints.len(), jsons.len(), "every blueprint archived");
        assert_eq!(built.blueprint_slugs.len(), jsons.len());

        for path in &jsons {
            let json: JsonBlueprint = read_json(path).expect("parse blueprint");
            let row = a
                .blueprints
                .iter()
                .find(|b| b.slug.as_str() == json.prefab_id)
                .unwrap_or_else(|| panic!("{} missing from the archive", json.prefab_id));
            let back = JsonBlueprint::from_archived(row);

            assert_eq!(back.prefab_id, json.prefab_id);
            assert_eq!(
                back.vertical_profile.roof_type,
                json.vertical_profile.roof_type
            );
            assert_eq!(
                back.vertical_profile.ridge_height_m,
                f32_of(json.vertical_profile.ridge_height_m),
                "{} ridge",
                json.prefab_id
            );
            assert_eq!(back.levels.len(), json.levels.len(), "{}", json.prefab_id);
            for (b, j) in back.levels.iter().zip(&json.levels) {
                assert_eq!(b.level_index, j.level_index);
                assert_eq!(b.elevation_range, pair64(j.elevation_range));
                assert_eq!(b.footprint_polygon.len(), j.footprint_polygon.len());
                assert_eq!(b.walls.len(), j.walls.len(), "walls");
                assert_eq!(b.doors.len(), j.doors.len(), "doors");
                assert_eq!(b.windows.len(), j.windows.len(), "windows");
                assert_eq!(b.stairs.len(), j.stairs.len(), "stairs");
                assert_eq!(b.furniture.len(), j.furniture.len(), "furniture");
                for (bw, jw) in b.walls.iter().zip(&j.walls) {
                    assert_eq!(bw.id, jw.id);
                    assert_eq!(bw.start, pair64(jw.start), "wall {} start", jw.id);
                    assert_eq!(bw.end, pair64(jw.end), "wall {} end", jw.id);
                    assert_eq!(bw.thickness, f32_of(jw.thickness));
                    assert_eq!(bw.is_exterior, jw.is_exterior);
                    assert_eq!(bw.material, jw.material);
                }
                for (bd, jd) in b.doors.iter().zip(&j.doors) {
                    assert_eq!((&bd.id, &bd.wall_id), (&jd.id, &jd.wall_id));
                    assert_eq!(bd.pos2_d, pair64(jd.pos2_d));
                    assert_eq!(bd.width_m, f32_of(jd.width_m));
                    assert_eq!(bd.has_glass, jd.has_glass);
                }
                for (bw, jw) in b.windows.iter().zip(&j.windows) {
                    assert_eq!((&bw.id, &bw.wall_id), (&jw.id, &jw.wall_id));
                    assert_eq!(bw.pos2_d, pair64(jw.pos2_d));
                    assert_eq!(bw.normal, pair64(jw.normal), "window {} normal", jw.id);
                    assert_eq!(bw.sill_height_m, f32_of(jw.sill_height_m));
                    assert_eq!(bw.fov_deg, f32_of(jw.fov_deg));
                }
                for (bs, js) in b.stairs.iter().zip(&j.stairs) {
                    assert_eq!(bs.id, js.id);
                    assert_eq!(bs.connects_to_level, js.connects_to_level);
                    assert_eq!(bs.step_count, js.step_count);
                    assert_eq!(bs.los_concealment, f32_of(js.los_concealment));
                }
                for (bf, jf) in b.furniture.iter().zip(&j.furniture) {
                    assert_eq!(
                        (&bf.id, &bf.name, &bf.category),
                        (&jf.id, &jf.name, &jf.category)
                    );
                    assert_eq!(bf.pos2_d, pair64(jf.pos2_d));
                    assert_eq!(bf.height_m, f32_of(jf.height_m));
                    assert_eq!(bf.blocks_movement, jf.blocks_movement);
                    assert_eq!(bf.los_cover, jf.los_cover);
                }
            }
        }
    }

    /// T-935.8 — the split the occluder host boots from, over the WHOLE committed corpus.
    ///
    /// Two things are pinned here, and both are load-bearing for line of sight:
    ///
    /// 1. **The census carries `blocks: false` rows and nothing else.** An archived row has no
    ///    instance records, so a `blocks: true` descriptor rebuilt from one enters
    ///    `WorldOccluder::descriptors`, fails `try_expand` on its empty instance list
    ///    (`trace.rs:341`), and is then skipped by `wanted()` forever (`trace.rs:443` only asks
    ///    for a descriptor it does not already hold). The prefab would trace against its coarse
    ///    AABB for the rest of the session with nothing logged. This assertion is the only thing
    ///    standing between that and the loader.
    /// 2. **Every row's `.bvh` list survives the index.** The oracle is the descriptor JSON's own
    ///    `blas_paths()`, re-read from disk — not the archive compared with itself — so a lost,
    ///    truncated or reordered `blas_index` is caught here.
    #[test]
    fn archive_boot_splits_the_whole_corpus_and_never_censuses_a_blocking_prefab() {
        let (_built, held) = built_and_read();
        let a = held.archive().expect("access");
        let boot = ArchiveBoot::from_archive(a);

        // The safety invariant goes FIRST, deliberately. Asserted after the counts, a flipped
        // branch reports "left: 301, right: 1322" — a bookkeeping mismatch — instead of naming the
        // prefabs that would stop occluding. Measured: that is exactly what the first version of
        // this test printed under the perturbation.
        let censused_blocker = boot.census.iter().find(|d| d.blocks);
        assert!(
            censused_blocker.is_none(),
            "prefab {:?} blocks and is in the census: rebuilt from the archive it has no instance \
             records, so insert_descriptor registers it and try_expand yields nothing — it stops \
             occluding for the whole session with nothing logged",
            censused_blocker.map(|d| (d.prefab_id, d.slug.as_str()))
        );
        assert!(
            boot.census.iter().all(|d| d.local_bounds.is_none()),
            "blocks: false carries no bounds"
        );
        assert_eq!(boot.unusable, 0, "every committed row resolves");
        assert_eq!(boot.census.len() + boot.blocking, 1623, "the whole corpus");
        assert_eq!(
            boot.blocking, 1322,
            "blocking prefabs stay on the JSON path"
        );
        assert_eq!(
            boot.census.len(),
            301,
            "non-blocking prefabs boot from the archive"
        );

        let by_pid: BTreeMap<u16, &Vec<String>> =
            boot.blas_by_pid.iter().map(|(p, v)| (*p, v)).collect();
        assert_eq!(by_pid.len(), boot.blas_by_pid.len(), "one row per pid");
        let mut with_blas = 0usize;
        for path in sorted_json_files(&prefabs_dir().join("descriptors")).expect("descriptors") {
            let json: PrefabDescriptor = read_json(&path).expect("parse descriptor");
            let pid = u16::try_from(json.prefab_id).expect("everon pids fit u16");
            let want: Vec<String> = json.blas_paths().iter().map(|s| (*s).to_string()).collect();
            assert_eq!(
                by_pid.get(&pid).map(|v| v.as_slice()),
                Some(&want[..]),
                "pid {pid}"
            );
            with_blas += usize::from(!want.is_empty());
        }
        assert_eq!(with_blas, 1322, "every blocking prefab has a sidecar list");
    }

    /// The remainder is a Workbench pass, and the command that runs it is part of the emit's
    /// output rather than a line in a doc that drifts.
    #[test]
    fn report_names_the_buildings_still_missing_levels_and_the_command_that_fills_them() {
        let built = build(&prefabs_dir()).expect("build");
        assert!(
            !built.without_levels.is_empty(),
            "the everon catalogue has building prefabs with no extracted blueprint"
        );
        let cmd = workbench_batch_command(&built.without_levels);
        assert!(cmd.contains("mcp wbcall EMCP_WB_TbdBlueprint"), "{cmd}");
        assert!(cmd.contains("\\\"action\\\":\\\"dump\\\"") || cmd.contains("\"action\":\"dump\""));
        assert!(cmd.contains("map blueprint-from-voxels --filter"), "{cmd}");
        assert!(cmd.contains(&built.without_levels[0]), "{cmd}");
    }

    fn f32_of(v: f64) -> f64 {
        f64::from(v as f32)
    }

    fn pair64(p: [f64; 2]) -> [f64; 2] {
        [f32_of(p[0]), f32_of(p[1])]
    }
}
