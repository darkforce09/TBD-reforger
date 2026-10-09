//! **Role:** unit tests of [`crate::water_export_images`]: `--roi` parsing, the images each mode
//! selects, and the golden run that draws a synthetic export end to end and compares the decoded
//! pixels of every image with the reference digests.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by `water_export_images.rs`.
//! **Signals & state:** the golden case writes a synthetic export and its images under the system
//! temporary folder and removes them.
//! **Invariants:** the synthetic export is a 32 × 32 grid over a 32 m world: mask `1` where
//! `x + y < 28`, depth `(53x + 29y) mod 1200`, one three-segment river, one lake and one pond with
//! bounds; the digests are SHA-256 of the decoded, unfiltered samples (16-bit samples most
//! significant byte first).

use std::path::{Path, PathBuf};

use super::*;

/// The decoded-pixel SHA-256 of each image of the synthetic export, by file name.
const GOLDEN_DIGESTS: [(&str, &str); 5] = [
    (
        "fixture-water-bathymetry.png",
        "04bdeab3d1650bb5b411af6dc2f0bb1c9cefd7d70cb69a61218b3be2e8779a67",
    ),
    (
        "fixture-water-bathymetry-dark.png",
        "7103043c595f83834e21ad2904cdb4600396ba5aaaf6fb685b473a895c00b14a",
    ),
    (
        "fixture-water-depth-16bit.png",
        "543b2bc070ac504c2795bdfc250444d27bb0c4c837fa3dafdc8009ab8b066ffd",
    ),
    (
        "fixture-water-mask.png",
        "e08fc386db650706e8df0bad9e3987db3e51abdb16502f8a3ff1ddc9b8433928",
    ),
    (
        "fixture-water-preview.png",
        "7103043c595f83834e21ad2904cdb4600396ba5aaaf6fb685b473a895c00b14a",
    ),
];

/// The synthetic export's metadata.
const META_JSON: &str = r#"{"widthPx":32,"heightPx":32,"worldSizeM":32,"planarResolutionM":1}"#;

/// The synthetic export's river.
const RIVERS_JSON: &str =
    r#"{"rivers":[{"widthM":3,"splinePoints":[[2,0,30],[10,0,24],[20,0,26],[30,0,18]]}]}"#;

/// The synthetic export's lake.
const LAKES_JSON: &str = r#"{"lakes":[{"surfaceElevationYM":2,"avgDepthM":2.5,"maxDepthM":4,"polygon":[[18,0,4],[28,0,6],[29,0,14],[22,0,16],[17,0,10]]}]}"#;

/// The synthetic export's pond.
const PONDS_JSON: &str = r#"{"ponds":[{"maxDepthM":1.2,"bounds":{"min":[5,0,12],"max":[11,0,18]},"perimeter":[[5,0,12],[11,0,12],[11,0,18],[5,0,18]]}]}"#;

/// A folder under the system temporary folder, removed on drop.
struct TemporaryFolder(PathBuf);

impl TemporaryFolder {
    fn new(case: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "tbd-water-export-images-{case}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir_all(&path).expect("create the temporary folder");
        Self(path)
    }
}

