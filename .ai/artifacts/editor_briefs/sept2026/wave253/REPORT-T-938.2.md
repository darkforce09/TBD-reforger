# REPORT T-938.2 — Measure and ring-buffer chunk-crossing uploads

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-938.2
slice/T-938.2
```

`export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. Native cargo on HOST Fedora. Did not create `/home/Samuel/.cache/tbd-target-T-938.2` — wasm check used the shared cache; the wave gate picks its own.

## measured_allocations_per_crossing

The audit's ≥10 at `world_host.rs:454-525` is the **landcover compose** (`push_landcover`). After T-173 P2 that mesh is stored on the host and a warm crossing early-outs before `Vec` collect / `compose_landcover_mesh`. Roads are the same (`last_road_sig`).

Instrumented `run_viewport` (debug log behind `?t9382=1` or `window.__t9382Log`; samples always on `window.__t9382`):

| sample | staging (host compose Vecs) | fetch_batch | packed_clones |
|---|---|---|---|
| warm crossing 1 | **0** | 1 | 6 |
| warm crossing 2 | **0** | 1 | 6 |
| warm crossing 3 | **0** | 1 | 6 |

**Decision number = staging = 0 (< 3).** Ticket: ship the counter only.

Evidence:

- `push_landcover` returns before compose when `landcover_mesh` is already `Some` (live ~562).
- `push_roads` returns before compose when `last_road_sig` matches.
- `fetch_and_queue` allocates one pending-chunk `Vec` per fetch (`fetch_batch=1`), not a GPU staging buffer.
- `push_to_engine` takes six packed clones from `WorldResidency::world_*()` (`fill_buf` / `outline_buf` / `strip_buf` / three glyph bufs) — `crates/map-engine-core/src/world/residency.rs:1500-1549` `.clone()`. Not owned by this slice. A host ring cannot stop those clones without slice accessors.

`:3000` is a foreign trunk `trunk` process, not this worktree's wasm, so the three rows are the instrumented warm path after cache warmup (same settle, three times), not a pan on that live tab.

## ring_or_counter_only

**counter_only** — staging per warm crossing is 0.

## perturbation

N/A — ring not implemented.

## gate_verdict_tail

Pending `cargo xtask platform wave gate --slice T-938.2` after this commit.

## files_outside_owns

[] (this report is required by the slice brief; not application code)

## found_not_fixed

- Six packed-buffer clones per real `push_to_engine` (`WorldResidency` by-value getters). Needs slice/`copy_into` APIs in `map-engine-core` (not in T-938.2 owns).
- `engine.rs` `upload_world_buildings` / outlines / fence strips still `Vec::with_capacity` + `create_buffer_init` per upload (T-938.1 found_not_fixed; T-938.3 owns `engine.rs`).

## deviations

- Ticket verify named `cargo xtask mk ci-local-leptos`; brief forbids ci-local / leptos-gates. Ran wasm32 `cargo check -p website-frontend`, `t628` fetch-order pin, `cargo xtask schema validate` is the gate's schema step, and the slice gate.
- Did not rebuild the process on `:3000`/`:8080` (leave them up).

## commits

Pending after gate fill.

## manual_checklist

1. Open the map editor with `?t9382=1` (or `window.__t9382Log = true`).
2. Wait until world chunks are resident, then pan across three chunk boundaries at constant zoom.
3. Console: `t9382 crossing #N staging=0 fetch_batch=1 packed_clones=6`. `window.__t9382.max_staging === 0`.
