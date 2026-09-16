//! Role: tbdc.
//! Position: `formats/containers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::formats::archives::codec::BinaryError;
use crate::formats::containers::header::CONTAINER_VERSION;
use crate::formats::containers::header::ContainerHeader;
use crate::formats::containers::header::HEADER_BYTES;

use crate::formats::pod::instance::ObjectInstancePod;
use crate::formats::pod::instance::POD_BYTES;
use bytemuck::Pod;
use bytemuck::Zeroable;

/// `objects/chunks/{cx}_{cy}.bin`.
pub const TBDC_MAGIC: [u8; 4] = *b"TBDC";

/// `objects/chunks/{cx}_{cy}.bin` — `count` [`ObjectInstancePod`] rows and nothing else. File length is exactly `32 + 32 * count`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct TbdcHeader {
    /// Magic.
    pub magic: [u8; 4],

    /// Version.
    pub version: u16,

    /// Flags.
    pub flags: u16,

    /// Instance rows in the payload.
    pub count: u32,

    /// Chunk index, signed — the same `cx`/`cy` as the `{cx}_{cy}` id.
    pub cx: i16,

    /// Cy.
    pub cy: i16,

    /// Reserved.
    pub reserved: [u8; 16],
}

const _: () = assert!(size_of::<TbdcHeader>() == HEADER_BYTES);

impl ContainerHeader for TbdcHeader {
    const MAGIC: [u8; 4] = TBDC_MAGIC;
    const VERSION: u16 = CONTAINER_VERSION;
    const NAME: &'static str = "TBDC";
    fn magic_bytes(&self) -> [u8; 4] {
        self.magic
    }
    fn format_version(&self) -> u16 {
        self.version
    }
}

impl TbdcHeader {
    /// A valid header for `count` rows of chunk `(cx, cy)`.
    #[must_use]
    pub fn new(cx: i16, cy: i16, count: u32) -> Self {
        Self {
            magic: TBDC_MAGIC,
            version: CONTAINER_VERSION,
            flags: 0,
            count,
            cx,
            cy,
            reserved: [0; 16],
        }
    }
}

impl TbdcHeader {
    /// Payload length this header implies — `None` when the product does not fit `usize`.
    #[must_use]
    pub fn payload_bytes(&self) -> Option<usize> {
        (self.count as usize).checked_mul(POD_BYTES)
    }
}

impl TbdcHeader {
    /// Whole-file length this header implies — `None` on the same overflow.
    #[must_use]
    pub fn file_bytes(&self) -> Option<usize> {
        HEADER_BYTES.checked_add(self.payload_bytes()?)
    }
}

impl TbdcHeader {
    /// The payload as instance rows, zero-copy, with the header's `count` enforced.
    pub fn instances<'a>(&self, payload: &'a [u8]) -> Result<&'a [ObjectInstancePod], BinaryError> {
        let Some(expected) = self.payload_bytes() else {
            return Err(BinaryError::LengthMismatch {
                what: Self::NAME,
                expected: usize::MAX,
                actual: payload.len(),
            });
        };
        if payload.len() != expected {
            return Err(BinaryError::LengthMismatch {
                what: Self::NAME,
                expected,
                actual: payload.len(),
            });
        }
        crate::formats::pod::instance::instances_from_bytes(payload)
    }
}
