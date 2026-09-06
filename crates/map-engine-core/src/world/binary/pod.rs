//! T-935.1 — Tier 1: [`ObjectInstancePod`], the 32-byte on-disk row of a `TBDC` chunk container.
//!
//! This is the *whole* per-instance wire format. One `objects/chunks/{cx}_{cy}.bin` is a
//! [`super::chunk_container::TbdcHeader`] followed by `count` of these and nothing else, so a
//! loader's entire ingest is a length check plus one [`bytemuck::cast_slice`] — no gzip, no serde,
//! no per-row allocation. `.bin` replaces the `.json.gz` chunk path measured as main-thread stalls
//! in `residency.rs` (audit.md Finding 1.4).
//!
//! # Why 32 bytes and not the operator's 24
//!
//! The 2026-09-04 sketch was `{x, y, z, rotation_deg, prefab_id: u32, class_code, _pad[3]}`, which
//! silently drops pitch, roll and scale. T-090.12.1 put all three on the wire — the everon manifest
//! carries `objects.schemaVersion` `1.1.0` with `transforms: "yaw+pitch+roll+scale"`, and
//! [`WorldChunk`](crate::world::WorldChunk) carries them as SoA columns (`pitch`, `roll`, `scale`).
//! Encoding 24 B would therefore lose data the JSON path already ships, so the POD keeps the full
//! transform and `prefab_id` narrows to the `u16` the SoA column already uses (everon's catalogue
//! is 1623 prefabs; `parse_chunk` stores `pid as u16`).
//!
//! # The three invariants, all machine-checked below
//!
//! * **32 bytes, align 4.** `const _: () = assert!(…)` — a field added, reordered or widened is a
//!   compile error in every consumer, which is the point of defining this once.
//! * **No padding.** Guaranteed structurally (seven `f32`, then `u16 + u8 + u8` filling the last
//!   four bytes exactly) and enforced by `#[derive(Pod)]`, which refuses a type whose size exceeds
//!   the sum of its fields.
//! * **Little-endian.** `bytemuck` casts are *native*-endian, so the format is only LE because
//!   every target is; the `target_endian` assert below turns that assumption into a build failure
//!   rather than a silent byte-swapped read on a big-endian host.

use bytemuck::{Pod, Zeroable};

use super::BinaryError;

/// On-disk size of one [`ObjectInstancePod`]. `TBDC` payload length is `count * POD_BYTES`.
pub const POD_BYTES: usize = 32;

/// The `pod` name recorded in the manifest `objects.binary` block
/// ([`ObjectsBinaryBlock`](crate::world::ObjectsBinaryBlock)), so the file and the struct that
/// reads it cannot drift apart unnoticed.
pub const POD_NAME: &str = "ObjectInstancePod";

/// One world object instance, exactly as it sits on disk and exactly as it is handed to the GPU.
///
/// Field order is the spec's (§2) and is load-bearing: it is what makes the struct padding-free.
/// Angles are Enfusion `GetAngles()` degrees, positions are world-space metres, and `scale` is the
/// uniform `GetScale()` factor (`1.0` when the source row is a v1 five-wide row that predates
/// T-090.12.1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ObjectInstancePod {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub scale: f32,
    /// Index into the prefab catalogue (`objects/prefabs.rkyv`), the `prefab_idx` SoA column.
    pub prefab_id: u16,
    /// Render-class code — the same byte as the `cls_codes` column
    /// ([`class_code`](crate::world::class_code)); `NO_CLASS` for an unclassified prefab.
    pub class_code: u8,
    /// Always written as `0`. Present so the struct is exactly 32 B with no compiler padding a
    /// `Pod` cast would expose as uninitialised bytes.
    pub _pad: u8,
}

const _: () = assert!(
    core::mem::size_of::<ObjectInstancePod>() == POD_BYTES,
    "ObjectInstancePod is the T-935 chunk wire row and MUST be exactly 32 bytes: every emitter, \
     every loader and every committed .bin file computes offsets as `32 + 32 * index`."
);
const _: () = assert!(
    core::mem::align_of::<ObjectInstancePod>() == 4,
    "ObjectInstancePod must stay 4-aligned: the TBDC header is 32 B, so a 4-aligned buffer puts \
     the payload at a 4-aligned offset and `cast_slice` succeeds without a copy."
);
const _: () = assert!(
    cfg!(target_endian = "little"),
    "TBD binary containers are little-endian on disk and bytemuck casts are NATIVE-endian, so a \
     big-endian build would read every field byte-swapped while reporting success. Add explicit \
     byte-swapping to the container parsers before enabling such a target."
);

impl ObjectInstancePod {
    /// A row with the identity transform (`scale = 1.0`), which is what a v1 five-wide JSON row
    /// decodes to. `Default` gives `scale = 0.0` — deliberately not the same thing — so use this
    /// whenever a JSON row is being widened to the POD.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            scale: 1.0,
            ..Self::default()
        }
    }
}

