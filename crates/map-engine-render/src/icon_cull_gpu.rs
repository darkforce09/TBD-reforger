//! T-151.8.1 — WebGPU icon instance cull GPU resources (wasm32 only).
//!
//! Compacts `IconStorage` (32 B) into a VERTEX|STORAGE output + atomic counter used for
//! `draw_indirect`. WebGL2 never constructs this — chunk draw-set cull remains the gate.
//!
//! Class R gate: GPU counter readback == [`crate::compute_cull::count_icons_in_frustum`].
//!
//! T-938.3: one compute pipeline, per-lane src/dst/counter/indirect; CPU
//! [`crate::compute_cull::count_icons_in_frustum`] runs only when the debug HUD flag is
//! on. Otherwise the visible count comes from the GPU readback (may lag one frame).

use crate::compute_cull::{ICON_STRIDE, cpu_count_for_encode, pack_icon_storage32};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// Indirect draw args: vertex_count=4, instance_count=atomic, first_vertex=0, first_instance=0.
pub const INDIRECT_STRIDE: u64 = 16;

/// Self-check / `upload_icons` lane (not a `LaneRole` discriminant).
const DEFAULT_LANE: u32 = 0;

struct LaneGpu {
    src_buf: Option<wgpu::Buffer>,
    src_capacity: u32,
    src_count: u32,
    dst_buf: Option<wgpu::Buffer>,
    dst_capacity: u32,
    counter_buf: wgpu::Buffer,
    indirect_buf: wgpu::Buffer,
    readback_buf: wgpu::Buffer,
    last_cpu_count: u32,
    last_gpu_count: Rc<Cell<u32>>,
    gpu_sampled: Rc<Cell<bool>>,
    readback_in_flight: Rc<Cell<bool>>,
    last_icons_20: Vec<u8>,
}

impl LaneGpu {
    fn create(device: &wgpu::Device) -> Self {
        let counter_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cull-counter"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let indirect_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cull-indirect"),
            size: INDIRECT_STRIDE,
            usage: wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cull-readback"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            src_buf: None,
            src_capacity: 0,
            src_count: 0,
            dst_buf: None,
            dst_capacity: 0,
            counter_buf,
            indirect_buf,
            readback_buf,
            last_cpu_count: 0,
            last_gpu_count: Rc::new(Cell::new(0)),
            gpu_sampled: Rc::new(Cell::new(false)),
            readback_in_flight: Rc::new(Cell::new(false)),
            last_icons_20: Vec::new(),
        }
    }
}

pub struct IconComputeCull {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_layout: wgpu::BindGroupLayout,
    pub params_buf: wgpu::Buffer,
    lanes: HashMap<u32, LaneGpu>,
    debug_hud: bool,
}

