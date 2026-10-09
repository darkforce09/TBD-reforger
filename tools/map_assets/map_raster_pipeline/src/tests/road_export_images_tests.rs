//! The golden test of the road export image lane.
//!
//! **Role:** runs [`super::run`] over a tiny synthetic road export (a 640 m world on a 64 px
//! canvas, all six layers, 2-D and 3-D points, a falsy and an explicit width, a one-point segment,
//! junctions of every degree rule) and compares the SHA-256 of every image's decoded samples with
//! the reference digests of the same export.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by `road_export_images.rs`.
//! **Signals & state:** the case writes its export and images into its own folder under the
//! system temporary folder and removes it when done.
//! **Invariants:** the decoder applies no transformation, so the digest covers the stored samples,
//! colour type and bit depth exactly; the export files are the exact JSON text the reference
//! images were drawn from.

use std::path::{Path, PathBuf};

use super::{RoadImageOptions, run};

/// The export files of the fixture, as `(file name, JSON text)`.
const FIXTURE_FILES: [(&str, &str); 7] = [
    (
        "roads_meta.json",
        r#"{"mapName":"fixture","worldSizeM":640,"totalSegments":7,"junctions":[{"pos":[100,0,100],"degree":3},{"pos":[300,0,320],"connectedSegments":[1,2,3]},{"pos":[500,0,500],"degree":2},{"pos":[50,600],"degree":4}]}"#,
    ),
    (
        "highways.json",
        r#"{"totalLengthM":1,"segments":[{"widthM":8,"points":[[20,0,20],[320,0,330],[620,0,600]]}]}"#,
    ),
    (
        "roads_paved.json",
        r#"{"totalLengthM":1,"segments":[{"points":[[40,600],[600,40]]}]}"#,
    ),
    (
        "roads_dirt.json",
        r#"{"totalLengthM":1,"segments":[{"widthM":30,"points":[[100,0,320],[110,0,322],[500,0,320]]},{"points":[[5,0,5]]}]}"#,
    ),
    (
        "tracks.json",
        r#"{"totalLengthM":1,"segments":[{"points":[[320,0,20],[321,0,21],[320,0,620]]}]}"#,
    ),
    (
        "paths.json",
        r#"{"totalLengthM":1,"segments":[{"widthM":0,"points":[[60,0,60],[200,0,90],[260,0,260]]}]}"#,
    ),
    (
        "runways.json",
        r#"{"totalLengthM":1,"segments":[{"points":[[400,0,100],[600,0,140]]}]}"#,
    ),
];

/// Each image the fixture run writes, with its colour type and the SHA-256 of its decoded samples.
const EXPECTED_IMAGES: [(&str, png::ColorType, &str); 8] = [
    (
        "fixture-roads-dark.png",
        png::ColorType::Rgb,
        "8821673ae06ce8a53602136fcfd6b37d086dd5ff98c9e17b3b0d005538b5255f",
    ),
    (
        "fixture-roads-transparent.png",
        png::ColorType::Rgba,
        "751447898f8fcc8dc7896017b304a180b26ced62e1ac8dce122646f91957ef78",
    ),
    (
        "layer-highways.png",
        png::ColorType::Rgba,
        "bc28b448723ce1ba874a884691249aec8d3d1625a0cb10a95051f8f4f66a9ad3",
    ),
    (
        "layer-paths.png",
        png::ColorType::Rgba,
        "0d86e317d89056f7c1060672c48efd15b67383a96a92e2a291627d2c11107331",
    ),
    (
        "layer-roads_dirt.png",
        png::ColorType::Rgba,
        "e06e398432e462210818e095dafd414ed14b8fe683c10716355193838d5b4e45",
    ),
    (
        "layer-roads_paved.png",
        png::ColorType::Rgba,
        "3b898ad8699e4a2bbc197e1328448a664108ea499929fb1c0f6751bec66d8a67",
    ),
    (
        "layer-runways.png",
        png::ColorType::Rgba,
        "30233484c7d1a474c590b1547135df9f435a762de1f3f2d326576e0a1a1d179a",
    ),
    (
        "layer-tracks.png",
        png::ColorType::Rgba,
        "06811d9c48f489bcda1983ea435c641447e57ec7c17c1c6930a98d4526d5e0e2",
    ),
];

/// A temporary folder holding the fixture export and its images, removed on drop.
struct FixtureFolder(PathBuf);

impl FixtureFolder {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "tbd-road-export-images-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let roads = path.join("roads");
        std::fs::create_dir_all(&roads).expect("create the fixture roads folder");
        for (name, text) in FIXTURE_FILES {
            std::fs::write(roads.join(name), text).expect("write a fixture file");
        }
        Self(path)
    }
}

impl Drop for FixtureFolder {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// The colour type, bit depth, size and decoded samples of the PNG at `path`.
fn decode_png(path: &Path) -> (png::ColorType, png::BitDepth, (u32, u32), Vec<u8>) {
    let file = std::fs::File::open(path).expect("open an output image");
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().expect("read the PNG header");
    let mut samples = vec![0_u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut samples).expect("decode the PNG");
    samples.truncate(frame.buffer_size());
    let info = reader.info();
    (
        info.color_type,
        info.bit_depth,
        (info.width, info.height),
        samples,
    )
}

#[test]
fn fixture_export_draws_the_reference_images() {
    let folder = FixtureFolder::new();
    let out_dir = folder.0.join("images");
    let options = RoadImageOptions {
        roads_dir: folder.0.join("roads"),
        out_dir: Some(out_dir.clone()),
        size_px: 64,
        terrain: "fixture".to_string(),
        show_junctions: true,
    };
    assert_eq!(run(&options).expect("run road-images"), 0);

    let mut written: Vec<String> = std::fs::read_dir(&out_dir)
        .expect("list the output folder")
        .map(|entry| {
            entry
                .expect("an output entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    written.sort();
    let expected_names: Vec<&str> = EXPECTED_IMAGES.iter().map(|(name, _, _)| *name).collect();
    assert_eq!(written, expected_names);

    for (name, colour_type, digest) in EXPECTED_IMAGES {
        let (decoded_colour_type, bit_depth, size, samples) = decode_png(&out_dir.join(name));
        assert_eq!(decoded_colour_type, colour_type, "{name}");
        assert_eq!(bit_depth, png::BitDepth::Eight, "{name}");
        assert_eq!(size, (64, 64), "{name}");
        assert_eq!(content_digest::sha256_hex(&samples), digest, "{name}");
    }
}

#[test]
fn missing_roads_folder_is_refused() {
    let options = RoadImageOptions {
        roads_dir: std::env::temp_dir().join(format!(
            "tbd-road-export-images-missing-{}",
            std::process::id()
        )),
        out_dir: None,
        size_px: 64,
        terrain: "fixture".to_string(),
        show_junctions: false,
    };
    let error = run(&options).expect_err("a missing folder is refused");
    assert!(
        error.to_string().contains("roads folder not found"),
        "{error}"
    );
}
