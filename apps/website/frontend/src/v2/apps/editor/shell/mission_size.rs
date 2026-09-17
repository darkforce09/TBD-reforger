//! T-172 B9 — the toolbelt `SZ` payload estimate (missionSize.ts port). Pure + native-tested:
//! sample ≤ `SAMPLE_N` slots' JSON byte lengths, average × slot count + a fixed envelope for the
//! non-slot payload. Decimal `format_bytes` (one decimal from MB up), `—` handled by the caller.

/// Fixed overhead for the non-slot payload parts (meta/map/editor envelope) — missionSize.ts.
pub const SIZE_ENVELOPE_BYTES: usize = 2048;
/// How many slots the estimator serializes before extrapolating.
pub const SIZE_SAMPLE_N: usize = 20;

/// Estimate the compiled payload size from the doc's `slots_json` (an object keyed by slot id).
/// `None` when there are no slots (the readout shows `—`).
#[must_use]
pub fn estimate_compiled_bytes(slots_json: &str) -> Option<usize> {
    let slots: serde_json::Value = serde_json::from_str(slots_json).ok()?;
    let map = slots.as_object()?;
    let n = map.len();
    if n == 0 {
        return None;
    }
    let (mut sum, mut sampled) = (0usize, 0usize);
    for v in map.values().take(SIZE_SAMPLE_N) {
        sum += v.to_string().len();
        sampled += 1;
    }
    if sampled == 0 {
        return Some(SIZE_ENVELOPE_BYTES);
    }
    Some(sum / sampled * n + SIZE_ENVELOPE_BYTES)
}

/// Decimal byte formatter (lib/format.ts `formatBytes` shape): B and KB whole, MB/GB one decimal.
#[must_use]
pub fn format_bytes(bytes: usize) -> String {
    let b = bytes as f64;
    if b < 1_000.0 {
        format!("{bytes} B")
    } else if b < 1_000_000.0 {
        format!("{:.0} KB", b / 1_000.0)
    } else if b < 1_000_000_000.0 {
        format!("{:.1} MB", b / 1_000_000.0)
    } else {
        format!("{:.1} GB", b / 1_000_000_000.0)
    }
}

#[cfg(test)]
#[path = "tests/mission_size/estimation.rs"]
mod tests;
