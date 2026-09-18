use website_map_engine::world::environment::locations::route_labels::build_road_label_draw_set_from_archive;
use website_map_engine::world::environment::locations::route_labels::parse_road_names_json;
use website_map_engine::world::environment::locations::route_labels::road_names_from_archive;
use website_map_engine::world::environment::locations::route_placement::build_road_label_draw_set;
use website_map_engine::world::environment::locations::towns::height_labels_from_archive;
use website_map_engine::world::environment::locations::towns::locations_to_label_specs;
use website_map_engine::world::environment::locations::towns::towns_from_archive;
use website_map_engine::world::terrain::roads::network::RoadSegment;

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
    crate::repository_layout::terrain_dir(&repo_root(), "everon")
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
        drawn_totals.contains(&0) && drawn_totals.windows(2).any(|w| w[0] != w[1]),
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
    let a = access_checked::<MapLabelsArchive>(&aligned[pad..]).expect("access the file on disk");
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
