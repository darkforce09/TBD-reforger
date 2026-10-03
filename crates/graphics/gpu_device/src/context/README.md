# GPU context

The GPU of one browser canvas: the bootstrap the map renderer and the paper doll renderer share,
its resize and swapchain acquire, the web display handle every instance is built with, and the
GPU-free decisions the bootstrap applies.

## Contents

```text
crates/graphics/gpu_device/src/context/
├── frame_acquire.rs   `Acquired` and `acquire_frame`: the next image, a skip, or a typed error (WebAssembly)
├── gpu_context.rs     `GpuContext`: create, resize, acquire and the accessors (WebAssembly)
├── mod.rs             the module tree
├── surface_policy.rs  `BackendKind`, the size checks, the linear format pick, the timestamp decision
├── tests/             native tests of the surface policy and the error codes
└── web_display.rs     `WebDisplay` and `instance_descriptor` (WebAssembly)
```

## How it works

```text
canvas (device-pixel width × height)
   │ checked_canvas_size
   ▼
instance (WebGPU detection, or WebGL2 alone when forced) ─▶ surface ─▶ adapter (high performance)
   │ BackendKind, adapter limits, request_timestamps(want, adapter supports)
   ▼
device + queue (label; WebGL2: downlevel limits raised to the adapter's resolution limits)
   │ first_linear_format, default configuration, Fifo
   ▼
GpuContext ── resize(w, h): checked_surface_size, reconfigure
          └── acquire(): acquire_frame ─▶ Ready { frame, view } | Skip | Error
```

`GpuContext::create` fails with the `Error` variant of the step that failed; each message starts
with a stable code (`canvas-zero-size`, `create-surface`, `no-adapter`, `no-device`,
`srgb-only-surface`, `surface-unsupported-by-adapter`). `resize` takes device pixels the caller
has already rounded and clamped (the map renderer rounds and rejects non-positive CSS sizes, the
paper doll renderer clamps to 1) and refuses a zero side. `acquire_frame` reconfigures the
surface once on `Outdated` or `Lost`, skips on `Timeout` or `Occluded`, and fails with
`surface-acquire` or `surface-acquire-after-reconfigure`.

## Boundaries

- Depends on: `crate::error`; `wgpu` and `web-sys` in the WebAssembly build.
- Used by: the map renderer's `RenderEngine` (`crates/map_rendering/map_renderer/src/boot.rs`,
  `lifecycle.rs`) and the paper doll renderer (`crates/paper_doll/paper_doll_renderer/src/renderer.rs`,
  `frame_render.rs`), which create, resize and acquire through `GpuContext` and build their own
  shaders, layouts and pipelines on top; `gpu_frame`'s `frame::present::submit` presents the
  images acquired here.
- Rules: the surface format is never sRGB; a resize never clamps; the policy names no GPU type,
  so `tests/surface_policy_tests.rs` runs natively.
