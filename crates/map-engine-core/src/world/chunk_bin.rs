//! T-935.3 — `objects/chunks/{cx}_{cy}.bin` → [`WorldChunk`]: a header check and one cast, with
//! no gzip and no serde.
//!
//! This is the binary twin of [`parse_chunk`](super::chunk::parse_chunk). That function inflates a
//! `.json.gz` and walks a `serde_json::Value` array on the **main thread**, once per chunk, for
//! every chunk a pan brings into the viewport — the stall audit.md Finding 1.4 measured
//! (`residency.rs`'s ingest, `chunk.rs:46-97`). The bytes here were written by the T-935.2 emitter
//! (`tools/tbd-tools/src/world/binary_emit.rs`) *through the same narrowing the JSON loader uses*,
//! so the columns this produces are the columns the gz path produces, and
//! `everon_chunk_bin_columns_equal_the_gz_decode` below pins that over all 315 committed everon
//! chunks.
//!
//! # Why the parser is here and not in `residency.rs`
//!
//! `residency.rs` is an allowlisted SIZE-3 file. It gets the two call sites
//! ([`WorldResidency::ingest_chunk_bin`](super::WorldResidency::ingest_chunk_bin)) and nothing
//! else; the decode, the validation and its tests live in this file.
//!
//! # What is checked, and why each check exists
//!
//! Everything below happens **before** a single byte is reinterpreted, because a `cast_slice` over
//! a buffer whose shape has been assumed rather than checked does not fail — it succeeds and
//! returns nonsense.
//!
//! * **Magic, then version** ([`ContainerHeader::validate`]). A git-LFS pointer file starts
//!   `b"vers"`; a future TBDC v2 with a wider row starts `b"TBDC"`. Those are different problems
//!   and get different errors.
//! * **Declared length == actual length.** [`TbdcHeader::payload_bytes`] is checked arithmetic and
//!   returns [`Option`] because `usize` is **32 bits on `wasm32`**, the target this format exists
//!   for: `count * 32` wraps above `2^27` there. `None` is an error here, never an `unwrap` and
//!   never a wrapped number — the check exists so a truncated download cannot read as a
//!   short-but-valid chunk with objects silently missing, and a check that overflows is not a
//!   check.
//! * **The header's own `(cx, cy)` against the chunk that was asked for**
//!   ([`parse_chunk_bin_for`]). This is the *meaning* check on top of the layout checks, and the
//!   only reason the header carries `cx`/`cy` at all. A `.bin` served for the wrong tile — a
//!   mis-keyed emit, a CDN cache collision, an off-by-one in the URL fill — is byte-perfect and
//!   passes every check above. Inserted under the requested id it would file another tile's
//!   objects into this tile's spatial-index cells, mark the real tile resident, and never error
//!   again: exactly the silent-and-permanent failure that layout validation alone does not catch.
//!
//! # Alignment
//!
//! `bytemuck` will not reinterpret a `&[u8]` that is not 4-aligned, and a `Vec<u8>` off
//! `fetch_bytes` carries no alignment guarantee at all. The answer is one copy into a `Vec<u32>`
//! ([`align4`]), taken **only** when the in-place cast reports
//! [`BinaryError::Misaligned`] — never an unchecked cast, and never an `unwrap` on the cast's
//! result.

use std::collections::HashMap;
use std::fmt;

use super::binary::BinaryError;
use super::binary::chunk_container::{ContainerHeader, TbdcHeader};
use super::binary::pod::{ObjectInstancePod, instances_from_bytes};
use super::chunk::WorldChunk;
use super::classify::NO_CLASS;

/// The `{cx}` placeholder in the manifest's `objects.binary.chunks` template.
const CX_PLACEHOLDER: &str = "{cx}";
/// The `{cy}` placeholder in the manifest's `objects.binary.chunks` template.
const CY_PLACEHOLDER: &str = "{cy}";

