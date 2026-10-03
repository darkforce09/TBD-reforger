//! Trigger authoring controls and owner-link display.

#[cfg(target_arch = "wasm32")]
use super::*;

mod attributes;
mod owner_line;
mod panel;

#[cfg(target_arch = "wasm32")]
use attributes::*;
#[cfg(target_arch = "wasm32")]
use owner_line::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use panel::triggers_panel;
