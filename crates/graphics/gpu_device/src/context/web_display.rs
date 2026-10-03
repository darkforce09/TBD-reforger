//! The web display handle and the instance descriptor every browser instance is built with.
//!
//! **Role:** [`crate::context::web_display::WebDisplay`], the display handle of the browser (there is one, and it carries no
//! data), and [`crate::context::web_display::instance_descriptor`], which builds a `wgpu::InstanceDescriptor` over it for the
//! backends asked for.
//! **Position:** [`crate::context::gpu_context::GpuContext::create`] builds its instance with it;
//! a caller that probes adapters before creating a context uses it directly.
//! **Signals & state:** none.
//! **Invariants:** `wasm32` only; the descriptor's backends are exactly the ones passed in.

/// The browser's display handle: a unit type, because a web page has exactly one display.
#[derive(Debug)]
pub struct WebDisplay;

impl wgpu::rwh::HasDisplayHandle for WebDisplay {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        Ok(wgpu::rwh::DisplayHandle::web())
    }
}

/// An instance descriptor over [`WebDisplay`] limited to `backends`.
#[must_use]
pub fn instance_descriptor(backends: wgpu::Backends) -> wgpu::InstanceDescriptor {
    let mut desc = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(WebDisplay));
    desc.backends = backends;
    desc
}
