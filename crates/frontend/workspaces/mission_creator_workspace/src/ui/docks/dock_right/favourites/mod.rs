//! Starred asset storage and right-dock views.

use super::*;

mod panels;
mod store;

#[cfg(target_arch = "wasm32")]
pub(super) use panels::*;
#[cfg(test)]
pub(crate) use store::FavouriteAsset;
pub use store::Favourites;
#[cfg(target_arch = "wasm32")]
pub use store::load_favourites;
#[cfg(target_arch = "wasm32")]
pub(super) use store::save_favourites;
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use store::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use store::{FavouriteRow, resolve_favourites};
