//! Role: tbdb.
//! Position: `formats/containers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::formats::containers::header::CONTAINER_VERSION;
use crate::formats::containers::header::ContainerHeader;
use crate::formats::containers::header::HEADER_BYTES;
use bytemuck::Pod;
use bytemuck::Zeroable;

/// `water/bathymetry.tbd-bath`.
pub const TBDB_MAGIC: [u8; 4] = *b"TBDB";

/// `water/bathymetry.tbd-bath` — a mip pyramid of depth + water-mask levels.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TbdbHeader {
    /// Magic.
    pub magic: [u8; 4],

    /// Version.
    pub version: u16,

    /// Levels present, `L = 0..mip_count`.
    pub mip_count: u16,

    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,

    /// Metres per depth unit.
    pub depth_scale: f32,

    /// Reserved.
    pub reserved: [u8; 12],
}

const _: () = assert!(size_of::<TbdbHeader>() == HEADER_BYTES);

impl ContainerHeader for TbdbHeader {
    const MAGIC: [u8; 4] = TBDB_MAGIC;
    const VERSION: u16 = CONTAINER_VERSION;
    const NAME: &'static str = "TBDB";
    fn magic_bytes(&self) -> [u8; 4] {
        self.magic
    }
    fn format_version(&self) -> u16 {
        self.version
    }
}

/// Where one bathymetry level's two blocks live, **relative to the start of the payload** (add [`HEADER_BYTES`] for a file offset / HTTP Range).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelSpan {
    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,

    /// Depth offset.
    pub depth_offset: usize,

    /// Depth bytes.
    pub depth_bytes: usize,

    /// Mask offset.
    pub mask_offset: usize,

    /// Mask bytes.
    pub mask_bytes: usize,

    /// `depth_bytes + mask_bytes` rounded up to a multiple of 4 — the stride to the next level.
    pub stride: usize,
}

impl TbdbHeader {
    /// A valid header for a `width × height` base with `mip_count` levels.
    #[must_use]
    pub fn new(width: u32, height: u32, mip_count: u16, depth_scale: f32) -> Self {
        Self {
            magic: TBDB_MAGIC,
            version: CONTAINER_VERSION,
            mip_count,
            width,
            height,
            depth_scale,
            reserved: [0; 12],
        }
    }
}

impl TbdbHeader {
    /// Dimensions of level `level`, or `None` past `mip_count` — or past what a `u32` can shift.
    ///
    /// ```text
    /// panicked at chunk_container.rs:418:15: attempt to shift right with overflow
    /// ```
    #[must_use]
    pub fn level_dims(&self, level: u16) -> Option<(u32, u32)> {
        if level >= self.mip_count || u32::from(level) >= u32::BITS {
            return None;
        }
        let shift = u32::from(level);
        Some(((self.width >> shift).max(1), (self.height >> shift).max(1)))
    }
}

impl TbdbHeader {
    /// Offsets and sizes of level `level`, or `None` past `mip_count`. Computed from the header alone, which is what lets a Range reader fetch one level without the file.
    #[must_use]
    pub fn level_span(&self, level: u16) -> Option<LevelSpan> {
        let mut offset = 0_usize;
        for l in 0..=level {
            let (w, h) = self.level_dims(l)?;
            let texels = (w as usize).checked_mul(h as usize)?;
            let depth_bytes = texels.checked_mul(size_of::<u16>())?;
            let mask_bytes = texels;
            let stride = depth_bytes
                .checked_add(mask_bytes)?
                .checked_next_multiple_of(4)?;
            if l == level {
                return Some(LevelSpan {
                    width: w,
                    height: h,
                    depth_offset: offset,
                    depth_bytes,
                    mask_offset: offset.checked_add(depth_bytes)?,
                    mask_bytes,
                    stride,
                });
            }
            offset += stride;
        }
        None
    }
}

impl TbdbHeader {
    /// Payload length the whole pyramid implies.
    #[must_use]
    pub fn payload_bytes(&self) -> usize {
        (0..self.mip_count)
            .filter_map(|l| self.level_span(l))
            .map(|s| s.stride)
            .sum()
    }
}

impl TbdbHeader {
    /// Decode one quantised depth to metres.
    #[must_use]
    pub fn metres(&self, depth: u16) -> f32 {
        f32::from(depth) * self.depth_scale
    }
}
