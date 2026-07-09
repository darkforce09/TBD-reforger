//! T-151.8 — pack R32Uint chunk counts → RGBA8 heatmap texels (visual only).
//! Class R switch/texel-sum gates live in `map-engine-core::density_ladder`; this is display.

/// Convert little-endian u32 count grid → RGBA8Unorm heatmap (transparent zeros).
#[must_use]
pub fn density_counts_to_rgba(counts_le: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let n = (width as usize).checked_mul(height as usize)?;
    if counts_le.len() != n * 4 {
        return None;
    }
    let mut max_c = 1u32;
    for i in 0..n {
        let v = u32::from_le_bytes(counts_le[i * 4..i * 4 + 4].try_into().ok()?);
        max_c = max_c.max(v);
    }
    let mut rgba = vec![0u8; n * 4];
    for i in 0..n {
        let v = u32::from_le_bytes(counts_le[i * 4..i * 4 + 4].try_into().ok()?);
        if v == 0 {
            continue;
        }
        let t = (f64::from(v) / f64::from(max_c)).clamp(0.0, 1.0);
        let g = (40.0 + 180.0 * t) as u8;
        let a = (50.0 + 160.0 * t) as u8;
        let o = i * 4;
        rgba[o] = 30;
        rgba[o + 1] = g;
        rgba[o + 2] = 50;
        rgba[o + 3] = a;
    }
    Some(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_grid_is_transparent() {
        let counts = [0u8; 4 * 2 * 2];
        let rgba = density_counts_to_rgba(&counts, 2, 2).unwrap();
        assert!(rgba.iter().all(|&b| b == 0));
    }

    #[test]
    fn nonzero_gets_alpha() {
        let mut counts = vec![0u8; 4];
        counts[0..4].copy_from_slice(&100u32.to_le_bytes());
        let rgba = density_counts_to_rgba(&counts, 1, 1).unwrap();
        assert!(rgba[3] > 0);
    }
}
