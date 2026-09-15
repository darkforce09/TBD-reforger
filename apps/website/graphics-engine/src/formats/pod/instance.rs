//! Role: instance.
//! Position: `formats/pod` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use bytemuck::{Pod, Zeroable};

use crate::formats::archives::codec::BinaryError;

/// On-disk size of one [`ObjectInstancePod`]. `TBDC` payload length is `count * POD_BYTES`.
pub const POD_BYTES: usize = 32;

/// The `pod` name recorded in the manifest `objects.binary` block ([`ObjectsBinaryBlock`](crate::world::ObjectsBinaryBlock)), so the file and the struct that reads it cannot drift apart unnoticed.
pub const POD_NAME: &str = "ObjectInstancePod";

/// One world object instance, exactly as it sits on disk and exactly as it is handed to the GPU.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ObjectInstancePod {
    /// X.
    pub x: f32,

    /// Y.
    pub y: f32,

    /// Z.
    pub z: f32,

    /// Yaw.
    pub yaw: f32,

    /// Pitch.
    pub pitch: f32,

    /// Roll.
    pub roll: f32,

    /// Scale.
    pub scale: f32,

    /// Index into the prefab catalogue (`objects/prefabs.rkyv`), the `prefab_idx` SoA column.
    pub prefab_id: u16,

    /// Render-class code — the same byte as the `cls_codes` column ([`class_code`](crate::world::class_code)); `NO_CLASS` for an unclassified prefab.
    pub class_code: u8,

    /// Always written as `0`. Present so the struct is exactly 32 B with no compiler padding a `Pod` cast would expose as uninitialised bytes.
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
    /// A row with the identity transform (`scale = 1.0`), which is what a v1 five-wide JSON row decodes to. `Default` gives `scale = 0.0` — deliberately not the same thing — so use this whenever a JSON row is being widened to the POD.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            scale: 1.0,
            ..Self::default()
        }
    }
}

/// Reinterpret a `TBDC` payload as instance rows, zero-copy.
pub fn instances_from_bytes(bytes: &[u8]) -> Result<&[ObjectInstancePod], BinaryError> {
    if !bytes.len().is_multiple_of(POD_BYTES) {
        return Err(BinaryError::LengthMismatch {
            what: "TBDC payload",
            expected: bytes.len().next_multiple_of(POD_BYTES),
            actual: bytes.len(),
        });
    }
    if bytes.is_empty() {
        return Ok(&[]);
    }
    bytemuck::try_cast_slice(bytes).map_err(|_| BinaryError::Misaligned {
        what: POD_NAME,
        align: 4,
    })
}

/// The bytes an emitter writes for a `TBDC` payload. Infallible: shrinking the alignment is always sound, so this is the direction that never needs a copy.
#[must_use]
pub fn instances_to_bytes(rows: &[ObjectInstancePod]) -> &[u8] {
    bytemuck::cast_slice(rows)
}

#[cfg(test)]
#[path = "tests/instance_tests.rs"]
mod tests;
