use website_map_engine::world::terrain::water::vectors::Bathymetry;
use website_map_engine::world::terrain::water::vectors::WaterAt;
use website_map_engine::world::terrain::water::vectors::WaterMask;
use website_map_engine::world::terrain::water::vectors::WaterVectors;

use super::*;

/// A 4×4 grid with three wet texels at three different depths, so `max` is distinguishable
/// from `first`, `last` and `any`, and so no level folds to a uniform answer.
const DEPTH_4X4: &str = "7 0 0 0\n0 0 0 0\n0 30 0 0\n0 0 0 12\n";
const MASK_4X4: &str = "2 0 0 0\n0 0 0 0\n0 3 0 0\n0 0 0 2\n";
const SCALE: f32 = 0.1;

fn meta_json(w: u32, h: u32) -> String {
    format!("{{\"widthPx\":{w},\"heightPx\":{h},\"depthScaleToMeters\":0.1}}")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "tbd-t935-9-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(p.join("scratch").join(STAGING_WATER)).expect("mkdir scratch");
        Self(p)
    }

    /// Where `emit_water` will look for the export: through the same resolver it uses.
    fn staging(&self) -> PathBuf {
        scratch_dir(self.0.to_str().expect("utf8")).join(STAGING_WATER)
    }

    /// Write the four staging files, then run the whole `emit_water` CLI path over them.
    fn stage(&self, depth: &str, mask: &str, meta: &str, vectors: &str) -> &Path {
        let s = self.staging();
        std::fs::write(s.join(DEPTH_TXT), depth).expect("depth");
        std::fs::write(s.join(MASK_TXT), mask).expect("mask");
        std::fs::write(s.join(META_JSON), meta).expect("meta");
        std::fs::write(s.join(VECTORS_JSON), vectors).expect("vectors");
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn vectors_json() -> String {
    r#"{
      "lakes": [{
        "id": "lake_1", "name": "Lake 1", "surfaceElevationYM": 84.762,
        "polygon": [[0,84.762,0],[4,84.762,0],[4,84.762,4],[0,84.762,4],[0,84.762,0]]
      }],
      "rivers": [{
        "id": "river_1", "name": "River 1", "averageWidthM": 12.5,
        "nodes": [
          {"pos": [0,1.5,1], "widthM": 12.5},
          {"pos": [2,1.5,1], "widthM": 12.5},
          {"pos": [4,1.5,1], "widthM": 12.5}
        ]
      }],
      "ponds": [{
        "id": "pond_1", "surfaceElevationYM": 3.25,
        "perimeter": [[1,3.25,1],[2,3.25,1],[2,3.25,2]]
      }],
      "inlandWaterBodies": []
    }"#
    .to_string()
}

fn bathymetry_of(dir: &Path) -> Bathymetry {
    let raw = std::fs::read(dir.join(BATHYMETRY_TBDB)).expect("read bath");
    Bathymetry::from_bytes(&raw).expect("parse emitted bathymetry")
}

#[test]
fn mip_count_halves_to_one_texel() {
    assert_eq!(mip_count(1, 1), 1);
    assert_eq!(mip_count(2, 2), 2);
    assert_eq!(mip_count(4, 4), 3);
    assert_eq!(mip_count(5, 5), 3);
    assert_eq!(mip_count(12_800, 12_800), 14);
    // The coarsest level of every case above must actually be 1×1 on the longer side.
    for (w, h) in [(1_u32, 1_u32), (4, 4), (5, 3), (12_800, 12_800), (1, 9)] {
        let last = mip_count(w, h) - 1;
        assert_eq!(
            ((w >> u32::from(last)).max(1)).max((h >> u32::from(last)).max(1)),
            1,
            "{w}x{h} does not reach one texel at level {last}"
        );
    }
}

