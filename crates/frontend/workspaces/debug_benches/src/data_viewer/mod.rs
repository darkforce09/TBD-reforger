//! URL-only inspection of the complete published Workbench dataset.
//!
//! The panel modules are public in the browser build: a component used outside its own module is
//! a documented `pub` item of a `pub mod`, since leptos's `#[component]` derives a typed builder
//! whose `build` method is always `pub`.
#[cfg(target_arch = "wasm32")]
mod data;
#[cfg(target_arch = "wasm32")]
pub mod field_inventory;
#[cfg(target_arch = "wasm32")]
pub mod layout;
pub mod navigation_state;
#[cfg(target_arch = "wasm32")]
pub mod overview;
/// The route component and the viewer context its panels share.
#[cfg(target_arch = "wasm32")]
pub mod page;
#[cfg(target_arch = "wasm32")]
pub mod relationships;
#[cfg(target_arch = "wasm32")]
pub mod resources;
#[cfg(target_arch = "wasm32")]
pub mod source_inspector;
#[cfg(target_arch = "wasm32")]
pub use page::DataViewerPage;

pub mod browsing_state;
#[cfg(target_arch = "wasm32")]
pub mod resource_data;
