//! Role: console.
//! Position: `diagnostics/platform` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

macro_rules! graphics_warn {
    ($($arg:tt)*) => { web_sys::console::warn_1(&format!($($arg)*).into()) };
}
macro_rules! graphics_error {
    ($($arg:tt)*) => { web_sys::console::error_1(&format!($($arg)*).into()) };
}
macro_rules! graphics_log {
    ($($arg:tt)*) => { web_sys::console::log_1(&format!($($arg)*).into()) };
}

/// Re-export `graphics_erroraserror`.
pub(crate) use graphics_error as error;

/// Re-export `graphics_logaslog`.
pub(crate) use graphics_log as log;

/// Re-export `graphics_warnaswarn`.
pub(crate) use graphics_warn as warn;