/// Everything that can be wrong with a chunk `.bin` *as a chunk*.
///
/// [`BinaryError`] already covers everything that can be wrong with it as a buffer; this adds the
/// one failure that only means something once a caller has said which chunk it asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkBinError {
    /// The buffer is not a well-formed `TBDC` container.
    Format(BinaryError),
    /// Well-formed, but it is a different tile than the one requested.
    IdMismatch {
        /// The chunk id the loader fetched.
        requested: String,
        /// The `{cx}_{cy}` the header itself declares.
        header: String,
    },
}

impl fmt::Display for ChunkBinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(e) => write!(f, "{e}"),
            Self::IdMismatch { requested, header } => write!(
                f,
                "TBDC: chunk {requested} was fetched but the header declares {header}"
            ),
        }
    }
}

impl core::error::Error for ChunkBinError {}

impl From<BinaryError> for ChunkBinError {
    fn from(e: BinaryError) -> Self {
        Self::Format(e)
    }
}

/// Parse one `TBDC` chunk container into the SoA [`WorldChunk`] the residency stores.
///
/// The chunk's `id`, `cx` and `cy` come from the header, so this needs nothing but the bytes. Use
/// [`parse_chunk_bin_for`] when the caller knows which chunk it asked for — it does.
///
/// # Errors
/// [`BinaryError`] for a short buffer, a wrong magic, an unsupported version, a payload whose
/// length disagrees with the header's `count` (including a `count` whose byte length overflows
/// `usize`), or a payload that is neither castable in place nor a whole number of rows. Never
/// panics: a corrupt asset must cost one chunk, not the frame.
pub fn parse_chunk_bin(bytes: &[u8]) -> Result<WorldChunk, BinaryError> {
    // `read`, not `parse`: the 32-byte header is copied out by value, so a 1-aligned network
    // buffer is not rejected before its payload has even been looked at.
    let (header, payload) = TbdcHeader::read(bytes)?;
    let Some(expected) = header.payload_bytes() else {
        // Only reachable on a 32-bit target (`count > 2^27`), where the multiply would wrap. Such
        // a header cannot be satisfied by any real buffer, so it is a length mismatch — reported
        // with the count that caused it rather than with the wrapped product.
        return Err(BinaryError::LengthMismatch {
            what: TbdcHeader::NAME,
            expected: usize::MAX,
            actual: payload.len(),
        });
    };
    if payload.len() != expected {
        return Err(BinaryError::LengthMismatch {
            what: TbdcHeader::NAME,
            expected,
            actual: payload.len(),
        });
    }
    match instances_from_bytes(payload) {
        Ok(rows) => Ok(columns(&header, rows)),
        Err(BinaryError::Misaligned { .. }) => {
            let words = align4(payload);
            let rows = instances_from_bytes(bytemuck::cast_slice(&words))?;
            Ok(columns(&header, rows))
        }
        Err(other) => Err(other),
    }
}

/// [`parse_chunk_bin`] plus the check that the bytes are the chunk that was actually requested.
///
/// This is the entry point the loader uses. See the module docs for why the id check is not
/// optional: a mis-served but well-formed `.bin` is invisible to every other check in this file.
///
/// # Errors
/// [`ChunkBinError::Format`] for anything [`parse_chunk_bin`] rejects, or
/// [`ChunkBinError::IdMismatch`] when the header's `{cx}_{cy}` is not `id`.
pub fn parse_chunk_bin_for(id: &str, bytes: &[u8]) -> Result<WorldChunk, ChunkBinError> {
    let chunk = parse_chunk_bin(bytes)?;
    if chunk.id != id {
        return Err(ChunkBinError::IdMismatch {
            requested: id.to_string(),
            header: chunk.id,
        });
    }
    Ok(chunk)
}

