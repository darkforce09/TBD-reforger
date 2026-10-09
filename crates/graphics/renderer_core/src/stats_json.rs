//! The flat JSON object a renderer reports its statistics as.
//!
//! **Role:** [`StatsJson`] writes one flat JSON object field by field, in the order the caller
//! adds them: text, counts, flags, fixed-precision decimals and nullable decimals.
//! **Position:** the renderer's statistics report builds its object with it; the frontend's debug
//! HUD and the browser gates read the object back by key.
//! **Signals & state:** the object text under construction, owned by the writer.
//! **Invariants:** no whitespace between tokens; a decimal is written exactly as
//! `format!("{:.places$}", value)` writes it (so a non-finite value prints as Rust does, `NaN` or
//! `inf`); an absent nullable decimal is `null`; a quote, a backslash or a control character in a
//! key or a text value is escaped, every other character is written as given.

use std::fmt::Write as _;

/// A flat JSON object under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsJson {
    text: String,
    empty: bool,
}

impl Default for StatsJson {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsJson {
    /// An empty object.
    #[must_use]
    pub fn new() -> Self {
        Self {
            text: String::from("{"),
            empty: true,
        }
    }

    /// Add `"key":"value"`.
    pub fn text(&mut self, key: &str, value: &str) -> &mut Self {
        self.key(key);
        self.text.push('"');
        push_escaped(&mut self.text, value);
        self.text.push('"');
        self
    }

    /// Add `"key":value` for an unsigned count.
    pub fn count(&mut self, key: &str, value: impl Into<u64>) -> &mut Self {
        self.key(key);
        let _ = write!(self.text, "{}", value.into());
        self
    }

    /// Add `"key":true` or `"key":false`.
    pub fn flag(&mut self, key: &str, value: bool) -> &mut Self {
        self.key(key);
        self.text.push_str(if value { "true" } else { "false" });
        self
    }

    /// Add `"key":value` with exactly `places` digits after the decimal point.
    pub fn decimal(&mut self, key: &str, value: f64, places: usize) -> &mut Self {
        self.key(key);
        let _ = write!(self.text, "{value:.places$}");
        self
    }

    /// Add `"key":value` with exactly `places` decimals, or `"key":null` when there is no value.
    pub fn optional_decimal(&mut self, key: &str, value: Option<f64>, places: usize) -> &mut Self {
        match value {
            Some(v) => self.decimal(key, v, places),
            None => {
                self.key(key);
                self.text.push_str("null");
                self
            }
        }
    }

    /// Close the object and return its text.
    #[must_use]
    pub fn finish(mut self) -> String {
        self.text.push('}');
        self.text
    }

    fn key(&mut self, key: &str) {
        if !self.empty {
            self.text.push(',');
        }
        self.empty = false;
        self.text.push('"');
        push_escaped(&mut self.text, key);
        self.text.push_str("\":");
    }
}

/// Append `raw` with the JSON string escapes a quote, a backslash and a control character need.
fn push_escaped(out: &mut String, raw: &str) {
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
}
