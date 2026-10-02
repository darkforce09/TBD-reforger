//! Formatted messages to the browser console.
//!
//! **Role:** [`crate::console_log!`], [`crate::console_warn!`] and [`crate::console_error!`]
//! format their arguments like `format!` and write the text to the browser console at their
//! level, through [`log`], [`warn`] and [`error`].
//! **Position:** foundation, wasm32 only, over `web_sys::console`; the map engine's satellite,
//! preference and occluder loaders log through it.
//! **Signals & state:** none; each call writes one console line.
//! **Invariants:** one macro call evaluates its arguments once and writes exactly one message at
//! its macro's level.

/// Writes `message` to the browser console at the log level.
pub fn log(message: &str) {
    web_sys::console::log_1(&message.into());
}

/// Writes `message` to the browser console at the warning level.
pub fn warn(message: &str) {
    web_sys::console::warn_1(&message.into());
}

/// Writes `message` to the browser console at the error level.
pub fn error(message: &str) {
    web_sys::console::error_1(&message.into());
}

/// Formats its arguments like `format!` and writes the text to the browser console at the log
/// level.
#[macro_export]
macro_rules! console_log {
    ($($argument:tt)*) => {
        $crate::console::log(&::std::format!($($argument)*))
    };
}

/// Formats its arguments like `format!` and writes the text to the browser console at the
/// warning level.
#[macro_export]
macro_rules! console_warn {
    ($($argument:tt)*) => {
        $crate::console::warn(&::std::format!($($argument)*))
    };
}

/// Formats its arguments like `format!` and writes the text to the browser console at the error
/// level.
#[macro_export]
macro_rules! console_error {
    ($($argument:tt)*) => {
        $crate::console::error(&::std::format!($($argument)*))
    };
}