/// Fill the manifest's `objects.binary.chunks` template (`objects/chunks/{cx}_{cy}.bin`) for one
/// chunk id, relative to the terrain asset base.
///
/// `None` — meaning *keep using the `.json.gz` path* — when the id is not `{int}_{int}` or when the
/// template carries neither placeholder. The second case matters: a template without `{cx}`/`{cy}`
/// would otherwise resolve every chunk in the world to one constant URL, and the loader would
/// cheerfully fetch it 315 times and fill the map with copies of a single tile. Falling back to the
/// path that works is the only safe reading of a template this loader does not understand.
#[must_use]
pub fn chunk_bin_path(template: &str, id: &str) -> Option<String> {
    if !template.contains(CX_PLACEHOLDER) || !template.contains(CY_PLACEHOLDER) {
        return None;
    }
    let (cx, cy) = id.split_once('_')?;
    // Parsed, not just split: the values are pasted into a URL, so "a_b" or "../.." must not
    // become one. Chunk indices are small signed integers (`TbdcHeader` stores them as `i16`).
    cx.parse::<i16>().ok()?;
    cy.parse::<i16>().ok()?;
    Some(
        template
            .replace(CX_PLACEHOLDER, cx)
            .replace(CY_PLACEHOLDER, cy),
    )
}

/// One copy of `payload` into a 4-aligned `Vec<u32>` so the rows can be cast.
///
/// `payload.len()` is a multiple of 32 by the time this is called (the length check above), so the
/// `/ 4` is exact and no byte is dropped.
fn align4(payload: &[u8]) -> Vec<u32> {
    let mut words = vec![0_u32; payload.len() / 4];
    let dst: &mut [u8] = bytemuck::cast_slice_mut(&mut words);
    dst.copy_from_slice(payload);
    words
}