/// THE SLICE'S PIN. The emitted pyramid's every level agrees with its own level 0: a coarse
/// texel is water iff ANY level-0 texel folding into it is, and its depth is that block's MAX.
/// Read back through the CORE reader, so emitter and loader are pinned to each other rather
/// than to a private copy of the rule.
#[test]
fn every_emitted_mip_level_agrees_with_level_zero() {
    let s = Scratch::new("mips");
    let dir = s.stage(DEPTH_4X4, MASK_4X4, &meta_json(4, 4), &vectors_json());
    assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
    let b = bathymetry_of(dir);
    assert_eq!((b.width(), b.height(), b.level_count()), (4, 4, 3));
    let base = b.level(0).expect("level 0");
    assert_eq!(
        base.depth,
        &[7, 0, 0, 0, 0, 0, 0, 0, 0, 30, 0, 0, 0, 0, 0, 12]
    );
    assert_eq!(base.mask, &[1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1]);

    for level in 1..b.level_count() {
        let grid = b.level(level).expect("level");
        let n = (grid.width * grid.height) as usize;
        let (mut want_mask, mut want_depth) = (vec![0_u8; n], vec![0_u16; n]);
        for z in 0..base.height {
            for x in 0..base.width {
                let (mut tx, mut tz) = (x, z);
                for l in 1..=level {
                    let (lw, lh) = b.header().level_dims(l).expect("dims");
                    tx = downsample_index(tx, lw);
                    tz = downsample_index(tz, lh);
                }
                let src = base.index(x, z).expect("src");
                let dst = grid.index(tx, tz).expect("dst");
                want_mask[dst] |= u8::from(base.mask[src] != 0);
                want_depth[dst] = want_depth[dst].max(base.depth[src]);
            }
        }
        assert_eq!(grid.mask, &want_mask[..], "level {level} mask");
        assert_eq!(grid.depth, &want_depth[..], "level {level} depth");
        assert!(
            want_mask.contains(&1) && want_depth.iter().any(|&d| d > 0),
            "level {level} oracle is vacuous"
        );
    }
    // The coarsest level is one texel and it is wet — three separate lakes cannot summarise
    // to dry ground.
    let top = b.level(b.level_count() - 1).expect("top");
    assert_eq!((top.width, top.height), (1, 1));
    assert_eq!((top.mask[0], top.depth[0]), (1, 30));
}

/// The container the emitter writes answers the placement question the SPA will ask of it —
/// end to end, through `WaterMask`, on the real file bytes.
#[test]
fn the_emitted_container_answers_the_placement_query() {
    let s = Scratch::new("query");
    let dir = s.stage(DEPTH_4X4, MASK_4X4, &meta_json(4, 4), &vectors_json());
    assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
    let raw = std::fs::read(dir.join(BATHYMETRY_TBDB)).expect("read");
    let m = WaterMask::from_bytes(&raw, [0.0, 0.0, 3.0, 3.0]).expect("mask");
    // 4 samples over [0, 3] → 1 m apart; the wet texel (1, 2) is world (1, 2).
    assert_eq!(m.sample(1.0, 2.0), WaterAt::Water { depth_m: 3.0 });
    assert!(m.is_water(1.0, 2.0));
    assert!(!m.is_known_dry_land(1.0, 2.0));
    assert_eq!(m.sample(2.0, 1.0), WaterAt::Dry);
    assert!(m.is_known_dry_land(2.0, 1.0));
    assert_eq!(m.depth_m(0.0, 0.0), Some(f32::from(7_u8) * SCALE));
    // Off the map: no reading, and above all not "known dry land".
    assert_eq!(m.sample(-1.0, 2.0), WaterAt::Unknown);
    assert!(!m.is_water(-1.0, 2.0) && !m.is_known_dry_land(-1.0, 2.0));
    assert_eq!(m.depth_m(99.0, 99.0), None);
}

#[test]
fn the_vectors_archive_carries_every_lane_with_its_surface_height() {
    let s = Scratch::new("vectors");
    let dir = s.stage(DEPTH_4X4, MASK_4X4, &meta_json(4, 4), &vectors_json());
    assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
    let raw = std::fs::read(dir.join(WATER_VECTORS_RKYV)).expect("read vectors");
    let v = WaterVectors::from_bytes(&raw).expect("parse through the core reader");
    assert_eq!(v.counts().expect("counts"), (1, 1, 1));
    let a = v.archive().expect("archive");
    assert_eq!(a.lakes[0].id.as_str(), "lake_1");
    assert_eq!(a.lakes[0].surface_y.to_native(), 84.762);
    // The closing point was dropped: 5 authored, 4 stored.
    assert_eq!(a.lakes[0].ring.len(), 4);
    assert_eq!(a.lakes[0].ring[1][0].to_native(), 4.0);
    // `[x, y, z]` → `[x, z]`: the ring must carry the PLAN, never the elevation.
    assert_eq!(a.lakes[0].ring[2][1].to_native(), 4.0);
    assert_eq!(a.rivers[0].width_m.to_native(), 12.5);
    assert_eq!(a.rivers[0].centerline.len(), 3);
    assert_eq!(a.rivers[0].centerline[1][1].to_native(), 1.0);
    // The pond lane fires: `perimeter` is read as a ring, not silently dropped.
    assert_eq!(a.ponds[0].id.as_str(), "pond_1");
    assert_eq!(a.ponds[0].surface_y.to_native(), 3.25);
    assert_eq!(a.ponds[0].ring.len(), 3);
}

