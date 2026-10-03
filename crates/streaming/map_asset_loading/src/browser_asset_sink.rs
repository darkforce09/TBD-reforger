//! **Role:** the asset sink as the browser loaders hold it: [`BrowserAssetSink`], a
//! `map_streaming_model` sink whose browser image is an `ImageBitmap`, and
//! [`BrowserAssetSinkHandle`], the shared handle over its slot.
//! **Position:** `browser_asset_sink` in `map_asset_loading`; the map host, the world, forest,
//! label and relief loaders and the satellite loads take a [`BrowserAssetSinkHandle`]; the
//! embedding frontend passes its renderer cell, an `Rc<RefCell<Option<_>>>` of a sink, which coerces to it.
//! **Signals & state:** none here; the handle's slot is the frontend's.
//! **Invariants:** the loaders reach the renderer only through this handle.

use map_streaming_model::asset_sink::{MapAssetSink, SharedMapAssetSink};

/// The map asset sink the browser loaders write to, borrowed for `'sink`.
pub type BrowserAssetSink<'sink> = dyn MapAssetSink<BrowserImage = web_sys::ImageBitmap> + 'sink;

/// The shared handle over the browser asset sink's slot.
pub type BrowserAssetSinkHandle = SharedMapAssetSink<web_sys::ImageBitmap>;
