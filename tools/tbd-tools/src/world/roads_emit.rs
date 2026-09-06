//! T-935.6 — the rkyv twin of `objects/roads.json.gz`.
//!
//! `build-roads` writes the road network twice: the gzip-9 JSON the shipped loader still fetches,
//! and a `roads/road_network.rkyv` [`RoadNetworkArchive`]. Dual emission is deliberate and stays
//! until T-935.11/.13 flip the URL — deleting either write before then blinds one reader.
//!
//! # Why the emitter centrelines instead of copying the export's quad soup
//!
//! `roads.json.gz` ships road *surface* quads: alternating cross-edge point pairs, ~10× the
//! vertices of the line the map actually draws. Every page load then paid for
//! [`extract_road_centerline`](map_engine_core::world::extract_road_centerline) over all of it on
//! the main thread. The archive stores the centrelined result, so the work happens once here.
//!
//! # Why it narrows the JSON rather than the builder's own model
//!
//! Same reason [`binary_emit`](super::binary_emit) does: the JSON round trip is not the identity.
//! `build_roads_from_topo` holds `f64` topo vertices, writes them through
//! [`round2`](super::jsval::round2) and `js_num`, and the loader centrelines *those*. Reading the
//! file that was just written puts the emitter on the loader's own chain
//! (`bytes_to_json` → `parse_roads_payload`), so the archive equals the JSON decode by
//! construction rather than by coincidence — including the drops: a segment the loader rejects
//! (unknown class, under two centreline vertices, a non-finite point) is absent from both.
//!
//! # Class codes
//!
//! `road_class` is a byte on the wire. The table is
//! [`road_class_code`](map_engine_core::world::road_class_code) /
//! [`road_class_name`](map_engine_core::world::road_class_name) in `map-engine-core` — *one*
//! table, linked by both the writer here and the reader in `world::roads`, so they cannot drift
//! into disagreeing about what a byte means. A class the table cannot code is a hard error here
//! and a hard error there; neither side invents a fallback.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use map_engine_core::world::binary::archives::{
    ARCHIVE_SCHEMA_VERSION, RoadNetworkArchive, RoadSegmentArchive,
};
use map_engine_core::world::binary::{access_checked, to_bytes};
use map_engine_core::world::{RoadSegment, bytes_to_json, parse_roads_payload, road_class_code};

use crate::serve::repo_root;

/// The gzip-JSON road export, relative to a terrain directory.
pub const ROADS_GZ: &str = "objects/roads.json.gz";

/// The rkyv road network, relative to a terrain directory. Matches the manifest's
/// `objects.binary.roads` path (`map_engine_core::world::ObjectsBinaryBlock`), which T-935.11
/// will point the SPA at.
pub const ROAD_NETWORK_RKYV: &str = "roads/road_network.rkyv";

