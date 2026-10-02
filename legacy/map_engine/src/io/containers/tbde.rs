//! Role: tbde.
//! Position: `io/containers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::archives::codec::BinaryError;
use crate::io::containers::header::CONTAINER_VERSION;
use crate::io::containers::header::ContainerHeader;
use crate::io::containers::header::HEADER_BYTES;
use bytemuck::Pod;
use bytemuck::Zeroable;

/// `dem/elevation.dem`.
pub const TBDE_MAGIC: [u8; 4] = *b"TBDE";

/// `dem/elevation.dem` — `width * height` `u16` samples, row-major, row 0 = north edge (the same orientation as the 16-bit PNG it replaces). Metres are `offset_m + sample * scale_m`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TbdeHeader {
    /// Magic.
    pub magic: [u8; 4],

    /// Version.
    pub version: u16,

    /// Flags.
    pub flags: u16,

    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,

    /// Metres per quantisation step: `(max_m - min_m) / 65535`.
    pub scale_m: f32,

    /// Metres at sample `0` — everon's `-204.78`.
    pub offset_m: f32,

    /// Reserved.
    pub reserved: [u8; 8],
}

const _: () = assert!(size_of::<TbdeHeader>() == HEADER_BYTES);

impl ContainerHeader for TbdeHeader {
    const MAGIC: [u8; 4] = TBDE_MAGIC;
    const VERSION: u16 = CONTAINER_VERSION;
    const NAME: &'static str = "TBDE";
    fn magic_bytes(&self) -> [u8; 4] {
        self.magic
    }
    fn format_version(&self) -> u16 {
        self.version
    }
}

impl TbdeHeader {
    /// A valid header for a `width × height` grid spanning `[min_m, max_m]`.
    #[must_use]
    pub fn new(width: u32, height: u32, min_m: f32, max_m: f32) -> Self {
        Self {
            magic: TBDE_MAGIC,
            version: CONTAINER_VERSION,
            flags: 0,
            width,
            height,
            scale_m: (max_m - min_m) / f32::from(u16::MAX),
            offset_m: min_m,
            reserved: [0; 8],
        }
    }
}

impl TbdeHeader {
    /// Samples the header implies — `None` on 32-bit overflow (see `TbdcHeader::payload_bytes`).
    #[must_use]
    pub fn sample_count(&self) -> Option<usize> {
        (self.width as usize).checked_mul(self.height as usize)
    }
}

impl TbdeHeader {
    /// Payload length this header implies — `None` on 32-bit overflow (see `TbdcHeader`).
    #[must_use]
    pub fn payload_bytes(&self) -> Option<usize> {
        self.sample_count()?.checked_mul(size_of::<u16>())
    }
}

impl TbdeHeader {
    /// The payload as `u16` samples, zero-copy, with `width * height` enforced.
    pub fn samples<'a>(&self, payload: &'a [u8]) -> Result<&'a [u16], BinaryError> {
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
        bytemuck::try_cast_slice(payload).map_err(|_| BinaryError::Misaligned {
            what: Self::NAME,
            align: align_of::<u16>(),
        })
    }
}

impl TbdeHeader {
    /// Decode one quantised sample to metres.
    #[must_use]
    pub fn metres(&self, sample: u16) -> f32 {
        self.offset_m + f32::from(sample) * self.scale_m
    }
}
