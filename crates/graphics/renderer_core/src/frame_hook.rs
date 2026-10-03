//! The per-frame callbacks a renderer runs before it encodes a frame.
//!
//! **Role:** [`FrameHook`] is what a layer implements to hear that the camera changed (and
//! re-derive what depends on it, such as a zoom uniform) and that a frame is about to be encoded
//! (and commit work it deferred, such as a compute cull's inputs); [`FrameHooks`] is the
//! registration-ordered list the renderer owns and runs.
//! **Position:** the renderer registers each layer that needs a callback once, calls
//! [`FrameHooks::camera_changed`] after it moves its camera and [`FrameHooks::before_encode`]
//! once per submitted frame before it encodes; each hook receives the renderer (or the part of it
//! the renderer lends), typed by the `Renderer` parameter.
//! **Signals & state:** the boxed hooks and whatever state each hook keeps.
//! **Invariants:** hooks run in the order they were registered, every hook on every call; a hook
//! that does not override a callback does nothing for it; the list never reorders or drops a hook.

/// A layer's per-frame callbacks.
///
/// `Renderer` is what the renderer lends a hook while it runs: the renderer itself, or a view of
/// the parts a hook may touch. Both callbacks default to doing nothing, so a hook overrides only
/// the ones it needs.
pub trait FrameHook<Renderer: ?Sized> {
    /// The camera moved, zoomed or resized since the last call.
    fn camera_changed(&mut self, renderer: &mut Renderer) {
        let _ = renderer;
    }

    /// A frame is about to be encoded.
    fn before_encode(&mut self, renderer: &mut Renderer) {
        let _ = renderer;
    }
}

/// The renderer's hooks, in registration order.
pub struct FrameHooks<Renderer: ?Sized> {
    hooks: Vec<Box<dyn FrameHook<Renderer>>>,
}

impl<Renderer: ?Sized> Default for FrameHooks<Renderer> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Renderer: ?Sized> FrameHooks<Renderer> {
    /// No hook registered.
    #[must_use]
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Register `hook` after every hook registered before it.
    pub fn register(&mut self, hook: Box<dyn FrameHook<Renderer>>) {
        self.hooks.push(hook);
    }

    /// How many hooks are registered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// Whether no hook is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Tell every hook, in registration order, that the camera changed.
    pub fn camera_changed(&mut self, renderer: &mut Renderer) {
        for hook in &mut self.hooks {
            hook.camera_changed(renderer);
        }
    }

    /// Tell every hook, in registration order, that a frame is about to be encoded.
    pub fn before_encode(&mut self, renderer: &mut Renderer) {
        for hook in &mut self.hooks {
            hook.before_encode(renderer);
        }
    }
}

#[cfg(test)]
#[path = "tests/frame_hook_tests.rs"]
mod tests;
