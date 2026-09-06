//! T-935.2 — the binary twin of every `objects/chunks/{cx}_{cy}.json.gz`.
//!
//! `build-objects` writes each chunk twice: the gzip-9 JSON the shipped loader still reads, and a
//! `TBDC`-framed `{cx}_{cy}.bin` of raw [`ObjectInstancePod`] rows. Dual emission is deliberate and
//! stays until T-935.13 flips the manifest — deleting either write before then blinds one reader.
//!
//! # Why the emitter narrows the JSON rows instead of the builder's own `f64`s
//!
//! The obvious emitter is `pod.x = chunk_row.x as f32`, straight off the in-memory row. It is
//! wrong, and the parity test is what proves it: the JSON round trip is not the identity on the
//! values this pipeline holds.
//!
//! * **Negative zero.** `jsval::js_num(-0.0)` takes the integral branch and prints `0`, so the
//!   loader reads `+0.0` (f32 bits `0x0000_0000`) where the builder held `-0.0` (bits
//!   `0x8000_0000`). Bit-equality would fail on a value both sides call "zero".
//! * **Five-wide rows.** [`chunk_row_values`](super::jsval::chunk_row_values) drops
//!   `pitch`/`roll`/`scale` whenever [`trailers_trivial`](super::jsval::trailers_trivial) holds, and
//!   the loader then *defaults* them (`0`, `0`, `1`). A `-0.0` roll is trivial by `==` but is not
//!   the `+0.0` the loader will produce.
//! * **Non-positive scale.** `narrow_instance_row_v2` clamps a `<= 0.0` scale to `1.0`; the raw
//!   row does not.
//!
//! So the emitter reads the *same* `Value` rows that are about to be gzipped, through the *same*
//! [`narrow_instance_row_v2`] the loader uses. The `.bin` is then equal to the `.json.gz` decode by
//! construction rather than by coincidence, including the skip rule: a row the narrow rejects is
//! dropped with no gap in either file, so row `i` means the same instance in both.
//!
//! # Class codes
//!
//! `class_code` is the one column the JSON chunk does not carry — the loader derives it from
//! `prefabs.json.gz`. [`class_code_table`] derives it from the very document that is about to be
//! written, through `narrow_prefab_rows` + `build_prefab_maps`, i.e. the loader's own chain. The
//! parity test therefore pins the *indexing* (row `i` gets prefab `i`'s code), not the taxonomy —
//! `classify.rs`'s own tests own that.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use map_engine_core::world::binary::chunk_container::{ContainerHeader, TbdcHeader};
use map_engine_core::world::binary::pod::{ObjectInstancePod, instances_to_bytes};
use map_engine_core::world::{
    NO_CLASS, build_prefab_maps, narrow_instance_row_v2, narrow_prefab_rows,
};

/// Prefab id (`pid.to_bits()`, the loader's key) → render-class code, for the prefab catalogue
/// document `build-objects` is about to write.
///
/// Keyed by the f64 bit pattern rather than an index so it matches
/// [`build_prefab_maps`](map_engine_core::world::build_prefab_maps) exactly: a chunk row's `pid`
/// is an f64 and a `Vec` index would silently truncate a non-integral or out-of-range one.
#[must_use]
pub fn class_code_table(prefabs_doc: &Value) -> HashMap<u64, u8> {
    let (by_id, _has_oversized) = build_prefab_maps(narrow_prefab_rows(prefabs_doc));
    by_id.into_iter().map(|(k, e)| (k, e.code)).collect()
}

/// Narrow the chunk's JSON rows to wire rows, in order, skipping exactly what the loader skips.
///
/// `class_by_pid` comes from [`class_code_table`]; an unknown prefab takes
/// [`NO_CLASS`](map_engine_core::world::NO_CLASS), which is what `parse_chunk` stores.
#[must_use]
pub fn pods_from_rows(rows: &[Value], class_by_pid: &HashMap<u64, u8>) -> Vec<ObjectInstancePod> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(r) = narrow_instance_row_v2(row) else {
            continue;
        };
        out.push(ObjectInstancePod {
            x: r.x as f32,
            y: r.y as f32,
            z: r.z as f32,
            yaw: r.rot as f32,
            pitch: r.pitch as f32,
            roll: r.roll as f32,
            scale: r.scale as f32,
            // `pid as u16` is the loader's `prefab_idx` store verbatim (JS `Uint16Array`); everon
            // pids top out at 1623, three orders of magnitude below the wrap.
            prefab_id: r.pid as u16,
            class_code: class_by_pid
                .get(&r.pid.to_bits())
                .copied()
                .unwrap_or(NO_CLASS),
            _pad: 0,
        });
    }
    out
}

