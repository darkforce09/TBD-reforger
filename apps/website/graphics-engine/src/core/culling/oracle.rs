//! Role: oracle.
//! Position: `core/culling` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use bytemuck::{Pod, Zeroable};

/// Compute workgroup size in `cs_icon_cull` (`shader.wgsl`).
pub const CULL_WORKGROUP: usize = 64;

/// Icon instance stride (matches [`crate::scene::IconInstance`] / glyph pack).
pub const ICON_STRIDE: usize = 20;

/// Frustum in the same space as icon `pos` (anchor-relative meters for GPU buffers).
pub type Frustum = [f64; 4];

/// One icon as read from a packed 20 B stream (no padding).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct IconCullSample {
    /// Pos.
    pub pos: [f32; 2],

    /// Size.
    pub size: f32,

    /// Yaw.
    pub yaw: i16,

    /// Glyph.
    pub glyph: u16,

    /// Tint.
    pub tint: u32,
}

/// True when the icon's AABB intersects `[min_x,min_y,max_x,max_y]` (inclusive edges).
#[must_use]
pub fn icon_intersects_frustum(pos_x: f32, pos_y: f32, size: f32, frustum: Frustum) -> bool {
    let half = (size * 0.5).max(0.0);
    let imin_x = pos_x - half;
    let imax_x = pos_x + half;
    let imin_y = pos_y - half;
    let imax_y = pos_y + half;
    #[allow(clippy::cast_possible_truncation)]
    let (fmin_x, fmax_x, fmin_y, fmax_y) = (
        frustum[0].min(frustum[2]) as f32,
        frustum[0].max(frustum[2]) as f32,
        frustum[1].min(frustum[3]) as f32,
        frustum[1].max(frustum[3]) as f32,
    );
    imax_x >= fmin_x && imin_x <= fmax_x && imax_y >= fmin_y && imin_y <= fmax_y
}

/// Compact `src` (packed 20 B icons) into `dst`, preserving encounter order. Returns the surviving instance count (Class R).
#[must_use]
pub fn compact_icons_cpu(src: &[u8], frustum: Frustum) -> (Vec<u8>, u32) {
    if !src.len().is_multiple_of(ICON_STRIDE) {
        return (Vec::new(), 0);
    }
    let n = src.len() / ICON_STRIDE;
    let mut out = Vec::with_capacity(src.len());
    let mut count = 0u32;
    for i in 0..n {
        let off = i * ICON_STRIDE;
        let chunk = &src[off..off + ICON_STRIDE];
        let px = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
        let py = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
        let size = f32::from_le_bytes(chunk[8..12].try_into().unwrap());
        if icon_intersects_frustum(px, py, size, frustum) {
            out.extend_from_slice(chunk);
            count += 1;
        }
    }
    (out, count)
}

/// Count-only (no alloc of compacted bytes) — used for 1k-frustum Class R scans.
#[must_use]
pub fn count_icons_in_frustum(src: &[u8], frustum: Frustum) -> u32 {
    if !src.len().is_multiple_of(ICON_STRIDE) {
        return 0;
    }
    let n = src.len() / ICON_STRIDE;
    let mut count = 0u32;
    for i in 0..n {
        let off = i * ICON_STRIDE;
        let chunk = &src[off..off + ICON_STRIDE];
        let px = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
        let py = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
        let size = f32::from_le_bytes(chunk[8..12].try_into().unwrap());
        if icon_intersects_frustum(px, py, size, frustum) {
            count += 1;
        }
    }
    count
}

/// Per-frame CPU oracle used by `encode_cull`.
#[must_use]
pub fn cpu_count_for_encode(src: &[u8], frustum: Frustum, debug_hud: bool) -> Option<u32> {
    if !debug_hud {
        return None;
    }
    Some(count_icons_in_frustum(src, frustum))
}

/// Shader reduce barrier before atomic.
#[must_use]
pub fn shader_reduce_barrier_before_atomic() -> bool {
    let src = include_str!("../../shaders/shader.wgsl");
    let Some(fn_at) = src.find("fn cs_icon_cull") else {
        return false;
    };
    let body = &src[fn_at..];
    let Some(add) = body.find("atomicAdd(&cull_counter") else {
        return false;
    };
    let Some(bar) = body.find("workgroupBarrier()") else {
        return false;
    };
    bar < add
}

