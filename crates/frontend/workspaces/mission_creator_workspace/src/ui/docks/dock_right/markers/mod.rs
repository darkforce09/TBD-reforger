//! Marker icon vocabulary and authoring panel.

#[cfg(target_arch = "wasm32")]
use super::*;

#[cfg(target_arch = "wasm32")]
mod glyph_preview;
mod panel;

#[cfg(target_arch = "wasm32")]
use glyph_preview::marker_glyph_svg;
#[cfg(target_arch = "wasm32")]
use mission_creator_state::marker_icons::{
    canonical_marker_rows, marker_icon_is_authorable, marker_icons,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use panel::markers_panel;
