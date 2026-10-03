//! The shared visual primitives: the pieces every page and panel is assembled from.
//!
//! **Role:** groups the design-system primitives — the icon, the page header, the status pill, the
//! three form controls with the state recipes they wear, and the two overlay surfaces with the
//! registry they share.
//! **Position:** the bottom of the foundation: pages, workspaces and the session's content gates
//! compose these, and nothing here imports the session, the transport or any page.
//! **Signals & state:** the primitives are stateless. The one exception is the overlay registry,
//! which tracks which surfaces are open so that the dismiss key and the paint order agree.
//! **Invariants:** a primitive holds no state that belongs to its caller. The form controls are
//! uncontrolled by construction: the value is read from the caller's signal and written to the DOM
//! property, so ownership stays with the caller and a fast-changing value costs a property write
//! rather than a render.

pub mod badge;
#[cfg(target_arch = "wasm32")]
pub mod dialog;
#[cfg(any(target_arch = "wasm32", test))]
pub mod icons;
#[cfg(any(target_arch = "wasm32", test))]
pub mod modal_stack;
#[cfg(any(target_arch = "wasm32", test))]
pub mod page_header;
pub mod search_box;
#[cfg(target_arch = "wasm32")]
pub mod select;
pub mod sheet;
#[cfg(target_arch = "wasm32")]
pub mod slider;
#[cfg(any(target_arch = "wasm32", test))]
pub mod split_pane;
#[cfg(target_arch = "wasm32")]
pub mod toast;
#[cfg(any(target_arch = "wasm32", test))]
pub mod tokens;

#[cfg(target_arch = "wasm32")]
pub use badge::badge_class;
#[cfg(target_arch = "wasm32")]
pub use dialog::Dialog;
#[cfg(any(target_arch = "wasm32", test))]
pub use icons::MaterialIcon;
#[cfg(any(target_arch = "wasm32", test))]
pub use icons::DEFAULT_AVATAR;
#[cfg(target_arch = "wasm32")]
pub use page_header::cn;
#[cfg(target_arch = "wasm32")]
pub use page_header::PageHeader;
#[cfg(target_arch = "wasm32")]
pub use search_box::SearchBox;
#[cfg(target_arch = "wasm32")]
pub use select::Select;
#[cfg(target_arch = "wasm32")]
pub use sheet::Sheet;
#[cfg(target_arch = "wasm32")]
pub use slider::Slider;

#[cfg(test)]
#[path = "tests/ui.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/ui_t633_range_and_select.rs"]
mod t633_range_and_select;
