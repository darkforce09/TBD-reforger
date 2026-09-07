//! T-151.8.1 — instance frustum cull / compaction (Class R).
//!
//! Shared AABB rule for CPU reference and the WebGPU compute shader:
//! an icon survives iff its axis-aligned square `[pos ± half]` intersects the frustum.
//! `half = size * 0.5` (glyph size is already world meters after min-px).
//!
//! WebGL2 has no storage buffers — callers keep chunk-granularity draw-set cull and may
//! still run this CPU compact for parity tests.

use bytemuck::{Pod, Zeroable};

/// Compute workgroup size in `cs_icon_cull` (`shader.wgsl`).
pub const CULL_WORKGROUP: usize = 64;

/// Icon instance stride (matches [`crate::scene::IconInstance`] / glyph pack).
pub const ICON_STRIDE: usize = 20;

/// Frustum in the same space as icon `pos` (anchor-relative meters for GPU buffers).
pub type Frustum = [f64; 4]; // min_x, min_y, max_x, max_y

/// One icon as read from a packed 20 B stream (no padding).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct IconCullSample {
    pub pos: [f32; 2],
    pub size: f32,
    pub yaw: i16,
    pub glyph: u16,
    pub tint: u32,
}

/// True when the icon's AABB intersects `[min_x,min_y,max_x,max_y]` (inclusive edges).
///
/// T-151.11.4 (audit X-03): the comparison runs entirely in **f32** — the frustum is quantized
/// exactly like the GPU uniform (`params` are `f32` in `encode_cull` / the WGSL shader), so the
/// CPU oracle and the GPU kernel share one arithmetic domain and their counts are equal by
/// construction, not merely "close". (Previously the CPU widened to f64 while the GPU ran f32 —
/// an icon exactly on a quantization-shifted edge could disagree.)
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

/// Compact `src` (packed 20 B icons) into `dst`, preserving encounter order.
/// Returns the surviving instance count (Class R).
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
///
/// T-938.3: the scan runs **only** when the debug HUD flag is on. Otherwise the visible
/// count comes from the GPU counter readback (may lag one frame).
#[must_use]
pub fn cpu_count_for_encode(src: &[u8], frustum: Frustum, debug_hud: bool) -> Option<u32> {
    if !debug_hud {
        return None;
    }
    Some(count_icons_in_frustum(src, frustum))
}

