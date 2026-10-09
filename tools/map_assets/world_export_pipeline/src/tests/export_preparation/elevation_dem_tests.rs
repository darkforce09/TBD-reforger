use std::path::PathBuf;

use terrain_elevation::png::decode_png_gray16;
use terrain_elevation::raw::RawDem;
use world_file_formats::containers::header::HEADER_BYTES;

use super::*;

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
    let dir = std::env::temp_dir().join(format!("elevation-dem-{tag}-{}", std::process::id()));
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

/// The emitted `.dem` of a grid that spans the whole `u16` range keeps every sample bit for bit
/// and gives back the same metres as the PNG quantisation, within the `f32` rounding of the
/// header's scale and offset.
#[test]
fn elevation_dem_metres_agree_with_the_png_quantisation() {
    use terrain_elevation::sampling::uint16_to_meters;

    const SYNTHETIC_W: u32 = 37;
    const SYNTHETIC_H: u32 = 23;
    let (min_m, max_m) = (DEM_DEFAULT_MIN_M, DEM_DEFAULT_MAX_M);
    let cells = SYNTHETIC_W * SYNTHETIC_H;
    let raster: Vec<u16> = (0..cells)
        .map(|i| {
            let step = u64::from(i) * u64::from(u16::MAX) / u64::from(cells - 1);
            u16::try_from(step).expect("ramp stays inside u16")
        })
        .collect();
    assert_eq!(raster.first(), Some(&0));
    assert_eq!(raster.last(), Some(&u16::MAX));

    let dir = tmpdir("synthetic-metres");
    let p = dir.join("elevation.dem");
    write_elevation_dem(&p, SYNTHETIC_W, SYNTHETIC_H, min_m, max_m, &raster).expect("write");
    let on_disk = std::fs::read(&p).expect("read");
    assert_eq!(
        on_disk.len(),
        HEADER_BYTES + 2 * (cells as usize),
        "file length must be 32 + 2 x width x height"
    );
    let dem = RawDem::parse(&on_disk).expect("parse");
    assert_eq!((dem.width(), dem.height()), (SYNTHETIC_W, SYNTHETIC_H));
    assert_eq!(dem.samples, raster, "every sample must be identical");

    let mut worst = 0.0_f64;
    for y in 0..SYNTHETIC_H {
        for x in 0..SYNTHETIC_W {
            let v = raster[(y * SYNTHETIC_W + x) as usize];
            let png_m = uint16_to_meters(f64::from(v), min_m, max_m) as f32;
            let raw_m = dem.metres(x, y).expect("in range");
            worst = worst.max((f64::from(raw_m) - f64::from(png_m)).abs());
        }
    }
    assert!(worst <= 1e-4, "worst metre delta {worst}");
    let _ = std::fs::remove_dir_all(&dir);
}
