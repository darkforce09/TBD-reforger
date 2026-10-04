//! The single-page app's design-system primitives and its small shared helpers.
//!
//! **Role:** the pieces every page and panel is assembled from — the icon, the page header, the
//! status pill, the three form controls with the state recipes they wear, the split pane, the
//! toasts, and the two overlay surfaces with the registry they share — and the helpers with no
//! domain of their own: byte, date and countdown formatting, UTC instants, the content URL policy,
//! the avatar sanitiser and the one clipboard write.
//! **Position:** the bottom of the frontend foundation crates: pages, workspaces and the session's
//! content gates compose these, and nothing here imports the session, the transport or any page.
//! **Signals & state:** the primitives are stateless, except the overlay registry
//! ([`modal_stack`]), which tracks which surfaces are open so that the dismiss key and the paint
//! order agree; the time helpers read the browser clock and time zone.
//! **Invariants:** a primitive holds no state that belongs to its caller, and the form controls
//! are uncontrolled by construction (the value is read from the caller's signal and written to the
//! DOM property); every helper is total — an input it cannot read produces a placeholder rather
//! than a panic. Only code that calls a browser API directly is gated to the wasm32 build.

pub mod badge;
pub mod byte_formatting;
pub mod clipboard;
pub mod countdown;
pub mod datefmt;
pub mod dialog;
pub mod icons;
pub mod modal_stack;
pub mod page_header;
pub mod prelude;
pub mod safe_url;
pub mod sanitize;
pub mod search_box;
pub mod select;
pub mod sheet;
pub mod slider;
pub mod split_pane;
pub mod toast;
pub mod tokens;
pub mod utc_timestamp;

pub use badge::badge_class;
pub use dialog::Dialog;
pub use icons::{DEFAULT_AVATAR, MaterialIcon};
pub use page_header::{PageHeader, cn};
pub use sanitize::safe_avatar_url;
pub use search_box::SearchBox;
pub use select::Select;
pub use sheet::Sheet;
pub use slider::Slider;

#[cfg(test)]
#[path = "tests/ui.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/ui_t633_range_and_select.rs"]
mod t633_range_and_select;

/// The production text of the primitives' source files, for the guard tests that pin it.
#[cfg(test)]
#[path = "tests/source_pins.rs"]
mod source_pins;
