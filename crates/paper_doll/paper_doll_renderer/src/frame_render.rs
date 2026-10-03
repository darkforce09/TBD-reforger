//! The paper doll's damage-driven frame.
//!
//! **Role:** [`PaperDollRenderer::render`] writes the camera uniform, acquires the swapchain image
//! through the GPU context, draws the doll into it and presents it, only when something changed.
//! **Position:** called by the Arsenal host once per animation frame; the renderer's state
//! changes (`renderer.rs`) set the dirty flag it reads.
//! **Signals & state:** clears the renderer's dirty flag after a presented frame.
//! **Invariants:** an unchanged renderer acquires no surface image; a skipped acquire (timeout or
//! occluded surface) leaves the frame dirty so the next tick draws it; the uniform is the orbit
//! camera's wgpu matrix for the yaw and CSS size plus a zero `params` vector (the lit path).

use crate::doll_draw::{InstanceDraw, doll_pass, draw_doll};
use crate::error::Result;
use crate::renderer::PaperDollRenderer;
use camera_math::orbit::projection::view_proj_wgpu;
use gpu_device::Acquired;

impl PaperDollRenderer {
    /// Draw a frame when something changed (or continuously when asked); otherwise return at once
    /// without acquiring the surface.
    ///
    /// # Errors
    /// [`crate::Error::Gpu`] when the surface hands out no image, even after one reconfigure.
    pub fn render(&mut self) -> Result<()> {
        if !self.dirty && !self.continuous {
            return Ok(());
        }
        let mvp = view_proj_wgpu(self.yaw, self.css_width, self.css_height);
        let mut uniform = [0f32; 20];
        uniform[..16].copy_from_slice(&mvp);
        self.gpu
            .queue()
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniform));

        let Acquired::Ready { frame, view } = self.gpu.acquire()? else {
            return Ok(());
        };
        let device = self.gpu.device();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("doll"),
        });
        {
            let mut pass = doll_pass(&mut encoder, &view, &self.depth);
            draw_doll(
                &mut pass,
                &self.pipeline,
                &self.bind_group,
                &self.meshes,
                &InstanceDraw {
                    buffer: &self.instance_buffer,
                    cube_instances: self.cube_instances,
                    cylinder_instances: self.cylinder_instances,
                },
            );
        }
        self.gpu.queue().submit(Some(encoder.finish()));
        frame.present();
        self.dirty = false;
        Ok(())
    }
}