#[test]
fn bytes_are_deterministic_across_runs() {
    let mut runs = Vec::new();
    for tag in ["det-a", "det-b"] {
        let s = Scratch::new(tag);
        let dir = s.stage(DEPTH_4X4, MASK_4X4, &meta_json(4, 4), &vectors_json());
        assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
        runs.push((
            std::fs::read(dir.join(BATHYMETRY_TBDB)).expect("bath"),
            std::fs::read(dir.join(WATER_VECTORS_RKYV)).expect("vectors"),
        ));
    }
    assert_eq!(
        runs[0].0, runs[1].0,
        "bathymetry bytes are not deterministic"
    );
    assert_eq!(runs[0].1, runs[1].1, "vector bytes are not deterministic");
    // Every level is padded to 4, so the payload is too — and the file with it.
    assert!(runs[0].0.len().is_multiple_of(4));
}

/// Odd dimensions: 5 → 2 → 1. The last fold has to absorb a third source column, and the
/// emitted level must still be the size the header's ladder claims.
#[test]
fn odd_dimensions_emit_a_consistent_pyramid() {
    let depth: String = (0..5)
        .map(|z| {
            (0..5)
                .map(|x| if (x, z) == (4, 4) { "9" } else { "0" })
                .collect::<Vec<_>>()
                .join(" ")
                + "\n"
        })
        .collect();
    let mask: String = depth.replace('9', "2");
    let s = Scratch::new("odd");
    let dir = s.stage(&depth, &mask, &meta_json(5, 5), &vectors_json());
    assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
    let b = bathymetry_of(dir);
    assert_eq!(b.level_count(), 3);
    for level in 0..3 {
        let g = b.level(level).expect("level");
        let want = (5_u32 >> u32::from(level)).max(1);
        assert_eq!((g.width, g.height), (want, want), "level {level}");
        assert!(
            g.mask.iter().any(|&m| m != 0) && g.depth.contains(&9),
            "level {level} lost the corner lake the odd tail clamps into"
        );
    }
}

#[test]
fn a_raster_that_disagrees_with_the_meta_is_refused() {
    let s = Scratch::new("short");
    // Three rows where the meta says four.
    let dir = s.stage(
        "0 0 0 0\n0 0 0 0\n0 0 0 0\n",
        MASK_4X4,
        &meta_json(4, 4),
        &vectors_json(),
    );
    let err = emit_water(dir.to_str().expect("utf8")).expect_err("must refuse");
    assert!(format!("{err:#}").contains("3 rows, expected 4"), "{err:#}");

    let s2 = Scratch::new("wide");
    let dir2 = s2.stage(
        DEPTH_4X4,
        "2 0 0 0 0\n0 0 0 0\n0 0 0 0\n0 0 0 0\n",
        &meta_json(4, 4),
        &vectors_json(),
    );
    let err2 = emit_water(dir2.to_str().expect("utf8")).expect_err("must refuse");
    assert!(
        format!("{err2:#}").contains("5 values, expected 4"),
        "{err2:#}"
    );
}

#[test]
fn a_depth_that_does_not_fit_the_container_is_refused_not_truncated() {
    let s = Scratch::new("huge");
    let dir = s.stage(
        "65536 0 0 0\n0 0 0 0\n0 0 0 0\n0 0 0 0\n",
        MASK_4X4,
        &meta_json(4, 4),
        &vectors_json(),
    );
    let err = emit_water(dir.to_str().expect("utf8")).expect_err("must refuse");
    assert!(format!("{err:#}").contains("does not fit"), "{err:#}");
}

