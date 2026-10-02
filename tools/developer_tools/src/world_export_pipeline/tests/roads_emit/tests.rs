use std::collections::BTreeMap;

use website_map_engine::streaming::loaders::store::WorldError;
use website_map_engine::streaming::loaders::store::WorldStore;

use super::*;

/// The committed everon export ships 887 segments across five classes (`store.rs`'s census pin
/// and `chunk_partitioner::road_census` both say so). Re-pin deliberately if the export changes — a
/// silently shrinking corpus is how a parity test stops proving anything.
const EVERON_SEGMENTS: usize = 887;
const EVERON_CLASSES: usize = 5;
/// Centreline vertices summed over the whole island; a floor, not an equality, so an empty or
/// LFS-pointered tree cannot pass while a legitimate re-export can.
const EVERON_POINT_FLOOR: usize = 20_000;

fn everon_dir() -> PathBuf {
    terrain_dir(&repo_root(), "everon")
}

fn everon_json_segments() -> Vec<RoadSegment> {
    let raw = std::fs::read(everon_dir().join(ROADS_GZ)).expect("everon roads.json.gz");
    parse_roads_payload(&bytes_to_json(&raw).expect("roads decode"))
}

fn tmp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("t935-6-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("tempdir");
    d
}

/// THE SLICE'S PIN. Emit the everon archive from the committed JSON, load it back through the
/// shipped public path (`WorldStore::load_roads`, magic sniff and all), and prove the two
/// networks are the same one: segment count, ids, classes, widths and points.
///
/// f32s compare by `to_bits()`, not `==`: `==` calls `-0.0` equal to `+0.0` and `NaN` unequal
/// to itself, and the point of a binary twin is that the *bytes* agree.
///
/// The oracle is the JSON network narrowed to f32 — `f64::from(v as f32)` — because that is
/// the archive's actual contract (`archives.rs`: the wire row is the compact f32 twin of the
/// parser struct). Stating it as `==` on the f64 would be a test of a promise nothing makes.
/// `archive_stays_within_f32_of_the_json_metres` below is what keeps that projection honest.
#[test]
fn everon_archive_equals_the_json_road_network() {
    let json = everon_json_segments();
    assert_eq!(
        json.len(),
        EVERON_SEGMENTS,
        "everon road corpus changed; re-pin EVERON_SEGMENTS deliberately"
    );

    let dir = tmp_dir("parity");
    let out = dir.join(ROAD_NETWORK_RKYV);
    let size = write_road_network_rkyv(&out, &to_archive(&json).expect("to_archive"))
        .expect("write archive");
    assert!(size > 0, "empty archive file");

    let bytes = std::fs::read(&out).expect("read back archive");
    assert!(
        bytes_to_json(&bytes).is_err(),
        "the archive must not be parseable as JSON, or this test has not proven the rkyv \
         route ran"
    );
    assert_ne!(&bytes[..2], &[0x1f, 0x8b], "must not carry gzip magic");

    let mut store = WorldStore::new();
    assert_eq!(
        store.load_roads(&bytes).expect("load archive"),
        EVERON_SEGMENTS
    );

    let mut points = 0usize;
    let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, (a, j)) in store.roads.iter().zip(json.iter()).enumerate() {
        assert_eq!(a.id, j.id, "segment {i}: id");
        assert_eq!(a.road_class, j.road_class, "segment {i}: class");
        #[allow(clippy::cast_possible_truncation)]
        let want_w = f64::from(j.width_m as f32);
        assert_eq!(
            a.width_m.to_bits(),
            want_w.to_bits(),
            "segment {i} ({}): width",
            j.id
        );
        assert_eq!(
            a.points.len(),
            j.points.len(),
            "segment {i} ({}): centerline vertex count",
            j.id
        );
        for (k, (pa, pj)) in a.points.iter().zip(j.points.iter()).enumerate() {
            for axis in 0..2 {
                #[allow(clippy::cast_possible_truncation)]
                let want = f64::from(pj[axis] as f32);
                assert_eq!(
                    pa[axis].to_bits(),
                    want.to_bits(),
                    "segment {i} ({}) point {k} axis {axis}",
                    j.id
                );
            }
        }
        points += a.points.len();
        *classes.entry(a.road_class.as_str()).or_default() += 1;
    }
    assert_eq!(classes.len(), EVERON_CLASSES, "class spread: {classes:?}");
    assert_eq!(
        classes.values().sum::<usize>(),
        EVERON_SEGMENTS,
        "every segment lands in exactly one class"
    );
    assert!(
        points >= EVERON_POINT_FLOOR,
        "only {points} centreline vertices compared — the corpus is empty or LFS-pointered, \
         so this test proved nothing"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The f32 projection has to be a *narrowing*, not a change of answer. Everon is 12.8 km wide
/// and f32 carries ~7 significant digits, so no coordinate may move by as much as a
/// millimetre. Without this, the parity test above would happily bless an emitter that halved
/// every coordinate — both sides would agree on the wrong number.
#[test]
fn archive_stays_within_f32_of_the_json_metres() {
    let json = everon_json_segments();
    let archive = to_archive(&json).expect("to_archive");
    let mut worst = 0.0_f64;
    let mut worst_w = 0.0_f64;
    for (a, j) in archive.segments.iter().zip(json.iter()) {
        worst_w = worst_w.max((f64::from(a.width_m) - j.width_m).abs());
        for (pa, pj) in a.centerline.iter().zip(j.points.iter()) {
            worst = worst
                .max((f64::from(pa[0]) - pj[0]).abs())
                .max((f64::from(pa[1]) - pj[1]).abs());
        }
    }
    assert!(worst < 1e-3, "worst coordinate drift {worst} m (>= 1 mm)");
    assert!(worst_w < 1e-3, "worst width drift {worst_w} m (>= 1 mm)");
    // …and it is a real narrowing, not a no-op over integers: something must have moved.
    assert!(worst > 0.0, "no coordinate lost any precision — suspicious");
}

/// `emit_road_network` writes the manifest's path, under the terrain directory, creating the
/// `roads/` directory that no export makes.
#[test]
fn emit_writes_the_manifest_path_and_creates_its_directory() {
    let dir = tmp_dir("emit");
    let terrain = dir.join("everon");
    std::fs::create_dir_all(terrain.join("objects")).expect("objects dir");
    std::fs::copy(everon_dir().join(ROADS_GZ), terrain.join(ROADS_GZ)).expect("copy roads.json.gz");
    assert!(!terrain.join("roads").exists(), "precondition");

    let (out, bytes) = emit_road_network(&terrain).expect("emit");
    assert_eq!(out, terrain.join("roads/road_network.rkyv"));
    assert_eq!(std::fs::metadata(&out).expect("stat").len() as usize, bytes);

    let mut store = WorldStore::new();
    assert_eq!(
        store
            .load_roads(&std::fs::read(&out).expect("read"))
            .expect("load"),
        EVERON_SEGMENTS
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A class the wire table cannot code is refused at write time, not encoded as a zero byte
/// that the loader would later reject on the user's machine.
#[test]
fn unknown_class_is_refused_at_write_time() {
    let seg = RoadSegment {
        id: "r0".to_string(),
        road_class: "hyperloop".to_string(),
        points: vec![[0.0, 0.0], [1.0, 1.0]],
        width_m: 3.0,
    };
    let err = to_archive(&[seg]).expect_err("must refuse");
    let msg = format!("{err:#}");
    assert!(msg.contains("hyperloop"), "{msg}");
    assert!(msg.contains("cannot code"), "{msg}");
}

/// An empty road set is refused rather than written over a committed archive (the refuse-empty rule,
/// applied to the binary lane).
#[test]
fn empty_road_json_is_refused() {
    let dir = tmp_dir("empty");
    std::fs::create_dir_all(dir.join("objects")).expect("objects dir");
    std::fs::write(
        dir.join(ROADS_GZ),
        br#"{"schemaVersion":"1.0.0","roadSegments":[]}"#,
    )
    .expect("write empty roads");
    let err = build_road_network_archive(&dir).expect_err("must refuse");
    let msg = format!("{err:#}");
    assert!(msg.contains("refusing empty write (roads-rkyv)"), "{msg}");
    assert!(!dir.join(ROAD_NETWORK_RKYV).exists(), "nothing written");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The CLI reports a missing terrain directory as exit 1, not a panic and not a success.
#[test]
fn cli_reports_a_missing_terrain_directory() {
    let missing = std::env::temp_dir().join("t935-6-does-not-exist");
    let _ = std::fs::remove_dir_all(&missing);
    assert_eq!(
        emit_road_network_cli("nowhere", Some(&missing)).expect("no panic"),
        1
    );
}

/// The sniff is reachable from this side too: the same `WorldStore` accepts the gzip JSON the
/// emitter reads and the archive it writes, and answers an empty buffer with an error.
#[test]
fn store_accepts_both_sources_and_refuses_nothing_at_all() {
    let gz = std::fs::read(everon_dir().join(ROADS_GZ)).expect("roads.json.gz");
    let mut store = WorldStore::new();
    assert_eq!(store.load_roads(&gz).expect("json route"), EVERON_SEGMENTS);
    assert!(matches!(
        store.load_roads(&[]),
        Err(WorldError::EmptyPayload)
    ));
}
