//! Byte counts written for people to read.
//!
//! **Role:** turns a byte count into the short size label the interface shows beside a file,
//! payload or download.
//! **Position:** called at render time: [`format_bytes`] by the mission library's upload panel and
//! by the Mission Creator's toolbelt and top strip, [`format_download_size`] by the modpacks page
//! and the dashboard's modpack card; it reads nothing and stores nothing.
//! **Signals & state:** none; pure functions.
//! **Invariants:** [`format_bytes`] uses decimal units (1 KB = 1 000 B), bytes and kilobytes are
//! whole numbers, megabytes and gigabytes carry one decimal. [`format_download_size`] uses binary
//! units (1 MB = 1 024² B, 1 GB = 1 024³ B), whole megabytes below a gigabyte. Every count has a
//! label, so both functions are total.

/// A byte count in decimal units: "999 B", "2 KB", "141.6 MB", "1.5 GB".
///
/// Below 1 000 the exact count is written in bytes; kilobytes round to a whole number; megabytes
/// and gigabytes round to one decimal. Gigabytes are the largest unit.
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

/// A download size in binary units, as the platform quotes modpack sizes: "0 B", "500 MB",
/// "1.5 GB".
///
/// A count below one byte (zero or a negative wire value) reads `0 B`; from one gibibyte
/// (1 024³ B) up the size is written in gigabytes to one decimal; everything between rounds to
/// whole megabytes of 1 024² B, so a non-zero count under half a megabyte reads `0 MB`.
#[must_use]
pub fn format_download_size(bytes: i64) -> String {
    if bytes < 1 {
        return "0 B".into();
    }
    let gb = bytes as f64 / 1024f64.powi(3);
    if gb >= 1.0 {
        return format!("{gb:.1} GB");
    }
    format!("{:.0} MB", bytes as f64 / 1024f64.powi(2))
}

#[cfg(test)]
#[path = "tests/byte_formatting.rs"]
mod tests;
