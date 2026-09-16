//! Role: Module boundary for renderers/pipelines.
//! Position: `pipeline` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Quad.
#[cfg(target_arch = "wasm32")]
pub mod quad;

/// Textured.
#[cfg(target_arch = "wasm32")]
pub mod textured;

/// Vector.
#[cfg(target_arch = "wasm32")]
pub mod vector;

/// Text.
#[cfg(target_arch = "wasm32")]
pub mod text;

/// Icon.
#[cfg(target_arch = "wasm32")]
pub mod icon;

/// Building.
#[cfg(target_arch = "wasm32")]
pub mod building;

/// The map shader module every constructor above compiles against.
///
/// T-0xx Phase 2B (Kind A): `RenderEngine::create` used to run this `create_shader_module`
/// itself, reaching straight into `website_graphics_engine::shaders::SHADER_WGSL`. Compiling
/// a shader module is GPU resource creation and gate rule 3b says that is this crate's job;
/// the source now never leaves the crate that owns it. Every `create_*_pipeline` below wants
/// the `&ShaderModule` this returns, so it belongs beside them and not in `shaders/`, which
/// holds text.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn create_map_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("quad-instanced"),
        source: wgpu::ShaderSource::Wgsl(crate::shaders::SHADER_WGSL.into()),
    })
}
