//! The shared handle the map host and its loaders reach the asset sink through.
//!
//! **Role:** [`MapAssetSinkSlot`], a place that holds a sink once the renderer has booted, and
//! [`SharedMapAssetSink`], the reference-counted handle over a slot the host, the loaders and
//! their futures share.
//! **Position:** the embedding frontend creates an `Rc<RefCell<Option<Renderer>>>`, which
//! coerces to a [`SharedMapAssetSink`]; the host and loaders borrow it per write.
//! **Signals & state:** the slot's interior mutability (`RefCell`); one thread, the page's.
//! **Invariants:** an empty slot hands out no sink, so every write before boot or after teardown
//! is a no-op at the call site; a borrow lives for one write sequence and never across an await.

use crate::asset_sink::sink::MapAssetSink;
use std::cell::RefCell;
use std::rc::Rc;

/// A place that holds a map asset sink once there is one.
pub trait MapAssetSinkSlot {
    /// The browser image handle of the sink the slot holds.
    type BrowserImage;

    /// The held sink for reading, or `None` while the slot is empty.
    fn sink(&self) -> Option<&dyn MapAssetSink<BrowserImage = Self::BrowserImage>>;

    /// The held sink for writing, or `None` while the slot is empty.
    fn sink_mut(&mut self) -> Option<&mut dyn MapAssetSink<BrowserImage = Self::BrowserImage>>;
}

impl<S: MapAssetSink> MapAssetSinkSlot for Option<S> {
    type BrowserImage = S::BrowserImage;

    fn sink(&self) -> Option<&dyn MapAssetSink<BrowserImage = S::BrowserImage>> {
        self.as_ref()
            .map(|sink| sink as &dyn MapAssetSink<BrowserImage = S::BrowserImage>)
    }

    fn sink_mut(&mut self) -> Option<&mut dyn MapAssetSink<BrowserImage = S::BrowserImage>> {
        self.as_mut()
            .map(|sink| sink as &mut dyn MapAssetSink<BrowserImage = S::BrowserImage>)
    }
}

/// The shared handle over a sink slot; an `Rc<RefCell<Option<S>>>` of any sink `S` with this
/// browser image type coerces to it.
pub type SharedMapAssetSink<BrowserImage> =
    Rc<RefCell<dyn MapAssetSinkSlot<BrowserImage = BrowserImage>>>;
