//! Sanitising the values the interface renders into attributes.
//!
//! **Role:** turns a stored value into one that is safe to put in a link or an image source.
//! **Position:** called at render time, on the way into the attribute.
//! **Signals & state:** none.
//! **Invariants:** only *stored* values are filtered. The placeholder this falls back to is a data
//! URI the app ships itself, so it deliberately bypasses the scheme allowlist — filtering it would
//! reject the very value the filter falls back to.

#[cfg(any(target_arch = "wasm32", test))]
use super::super::ui::DEFAULT_AVATAR;
#[cfg(any(target_arch = "wasm32", test))]
use http_url_guard::is_http_url;

/// An avatar source: the stored URL when it is an ordinary web address, and the shipped placeholder
/// otherwise.
#[cfg(any(target_arch = "wasm32", test))]
pub fn safe_avatar_url(stored: &str) -> String {
    if is_http_url(stored) {
        stored.to_string()
    } else {
        DEFAULT_AVATAR.to_string()
    }
}
