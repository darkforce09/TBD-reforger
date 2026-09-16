# Graphics engine

`website-graphics-engine` owns map and mannequin rendering, camera math, spatial queries,
terrain formats, and asset streaming. The frontend supplies the canvas and preference readers.
It owns UI signals and registers the render context through its lifecycle cleanup mechanism.

## Modules

- `src/core`: device lifetime, buffers, culling, damage, and the 48 ordered draw lanes.
- `src/camera`: orthographic and orbit projection, viewport controls, and coordinate conversion.
- `src/renderers`: GPU pipelines, batching, bitmap text, and primitive composition.
- `src/spatial`: point indexes, BVHs, terrain LOS, viewsheds, and world occluders.
- `src/terrain`: DEMs, relief, satellite streaming, roads, and water geometry.
- `src/architecture`: building blueprints, compound transforms, sections, and structure LOS.
- `src/streaming`: chunk residency, fetch and upload scheduling, memory budgets, and host callbacks.
- `src/formats`: container headers, POD records, validated archives, and density codecs.
- `src/environment`: buildings, vegetation, settlement labels, and world classification.
- `src/symbology`: bespoke unit roles and vehicle glyphs, side tints, labels, and squad links.
- `src/doll`: mannequin scene, rendering, and equipment-region picking.
- `src/shaders`: map and mannequin WGSL.
- `src/diagnostics`: readback checks, frame timing, benchmarks, and browser diagnostics.

## Boundaries and features

The crate has no dependency on Leptos or on any editor application state. (It had none on
`website-mission-core` either, until T-0xx Phase 2A folded that crate in — the mission document
types now live here, under `src/data`.) Its WebAssembly
platform boundary uses canvas, browser fetch, image decoding, timers, and console APIs. Native
builds expose the geometry, codecs, and state machines without browser execution.

The default feature is `scenario` alone — the headless mission compiler and validator, three
optional serde crates and nothing else. It is the tier `website-api` links, and it is the only
tier that pulls no graphics crate, no PNG, no rkyv and no flate2.

Above it the chain is `world` → `io` → `streaming` → `render`: `world` enables `bvh` and PNG
decoding and links `website-graphics-engine`; `io` adds the rkyv archive formats; `streaming` adds
the loader/scheduler stack (flate2); `render` adds the GPU frame path. `store` (`scenario` + yrs)
is the CRDT document layer and `editing` is `store` + `world`. `streaming` is deliberately its own
axis rather than part of `world`: real consumers take the world/io pair without the scheduler.

Every module in `lib.rs` is gated on the feature it belongs to, so a consumer that asks for one
tier does not compile the shells of the others.

Wire layouts, numeric precision, lane ordering, fetch concurrency, upload budgets, and cleanup
ownership are preserved. Production files stay below 500 lines and test files below 1,000.

## Verification

With the workspace target directory configured, run `cargo test -p website-graphics-engine
--all-features` and `cargo check -p website-graphics-engine --target wasm32-unknown-unknown`.
The test tripwire rejects feature sets that would silently omit parts of the suite. Camera
integration tests pin deck.gl parity; native tests cover format round trips, geometry, residency,
and memory accounting. Browser readback checks live under `diagnostics`.

Capability gaps are recorded in the owning module READMEs. Symbols are bespoke and text uses
the existing bitmap atlas.
