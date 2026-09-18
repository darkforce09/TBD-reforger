use std::path::PathBuf;

use website_map_engine::io::containers::header::HEADER_BYTES;
use website_map_engine::world::terrain::dem::png::decode_png_gray16;
use website_map_engine::world::terrain::dem::raw::RawDem;

use super::*;
use crate::repository_layout::{terrain_dir, terrain_manifest_path};

/// A distinct, non-square grid: a width/height swap anywhere in the emit or the read is a
/// different file, and both `u16` endpoints are present.
const W: u32 = 4;
const H: u32 = 3;
fn grid() -> Vec<u16> {
    vec![
        0, 1, 65535, 32768, 40000, 7, 60000, 100, 12345, 2, 511, 65534,
    ]
}

fn tmpdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t935-4-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tempdir");
    dir
}

/// The emitter's frame: 32-byte header, then exactly `2 * width * height` bytes, and the file
/// reads back through the loader's own parser as the samples that went in.
#[test]
fn write_elevation_dem_frames_the_grid_and_round_trips() {
    let dir = tmpdir("frame");
    let p = dir.join("elevation.dem");
    let s = grid();
    write_elevation_dem(&p, W, H, -204.78, 375.53, &s).expect("write");

    let bytes = std::fs::read(&p).expect("read");
    assert_eq!(
        bytes.len(),
        HEADER_BYTES + 2 * (W as usize) * (H as usize),
        "file length must be 32 + 2 x width x height"
    );
    assert_eq!(&bytes[..4], b"TBDE");
    // The wire layout, byte for byte: version, flags, width, height, then sample 0 LE.
    assert_eq!(&bytes[4..6], &1_u16.to_le_bytes());
    assert_eq!(&bytes[6..8], &0_u16.to_le_bytes());
    assert_eq!(&bytes[8..12], &W.to_le_bytes());
    assert_eq!(&bytes[12..16], &H.to_le_bytes());
    assert_eq!(&bytes[HEADER_BYTES..HEADER_BYTES + 2], &s[0].to_le_bytes());
    // …and sample 2 (65535) proves the payload is LE, not BE.
    assert_eq!(
        &bytes[HEADER_BYTES + 4..HEADER_BYTES + 6],
        &65535_u16.to_le_bytes()
    );

    let dem = RawDem::parse(&bytes).expect("parse");
    assert_eq!((dem.width(), dem.height()), (W, H));
    assert_eq!(dem.samples, s);
    #[allow(clippy::cast_possible_truncation)]
    let want_scale = ((375.53_f64 - -204.78_f64) as f32) / 65535.0;
    assert_eq!(dem.header.offset_m, -204.78_f64 as f32);
    assert!(
        (dem.header.scale_m - want_scale).abs() <= f32::EPSILON,
        "{} vs {want_scale}",
        dem.header.scale_m
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A sample count that disagrees with the header dims is refused, and nothing is written — the
/// alternative is a file whose header lies about its own payload.
#[test]
fn write_elevation_dem_refuses_a_grid_that_is_not_width_times_height() {
    let dir = tmpdir("mismatch");
    let p = dir.join("elevation.dem");
    let err = write_elevation_dem(&p, W, H, 0.0, 1.0, &grid()[..11]).expect_err("must refuse");
    assert!(
        format!("{err:#}").contains("needs 12 samples, got 11"),
        "{err:#}"
    );
    assert!(!p.exists(), "nothing may be written for a rejected grid");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **The dual-emission acceptance test.** One `raw_u16_to_dem_png` run over a synthetic ASCII
/// raster must write both files, and the `.dem` must decode to exactly the grid the `.png`
/// decodes to — through the two independent decoders, not through the emitter's own memory.
#[test]
fn raw_u16_to_dem_png_also_emits_elevation_dem_with_the_same_grid() {
    let dir = tmpdir("dual");
    let s = grid();
    let raster = dir.join("heightmap.txt");
    std::fs::write(
        &raster,
        s.chunks(W as usize)
            .map(|row| row.iter().map(u16::to_string).collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .expect("raster");
    let meta = dir.join("meta.json");
    std::fs::write(
        &meta,
        serde_json::to_string(&json!({
            "widthPx": W, "heightPx": H,
            "heightRangeMinM": -204.78, "heightRangeMaxM": 375.53,
        }))
        .expect("meta json"),
    )
    .expect("meta");
    let png = dir.join("everon-dem-16bit.png");

    assert_eq!(
        raw_u16_to_dem_png(&raster, &meta, &png).expect("convert"),
        0,
        "the converter must succeed"
    );

    let dem_path = dir.join("elevation.dem");
    assert!(png.exists(), "the PNG emit must be untouched");
    assert!(dem_path.exists(), "elevation.dem must be written beside it");

    let (png_raster, pw, ph) =
        decode_png_gray16(&std::fs::read(&png).expect("read png")).expect("png decode");
    let dem = RawDem::parse(&std::fs::read(&dem_path).expect("read dem")).expect("dem parse");
    assert_eq!((dem.width(), dem.height()), (pw, ph), "dims must agree");
    assert_eq!(dem.samples, png_raster, "the two files must carry one grid");
    assert_eq!(dem.samples, s, "…and it must be the raster's own values");
    // The header carries the meta's range, so a reader needs no manifest to get metres.
    assert_eq!(dem.header.offset_m, -204.78_f64 as f32);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A meta without the `heightRange*` keys falls back to the plugin's fixed V4 range rather
/// than quantising against 0..0 and flattening the terrain.
#[test]
fn elevation_dem_falls_back_to_the_v4_range_when_meta_omits_it() {
    let dir = tmpdir("v4");
    let s = grid();
    let raster = dir.join("heightmap.txt");
    std::fs::write(
        &raster,
        s.iter().map(u16::to_string).collect::<Vec<_>>().join(" "),
    )
    .expect("raster");
    let meta = dir.join("meta.json");
    std::fs::write(
        &meta,
        serde_json::to_string(&json!({ "widthPx": W, "heightPx": H })).expect("meta json"),
    )
    .expect("meta");
    let png = dir.join("d.png");
    assert_eq!(
        raw_u16_to_dem_png(&raster, &meta, &png).expect("convert"),
        0
    );
    let dem =
        RawDem::parse(&std::fs::read(dir.join("elevation.dem")).expect("read")).expect("dem parse");
    assert_eq!(dem.header.offset_m, DEM_DEFAULT_MIN_M as f32);
    assert_eq!(
        dem.header.scale_m,
        ((DEM_DEFAULT_MAX_M - DEM_DEFAULT_MIN_M) as f32) / 65535.0
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **The everon spot check.** Decodes the committed 6400x6400 DEM PNG, emits the `.dem` from
/// its raster and re-reads it, asserting all 40,960,000 samples are identical and metres agree
/// to within the `f32` rounding of the header's scale/offset.
///
/// NOT `#[ignore]`, as of T-946. It was, because `packages/map-assets/**/*.png` is git-LFS and
/// in a slice worktree the file is a 133-byte pointer — but a blanket ignore also hid it from
/// the WAVE gate, which runs on main where the payload is real. The wave 238 verifier found
/// that `test xtask+tbd-tools PASS` had covered the emitter's unit tests and not the one test
/// that compares the emitted `.dem` against the shipped DEM. It costs 3.5 s there.
///
/// So the skip is now conditional on the evidence rather than declared: a pointer file is a
/// few hundred bytes, the real DEM is 71.9 MB, and a skip SAYS SO on stdout instead of
/// vanishing into an ignore count.
#[test]
fn everon_elevation_dem_matches_the_shipped_png() {
    use website_map_engine::world::terrain::dem::sampling::uint16_to_meters;

    let root = repo_root();
    let png = terrain_dir(&root, "everon").join("dem/everon-dem-16bit.png");
    // An LFS pointer is ~133 B; the real 6400x6400 16-bit PNG is 71.9 MB. Anything in between
    // is neither, and is worth failing on rather than skipping past.
    const LFS_POINTER_MAX: u64 = 4096;
    match std::fs::metadata(&png) {
        Ok(m) if m.len() <= LFS_POINTER_MAX => {
            println!(
                "skip-lfs: {} is {} bytes — a git-LFS pointer, not the DEM. \
                 Hydrate with `git lfs pull --include packages/map-assets/everon/dem/`",
                png.display(),
                m.len()
            );
            return;
        }
        Ok(_) => {}
        Err(e) => panic!("{}: {e}", png.display()),
    }
    let bytes = std::fs::read(&png).unwrap_or_else(|e| panic!("{}: {e}", png.display()));
    assert!(
        bytes.len() > 1_000_000,
        "{} is {} bytes — this is the git-lfs pointer, not the DEM; \
         `git lfs pull --include {}` first",
        png.display(),
        bytes.len(),
        png.display()
    );
    let (raster, w, h) = decode_png_gray16(&bytes).expect("png decode");
    assert_eq!((w, h), (6400, 6400), "everon DEM dims");

    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(terrain_manifest_path(&root, "everon")).expect("manifest"),
    )
    .expect("manifest json");
    let min_m = manifest["dem"]["heightRangeMinM"].as_f64().expect("min");
    let max_m = manifest["dem"]["heightRangeMaxM"].as_f64().expect("max");

    let dir = tmpdir("everon");
    let p = dir.join("elevation.dem");
    write_elevation_dem(&p, w, h, min_m, max_m, &raster).expect("write");
    let on_disk = std::fs::read(&p).expect("read");
    assert_eq!(
        on_disk.len(),
        HEADER_BYTES + 2 * 6400 * 6400,
        "everon .dem is 32 + 81,920,000 bytes"
    );
    let dem = RawDem::parse(&on_disk).expect("parse");
    assert_eq!(
        dem.samples, raster,
        "all 40,960,000 samples must be identical"
    );

    let mut worst = 0.0_f64;
    for y in 0..h {
        for x in 0..w {
            let v = raster[(y * w + x) as usize];
            let png_m = uint16_to_meters(f64::from(v), min_m, max_m) as f32;
            let raw_m = dem.metres(x, y).expect("in range");
            worst = worst.max((f64::from(raw_m) - f64::from(png_m)).abs());
        }
    }
    println!(
        "everon spot check: {}x{} samples bit-identical; worst metre delta {worst:e} m \
         (quantisation step {} m)",
        w,
        h,
        (max_m - min_m) / 65535.0
    );
    assert!(worst <= 1e-4, "worst metre delta {worst}");
    let _ = std::fs::remove_dir_all(&dir);
}
