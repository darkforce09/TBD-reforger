//! Small shared helpers with no domain of their own.
//!
//! **Role:** date and time formatting, the UTC instants read and written without the browser
//! clock, the live countdown, the sanitiser that guards values on their way into an attribute, and
//! the one clipboard write, which reports whether the copy landed.
//! **Position:** called at render time from anywhere, and the clipboard write from click handlers;
//! nothing here fetches, stores or decides.
//! **Signals & state:** none, though the time helpers read the browser clock and time zone.
//! **Invariants:** every function is total — an input that cannot be read produces a placeholder
//! rather than a panic.

pub mod clipboard;
pub mod countdown;
pub mod datefmt;
pub mod sanitize;
pub mod utc_timestamp;

#[allow(unused_imports)]
pub use countdown::countdown_label;
pub use sanitize::safe_avatar_url;
