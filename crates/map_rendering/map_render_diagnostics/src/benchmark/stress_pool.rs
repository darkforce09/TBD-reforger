//! **Role:** the stress pool: `seed_stress` and `clear_stress` fill and empty the `Stress` lane.
//! **Position:** `benchmark` of the map render diagnostics; the Mission Creator's viewport bridge
//! publishes both as properties of `window.__editorBench`; they reach the engine only through
//! `map_renderer::diagnostic_accessors::DiagnosticStressPool`.
//! **Signals & state:** writes the batch list, the texture records, the slot lane pool, the
//! staging buffer and the stress counters.
//! **Invariants:** the calibration batch stays the batch list's last entry; after `seed_stress(n, _)`
//! the stress pool holds exactly n quads; `clear_stress` destroys every buffer it drops except the
//! pooled sprite lanes', which the slot symbology owns.

use gpu_frame::frame::{DrawBatch, DrawPayload, InstanceBuffer};
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use map_renderer::diagnostic_accessors::DiagnosticStressPool;
use map_renderer::engine::RenderEngine;
use render_primitives::draw::instances::CHUNK_CAPACITY;
use renderer_core::packet_bindings::PIPE_QUAD;
use symbology_layers_gpu::slot_symbology::is_pooled_icon_lane;
use time_source::{BrowserClock, Clock};

/// Streams `n` deterministic stress quads into the chunked pool. Integer accounting:
/// `stats().instances` afterwards equals exactly `n`.
pub fn seed_stress(engine: &mut RenderEngine, n: u32, seed: u32) {
    clear_stress(engine);

    let DiagnosticStressPool {
        device,
        queue,
        batches,
        staging,
        stress_instances,
        staging_peak_bytes,
        generation_ms,
        upload_ms,
        ..
    } = engine.diagnostic_stress_pool();
    let calibration = batches.pop().expect("calibration batch always present");
    let seed = u64::from(seed);
    let mut remaining = n as usize;
    let mut chunk_idx: u32 = 0;
    let mut gen_ms = 0.0;
    let mut upload_total_ms = 0.0;
    while remaining > 0 {
        let count = remaining.min(CHUNK_CAPACITY);
        let g0 = BrowserClock.now_unix_ms_f64();
        crate::benchmark::stress_scene::stress_chunk_into(chunk_idx, count, seed, staging);
        gen_ms += BrowserClock.now_unix_ms_f64() - g0;

        let u0 = BrowserClock.now_unix_ms_f64();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stress-chunk"),
            size: (count * 32) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(staging));
        upload_total_ms += BrowserClock.now_unix_ms_f64() - u0;

        #[allow(clippy::cast_possible_truncation)]
        batches.push(DrawBatch {
            lane: lane_id(LaneRole::Stress),
            visible: true,
            pipeline: PIPE_QUAD,
            payload: DrawPayload::Quads(InstanceBuffer::whole(buffer, 32, count as u32)),
        });
        *stress_instances += count as u64;
        *staging_peak_bytes = (*staging_peak_bytes).max(staging.capacity() as u64 * 32);
        remaining -= count;
        chunk_idx += 1;
    }
    batches.push(calibration);
    *generation_ms = gen_ms;
    *upload_ms = upload_total_ms;
}

/// Drops the stress pool with every other batch but the calibration batch, destroying their
/// buffers and textures eagerly.
pub fn clear_stress(engine: &mut RenderEngine) {
    let DiagnosticStressPool {
        batches,
        textured_lanes,
        slot_symbology,
        stress_instances,
        generation_ms,
        upload_ms,
        ..
    } = engine.diagnostic_stress_pool();
    let calibration = batches.pop().expect("calibration batch always present");
    for batch in batches.drain(..) {
        let lane = batch.lane;
        match batch.payload {
            DrawPayload::Quads(i) | DrawPayload::OrientedQuads(i) => i.buffer.destroy(),
            DrawPayload::Sprites { instances, .. } => {
                if !is_pooled_icon_lane(lane) {
                    instances.buffer.destroy();
                }
            }
            DrawPayload::SpritesWithText { sprites, text, .. } => {
                sprites.buffer.destroy();
                if let Some(run) = text {
                    run.glyphs.buffer.destroy();
                }
            }

            // A textured batch carries a bind-group id, not its texture: the handle is in the
            // texture records, drained below, so the same textures are destroyed.
            DrawPayload::TexturedRect { .. } => {}
            DrawPayload::Lines(v) => v.vertices.destroy(),
            DrawPayload::Text(run) => run.glyphs.buffer.destroy(),
            DrawPayload::Indexed(m) => {
                m.vertices.destroy();
                m.indices.destroy();
            }
        }
    }
    for (_, tex) in textured_lanes.drain(..) {
        tex.texture().destroy();
    }
    batches.push(calibration);
    slot_symbology.clear_lane_pool();
    *stress_instances = 0;
    *generation_ms = 0.0;
    *upload_ms = 0.0;
}
