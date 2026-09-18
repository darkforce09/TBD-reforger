use std::path::PathBuf;

use serde_json::json;
use website_map_engine::io::containers::header::HEADER_BYTES;
use website_map_engine::io::pod::instance::POD_BYTES;
use website_map_engine::streaming::loaders::chunk::parse_chunk;
use website_map_engine::streaming::loaders::store::bytes_to_json;

use super::*;
use crate::browser_testing::server::repo_root;

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