/// `true` when `cs_icon_cull` hits a `workgroupBarrier` **before** the workgroup
/// `atomicAdd` of the reduced count. Removing that barrier is the T-938.3 perturbation.
#[must_use]
pub fn shader_reduce_barrier_before_atomic() -> bool {
    let src = include_str!("shader.wgsl");
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
///
/// `encode_cull` encodes all lanes into ONE `CommandEncoder` and the frame is one submit.
/// `Queue::write_buffer` is staged: every write in a submit is applied before any command
/// buffer in it runs. So a `cull_params` buffer owned by `IconComputeCull` and rewritten
/// per lane hands EVERY lane the `src_count` of whichever lane was encoded last — and the
/// lane order is `HashMap::keys()`, so which one wins is nondeterministic per run.
///
/// With one lane that was invisible. T-938.3 made nine lanes normal (`upload_icon_lane` for
/// trees/props/badges, `upload_slot_role_lane` for slots/drag/clusters/vehicles/comments/
/// preview), so on a loaded mission the short lane's count silently truncates the long lane —
/// 368 of 380 slot icons vanish — or the long count over-runs the short lane's buffers.
///
/// `icon_cull_gpu` is `cfg(target_arch = "wasm32")`, so this is the native harness for it:
/// the ownership is read off the source. Reverting either site to `self.params_buf` is the
/// perturbation and turns [`per_lane_cull_params_not_shared`] red.
#[must_use]
pub fn cull_params_is_per_lane() -> bool {
    let src = include_str!("icon_cull_gpu.rs");
    let prod = src.split("#[cfg(test)]").next().unwrap_or(src);
    // The uniform is written from, and bound out of, the lane slot — never the shared struct.
    prod.contains("queue.write_buffer(&slot.params_buf, 0, &params);")
        && prod.contains("resource: slot.params_buf.as_entire_binding(),")
        // ...and no shared copy survives on IconComputeCull to be bound by accident.
        && !prod.contains("self.params_buf")
}

/// GPU visible count as the workgroup-local reduce in `cs_icon_cull` would produce.
///
/// With the reduce-barrier present this equals [`count_icons_in_frustum`]. Without it,
/// only local-id 0's visibility bit is visible to the reduce (the race).
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

/// Pack a 32 B storage-friendly record for the WebGPU compute shader (std430-ish).
/// Layout: pos.xy, size, yaw_i32, glyph_u32, tint_u32, pad_u32×2 → 32 B.
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
        d[0..12].copy_from_slice(&s[0..12]); // pos + size
        let yaw = i16::from_le_bytes(s[12..14].try_into().unwrap()) as i32;
        let glyph = u32::from(u16::from_le_bytes(s[14..16].try_into().unwrap()));
        let tint = u32::from_le_bytes(s[16..20].try_into().unwrap());
        d[12..16].copy_from_slice(&yaw.to_le_bytes());
        d[16..20].copy_from_slice(&glyph.to_le_bytes());
        d[20..24].copy_from_slice(&tint.to_le_bytes());
        // pad 24..32 already zero
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
        out.extend_from_slice(&s[0..12]); // pos + size
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
mod tests {
    use super::*;

    fn pack_one(x: f32, y: f32, size: f32) -> [u8; 20] {
        let mut b = [0u8; 20];
        b[0..4].copy_from_slice(&x.to_le_bytes());
        b[4..8].copy_from_slice(&y.to_le_bytes());
        b[8..12].copy_from_slice(&size.to_le_bytes());
        b
    }

    #[test]
    fn class_r_inside_outside() {
        let frustum = [0.0, 0.0, 100.0, 100.0];
        assert!(icon_intersects_frustum(50.0, 50.0, 10.0, frustum));
        assert!(!icon_intersects_frustum(200.0, 200.0, 10.0, frustum));
        // Edge touch
        assert!(icon_intersects_frustum(100.0, 50.0, 0.0, frustum));
    }

    #[test]
    fn class_r_compact_preserves_order_and_count() {
        let mut src = Vec::new();
        src.extend_from_slice(&pack_one(10.0, 10.0, 4.0)); // in
        src.extend_from_slice(&pack_one(500.0, 500.0, 4.0)); // out
        src.extend_from_slice(&pack_one(20.0, 20.0, 4.0)); // in
        let frustum = [0.0, 0.0, 100.0, 100.0];
        let (out, n) = compact_icons_cpu(&src, frustum);
        assert_eq!(n, 2);
        assert_eq!(out.len(), 40);
        assert_eq!(&out[0..20], &src[0..20]);
        assert_eq!(&out[20..40], &src[40..60]);
    }

    #[test]
    fn class_r_1k_random_frusta_count_stable() {
        // Deterministic LCG — two independent walks must agree (oracle self-check).
        let mut src = Vec::new();
        let mut s = 0xC0FFEE_u32;
        for _ in 0..500 {
            s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let x = (s >> 8) as f32 / 16_777_216.0 * 12800.0 - 6400.0;
            s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let y = (s >> 8) as f32 / 16_777_216.0 * 12800.0 - 6400.0;
            src.extend_from_slice(&pack_one(x, y, 8.0));
        }
        let mut seed = 42_u32;
        for _ in 0..1000 {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let cx = (seed >> 8) as f64 / 16_777_216.0 * 12800.0 - 6400.0;
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let cy = (seed >> 8) as f64 / 16_777_216.0 * 12800.0 - 6400.0;
            let half = 400.0;
            let frustum = [cx - half, cy - half, cx + half, cy + half];
            let a = count_icons_in_frustum(&src, frustum);
            let (bytes, b) = compact_icons_cpu(&src, frustum);
            assert_eq!(a, b);
            assert_eq!(bytes.len(), (b as usize) * ICON_STRIDE);
        }
    }

    #[test]
    fn storage32_roundtrip() {
        let mut src = Vec::new();
        src.extend_from_slice(&pack_one(1.5, -2.5, 3.0));
        let s32 = pack_icon_storage32(&src);
        assert_eq!(s32.len(), 32);
        let back = unpack_icon_storage32(&s32, 1);
        assert_eq!(back, src);
    }

    fn fixture_src_frustum() -> (Vec<u8>, Frustum) {
        let mut src = Vec::new();
        src.extend_from_slice(&pack_one(10.0, 10.0, 4.0)); // in
        src.extend_from_slice(&pack_one(500.0, 500.0, 4.0)); // out
        src.extend_from_slice(&pack_one(20.0, 20.0, 4.0)); // in
        src.extend_from_slice(&pack_one(30.0, 30.0, 4.0)); // in
        (src, [0.0, 0.0, 100.0, 100.0])
    }

    /// T-938.3 defect: `encode_cull` currently calls [`count_icons_in_frustum`] even
    /// when the debug HUD flag is off. This test is RED on that policy.
    #[test]
    fn t938_3_debug_hud_off_skips_cpu_count() {
        let (src, frustum) = fixture_src_frustum();
        let n = cpu_count_for_encode(&src, frustum, false);
        assert!(
            n.is_none(),
            "debug HUD off must skip the CPU frustum scan; got {n:?}"
        );
    }

    #[test]
    fn t938_3_gpu_visible_count_equals_cpu_on_fixture() {
        let (src, frustum) = fixture_src_frustum();
        let cpu = count_icons_in_frustum(&src, frustum);
        let gpu = gpu_workgroup_visible_count(&src, frustum);
        assert_eq!(cpu, 3);
        assert_eq!(
            gpu, cpu,
            "GPU workgroup reduce must match the CPU oracle on the fixture frustum"
        );
    }

    #[test]
    fn t938_3_debug_hud_on_still_counts() {
        let (src, frustum) = fixture_src_frustum();
        let n = cpu_count_for_encode(&src, frustum, true);
        assert_eq!(n, Some(3));
    }

    /// T-938.3 BLOCKER (wave 253): one shared `cull_params` uniform across nine lanes.
    #[test]
    fn per_lane_cull_params_not_shared() {
        assert!(
            cull_params_is_per_lane(),
            "every cull lane must own its cull_params uniform: one shared buffer plus one \
             staged write per lane makes all lanes read the last lane's src_count"
        );
    }
}