#[test]
fn a_meta_that_cannot_describe_a_grid_is_refused() {
    for bad in [
        r#"{"widthPx":0,"heightPx":4,"depthScaleToMeters":0.1}"#,
        r#"{"widthPx":4,"heightPx":0,"depthScaleToMeters":0.1}"#,
        r#"{"widthPx":4,"heightPx":4,"depthScaleToMeters":0}"#,
        r#"{"widthPx":4,"heightPx":4}"#,
        r#"{"heightPx":4,"depthScaleToMeters":0.1}"#,
    ] {
        assert!(parse_meta(bad).is_err(), "{bad}");
    }
    assert_eq!(
        parse_meta(r#"{"widthPx":12800,"heightPx":12800,"depthScaleToMeters":0.1}"#)
            .expect("everon meta"),
        RasterMeta {
            width: 12_800,
            height: 12_800,
            depth_scale: SCALE
        }
    );
}

#[test]
fn a_vector_export_the_archive_cannot_represent_is_refused() {
    for (bad, needle) in [
        (
            r#"{"lakes":[{"surfaceElevationYM":1,"polygon":[[0,0,0],[1,0,0],[1,0,1]]}]}"#,
            "id",
        ),
        (
            r#"{"lakes":[{"id":"l","polygon":[[0,0,0],[1,0,0],[1,0,1]]}]}"#,
            "surfaceElevationYM",
        ),
        (
            r#"{"lakes":[{"id":"l","surfaceElevationYM":1,"polygon":[[0,0,0],[1,0,0]]}]}"#,
            "3 distinct points",
        ),
        (
            r#"{"lakes":[{"id":"l","surfaceElevationYM":1,"polygon":[[0,0],[1,0],[1,1]]}]}"#,
            "[x, y, z]",
        ),
        (
            r#"{"rivers":[{"id":"r","averageWidthM":1,"nodes":[{"pos":[0,0,0]}]}]}"#,
            "2 points",
        ),
        (r#"{"rivers":[{"id":"r","averageWidthM":1}]}"#, "nodes"),
    ] {
        let err = build_water_vectors(bad).expect_err(bad);
        assert!(format!("{err:#}").contains(needle), "{bad} → {err:#}");
    }
}

#[test]
fn an_export_with_no_water_at_all_is_refused() {
    let s = Scratch::new("empty");
    let dir = s.stage(
        DEPTH_4X4,
        MASK_4X4,
        &meta_json(4, 4),
        r#"{"lakes":[],"rivers":[]}"#,
    );
    let err = emit_water(dir.to_str().expect("utf8")).expect_err("must refuse");
    assert!(
        format!("{err:#}").contains("refusing empty write"),
        "{err:#}"
    );
    assert!(
        !dir.join(WATER_VECTORS_RKYV).exists(),
        "nothing may be written"
    );
}

#[test]
fn a_missing_terrain_or_staging_directory_exits_one() {
    assert_eq!(emit_water("/nonexistent/terrain/xyzzy").expect("no dir"), 1);
    let s = Scratch::new("nostaging");
    std::fs::remove_dir_all(s.staging()).expect("drop staging");
    assert_eq!(
        emit_water(s.0.to_str().expect("utf8")).expect("no staging"),
        1
    );
}

#[test]
fn terrain_dir_takes_an_id_or_a_directory() {
    assert_eq!(
        terrain_dir("everon"),
        crate::repository_layout::terrain_dir(&repo_root(), "everon")
    );
    let here = repo_root();
    assert_eq!(terrain_dir(here.to_str().expect("utf8")), here);
}

/// The export scratch is a sibling of the served tree, never nested inside it — for a terrain id
/// through the repository layout, and for a directory argument under its own `scratch/`.
#[test]
fn scratch_dir_pairs_with_terrain_dir_without_nesting_inside_it() {
    assert_eq!(
        scratch_dir("everon"),
        crate::repository_layout::map_scratch_dir(&repo_root(), "everon")
    );
    assert!(!scratch_dir("everon").starts_with(terrain_dir("everon")));
    let here = repo_root();
    assert_eq!(
        scratch_dir(here.to_str().expect("utf8")),
        here.join("scratch")
    );
}
