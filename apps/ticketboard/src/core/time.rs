use std::time::{SystemTime, UNIX_EPOCH};
// ---- OS integration ----

/// Wall-clock seconds since the Unix epoch — the banner's timestamp source
/// (rendered as explicit UTC; the registry's own timestamps are UTC too).
pub(crate) fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
/// `"HH:MM:SS UTC"` from seconds since the Unix epoch — explicit-UTC on purpose:
/// no timezone dependency, and the registry's own timestamps are UTC.
pub fn utc_hms(secs_since_epoch: u64) -> String {
    let h = (secs_since_epoch / 3600) % 24;
    let m = (secs_since_epoch / 60) % 60;
    let s = secs_since_epoch % 60;
    format!("{h:02}:{m:02}:{s:02} UTC")
}
