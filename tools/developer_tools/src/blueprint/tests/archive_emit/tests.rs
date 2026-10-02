use super::*;
use crate::repository_layout::terrain_dir;
use website_map_engine::spatial::los::world::descriptor::ArchiveBoot;
use website_map_engine::spatial::los::world::descriptor::BuildingArchiveBytes;

fn prefabs_dir() -> PathBuf {
    terrain_dir(&crate::repository_paths::test_repo_root(), "everon").join("prefabs")
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

/// The split the occluder host boots from, over the WHOLE committed corpus.
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
