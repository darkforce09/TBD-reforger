//! T-935.7 — `locations/map_labels.rkyv`: the binary twin of the three cartographic label files.
//!
//! `map labels-rkyv` reads `locations.json`, `height-labels.json` and `road-names.json` and writes
//! one [`MapLabelsArchive`]. Dual emission, per spec §7 wave 2: the JSON files stay the source of
//! truth and stay on disk, the archive is derived from them, and the SPA only reads it once
//! T-935.13 puts a `labels` block in the manifest.
//!
//! # Why the road lane needs a fourth file
//!
//! `road-names.json` is a *curated route list* — name plus segment ids — while the archive's
//! `RoadNameLabel` is a **baked placement**: anchor, angle, class byte. Turning one into the other
//! needs the centreline geometry those segment ids point at, which lives in
//! `objects/roads.json.gz`. That file is read exactly the way the SPA reads it
//! (`bytes_to_json`/`parse_roads_payload` — the same chain behind `WorldStore::load_roads_gz`), so
//! the anchors the emitter bakes are the anchors `place_road_labels` would have produced in the
//! browser. The three JSON files remain the *sources*; roads.json.gz is the geometry they join to.
//!
//! # What is refused rather than approximated
//!
//! * A curated `minDeckZoom` the wire's class byte cannot express — see
//!   [`road_names_to_archive`](map_engine_core::world::road_names_to_archive).
//! * `road-names.json` present without `objects/roads.json.gz`: the alternative is an archive with
//!   a silently empty road lane that a loader would trust.
//! * An archive with nothing in any lane (`refuse_empty_write`, T-537/T-383).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use map_engine_core::world::binary::archives::{ARCHIVE_SCHEMA_VERSION, MapLabelsArchive};
use map_engine_core::world::binary::{access_checked, to_bytes};
use map_engine_core::world::{
    bytes_to_json, height_labels_to_archive, parse_height_labels_json, parse_locations_json,
    parse_road_names_json, parse_roads_payload, road_names_to_archive, towns_to_archive,
};

use crate::serve::repo_root;

/// Terrain-relative source paths.
pub const LOCATIONS_JSON: &str = "locations.json";
/// See [`LOCATIONS_JSON`].
pub const HEIGHT_LABELS_JSON: &str = "height-labels.json";
/// See [`LOCATIONS_JSON`].
pub const ROAD_NAMES_JSON: &str = "road-names.json";
/// The centreline geometry the curated road names join to.
pub const ROADS_GZ: &str = "objects/roads.json.gz";
/// Terrain-relative output path (spec §4).
pub const MAP_LABELS_RKYV: &str = "locations/map_labels.rkyv";

fn read_to_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}

/// Build the archive for one terrain directory.
///
/// # Errors
/// When a required source is missing or malformed, or when a curated road entry's visibility floor
/// cannot be encoded — see the module docs.
pub fn build_map_labels_archive(terrain_dir: &Path) -> Result<MapLabelsArchive> {
    let locations = parse_locations_json(&read_to_string(&terrain_dir.join(LOCATIONS_JSON))?)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let heights = parse_height_labels_json(&read_to_string(&terrain_dir.join(HEIGHT_LABELS_JSON))?)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let names_path = terrain_dir.join(ROAD_NAMES_JSON);
    let roads_path = terrain_dir.join(ROADS_GZ);
    let road_names = if names_path.exists() {
        if !roads_path.exists() {
            bail!(
                "{} exists but {} does not — the curated names cannot be anchored without the \
                 centreline geometry, and an empty road lane would be trusted by the loader",
                names_path.display(),
                roads_path.display()
            );
        }
        let names = parse_road_names_json(&read_to_string(&names_path)?)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let raw =
            std::fs::read(&roads_path).with_context(|| format!("read {}", roads_path.display()))?;
        let doc =
            bytes_to_json(&raw).map_err(|e| anyhow::anyhow!("{roads_path:?} decode: {e:?}"))?;
        let segments = parse_roads_payload(&doc);
        if segments.is_empty() {
            bail!("{} decoded to 0 road segments", roads_path.display());
        }
        road_names_to_archive(&names, &segments).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        Vec::new()
    };

    Ok(MapLabelsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        towns: towns_to_archive(&locations),
        height_labels: height_labels_to_archive(&heights),
        road_names,
    })
}

