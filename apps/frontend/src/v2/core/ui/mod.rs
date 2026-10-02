//! The shared visual primitives: the pieces every page and panel is assembled from.
//!
//! **Role:** groups the design-system primitives — the icon, the page header, the status pill, the
//! content gates, the three form controls, and the two overlay surfaces with the registry they
//! share.
//! **Position:** the top of the presentation layer. Pages compose these; nothing here knows about
//! any particular page.
//! **Signals & state:** the primitives are stateless. The one exception is the overlay registry,
//! which tracks which surfaces are open so that the dismiss key and the paint order agree.
//! **Invariants:** a primitive holds no state that belongs to its caller. The form controls are
//! uncontrolled by construction: the value is read from the caller's signal and written to the DOM
//! property, so ownership stays with the caller and a fast-changing value costs a property write
//! rather than a render.

pub mod badge;
pub mod dialog;
pub mod gates;
pub mod icons;
pub mod modal_stack;
pub mod page_header;
pub mod search_box;
pub mod select;
pub mod sheet;
pub mod slider;
pub mod split_pane;
pub mod toast;

#[allow(unused_imports)]
pub use badge::badge_class;
pub use dialog::Dialog;
pub use gates::{AdminGate, AuthGate};
pub use icons::{MaterialIcon, DEFAULT_AVATAR};
pub use page_header::{cn, PageHeader};
#[allow(unused_imports)]
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
