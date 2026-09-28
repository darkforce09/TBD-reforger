//! URL-only inspection of the complete published Workbench dataset.
#[cfg(target_arch = "wasm32")]
mod data;
#[cfg(target_arch = "wasm32")]
mod field_inventory;
#[cfg(target_arch = "wasm32")]
mod layout;
pub mod navigation_state;
#[cfg(target_arch = "wasm32")]
mod overview;
#[cfg(target_arch = "wasm32")]
mod page;
#[cfg(target_arch = "wasm32")]
mod relationships;
#[cfg(target_arch = "wasm32")]
mod resources;
#[cfg(target_arch = "wasm32")]
mod source_inspector;
#[cfg(target_arch = "wasm32")]
pub use page::DataViewerPage;
/// The viewer page on non-browser targets, where its panels are not compiled: it renders nothing
/// and keeps the route table building for the native test suite.
#[cfg(not(target_arch = "wasm32"))]
#[leptos::component]
pub fn DataViewerPage() -> impl leptos::IntoView {
    ()
}

pub mod browsing_state;
#[cfg(target_arch = "wasm32")]
mod resource_data;
