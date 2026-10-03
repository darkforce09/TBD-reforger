//! The icon glyph, and the placeholder avatar.
//!
//! **Role:** the two smallest shared visual primitives — a font-ligature icon span, and the neutral
//! avatar shown when an account has no picture of its own.
//! **Position:** used everywhere; neither owns any layout.
//! **Signals & state:** none.
//! **Invariants:** the icon is a font ligature rather than an image, so its name is the text
//! content and its size follows the surrounding font size. The avatar is an inline data URI, so it
//! needs no network request and cannot fail to load.

use leptos::prelude::*;

use super::page_header::cn;

/// A neutral inline avatar, shown when an account has no picture of its own.
///
/// An encoded SVG data URI rather than a file, so it renders without a request and never 404s.
pub const DEFAULT_AVATAR: &str = "data:image/svg+xml;utf8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2064%2064%22%3E%3Crect%20width%3D%2264%22%20height%3D%2264%22%20rx%3D%228%22%20fill%3D%22%23394150%22%2F%3E%3Ccircle%20cx%3D%2232%22%20cy%3D%2225%22%20r%3D%2212%22%20fill%3D%22%237a8699%22%2F%3E%3Cpath%20d%3D%22M12%2058c0-11%209-19%2020-19s20%208%2020%2019z%22%20fill%3D%22%237a8699%22%2F%3E%3C%2Fsvg%3E";

/// A Material Symbols icon: a span whose text content is the ligature name.
///
/// Renders one `<span>` carrying the icon font class plus whatever `class` adds. `filled` selects the
/// filled variant by setting the font variation axis inline. Has no slot of its own — callers place it
/// directly wherever a glyph belongs.
#[component]
pub fn MaterialIcon(
    name: &'static str,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] filled: bool,
) -> impl IntoView {
    let style = filled.then_some("font-variation-settings: \"FILL\" 1;");
    view! { <span class=cn(&["material-symbols-outlined", class]) style=style>{name}</span> }
}