impl Drop for TemporaryFolder {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// One ASCII grid of the synthetic export: rows of space-separated values, each row ending in a
/// line feed.
fn synthetic_grid(value: impl Fn(usize, usize) -> usize) -> String {
    let mut text = String::new();
    for y in 0..32 {
        let row: Vec<String> = (0..32).map(|x| value(x, y).to_string()).collect();
        text.push_str(&row.join(" "));
        text.push('\n');
    }
    text
}

/// Writes the synthetic export into `folder`.
fn write_synthetic_export(folder: &Path) {
    let mask = synthetic_grid(|x, y| usize::from(x + y < 28));
    let depth = synthetic_grid(|x, y| (x * 53 + y * 29) % 1200);
    for (name, text) in [
        ("bathymetry_mask.txt", mask.as_str()),
        ("bathymetry_depth.txt", depth.as_str()),
        ("water_meta.json", META_JSON),
        ("rivers.json", RIVERS_JSON),
        ("lakes.json", LAKES_JSON),
        ("ponds.json", PONDS_JSON),
    ] {
        std::fs::write(folder.join(name), text).expect("write a synthetic export file");
    }
}

/// The SHA-256 of the decoded samples of the PNG at `path`, decoded with no transformation.
fn decoded_pixel_digest(path: &Path) -> String {
    let file = std::fs::File::open(path).expect("open the image");
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().expect("read the image header");
    let mut samples = vec![0_u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut samples).expect("decode the image");
    samples.truncate(frame.buffer_size());
    content_digest::sha256_hex(&samples)
}

/// The options of a run over `export_dir` writing every image into `out_dir`.
fn all_images_options(export_dir: &Path, out_dir: &Path) -> WaterImageOptions {
    WaterImageOptions {
        export_dir: export_dir.to_path_buf(),
        out_dir: Some(out_dir.to_path_buf()),
        mode: WaterImageMode::All,
        terrain: "fixture".to_string(),
        dem_path: None,
        inland_only: false,
        resolution_m_per_px: None,
        region_of_interest: None,
        vector_enhance: true,
    }
}

#[test]
fn synthetic_export_draws_the_reference_pixels() {
    let folder = TemporaryFolder::new("golden");
    let export_dir = folder.0.join("export");
    let out_dir = folder.0.join("images");
    std::fs::create_dir_all(&export_dir).expect("create the export folder");
    write_synthetic_export(&export_dir);

    let exit_code = run(&all_images_options(&export_dir, &out_dir)).expect("run the lane");

    assert_eq!(exit_code, 0);
    for (name, digest) in GOLDEN_DIGESTS {
        assert_eq!(
            decoded_pixel_digest(&out_dir.join(name)),
            digest,
            "decoded pixels of {name}"
        );
    }
    let mut written: Vec<String> = std::fs::read_dir(&out_dir)
        .expect("list the images")
        .map(|entry| {
            entry
                .expect("an image entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    written.sort();
    let mut expected: Vec<String> = GOLDEN_DIGESTS
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    expected.sort();
    assert_eq!(written, expected, "exactly the five images, no duplicate");
}

#[test]
fn an_export_without_metadata_is_refused() {
    let folder = TemporaryFolder::new("no-metadata");
    let error = run(&all_images_options(&folder.0, &folder.0.join("images")))
        .expect_err("no metadata file");
    assert!(
        error
            .to_string()
            .contains("missing the water metadata file"),
        "{error}"
    );
}

#[test]
fn region_of_interest_reads_four_numbers() {
    assert_eq!(
        parse_region_of_interest("4000,5500,5500,7000"),
        Ok([4000.0, 5500.0, 5500.0, 7000.0])
    );
}

#[test]
fn region_of_interest_trims_spaces_around_each_number() {
    assert_eq!(
        parse_region_of_interest(" -12.5 , 0,\t3e2 ,4 "),
        Ok([-12.5, 0.0, 300.0, 4.0])
    );
}

#[test]
fn region_of_interest_refuses_three_values() {
    let error = parse_region_of_interest("1,2,3").expect_err("three values");
    assert!(error.contains("found 3"), "{error}");
}

#[test]
fn region_of_interest_refuses_text_and_non_finite_numbers() {
    assert!(parse_region_of_interest("1,2,NaN,4").is_err());
    assert!(parse_region_of_interest("1,2,inf,4").is_err());
    assert!(parse_region_of_interest("1,two,3,4").is_err());
    assert!(parse_region_of_interest("1,2,,4").is_err());
}

#[test]
fn water_and_bathymetry_modes_write_only_the_bathymetry() {
    for mode in [WaterImageMode::Water, WaterImageMode::Bathymetry] {
        let selection = mode.selection();
        assert!(selection.bathymetry);
        assert!(!selection.dark && !selection.depth16 && !selection.mask && !selection.preview);
    }
}

#[test]
fn all_mode_writes_every_image_and_each_single_mode_one() {
    let all = WaterImageMode::All.selection();
    assert!(all.bathymetry && all.dark && all.depth16 && all.mask && all.preview);
    assert!(WaterImageMode::Dark.selection().dark);
    assert!(WaterImageMode::Depth16.selection().depth16);
    assert!(WaterImageMode::Mask.selection().mask);
    assert!(WaterImageMode::Preview.selection().preview);
    assert!(!WaterImageMode::Preview.selection().bathymetry);
}
