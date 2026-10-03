//! The compiled payload-size estimate of the open mission.
//!
//! **Role:** estimates how many bytes the compiled mission payload takes, from the document's
//! slots, for the Mission Creator's toolbelt and top strip readouts.
//! **Position:** called by the Mission Creator's page effects and its top strip overlay; the
//! readouts label the estimate with [`crate::foundation::utils::byte_formatting::format_bytes`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** at most [`SIZE_SAMPLE_N`] slots are serialized; the estimate is their mean size
//! times the slot count plus [`SIZE_ENVELOPE_BYTES`], and no slots means no estimate.

/// Fixed overhead for the non-slot payload parts (the meta, map and editor envelope).
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

#[cfg(test)]
#[path = "tests/mission_size/estimation.rs"]
mod tests;
