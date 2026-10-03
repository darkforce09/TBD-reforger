//! Number formatting for the metrics tables.
//!
//! **Role:** `format_tokens` and `format_elapsed`.
//! **Position:** part of `crate::execution_metrics::measured`; the estimated tables reuse
//! `format_tokens`.
//! **Signals & state:** none; pure functions.
//! **Invariants:** formatting never changes a value, only its text.

// ---- display formatting ----

/// `1234567` → `"1,234,567"`.
pub fn format_tokens(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Whole seconds → `"45s"` / `"3m 00s"` / `"1h 02m 03s"`.
pub fn format_elapsed(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}