impl IconComputeCull {
    pub fn create(device: &wgpu::Device, shader: &wgpu::ShaderModule) -> Self {
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("icon-cull"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(32),
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("icon-cull"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("icon-cull"),
            layout: Some(&layout),
            module: shader,
            entry_point: Some("cs_icon_cull"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cull-params"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            bind_layout,
            params_buf,
            lanes: HashMap::new(),
            debug_hud: false,
        }
    }

    pub fn set_debug_hud(&mut self, on: bool) {
        self.debug_hud = on;
    }

    #[must_use]
    pub fn has_any_src(&self) -> bool {
        self.lanes.values().any(|l| l.src_count > 0)
    }

    /// Sum of per-lane CPU oracle counts (0 when the debug HUD flag is off).
    #[must_use]
    pub fn last_cpu_count(&self) -> u32 {
        self.lanes.values().map(|l| l.last_cpu_count).sum()
    }

    /// The GPU counter value for stats: the real sampled count once available, else the CPU
    /// oracle mirror (flagged by [`Self::gpu_sampled`]). Summed across lanes.
    #[must_use]
    pub fn gpu_count_for_stats(&self) -> u32 {
        self.lanes.values().map(lane_gpu_count_for_stats).sum()
    }

    #[must_use]
    pub fn lane_gpu_count_for_stats(&self, lane: u32) -> u32 {
        self.lanes.get(&lane).map_or(0, lane_gpu_count_for_stats)
    }

    #[must_use]
    pub fn gpu_sampled(&self) -> bool {
        self.lanes.values().any(|l| l.gpu_sampled.get())
    }

    /// Compacted instance buffer + indirect args for `draw_indirect`, if this lane has work.
    #[must_use]
    pub fn lane_draw(&self, lane: u32) -> Option<(&wgpu::Buffer, &wgpu::Buffer)> {
        let l = self.lanes.get(&lane)?;
        if l.src_count == 0 {
            return None;
        }
        Some((l.dst_buf.as_ref()?, &l.indirect_buf))
    }

    /// Counter staging buffer for the self-check readback (default lane).
    #[must_use]
    pub fn readback_buf(&self, lane: u32) -> Option<&wgpu::Buffer> {
        self.lanes.get(&lane).map(|l| &l.readback_buf)
    }

    /// T-151.11.4 (X-03): async-map the 4-byte counter readback after a submit that encoded a
    /// cull. In-flight guarded like `GpuTimer` so mappings never overlap; the callback stores
    /// the real GPU count + sets `gpu_sampled`.
    pub fn kick_readback(&self) {
        for lane in self.lanes.values() {
            kick_lane_readback(lane);
        }
    }

    pub fn upload_icons(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, icons_20: &[u8]) {
        self.upload_lane(device, queue, DEFAULT_LANE, icons_20);
    }

    pub fn upload_lane(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lane: u32,
        icons_20: &[u8],
    ) {
        let slot = self
            .lanes
            .entry(lane)
            .or_insert_with(|| LaneGpu::create(device));
        slot.last_icons_20 = icons_20.to_vec();
        let n = (icons_20.len() / ICON_STRIDE) as u32;
        slot.src_count = n;
        if n == 0 {
            slot.last_cpu_count = 0;
            slot.last_gpu_count.set(0);
            slot.gpu_sampled.set(true);
            return;
        }
        let packed = pack_icon_storage32(icons_20);
        if slot.src_capacity < n {
            slot.src_buf = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cull-src"),
                size: u64::from(n) * 32,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            slot.src_capacity = n;
        }
        if slot.dst_capacity < n {
            slot.dst_buf = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cull-dst"),
                size: u64::from(n) * 32,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            slot.dst_capacity = n;
        }
        if let Some(src) = &slot.src_buf {
            queue.write_buffer(src, 0, &packed);
        }
    }

    /// Encode clear + compute + copy counter → indirect instance_count for every uploaded lane.
    pub fn encode_cull(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frustum: [f64; 4],
    ) {
        let debug_hud = self.debug_hud;
        let lanes: Vec<u32> = self.lanes.keys().copied().collect();
        for lane in lanes {
            self.encode_lane(encoder, device, queue, frustum, lane, debug_hud);
        }
    }

    fn encode_lane(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frustum: [f64; 4],
        lane: u32,
        debug_hud: bool,
    ) {
        let Some(slot) = self.lanes.get_mut(&lane) else {
            return;
        };
        slot.last_cpu_count =
            cpu_count_for_encode(&slot.last_icons_20, frustum, debug_hud).unwrap_or(0);

        if slot.src_count == 0 {
            let args: [u32; 4] = [4, 0, 0, 0];
            queue.write_buffer(&slot.indirect_buf, 0, bytemuck::cast_slice(&args));
            slot.last_gpu_count.set(0);
            slot.gpu_sampled.set(true);
            return;
        }
        let Some(src) = slot.src_buf.as_ref() else {
            return;
        };
        let Some(dst) = slot.dst_buf.as_ref() else {
            return;
        };

        queue.write_buffer(&slot.counter_buf, 0, &0u32.to_le_bytes());

        let mut params = [0u8; 32];
        let f = [
            frustum[0].min(frustum[2]) as f32,
            frustum[1].min(frustum[3]) as f32,
            frustum[0].max(frustum[2]) as f32,
            frustum[1].max(frustum[3]) as f32,
        ];
        for (i, v) in f.iter().enumerate() {
            params[i * 4..(i + 1) * 4].copy_from_slice(&v.to_le_bytes());
        }
        params[16..20].copy_from_slice(&slot.src_count.to_le_bytes());
        queue.write_buffer(&self.params_buf, 0, &params);

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("icon-cull"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: src.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: slot.counter_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params_buf.as_entire_binding(),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("icon-cull"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(slot.src_count.div_ceil(64), 1, 1);
        }

        let seed: [u32; 4] = [4, 0, 0, 0];
        queue.write_buffer(&slot.indirect_buf, 0, bytemuck::cast_slice(&seed));
        encoder.copy_buffer_to_buffer(&slot.counter_buf, 0, &slot.indirect_buf, 4, 4);
        // Counter staging for the real readback — the render loop calls `kick_readback` after
        // this encoder's submit (T-151.11.4 / X-03). Readback may lag one frame (T-938.3).
        if !slot.readback_in_flight.get() {
            encoder.copy_buffer_to_buffer(&slot.counter_buf, 0, &slot.readback_buf, 0, 4);
        }
    }
}

fn lane_gpu_count_for_stats(lane: &LaneGpu) -> u32 {
    if lane.gpu_sampled.get() {
        lane.last_gpu_count.get()
    } else {
        lane.last_cpu_count
    }
}

fn kick_lane_readback(lane: &LaneGpu) {
    if lane.readback_in_flight.get() || lane.src_count == 0 {
        return;
    }
    lane.readback_in_flight.set(true);
    let buf = lane.readback_buf.clone();
    let count = lane.last_gpu_count.clone();
    let sampled = lane.gpu_sampled.clone();
    let flag = lane.readback_in_flight.clone();
    lane.readback_buf
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |res| {
            if res.is_ok() {
                {
                    let data = buf.slice(..).get_mapped_range();
                    let n = u32::from_le_bytes(data[0..4].try_into().expect("4 bytes"));
                    count.set(n);
                    sampled.set(true);
                }
                buf.unmap();
            }
            flag.set(false);
        });
}