/// Reinterpret a `TBDC` payload as instance rows, zero-copy.
///
/// `Err` (never a panic) when `bytes` is not a whole number of rows or when the slice is not
/// 4-aligned — a `Vec<u8>` straight off `fetch_bytes` frequently is not, and the loader
/// (T-935.3) answers that with one aligned copy rather than an unchecked cast.
pub fn instances_from_bytes(bytes: &[u8]) -> Result<&[ObjectInstancePod], BinaryError> {
    if !bytes.len().is_multiple_of(POD_BYTES) {
        return Err(BinaryError::LengthMismatch {
            what: "TBDC payload",
            expected: bytes.len().next_multiple_of(POD_BYTES),
            actual: bytes.len(),
        });
    }
    if bytes.is_empty() {
        // An empty chunk is a real, common chunk (315 everon tiles, plenty of them empty ocean).
        // `try_cast_slice` would reject it: an empty `&[u8]` carries a dangling 1-aligned pointer,
        // so the alignment check fails on a buffer that has no bytes to be misaligned.
        return Ok(&[]);
    }
    bytemuck::try_cast_slice(bytes).map_err(|_| BinaryError::Misaligned {
        what: POD_NAME,
        align: 4,
    })
}

/// The bytes an emitter writes for a `TBDC` payload. Infallible: shrinking the alignment is
/// always sound, so this is the direction that never needs a copy.
#[must_use]
pub fn instances_to_bytes(rows: &[ObjectInstancePod]) -> &[u8] {
    bytemuck::cast_slice(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ObjectInstancePod {
        ObjectInstancePod {
            x: 4096.5,
            y: 8192.25,
            z: -12.125,
            yaw: 271.5,
            pitch: -3.25,
            roll: 0.5,
            scale: 1.75,
            prefab_id: 1623,
            class_code: 7,
            _pad: 0,
        }
    }

    #[test]
    fn pod_is_thirty_two_bytes_and_four_aligned() {
        assert_eq!(core::mem::size_of::<ObjectInstancePod>(), 32);
        assert_eq!(core::mem::align_of::<ObjectInstancePod>(), 4);
        assert_eq!(POD_BYTES, 32);
    }

    /// The padding check the `Pod` derive cannot express: every one of the 32 bytes is reachable
    /// through a named field, so no byte of the wire row is compiler slack.
    #[test]
    fn every_byte_is_a_named_field() {
        let named = 7 * size_of::<f32>() + size_of::<u16>() + 2 * size_of::<u8>();
        assert_eq!(named, POD_BYTES);
    }

    #[test]
    fn round_trips_through_bytes_byte_for_byte() {
        let rows = [sample(), ObjectInstancePod::identity()];
        let bytes = instances_to_bytes(&rows);
        assert_eq!(bytes.len(), 2 * POD_BYTES);
        let back = instances_from_bytes(bytes).expect("aligned cast");
        assert_eq!(back, &rows[..]);
    }

    /// Field order is the format. Byte 0 must be the low byte of `x`, and the last four bytes must
    /// be `prefab_id` LE, `class_code`, `_pad` — a reorder that kept the size at 32 would still
    /// break every committed `.bin`, and only this test would catch it.
    #[test]
    fn field_offsets_are_the_wire_layout() {
        let rows = [sample()];
        let b = instances_to_bytes(&rows);
        assert_eq!(&b[0..4], &4096.5_f32.to_le_bytes());
        assert_eq!(&b[4..8], &8192.25_f32.to_le_bytes());
        assert_eq!(&b[8..12], &(-12.125_f32).to_le_bytes());
        assert_eq!(&b[12..16], &271.5_f32.to_le_bytes());
        assert_eq!(&b[16..20], &(-3.25_f32).to_le_bytes());
        assert_eq!(&b[20..24], &0.5_f32.to_le_bytes());
        assert_eq!(&b[24..28], &1.75_f32.to_le_bytes());
        assert_eq!(&b[28..30], &1623_u16.to_le_bytes());
        assert_eq!(b[30], 7);
        assert_eq!(b[31], 0);
    }

    #[test]
    fn identity_scale_is_one_and_default_is_not() {
        assert_eq!(ObjectInstancePod::identity().scale, 1.0);
        assert_eq!(ObjectInstancePod::default().scale, 0.0);
    }

    #[test]
    fn truncated_payload_is_err_not_panic() {
        let rows = [sample()];
        let bytes = instances_to_bytes(&rows);
        let err = instances_from_bytes(&bytes[..31]).expect_err("31 B is not a whole row");
        assert!(matches!(err, BinaryError::LengthMismatch { .. }), "{err}");
        assert!(instances_from_bytes(&bytes[..1]).is_err());
    }

    /// A 4-aligned backing buffer, so "one byte in" is *deterministically* misaligned. A `Vec` or a
    /// bare `[u8; N]` would leave the base address to the allocator or the stack frame, and this
    /// test would then pass or fail by luck.
    #[repr(align(4))]
    struct AlignedBuf([u8; 1 + POD_BYTES]);

    #[test]
    fn misaligned_slice_is_err_not_panic() {
        let mut buf = AlignedBuf([0; 1 + POD_BYTES]);
        buf.0[1..].copy_from_slice(bytemuck::bytes_of(&sample()));
        let err = instances_from_bytes(&buf.0[1..]).expect_err("odd address cannot cast");
        assert!(matches!(err, BinaryError::Misaligned { .. }), "{err}");
    }

    #[test]
    fn empty_payload_is_zero_rows_not_an_error() {
        assert_eq!(instances_from_bytes(&[]).expect("0 rows"), &[]);
    }
}
