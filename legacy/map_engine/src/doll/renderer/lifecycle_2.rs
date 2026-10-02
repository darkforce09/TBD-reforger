//! Role: lifecycle 2.
//! Position: `doll/renderer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::doll::renderer::lifecycle_1::DollEngine;
use crate::doll::renderer::pass::doll_pass;
use crate::doll::renderer::pass::draw_doll;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl DollEngine {
    /// Damage-driven frame: no-ops (no surface acquire) when nothing changed.
    pub fn render(&mut self) -> Result<(), JsError> {
        if !self.dirty && !self.continuous {
            return Ok(());
        }
        let mvp =
            crate::camera::orbit::projection::view_proj_wgpu(self.yaw, self.css_w, self.css_h);
        let mut uniform = [0f32; 20];
        uniform[..16].copy_from_slice(&mvp);

        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&uniform));

        use wgpu::CurrentSurfaceTexture as Cst;
        let frame = match self.surface.get_current_texture() {
            Cst::Success(f) | Cst::Suboptimal(f) => f,
            Cst::Timeout | Cst::Occluded => {
                return Ok(());
            }
            Cst::Outdated | Cst::Lost => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    Cst::Success(f) | Cst::Suboptimal(f) => f,
                    other => {
                        return Err(JsError::new(&format!(
                            "surface-acquire-after-reconfigure: {other:?}"
                        )));
                    }
                }
            }
            other => return Err(JsError::new(&format!("surface-acquire: {other:?}"))),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("doll"),
            });
        {
            let mut pass = doll_pass(&mut encoder, &view, &self.depth);
            draw_doll(
                &mut pass,
                &self.pipeline,
                &self.bind_group,
                &self.cube_vbuf,
                &self.cube_ibuf,
                self.cube_index_count,
                &self.cyl_vbuf,
                &self.cyl_ibuf,
                self.cyl_index_count,
                &self.inst_buf,
                self.n_cube,
                self.n_cyl,
            );
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.dirty = false;
        Ok(())
    }
}