/// `true` when every culled lane binds its **own** `cull_params` uniform.
#[must_use]
pub fn cull_params_is_per_lane() -> bool {
    let src = include_str!("compute.rs");
    let prod = src.split("#[cfg(test)]").next().unwrap_or(src);

    prod.contains("queue.write_buffer(&slot.params_buf, 0, &params);")
        && prod.contains("resource: slot.params_buf.as_entire_binding(),")
        && !prod.contains("self.params_buf")
}

/// GPU visible count as the workgroup-local reduce in `cs_icon_cull` would produce.
#[must_use]
pub fn gpu_workgroup_visible_count(src: &[u8], frustum: Frustum) -> u32 {
    gpu_workgroup_visible_count_ex(src, frustum, shader_reduce_barrier_before_atomic())
}

fn gpu_workgroup_visible_count_ex(src: &[u8], frustum: Frustum, barrier: bool) -> u32 {
    if !src.len().is_multiple_of(ICON_STRIDE) {
        return 0;
    }
    let n = src.len() / ICON_STRIDE;
    let mut total = 0u32;
    let mut base = 0usize;
    while base < n {
        let mut local = [0u32; CULL_WORKGROUP];
        for (lid, vis) in local.iter_mut().enumerate() {
            let idx = base + lid;
            if idx >= n {
                break;
            }
            let chunk = &src[idx * ICON_STRIDE..(idx + 1) * ICON_STRIDE];
            let px = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let py = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            let size = f32::from_le_bytes(chunk[8..12].try_into().unwrap());
            if icon_intersects_frustum(px, py, size, frustum) {
                *vis = 1;
            }
        }
        if !barrier {
            let v0 = local[0];
            local = [0u32; CULL_WORKGROUP];
            local[0] = v0;
        }
        total += local.iter().copied().sum::<u32>();
        base += CULL_WORKGROUP;
    }
    total
}

/// Pack a 32 B storage-friendly record for the WebGPU compute shader (std430-ish). Layout: pos.xy, size, yaw_i32, glyph_u32, tint_u32, pad_u32×2 → 32 B.
#[must_use]
pub fn pack_icon_storage32(src: &[u8]) -> Vec<u8> {
    if !src.len().is_multiple_of(ICON_STRIDE) {
        return Vec::new();
    }
    let n = src.len() / ICON_STRIDE;
    let mut out = vec![0u8; n * 32];
    for i in 0..n {
        let s = &src[i * ICON_STRIDE..(i + 1) * ICON_STRIDE];
        let d = &mut out[i * 32..(i + 1) * 32];
        d[0..12].copy_from_slice(&s[0..12]);
        let yaw = i16::from_le_bytes(s[12..14].try_into().unwrap()) as i32;
        let glyph = u32::from(u16::from_le_bytes(s[14..16].try_into().unwrap()));
        let tint = u32::from_le_bytes(s[16..20].try_into().unwrap());
        d[12..16].copy_from_slice(&yaw.to_le_bytes());
        d[16..20].copy_from_slice(&glyph.to_le_bytes());
        d[20..24].copy_from_slice(&tint.to_le_bytes());
    }
    out
}

/// Unpack 32 B storage records back to 20 B vertex instances (encounter order).
#[must_use]
pub fn unpack_icon_storage32(src32: &[u8], count: u32) -> Vec<u8> {
    let n = count as usize;
    if src32.len() < n * 32 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n * ICON_STRIDE);
    for i in 0..n {
        let s = &src32[i * 32..(i + 1) * 32];
        out.extend_from_slice(&s[0..12]);
        let yaw = i32::from_le_bytes(s[12..16].try_into().unwrap()) as i16;
        let glyph = u32::from_le_bytes(s[16..20].try_into().unwrap()) as u16;
        let tint = u32::from_le_bytes(s[20..24].try_into().unwrap());
        out.extend_from_slice(&yaw.to_le_bytes());
        out.extend_from_slice(&glyph.to_le_bytes());
        out.extend_from_slice(&tint.to_le_bytes());
    }
    out
}

#[cfg(test)]
#[path = "tests/oracle_tests.rs"]
mod tests;
