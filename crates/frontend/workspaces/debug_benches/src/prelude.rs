//! The items most callers of `debug_benches` name: the four route components the app mounts
//! (browser build only) and the interior lane set both map benches draw on.

#[cfg(target_arch = "wasm32")]
pub use crate::ballistics_agreement::BallisticsAgreementPage;
pub use crate::building_interior::InteriorLanes;
#[cfg(target_arch = "wasm32")]
pub use crate::building_viewer::BuildingViewerPage;
#[cfg(target_arch = "wasm32")]
pub use crate::data_viewer::DataViewerPage;
#[cfg(target_arch = "wasm32")]
pub use crate::world_los::WorldLosPage;