/// Serialise, **re-read through the validating reader**, then write. Returns the file size.
///
/// The read-back is not ceremony: `access_checked` is the only thing the SPA will ever use on this
/// file, so an archive that cannot survive it is a broken file whether or not `to_bytes` returned
/// `Ok`, and the emitter is the last place that can say so cheaply.
///
/// # Errors
/// When serialisation, validation or the write fails.
pub fn write_map_labels_rkyv(path: &Path, archive: &MapLabelsArchive) -> Result<usize> {
    let bytes = to_bytes(archive).map_err(|e| anyhow::anyhow!("{e}"))?;
    access_checked::<MapLabelsArchive>(&bytes)
        .map_err(|e| anyhow::anyhow!("emitted archive fails its own validating read: {e}"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    std::fs::write(path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(bytes.len())
}

/// Resolve the `--terrain` value: a directory path when it names one, else a terrain id under
/// `packages/map-assets`.
#[must_use]
pub fn terrain_dir(terrain: &str) -> PathBuf {
    let as_path = PathBuf::from(terrain);
    if as_path.is_dir() {
        return as_path;
    }
    repo_root().join("packages/map-assets").join(terrain)
}

/// `map labels-rkyv --terrain <id|dir>`.
///
/// # Errors
/// When the terrain directory is missing, or any step of the build/write fails.
pub fn emit_map_labels(terrain: &str) -> Result<u8> {
    let dir = terrain_dir(terrain);
    if !dir.is_dir() {
        eprintln!("labels-rkyv: no terrain directory at {}", dir.display());
        return Ok(1);
    }
    let archive = build_map_labels_archive(&dir)?;
    super::refuse_empty_write(
        "labels-rkyv",
        archive.towns.is_empty()
            && archive.height_labels.is_empty()
            && archive.road_names.is_empty(),
        "no towns, height labels or road names — refusing to write an empty map_labels.rkyv",
    )?;
    let out = dir.join(MAP_LABELS_RKYV);
    let bytes = write_map_labels_rkyv(&out, &archive)?;
    println!(
        "labels-rkyv: {} towns + {} height labels + {} road names → {} ({bytes} bytes)",
        archive.towns.len(),
        archive.height_labels.len(),
        archive.road_names.len(),
        out.display()
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use map_engine_core::world::{
        RoadSegment, build_road_label_draw_set, build_road_label_draw_set_from_archive,
        locations_to_label_specs, road_names_from_archive, towns_from_archive,
    };
    use map_engine_core::world::{height_labels_from_archive, parse_road_names_json};

    use super::*;

    /// Committed everon corpus at 2026-09-06. Floors, not equalities: they only have to make an
    /// empty / truncated source tree impossible to pass.
    const EVERON_TOWN_FLOOR: usize = 55;
    /// See [`EVERON_TOWN_FLOOR`].
    const EVERON_HEIGHT_FLOOR: usize = 20;
    /// See [`EVERON_TOWN_FLOOR`]. Six curated routes over eight segments.
    const EVERON_ROAD_NAME_FLOOR: usize = 6;
    /// The zoom ladder the editor actually spans (`peaks::HEIGHT_LABEL_MIN/MAX_ZOOM` is [-2, 3]),
    /// sampled either side of both road-name gates (0 and 1).
    const ZOOMS: [f64; 7] = [-2.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0];

    fn everon_dir() -> PathBuf {
        repo_root().join("packages/map-assets/everon")
    }

    /// A copy of `bytes` starting on a 16-byte boundary — what a loader must do with the
    /// 1-aligned `Vec<u8>` a file read hands back before `access_checked` will look at it.
    /// Hand-rolled rather than `rkyv::util::AlignedVec` so this crate needs no rkyv dependency.
    fn aligned16(bytes: &[u8]) -> (Vec<u8>, usize) {
        let mut buf: Vec<u8> = Vec::with_capacity(bytes.len() + 16);
        let pad = buf.as_ptr().align_offset(16);
        assert!(pad < 16, "align_offset could not resolve a real pointer");
        buf.resize(pad, 0);
        buf.extend_from_slice(bytes);
        assert_eq!(buf[pad..].as_ptr().align_offset(16), 0, "pad missed");
        (buf, pad)
    }

    fn everon_archive_bytes() -> impl AsRef<[u8]> {
        let archive = build_map_labels_archive(&everon_dir()).expect("build everon archive");
        assert!(
            archive.towns.len() >= EVERON_TOWN_FLOOR,
            "everon towns shrank to {}",
            archive.towns.len()
        );
        assert!(
            archive.height_labels.len() >= EVERON_HEIGHT_FLOOR,
            "everon height labels shrank to {}",
            archive.height_labels.len()
        );
        assert!(
            archive.road_names.len() >= EVERON_ROAD_NAME_FLOOR,
            "everon road names shrank to {}",
            archive.road_names.len()
        );
        to_bytes(&archive).expect("serialise")
    }

    fn everon_segments() -> Vec<RoadSegment> {
        let raw = std::fs::read(everon_dir().join(ROADS_GZ)).expect("roads.json.gz");
        parse_roads_payload(&bytes_to_json(&raw).expect("roads decode"))
    }

    /// THE SLICE'S PIN, lane 1. Every everon town survives the archive as the exact `f32`
    /// quantisation of its `locations.json` row, in order — and the packed label specs, which is
    /// what the glyph lane actually consumes, are equal outright.
    #[test]
    fn everon_towns_parse_the_same_from_either_source() {
        let json = parse_locations_json(
            &std::fs::read_to_string(everon_dir().join(LOCATIONS_JSON)).expect("locations.json"),
        )
        .expect("parse");
        let bytes = everon_archive_bytes();
        let back =
            towns_from_archive(access_checked::<MapLabelsArchive>(bytes.as_ref()).expect("access"));
        assert_eq!(back.len(), json.len());
        for (a, b) in back.iter().zip(&json) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.kind, b.kind, "{}", b.name);
            assert_eq!(a.x, f64::from(b.x as f32), "{}", b.name);
            assert_eq!(a.y, f64::from(b.y as f32), "{}", b.name);
            assert_eq!(a.importance, f64::from(b.importance as f32), "{}", b.name);
        }
        assert_eq!(
            locations_to_label_specs(&back),
            locations_to_label_specs(&json)
        );
    }

    /// Lane 2 — the spot heights. `value_m` is the label text, so it must be exact, not near.
    #[test]
    fn everon_height_labels_parse_the_same_from_either_source() {
        let json = parse_height_labels_json(
            &std::fs::read_to_string(everon_dir().join(HEIGHT_LABELS_JSON))
                .expect("height-labels.json"),
        )
        .expect("parse");
        let bytes = everon_archive_bytes();
        let back = height_labels_from_archive(
            access_checked::<MapLabelsArchive>(bytes.as_ref()).expect("access"),
        );
        assert_eq!(back.len(), json.len());
        for (a, b) in back.iter().zip(&json) {
            assert_eq!(a.value_m, b.value_m);
            assert_eq!(a.x, f64::from(b.x as f32));
            assert_eq!(a.y, f64::from(b.y as f32));
        }
    }

    /// Lane 3 — the one with no room on the wire for `priority`, `segment_id` or `minDeckZoom`.
    /// The oracle is the *rendered* road-name set: `build_road_label_draw_set` off the curated JSON
    /// versus the archive, at every zoom band, name for name and anchor for anchor.
    #[test]
    fn everon_road_labels_match_the_json_draw_set_at_every_zoom() {
        let names = parse_road_names_json(
            &std::fs::read_to_string(everon_dir().join(ROAD_NAMES_JSON)).expect("road-names.json"),
        )
        .expect("parse");
        let segments = everon_segments();
        assert!(
            segments.len() > 100,
            "everon roads shrank to {}",
            segments.len()
        );
        let bytes = everon_archive_bytes();
        let candidates = road_names_from_archive(
            access_checked::<MapLabelsArchive>(bytes.as_ref()).expect("access"),
        );

        let mut drawn_totals = Vec::new();
        for z in ZOOMS {
            let json = build_road_label_draw_set(&names, &segments, z);
            let arch = build_road_label_draw_set_from_archive(&candidates, z);
            assert_eq!(
                arch.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
                json.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
                "z={z}"
            );
            for (a, j) in arch.iter().zip(&json) {
                assert_eq!(a.x, f64::from(j.x as f32), "z={z} {}", j.name);
                assert_eq!(a.y, f64::from(j.y as f32), "z={z} {}", j.name);
                assert_eq!(
                    a.angle_deg,
                    f64::from(j.angle_deg as f32),
                    "z={z} {}",
                    j.name
                );
            }
            drawn_totals.push(json.len());
        }
        // The ladder must actually change: a lane that drew the same set everywhere would pass the
        // comparison above while proving nothing about the zoom gate the class byte encodes.
        assert!(
            drawn_totals.iter().any(|n| *n == 0) && drawn_totals.windows(2).any(|w| w[0] != w[1]),
            "road-name draw set is zoom-invariant across {ZOOMS:?}: {drawn_totals:?}"
        );
    }

    /// The curated `minDeckZoom: 0` routes are `road_paved`, whose class gate is z≥1 — so they are
    /// drawn in `0 ≤ z < 1` only because the override survived the wire. Pin that the override is
    /// load-bearing on the real corpus, so the parity test above cannot pass vacuously.
    #[test]
    fn everon_override_is_load_bearing_below_the_secondary_gate() {
        let names = parse_road_names_json(
            &std::fs::read_to_string(everon_dir().join(ROAD_NAMES_JSON)).expect("road-names.json"),
        )
        .expect("parse");
        let overridden: Vec<&str> = names
            .roads
            .iter()
            .filter(|e| e.min_deck_zoom.is_some())
            .map(|e| e.name.as_str())
            .collect();
        assert!(
            !overridden.is_empty(),
            "everon road-names.json has no override to test"
        );
        let drawn = build_road_label_draw_set(&names, &everon_segments(), 0.5);
        assert!(
            overridden
                .iter()
                .any(|n| drawn.iter().any(|l| l.name == *n)),
            "expected one of {overridden:?} at z=0.5; drew {:?}",
            drawn.iter().map(|l| &l.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_written_file_reads_back_through_access_checked() {
        let archive = build_map_labels_archive(&everon_dir()).expect("build");
        let out = std::env::temp_dir()
            .join(format!("tbd-t935-7-{}", std::process::id()))
            .join(MAP_LABELS_RKYV);
        let n = write_map_labels_rkyv(&out, &archive).expect("write");
        let disk = std::fs::read(&out).expect("read back");
        assert_eq!(disk.len(), n);
        // `fs::read` gives a 1-aligned Vec; copy onto a boundary the way a loader must, then
        // validate — the file on disk, not the buffer that produced it.
        let (aligned, pad) = aligned16(&disk);
        let a =
            access_checked::<MapLabelsArchive>(&aligned[pad..]).expect("access the file on disk");
        assert_eq!(a.towns.len(), archive.towns.len());
        assert_eq!(a.road_names.len(), archive.road_names.len());
        std::fs::remove_dir_all(out.parent().and_then(Path::parent).expect("temp dir")).ok();
    }

    #[test]
    fn road_names_without_geometry_are_refused() {
        let dir = std::env::temp_dir().join(format!("tbd-t935-7-nogeo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::copy(everon_dir().join(LOCATIONS_JSON), dir.join(LOCATIONS_JSON))
            .expect("copy locations");
        std::fs::copy(
            everon_dir().join(HEIGHT_LABELS_JSON),
            dir.join(HEIGHT_LABELS_JSON),
        )
        .expect("copy heights");
        std::fs::copy(
            everon_dir().join(ROAD_NAMES_JSON),
            dir.join(ROAD_NAMES_JSON),
        )
        .expect("copy road names");
        let err = build_map_labels_archive(&dir).expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(msg.contains("roads.json.gz"), "{msg}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn terrain_dir_takes_an_id_or_a_directory() {
        assert_eq!(terrain_dir("everon"), everon_dir());
        let here = repo_root();
        assert_eq!(terrain_dir(here.to_str().expect("utf8")), here);
    }
}
