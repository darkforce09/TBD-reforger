//! Small shared helpers with no domain of their own.
//!
//! **Role:** date and time formatting, the live countdown, and the sanitiser that guards values on
//! their way into an attribute.
//! **Position:** called at render time from anywhere; nothing here fetches, stores or decides.
//! **Signals & state:** none, though the time helpers read the browser clock and time zone.
//! **Invariants:** every function is total — an input that cannot be read produces a placeholder
//! rather than a panic.

pub mod countdown;
pub mod datefmt;
pub mod sanitize;

#[allow(unused_imports)]
pub use countdown::countdown_label;
pub use sanitize::safe_avatar_url;