/// Rows → the SoA columns, in the same order and the same shapes
/// [`parse_chunk`](super::chunk::parse_chunk) builds them: `positions` interleaved `[x0,y0,x1,y1…]`,
/// `rotations` from `yaw`, and `rows_by_class` the encounter-order gather lists with [`NO_CLASS`]
/// excluded.
fn columns(header: &TbdcHeader, rows: &[ObjectInstancePod]) -> WorldChunk {
    let n = rows.len();
    let mut positions: Vec<f32> = Vec::with_capacity(2 * n);
    let mut prefab_idx: Vec<u16> = Vec::with_capacity(n);
    let mut rotations: Vec<f32> = Vec::with_capacity(n);
    let mut z: Vec<f32> = Vec::with_capacity(n);
    let mut pitch: Vec<f32> = Vec::with_capacity(n);
    let mut roll: Vec<f32> = Vec::with_capacity(n);
    let mut scale: Vec<f32> = Vec::with_capacity(n);
    let mut cls_codes: Vec<u8> = Vec::with_capacity(n);
    let mut rows_by_class: HashMap<u8, Vec<u32>> = HashMap::new();

    for (i, r) in rows.iter().enumerate() {
        positions.push(r.x);
        positions.push(r.y);
        prefab_idx.push(r.prefab_id);
        rotations.push(r.yaw);
        z.push(r.z);
        pitch.push(r.pitch);
        roll.push(r.roll);
        scale.push(r.scale);
        cls_codes.push(r.class_code);
        if r.class_code != NO_CLASS {
            rows_by_class
                .entry(r.class_code)
                .or_default()
                .push(i as u32);
        }
    }

    WorldChunk {
        id: format!("{}_{}", header.cx, header.cy),
        cx: f64::from(header.cx),
        cy: f64::from(header.cy),
        // The header's own count, which the length check has just proven equals `rows.len()`.
        count: header.count,
        positions,
        prefab_idx,
        rotations,
        z,
        pitch,
        roll,
        scale,
        cls_codes,
        rows_by_class,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::super::binary::chunk_container::HEADER_BYTES;
    use super::super::binary::pod::POD_BYTES;
    use super::super::residency::{IngestOutcome, WorldResidency};
    use super::super::store::bytes_to_json;
    use super::super::{build_prefab_maps, narrow_prefab_rows, parse_chunk};
    use super::*;

    /// Every committed everon chunk. A shrinking corpus is how a parity test stops proving
    /// anything, so this is an equality and re-pinning it is a deliberate act.
    const EVERON_CHUNKS: usize = 315;
    /// Floor on the instances the parity sweep actually compared. Makes an empty tree, an
    /// all-LFS-pointer tree or a silently-skipping loop impossible to pass.
    const EVERON_INSTANCE_FLOOR: usize = 1_200_000;

    fn everon() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/map-assets/everon")
    }

    /// The independent encoder: fixed byte offsets and `to_le_bytes`, touching neither
    /// [`ObjectInstancePod`] nor `bytemuck`.
    ///
    /// This is the oracle that matters. Building the buffer with `bytemuck::cast_slice` over the
    /// same struct the parser casts back would move both sides together under a field reorder —
    /// `yaw` and `pitch` swapped in the declaration would still round-trip green while every
    /// committed `.bin` had become unreadable. Byte offsets cannot move with the struct.
    fn encode_by_offset(cx: i16, cy: i16, c: &WorldChunk) -> Vec<u8> {
        let n = c.count as usize;
        let mut out = Vec::with_capacity(HEADER_BYTES + POD_BYTES * n);
        out.extend_from_slice(b"TBDC");
        out.extend_from_slice(&1_u16.to_le_bytes()); // version
        out.extend_from_slice(&0_u16.to_le_bytes()); // flags
        out.extend_from_slice(&(n as u32).to_le_bytes());
        out.extend_from_slice(&cx.to_le_bytes());
        out.extend_from_slice(&cy.to_le_bytes());
        out.extend_from_slice(&[0_u8; 16]); // reserved
        for i in 0..n {
            out.extend_from_slice(&c.positions[2 * i].to_le_bytes());
            out.extend_from_slice(&c.positions[2 * i + 1].to_le_bytes());
            out.extend_from_slice(&c.z[i].to_le_bytes());
            out.extend_from_slice(&c.rotations[i].to_le_bytes());
            out.extend_from_slice(&c.pitch[i].to_le_bytes());
            out.extend_from_slice(&c.roll[i].to_le_bytes());
            out.extend_from_slice(&c.scale[i].to_le_bytes());
            out.extend_from_slice(&c.prefab_idx[i].to_le_bytes());
            out.push(c.cls_codes[i]);
            out.push(0);
        }
        assert_eq!(out.len(), HEADER_BYTES + POD_BYTES * n);
        out
    }

    /// f32 columns compare by `to_bits`: `==` calls `-0.0` equal to `+0.0` and `NaN` unequal to
    /// itself, and the whole point of a binary twin is that the bytes agree.
    fn bits(v: &[f32]) -> Vec<u32> {
        v.iter().map(|f| f.to_bits()).collect()
    }

    /// `expect_err` without the `Debug` bound on the `Ok` side — [`WorldChunk`] is a SoA of eight
    /// columns and deliberately does not derive `Debug` (`chunk.rs` owns that decision).
    fn err_of<E>(r: Result<WorldChunk, E>, msg: &str) -> E {
        match r {
            Ok(c) => panic!("{msg}: got Ok({} rows)", c.count),
            Err(e) => e,
        }
    }

    fn assert_columns_equal(got: &WorldChunk, want: &WorldChunk, ctx: &str) {
        assert_eq!(got.id, want.id, "{ctx}: id");
        assert_eq!(got.cx, want.cx, "{ctx}: cx");
        assert_eq!(got.cy, want.cy, "{ctx}: cy");
        assert_eq!(got.count, want.count, "{ctx}: count");
        assert_eq!(
            bits(&got.positions),
            bits(&want.positions),
            "{ctx}: positions"
        );
        assert_eq!(got.prefab_idx, want.prefab_idx, "{ctx}: prefab_idx");
        assert_eq!(
            bits(&got.rotations),
            bits(&want.rotations),
            "{ctx}: rotations"
        );
        assert_eq!(bits(&got.z), bits(&want.z), "{ctx}: z");
        assert_eq!(bits(&got.pitch), bits(&want.pitch), "{ctx}: pitch");
        assert_eq!(bits(&got.roll), bits(&want.roll), "{ctx}: roll");
        assert_eq!(bits(&got.scale), bits(&want.scale), "{ctx}: scale");
        assert_eq!(got.cls_codes, want.cls_codes, "{ctx}: cls_codes");
        assert_eq!(
            got.rows_by_class, want.rows_by_class,
            "{ctx}: rows_by_class"
        );
    }

    /// A synthetic two-row chunk with awkward values on purpose: a negative chunk index, a
    /// `NO_CLASS` row (must NOT appear in `rows_by_class`), a non-identity pitch/roll/scale and a
    /// negative coordinate.
    fn two_row_chunk() -> WorldChunk {
        WorldChunk {
            id: "-3_4".to_string(),
            cx: -3.0,
            cy: 4.0,
            count: 2,
            positions: vec![4096.5, 8192.25, -1.5, 0.0],
            prefab_idx: vec![1623, 7],
            rotations: vec![271.5, 0.0],
            z: vec![-12.125, 3.0],
            pitch: vec![-3.25, 0.0],
            roll: vec![0.5, 0.0],
            scale: vec![1.75, 1.0],
            cls_codes: vec![2, NO_CLASS],
            rows_by_class: HashMap::from([(2, vec![0])]),
        }
    }

    #[test]
    fn two_row_buffer_round_trips_from_hand_written_bytes() {
        let want = two_row_chunk();
        let bytes = encode_by_offset(-3, 4, &want);
        assert_eq!(bytes.len(), HEADER_BYTES + 2 * POD_BYTES);
        let got = parse_chunk_bin(&bytes).expect("well-formed two-row TBDC");
        assert_columns_equal(&got, &want, "two-row");
        // The NO_CLASS row is excluded from the gather lists, exactly as `parse_chunk` does.
        assert_eq!(got.rows_by_class.len(), 1);
    }

    #[test]
    fn empty_chunk_is_zero_rows_not_an_error() {
        let empty = WorldChunk {
            id: "0_0".to_string(),
            ..Default::default()
        };
        let bytes = encode_by_offset(0, 0, &empty);
        assert_eq!(bytes.len(), HEADER_BYTES);
        let got = parse_chunk_bin(&bytes).expect("an empty chunk is a real chunk");
        assert_eq!(got.count, 0);
        assert!(got.positions.is_empty() && got.rows_by_class.is_empty());
    }

    #[test]
    fn truncated_payload_is_err_not_a_short_chunk() {
        let bytes = encode_by_offset(-3, 4, &two_row_chunk());
        // One byte short of the two rows the header declares.
        let err = err_of(
            parse_chunk_bin(&bytes[..bytes.len() - 1]),
            "must not read short",
        );
        assert!(
            matches!(
                err,
                BinaryError::LengthMismatch {
                    expected: 64,
                    actual: 63,
                    ..
                }
            ),
            "{err}"
        );
        // A whole row missing is the dangerous case: it is a *valid* one-row payload by shape.
        let err = err_of(
            parse_chunk_bin(&bytes[..bytes.len() - POD_BYTES]),
            "row dropped",
        );
        assert!(matches!(err, BinaryError::LengthMismatch { .. }), "{err}");
    }

    #[test]
    fn wrong_magic_is_err() {
        let mut bytes = encode_by_offset(-3, 4, &two_row_chunk());
        bytes[..4].copy_from_slice(b"vers"); // a git-LFS pointer file
        let err = err_of(parse_chunk_bin(&bytes), "LFS pointer is not a chunk");
        assert!(matches!(err, BinaryError::BadMagic { .. }), "{err}");
    }

    #[test]
    fn wrong_version_is_err() {
        let mut bytes = encode_by_offset(-3, 4, &two_row_chunk());
        bytes[4..6].copy_from_slice(&2_u16.to_le_bytes());
        let err = err_of(
            parse_chunk_bin(&bytes),
            "v2 rows may not be read at the v1 stride",
        );
        assert!(
            matches!(err, BinaryError::UnsupportedVersion { actual: 2, .. }),
            "{err}"
        );
    }

    #[test]
    fn buffer_shorter_than_the_header_is_err() {
        let err = err_of(parse_chunk_bin(&[0_u8; 8]), "8 bytes is not a header");
        assert!(matches!(err, BinaryError::Truncated { .. }), "{err}");
        assert!(parse_chunk_bin(&[]).is_err());
    }

    /// A `count` whose byte length overflows a 32-bit `usize` must be an error, not a wrap. On
    /// 64-bit the multiply succeeds and the length check rejects it; on `wasm32`
    /// `payload_bytes()` is `None` and the `usize::MAX` arm rejects it. Both are `Err`, and
    /// neither is a panic — which is the whole reason the helper returns `Option`.
    #[test]
    fn overflowing_count_is_err_not_a_wrap() {
        let mut bytes = encode_by_offset(0, 0, &WorldChunk::default());
        bytes[8..12].copy_from_slice(&0x0800_0000_u32.to_le_bytes());
        let err = err_of(
            parse_chunk_bin(&bytes),
            "2^27 rows cannot be in 0 payload bytes",
        );
        assert!(matches!(err, BinaryError::LengthMismatch { .. }), "{err}");
        bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse_chunk_bin(&bytes).is_err());
    }

    /// A 4-aligned backing buffer, so "one byte in" is *deterministically* misaligned. A `Vec` or
    /// a bare `[u8; N]` would leave the base address to the allocator or the stack frame, and this
    /// test would then pass or fail by luck.
    #[repr(align(4))]
    struct AlignedBuf([u8; 1 + HEADER_BYTES + 2 * POD_BYTES]);

    #[test]
    fn misaligned_buffer_takes_the_copy_path_and_gives_the_same_columns() {
        let want = two_row_chunk();
        let mut backing = AlignedBuf([0; 1 + HEADER_BYTES + 2 * POD_BYTES]);
        backing.0[1..].copy_from_slice(&encode_by_offset(-3, 4, &want));
        let skewed = &backing.0[1..];

        // The in-place cast genuinely cannot serve this buffer — so the aligned copy is the branch
        // that produced the answer below, not an assertion about a path that never ran.
        let payload = &skewed[HEADER_BYTES..];
        assert!(
            matches!(
                instances_from_bytes(payload),
                Err(BinaryError::Misaligned { .. })
            ),
            "expected the zero-copy cast to refuse a 1-mod-4 payload"
        );

        let got = parse_chunk_bin(skewed).expect("the copy path handles a misaligned buffer");
        assert_columns_equal(&got, &want, "misaligned");
    }

    #[test]
    fn id_mismatch_is_rejected_even_though_the_bytes_are_perfect() {
        let bytes = encode_by_offset(-3, 4, &two_row_chunk());
        // The buffer is valid — it is simply the wrong tile.
        assert!(parse_chunk_bin(&bytes).is_ok());
        let err = err_of(
            parse_chunk_bin_for("18_0", &bytes),
            "wrong tile must not be accepted",
        );
        assert_eq!(
            err,
            ChunkBinError::IdMismatch {
                requested: "18_0".to_string(),
                header: "-3_4".to_string(),
            }
        );
        assert!(parse_chunk_bin_for("-3_4", &bytes).is_ok());
    }

    #[test]
    fn chunk_bin_path_fills_the_manifest_template() {
        let t = "objects/chunks/{cx}_{cy}.bin";
        assert_eq!(
            chunk_bin_path(t, "18_0").as_deref(),
            Some("objects/chunks/18_0.bin")
        );
        assert_eq!(
            chunk_bin_path(t, "-3_4").as_deref(),
            Some("objects/chunks/-3_4.bin")
        );
        // A template this loader does not understand falls back to the gz path rather than
        // resolving every chunk to one constant URL.
        assert_eq!(chunk_bin_path("objects/chunks/all.bin", "18_0"), None);
        assert_eq!(chunk_bin_path("objects/chunks/{cx}.bin", "18_0"), None);
        // Ids that are not `{int}_{int}` never reach a URL.
        assert_eq!(chunk_bin_path(t, "a_b"), None);
        assert_eq!(chunk_bin_path(t, ".._.."), None);
        assert_eq!(chunk_bin_path(t, "18"), None);
    }

    /// The manifest branch the loader keys on, on **real** inputs: the committed everon manifest
    /// carries no `objects.binary` block, so the gz path stays; a manifest that does carry one
    /// resolves to a `.bin` URL. Without this the binary branch in `world_host.rs` would be a
    /// mechanism nobody had shown could fire.
    #[test]
    fn manifest_decides_the_branch() {
        use super::super::manifest::parse_manifest_binary;

        let raw = fs::read_to_string(everon().join("manifest.json")).expect("everon manifest");
        let shipped: serde_json::Value = serde_json::from_str(&raw).expect("manifest json");
        assert!(
            parse_manifest_binary(&shipped).objects.is_none(),
            "the shipped everon manifest must still take the .json.gz path (T-935.13 flips it)"
        );

        let flipped: serde_json::Value = serde_json::json!({ "objects": { "binary": {
            "schemaVersion": "1.0.0", "container": "TBDC", "containerVersion": 1,
            "pod": "ObjectInstancePod", "podBytes": 32,
            "chunks": "objects/chunks/{cx}_{cy}.bin"
        }}});
        let block = parse_manifest_binary(&flipped)
            .objects
            .expect("binary block");
        assert!(
            block.matches_this_build(),
            "this build reads TBDC v1 / 32-byte rows"
        );
        assert_eq!(
            chunk_bin_path(&block.chunks, "18_0").as_deref(),
            Some("objects/chunks/18_0.bin")
        );

        // A block describing the operator's rejected 24-byte row must NOT be taken: reading it at
        // a 32-byte stride draws a map of garbage that never errors.
        let mut narrow = block.clone();
        narrow.pod_bytes = 24;
        assert!(!narrow.matches_this_build());
    }

    /* ───────────────────── the slice's pin: .bin columns == .json.gz columns ───────────────── */

    /// THE SLICE'S PIN. For all 315 committed everon chunks: gz-decode the chunk through
    /// `parse_chunk`, re-encode those columns as `TBDC` bytes with the independent byte-offset
    /// encoder, then prove [`parse_chunk_bin`] reads them back column-for-column, bit-for-bit.
    ///
    /// This is the acceptance criterion — "a chunk `.bin` ingests into a `WorldChunk` whose
    /// columns equal the gz decode" — over the real corpus rather than a synthetic row. The
    /// emitter's own direction (rows → bytes) is pinned by T-935.2's parity test; this pins the
    /// loader's (bytes → columns).
    #[test]
    fn everon_chunk_bin_columns_equal_the_gz_decode() {
        let objects = everon().join("objects");
        let prefabs_doc =
            bytes_to_json(&fs::read(objects.join("prefabs.json.gz")).expect("prefabs"))
                .expect("prefabs decode");
        let (prefab_by_id, _) = build_prefab_maps(narrow_prefab_rows(&prefabs_doc));
        assert!(!prefab_by_id.is_empty(), "empty prefab catalogue");

        let mut files: Vec<PathBuf> = fs::read_dir(objects.join("chunks"))
            .expect("everon objects/chunks")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.to_string_lossy().ends_with(".json.gz"))
            .collect();
        files.sort();
        assert_eq!(
            files.len(),
            EVERON_CHUNKS,
            "everon chunk corpus changed; re-pin EVERON_CHUNKS deliberately"
        );

        let mut total_rows = 0_usize;
        for path in &files {
            let id = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".json.gz"))
                .expect("chunk file name");
            let raw = bytes_to_json(&fs::read(path).expect("read chunk")).expect(id);
            let want = parse_chunk(id, &raw, &prefab_by_id).expect("gz decode");
            let (cx, cy) = id.split_once('_').expect("chunk id");
            let bytes = encode_by_offset(cx.parse().expect("cx"), cy.parse().expect("cy"), &want);
            let got = parse_chunk_bin_for(id, &bytes).expect("bin decode");
            assert_columns_equal(&got, &want, id);
            total_rows += want.count as usize;
        }
        assert!(
            total_rows >= EVERON_INSTANCE_FLOOR,
            "compared only {total_rows} instances; the corpus is not being read"
        );
    }

    /* ───────────────────────── the residency call sites (T-935.3) ───────────────────────── */

    fn everon_residency() -> WorldResidency {
        let mut r = WorldResidency::new();
        r.load_manifest_json(
            &fs::read_to_string(everon().join("manifest.json")).expect("manifest"),
        )
        .expect("manifest parse");
        r.load_prefabs_gz(&fs::read(everon().join("objects/prefabs.json.gz")).expect("prefabs"))
            .expect("prefabs parse");
        r
    }

    fn everon_chunk_bytes(id: &str) -> (Vec<u8>, Vec<u8>) {
        let gz = fs::read(
            everon()
                .join("objects/chunks")
                .join(format!("{id}.json.gz")),
        )
        .expect("chunk gz");
        let mut r = everon_residency();
        r.ingest_chunk_gz(id, &gz).expect("gz ingest");
        let chunk = r.chunk(id).expect("chunk resident").clone();
        let (cx, cy) = id.split_once('_').expect("chunk id");
        let bin = encode_by_offset(cx.parse().expect("cx"), cy.parse().expect("cy"), &chunk);
        (gz, bin)
    }

    /// `ingest_chunk_bin` must leave the residency in the state `ingest_chunk_gz` leaves it in —
    /// same outcome, same columns, same instance count — because it shares that bookkeeping rather
    /// than re-implementing it.
    #[test]
    fn ingest_chunk_bin_matches_ingest_chunk_gz() {
        let id = "18_0";
        let (gz, bin) = everon_chunk_bytes(id);

        let mut via_gz = everon_residency();
        let mut via_bin = everon_residency();
        let a = via_gz.ingest_chunk_gz(id, &gz).expect("gz ingest");
        let b = via_bin.ingest_chunk_bin(id, &bin).expect("bin ingest");

        assert_eq!(a, b, "same disposition");
        assert!(
            matches!(b, IngestOutcome::Applied(n) if n > 0),
            "18_0 is a populated chunk: {b:?}"
        );
        assert_eq!(
            via_gz.resident_instance_count(id),
            via_bin.resident_instance_count(id)
        );
        assert_columns_equal(
            via_bin.chunk(id).expect("bin chunk"),
            via_gz.chunk(id).expect("gz chunk"),
            "residency 18_0",
        );
        assert_eq!(via_bin.chunks_resident(), via_gz.chunks_resident());
    }

    /// A chunk that parses to zero instances must be stubbed and marked known-empty *forever* on
    /// both paths — the T-173 P3 policy that stops an empty tile being re-requested every pan.
    /// Synthetic on purpose: every one of the 315 committed everon cells has instances (a cell only
    /// exists in the index because it has some), so a real file cannot exercise this arm.
    #[test]
    fn ingest_chunk_bin_marks_an_empty_chunk_known_empty_like_the_gz_path() {
        let id = "4_9";
        let gz = br#"{"instances":[]}"#;
        let bin = encode_by_offset(
            4,
            9,
            &WorldChunk {
                id: id.to_string(),
                ..Default::default()
            },
        );
        let mut via_gz = everon_residency();
        let mut via_bin = everon_residency();
        assert_eq!(
            via_gz.ingest_chunk_gz(id, gz).expect("gz"),
            IngestOutcome::ParsedEmpty
        );
        assert_eq!(
            via_bin.ingest_chunk_bin(id, &bin).expect("bin"),
            IngestOutcome::ParsedEmpty
        );
        // The mark itself, not just the disposition — `known_empty_count` is what stops the refetch.
        assert!(via_gz.stats_json().contains("\"known_empty_count\":1"));
        assert!(via_bin.stats_json().contains("\"known_empty_count\":1"));
    }

    /// A corrupt or mis-served buffer must be an `Err` the caller can route to the retry-capped
    /// failure path, and must insert nothing.
    #[test]
    fn ingest_chunk_bin_rejects_corrupt_and_mis_served_buffers() {
        let (_, bin) = everon_chunk_bytes("18_0");
        let mut r = everon_residency();

        assert!(r.ingest_chunk_bin("18_0", &bin[..bin.len() - 1]).is_err());
        assert!(r.ingest_chunk_bin("18_0", b"vers").is_err());
        // Well-formed bytes for a different tile.
        let err = r
            .ingest_chunk_bin("17_0", &bin)
            .expect_err("wrong tile must not be filed under 17_0");
        assert!(matches!(err, ChunkBinError::IdMismatch { .. }), "{err}");
        assert_eq!(r.chunks_resident(), 0, "nothing may be inserted on error");
    }
}
