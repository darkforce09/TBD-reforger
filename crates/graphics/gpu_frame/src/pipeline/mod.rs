//! Role: the render pipeline constructors and the shader module they compile against.
//! Position: `pipeline` in the GPU frame crate; the map engine's render engine and its readback
//! probes build pipelines with these on the device `gpu_device` hands out.
//! Signals & state: none; every function builds a GPU object and returns it.
//! Invariants: every pipeline compiles against `render_primitives::shaders::SHADER_WGSL`, through
//! `create_render_shader` (`wasm32` only); the WGSL source never leaves `render_primitives`.

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

/// Oriented quads carrying their own basis.
#[cfg(target_arch = "wasm32")]
pub mod oriented_quad;

/// The shader module every constructor above compiles against.
///
/// Compiling a shader module is GPU resource creation, so it sits beside the pipeline
/// constructors that consume the `&ShaderModule` it returns; the WGSL text itself is
/// `render_primitives::shaders::SHADER_WGSL`.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn create_render_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("quad-instanced"),
        source: wgpu::ShaderSource::Wgsl(render_primitives::shaders::SHADER_WGSL.into()),
    })
}