/// Write `objects/chunks/{cx}_{cy}.bin`: a 32-byte `TBDC` header then `rows.len()` 32-byte
/// [`ObjectInstancePod`]s, little-endian and uncompressed. File length is exactly
/// `32 + 32 * rows.len()`.
///
/// # Errors
/// When `cx`/`cy` do not fit the header's `i16`, when the row count does not fit its `u32`, or when
/// the write fails. The two range checks are `try_from`, not `as`: a lossy cast would stamp a
/// wrapped chunk index into a file whose *name* still carried the right one, and every later reader
/// would trust the header.
pub fn write_chunk_bin(path: &Path, cx: i64, cy: i64, rows: &[ObjectInstancePod]) -> Result<()> {
    let cx16 = i16::try_from(cx).with_context(|| format!("chunk cx {cx} does not fit TBDC i16"))?;
    let cy16 = i16::try_from(cy).with_context(|| format!("chunk cy {cy} does not fit TBDC i16"))?;
    let count = u32::try_from(rows.len())
        .with_context(|| format!("chunk {cx}_{cy} has {} rows, over u32", rows.len()))?;
    let header = TbdcHeader::new(cx16, cy16, count);
    let mut buf = Vec::with_capacity(header.file_bytes().unwrap_or(0));
    buf.extend_from_slice(&header.to_header_bytes());
    buf.extend_from_slice(instances_to_bytes(rows));
    std::fs::write(path, &buf).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use map_engine_core::world::binary::chunk_container::HEADER_BYTES;
    use map_engine_core::world::binary::pod::POD_BYTES;
    use map_engine_core::world::{bytes_to_json, parse_chunk};
    use serde_json::json;

    use super::*;
    use crate::serve::repo_root;

    /// Every committed everon chunk. Re-pin deliberately if the export ever changes shape — a
    /// silently shrinking corpus is how a parity test stops proving anything.
    const EVERON_CHUNKS: usize = 315;
    /// `chunks/manifest.json` `instanceCount` sum at 2026-09-06. A floor, not an equality: it only
    /// has to make an empty / LFS-pointer tree impossible to pass.
    const EVERON_INSTANCE_FLOOR: usize = 1_200_000;

    fn objects_dir() -> PathBuf {
        repo_root().join("packages/map-assets/everon/objects")
    }

    /// A copy of `bytes` whose first byte sits on a 4-byte boundary, so the zero-copy
    /// `cast_slice` path is exercised **deterministically** rather than whenever the allocator
    /// happens to oblige. `fs::read` hands back a `Vec<u8>`, which is only 1-aligned by contract;
    /// the loader (T-935.3) faces the same problem on `fetch_bytes` and answers it the same way.
    fn aligned4(bytes: &[u8]) -> (Vec<u8>, usize) {
        let mut buf: Vec<u8> = Vec::with_capacity(bytes.len() + 4);
        let pad = buf.as_ptr().align_offset(4);
        assert!(pad < 4, "align_offset could not resolve a real pointer");
        buf.resize(pad, 0);
        buf.extend_from_slice(bytes);
        assert_eq!(buf[pad..].as_ptr().align_offset(4), 0, "pad missed");
        (buf, pad)
    }

    /// The independent decoder: fixed byte offsets and `from_le_bytes`, touching neither
    /// `ObjectInstancePod` nor `bytemuck`.
    ///
    /// This is the oracle that matters. A `cast_slice` decode shares the struct with the emitter,
    /// so a field **reorder** — `yaw` and `pitch` swapped in the declaration, say — would move both
    /// sides together and the cast test would stay green over a broken file. Byte offsets cannot
    /// move with it.
    fn decode_row_by_offset(b: &[u8]) -> ([f32; 7], u16, u8, u8) {
        let f = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        (
            [f(0), f(4), f(8), f(12), f(16), f(20), f(24)],
            u16::from_le_bytes([b[28], b[29]]),
            b[30],
            b[31],
        )
    }

    fn chunk_files() -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(objects_dir().join("chunks"))
            .expect("everon objects/chunks")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.to_string_lossy().ends_with(".json.gz"))
            .collect();
        files.sort();
        files
    }

    fn chunk_key(p: &Path) -> String {
        p.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".json.gz"))
            .expect("chunk file name")
            .to_string()
    }

    fn key_xy(key: &str) -> (i64, i64) {
        let mut it = key.split('_').map(|v| v.parse::<i64>().unwrap_or(0));
        (it.next().unwrap_or(0), it.next().unwrap_or(0))
    }

    /// THE SLICE'S PIN. For all 315 committed everon chunks: emit the `.bin`, then prove its
    /// decode is column-for-column what `parse_chunk` reads out of the `.json.gz`.
    ///
    /// f32s compare by `to_bits()`, not `==`: `==` would call `-0.0` equal to `+0.0` and `NaN`
    /// unequal to itself, and the whole point of a binary twin is that the bytes agree.
    #[test]
    fn every_everon_chunk_bin_decodes_to_the_json_columns() {
        let objects = objects_dir();
        let prefabs_raw = std::fs::read(objects.join("prefabs.json.gz")).expect("prefabs.json.gz");
        let prefabs_doc = bytes_to_json(&prefabs_raw).expect("prefabs decode");
        let (prefab_by_id, _) = build_prefab_maps(narrow_prefab_rows(&prefabs_doc));
        let class_by_pid = class_code_table(&prefabs_doc);
        assert!(!class_by_pid.is_empty(), "empty prefab catalogue");

        let files = chunk_files();
        assert_eq!(
            files.len(),
            EVERON_CHUNKS,
            "everon chunk corpus changed; re-pin EVERON_CHUNKS deliberately"
        );

        let dir = std::env::temp_dir().join(format!("t935-2-binparity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tempdir");

        let mut total_rows = 0usize;
        for path in &files {
            let key = chunk_key(path);
            let (cx, cy) = key_xy(&key);
            let raw = bytes_to_json(&std::fs::read(path).expect("read chunk")).expect(&key);
            let rows = raw
                .get("instances")
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("{key}: no instances array"));

            let pods = pods_from_rows(rows, &class_by_pid);
            let bin = dir.join(format!("{key}.bin"));
            write_chunk_bin(&bin, cx, cy, &pods).expect("write bin");

            let bytes = std::fs::read(&bin).expect("read bin");
            let oracle = parse_chunk(&key, &raw, &prefab_by_id).unwrap_or_else(|| panic!("{key}"));
            let n = oracle.count as usize;
            total_rows += n;

            assert_eq!(
                bytes.len(),
                HEADER_BYTES + POD_BYTES * n,
                "{key}: file length must be 32 + 32 x count"
            );

            let (buf, pad) = aligned4(&bytes);
            let (head, payload) =
                TbdcHeader::parse(&buf[pad..]).unwrap_or_else(|e| panic!("{key}: {e}"));
            assert_eq!(head.count as usize, n, "{key}: header count");
            assert_eq!(f64::from(head.cx), oracle.cx, "{key}: header cx");
            assert_eq!(f64::from(head.cy), oracle.cy, "{key}: header cy");
            assert_eq!(head.flags, 0, "{key}: flags");
            assert_eq!(head.reserved, [0u8; 16], "{key}: reserved");

            // The shipped zero-copy path (`cast_slice`, what the loader will call) …
            let insts = head
                .instances(payload)
                .unwrap_or_else(|e| panic!("{key}: {e}"));
            assert_eq!(insts.len(), n, "{key}: cast row count");

            for (i, inst) in insts.iter().enumerate() {
                // … cross-checked against the offset decoder, so a Pod field reorder cannot hide.
                let (fs_, pid, cls, pad_b) = decode_row_by_offset(&payload[i * POD_BYTES..]);
                let at = format!("{key}[{i}]");
                assert_eq!(fs_[0].to_bits(), inst.x.to_bits(), "{at}: offset/cast x");
                assert_eq!(
                    fs_[3].to_bits(),
                    inst.yaw.to_bits(),
                    "{at}: offset/cast yaw"
                );
                assert_eq!(pid, inst.prefab_id, "{at}: offset/cast prefab_id");
                assert_eq!(cls, inst.class_code, "{at}: offset/cast class_code");

                assert_eq!(
                    fs_[0].to_bits(),
                    oracle.positions[2 * i].to_bits(),
                    "{at}: x"
                );
                assert_eq!(
                    fs_[1].to_bits(),
                    oracle.positions[2 * i + 1].to_bits(),
                    "{at}: y"
                );
                assert_eq!(fs_[2].to_bits(), oracle.z[i].to_bits(), "{at}: z");
                assert_eq!(fs_[3].to_bits(), oracle.rotations[i].to_bits(), "{at}: yaw");
                assert_eq!(fs_[4].to_bits(), oracle.pitch[i].to_bits(), "{at}: pitch");
                assert_eq!(fs_[5].to_bits(), oracle.roll[i].to_bits(), "{at}: roll");
                assert_eq!(fs_[6].to_bits(), oracle.scale[i].to_bits(), "{at}: scale");
                assert_eq!(pid, oracle.prefab_idx[i], "{at}: prefab_id");
                assert_eq!(cls, oracle.cls_codes[i], "{at}: class_code");
                assert_eq!(pad_b, 0, "{at}: _pad must be zero");
            }
        }

        assert!(
            total_rows >= EVERON_INSTANCE_FLOOR,
            "only {total_rows} instances compared — the corpus is empty or LFS-pointered, \
             so this test proved nothing"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The 5-wide → identity-trailer rule and the `-0.0` normalisation the module doc warns about,
    /// pinned on a hand-built row so the failure names the cause instead of a chunk index.
    #[test]
    fn five_wide_row_takes_identity_trailers_and_positive_zero() {
        let rows = vec![json!([7, -0.0, 2.5, 0, -0.0])];
        let pods = pods_from_rows(&rows, &HashMap::new());
        assert_eq!(pods.len(), 1);
        assert_eq!(pods[0].pitch.to_bits(), 0.0f32.to_bits());
        assert_eq!(pods[0].roll.to_bits(), 0.0f32.to_bits());
        assert_eq!(pods[0].scale, 1.0);
        assert_eq!(pods[0].class_code, NO_CLASS, "unknown prefab is NO_CLASS");
        // `-0.0` survives as `-0.0` here because the JSON literally said `-0.0`; what matters is
        // that the emitter and `parse_chunk` agree, and they read the same token.
        assert_eq!(pods[0].x.to_bits(), (-0.0f32).to_bits());
    }

    /// A row the loader rejects must not reach the `.bin` either, and must not leave a gap.
    #[test]
    fn rejected_rows_are_skipped_with_no_gap() {
        let rows = vec![
            json!([1, 10.0, 20.0, 0, 0]),
            json!([2, "nope", 20.0, 0, 0]),
            json!([3, 30.0, 40.0, 0, 0]),
        ];
        let pods = pods_from_rows(&rows, &HashMap::new());
        assert_eq!(pods.len(), 2, "the malformed row must be dropped");
        assert_eq!(pods[0].prefab_id, 1);
        assert_eq!(pods[1].prefab_id, 3, "no gap: row 3 moves up to index 1");
    }

    /// An empty chunk is a real everon chunk (open ocean). It must still be a valid 32-byte frame,
    /// not a zero-length file.
    #[test]
    fn empty_chunk_is_a_bare_header() {
        let dir = std::env::temp_dir().join(format!("t935-2-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tempdir");
        let p = dir.join("0_0.bin");
        write_chunk_bin(&p, -3, 4, &[]).expect("write");
        let bytes = std::fs::read(&p).expect("read");
        assert_eq!(bytes.len(), HEADER_BYTES);
        let (buf, pad) = aligned4(&bytes);
        let (head, payload) = TbdcHeader::parse(&buf[pad..]).expect("frame");
        assert_eq!((head.cx, head.cy, head.count), (-3, 4, 0));
        assert!(head.instances(payload).expect("zero rows").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A chunk index outside `i16` is an error, never a wrapped header. `as i16` would write
    /// `cx = -32768` into a file called `32768_0.bin` and every reader downstream would believe it.
    #[test]
    fn out_of_range_chunk_index_is_an_error_not_a_wrap() {
        let dir = std::env::temp_dir().join(format!("t935-2-range-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tempdir");
        let p = dir.join("big.bin");
        let err = write_chunk_bin(&p, 32_768, 0, &[]).expect_err("must refuse");
        assert!(
            format!("{err:#}").contains("does not fit TBDC i16"),
            "{err:#}"
        );
        assert!(!p.exists(), "nothing may be written for a rejected index");
        assert!(write_chunk_bin(&p, 0, -32_769, &[]).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
