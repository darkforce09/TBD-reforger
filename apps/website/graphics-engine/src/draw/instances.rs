//! Role: draw instances.
//! Position: `draw` in the graphics engine.
//! Signals & state: the per-instance vertex layouts the shaders declare.
//! Invariants: byte layout only. A struct here is named for its SHAPE — an axis-aligned quad,
//! an oriented quad, an atlas sprite — never for the thing the caller happens to draw with it.

use bytemuck::{Pod, Zeroable};

/// Instance-buffer pool unit: 2^21 instances × 32 B = 64 MiB per GPU buffer — legal by construction under WebGPU's *default* `maxBufferSize` (256 MiB) with 4× headroom, so no device-limit negotiation is ever load-bearing (plan §S4 chunked pool).
pub const CHUNK_CAPACITY: usize = 2_097_152;

/// Unit quad (triangle-strip order) expanded per instance in the vertex shader via `pos = mix(inst.min, inst.max, unit_uv)`. Culling is disabled in the pipeline, so winding is irrelevant.
pub const UNIT_QUAD: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// One axis-aligned colored quad instance (anchor-relative meters).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadInstance {
    /// Anchor-relative [minX, minY] corner, meters.
    pub min: [f32; 2],

    /// Anchor-relative [maxX, maxY] corner, meters.
    pub max: [f32; 2],

    /// RGBA, linear 0..1 (rendered to a non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

/// Building instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct BuildingInstance {
    /// Anchor-relative [x, y] center, meters (world minus the caller's anchor).
    pub center: [f32; 2],

    /// Half-extents [hx, hy], meters (a size — NOT anchor-shifted).
    pub half: [f32; 2],

    /// `(cos(rad), sin(rad))`, `rad = deg·PI/180` — computed once (matching `obb::obb_corners`), so the fill quad and the outline ring coincide to f32 rounding.
    pub basis: [f32; 2],

    /// RGBA, linear 0..1 (`byte/255`; rendered to a non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

/// Canonical atlas glyph count value.
pub const ATLAS_GLYPH_COUNT: usize = 32;

/// Icon instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct IconInstance {
    /// Anchor-relative [x, y] center, meters.
    pub pos: [f32; 2],

    /// Glyph size in meters (min-px already applied on CPU).
    pub size: f32,

    /// Screen CCW angle as snorm16 (`angle_deg/180 * 32767`).
    pub yaw: i16,

    /// Index into the 28-entry UV uniform table.
    pub glyph: u16,

    /// Packed RGBA8 (r | g<<8 | b<<16 | a<<24).
    pub tint: u32,
}

#[cfg(test)]
#[path = "tests/instances_tests.rs"]
mod tests;
