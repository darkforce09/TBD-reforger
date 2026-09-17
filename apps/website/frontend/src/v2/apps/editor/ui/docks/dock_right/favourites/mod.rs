//! Starred asset storage and right-dock views.

use super::*;

mod panels;
mod store;

pub(super) use panels::*;
pub(super) use store::*;
pub use store::{
    load_favourites, resolve_favourites, save_favourites, FavouriteAsset, FavouriteRow, Favourites,
};
