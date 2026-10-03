//! Starred asset storage and right-dock views.

use super::*;

mod panels;
mod store;

#[cfg(target_arch = "wasm32")]
pub(super) use panels::*;
#[cfg(target_arch = "wasm32")]
pub use store::load_favourites;
#[cfg(target_arch = "wasm32")]
pub use store::save_favourites;
#[cfg(test)]
pub use store::FavouriteAsset;
pub(super) use store::*;
pub use store::{resolve_favourites, FavouriteRow, Favourites};