/// Centrelined [`RoadSegment`]s → the wire archive, in file order.
///
/// # Errors
/// When a segment's class is not in the shared class table. That is fatal rather than a skip or a
/// zero byte: the reader treats an unnameable code as corruption (`world::roads::from_archive`),
/// so writing one would produce a file that only fails at load time, on the user's machine.
pub fn to_archive(segments: &[RoadSegment]) -> Result<RoadNetworkArchive> {
    let mut out = Vec::with_capacity(segments.len());
    for (i, s) in segments.iter().enumerate() {
        let code = road_class_code(&s.road_class);
        if code == 0 {
            bail!(
                "road segment {i} (id {:?}) has class {:?}, which the wire class table cannot \
                 code — refusing to write an archive the loader would reject",
                s.id,
                s.road_class
            );
        }
        out.push(RoadSegmentArchive {
            id: s.id.clone(),
            road_class: code,
            #[allow(clippy::cast_possible_truncation)]
            width_m: s.width_m as f32,
            centerline: s
                .points
                .iter()
                .map(|p| {
                    #[allow(clippy::cast_possible_truncation)]
                    [p[0] as f32, p[1] as f32]
                })
                .collect(),
        });
    }
    Ok(RoadNetworkArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        segments: out,
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
pub fn write_road_network_rkyv(path: &Path, archive: &RoadNetworkArchive) -> Result<usize> {
    let bytes = to_bytes(archive).map_err(|e| anyhow::anyhow!("{e}"))?;
    access_checked::<RoadNetworkArchive>(&bytes)
        .map_err(|e| anyhow::anyhow!("emitted archive fails its own validating read: {e}"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    std::fs::write(path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(bytes.len())
}

/// Read `objects/roads.json.gz` from `terrain_dir` and build the archive from it.
///
/// # Errors
/// When the JSON is missing or undecodable, or when it centrelines to nothing — an empty network
/// is refused rather than written, the same guard `build-roads` puts on the JSON itself (T-537).
pub fn build_road_network_archive(terrain_dir: &Path) -> Result<RoadNetworkArchive> {
    let src = terrain_dir.join(ROADS_GZ);
    let raw = std::fs::read(&src).with_context(|| format!("read {}", src.display()))?;
    let doc = bytes_to_json(&raw).map_err(|e| anyhow::anyhow!("{} decode: {e}", src.display()))?;
    let segments = parse_roads_payload(&doc);
    super::refuse_empty_write(
        "roads-rkyv",
        segments.is_empty(),
        "zero centrelined road segments — refusing to write an empty road_network.rkyv",
    )?;
    to_archive(&segments)
}

/// Resolve a `--terrain` value the way `build_roads_from_topo` resolves its `--out`: an explicit
/// base wins, otherwise `packages/map-assets/<terrain>`.
#[must_use]
pub fn resolve_terrain_dir(terrain: &str, out_base: Option<&Path>) -> PathBuf {
    out_base.map_or_else(
        || repo_root().join("packages/map-assets").join(terrain),
        Path::to_path_buf,
    )
}

/// Build and write `roads/road_network.rkyv` for one terrain directory. Returns the path written
/// and its size, so the caller can print both emitted paths.
///
/// # Errors
/// When the source JSON is missing/undecodable/empty, or the write fails.
pub fn emit_road_network(terrain_dir: &Path) -> Result<(PathBuf, usize)> {
    let archive = build_road_network_archive(terrain_dir)?;
    let out = terrain_dir.join(ROAD_NETWORK_RKYV);
    let bytes = write_road_network_rkyv(&out, &archive)?;
    Ok((out, bytes))
}

/// `world roads-rkyv --terrain <id> [--out <base>]` — the archive on its own, from the committed
/// JSON.
///
/// `build-roads` emits both files, but it needs the game's `.pak` VFS to decode the topo section,
/// so it cannot run in CI or on a machine without Arma installed. This entry point re-derives the
/// archive from the committed `roads.json.gz` alone, which is what makes the binary lane
/// reproducible in the repo rather than only on the export box.
///
/// # Errors
/// When the terrain directory or its road export is missing, or the emit fails.
pub fn emit_road_network_cli(terrain: &str, out_base: Option<&Path>) -> Result<u8> {
    let dir = resolve_terrain_dir(terrain, out_base);
    if !dir.is_dir() {
        eprintln!("roads-rkyv: no terrain directory at {}", dir.display());
        return Ok(1);
    }
    let (out, bytes) = emit_road_network(&dir)?;
    println!(
        "roads-rkyv: {} → {} ({bytes} bytes)",
        terrain,
        out.display()
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use map_engine_core::world::{WorldError, WorldStore};

    use super::*;

    /// The committed everon export ships 887 segments across five classes (`store.rs`'s census pin
    /// and `build::road_census` both say so). Re-pin deliberately if the export changes — a
    /// silently shrinking corpus is how a parity test stops proving anything.
    const EVERON_SEGMENTS: usize = 887;
    const EVERON_CLASSES: usize = 5;
    /// Centreline vertices summed over the whole island; a floor, not an equality, so an empty or
    /// LFS-pointered tree cannot pass while a legitimate re-export can.
    const EVERON_POINT_FLOOR: usize = 20_000;

    fn everon_dir() -> PathBuf {
        repo_root().join("packages/map-assets/everon")
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
        std::fs::copy(everon_dir().join(ROADS_GZ), terrain.join(ROADS_GZ))
            .expect("copy roads.json.gz");
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

    /// An empty road set is refused rather than written over a committed archive (T-537's rule,
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
}
