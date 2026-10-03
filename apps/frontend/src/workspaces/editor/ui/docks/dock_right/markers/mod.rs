//! Marker icon vocabulary and authoring panel.

#[cfg(target_arch = "wasm32")]
use super::*;

mod icons;
mod panel;

pub(super) use icons::*;
#[cfg(test)]
pub use icons::{default_marker_icon, filter_marker_icons};
pub use icons::{marker_icon_is_authorable, marker_icons};
#[cfg(target_arch = "wasm32")]
pub(crate) use panel::markers_panel;
