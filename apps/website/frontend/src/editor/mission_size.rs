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
mod tests {
    use super::*;

    #[test]
    fn empty_slots_is_none() {
        assert_eq!(estimate_compiled_bytes("{}"), None);
        assert_eq!(estimate_compiled_bytes("not json"), None);
    }

    #[test]
    fn small_set_is_avg_times_n_plus_envelope() {
        // Two identical slots → avg = len(one), estimate = 2·len + envelope.
        let one = r#"{"x":1.0,"y":2.0,"role":"Rifleman"}"#;
        let slots = format!(r#"{{"s0":{one},"s1":{one}}}"#);
        let est = estimate_compiled_bytes(&slots).unwrap();
        let one_len = serde_json::from_str::<serde_json::Value>(one)
            .unwrap()
            .to_string()
            .len();
        assert_eq!(est, one_len * 2 + SIZE_ENVELOPE_BYTES);
    }

    #[test]
    fn format_bytes_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(2_048), "2 KB");
        assert_eq!(format_bytes(141_574_630), "141.6 MB");
        assert_eq!(format_bytes(1_500_000_000), "1.5 GB");
    }
}
