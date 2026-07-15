# T-159.15.1 — verify / root-cause correction

**Shipped:** `a425936d` · **tag** `T-159.15.1` · **branch** `t-159-leptos-ui` · **worktree**
`.ai/artifacts/worktrees/TBD-T-159` · **Date:** 2026-07-15

## What shipped

Damage-driven rAF render loop + wheel-zoom + resize on the Mission Creator editor, with
`RenderEngine` owned directly in the Leptos wasm (continues T-159.15.0 boundary collapse @
`3066f14c`).

## Root cause (corrected — do not repeat handoff hypothesis)

The pre-ship handoff (`.ai/artifacts/t159_15_render_loop_handoff.md` on the worktree) blamed
headless **WebGL2** missing `device.poll()` after `map_async`. That was **wrong for this
smoke environment**.

| Claim | Reality |
|-------|---------|
| Backend = WebGL2 / SwiftShader | Smoke default = **WebGPU / Dawn** (`backend=webgpu`) |
| `poll()` after `render()` fixes panic | `poll()` added; panic **persisted** (Dawn `poll` near no-op) |
| Damage-driven only | Panic also in continuous mode → not damage-specific |
| **Actual** | **GpuTimer** timestamp-readback lane double-maps its **16-byte** buffer on the **2nd** submit. Editor has no fps HUD → lane is pure overhead. |

**Fix used (handoff option 3):** `RenderEngine::disable_frame_timing()` drops the GpuTimer lane
(`render()`’s `take_timing` already treats `timer: None` as skip). **`poll()` kept** for
WebGL2-fallback + future cull-counter path.

## Gates (headless Chromium)

| Gate | Backend | Result |
|------|---------|--------|
| `smoke_editor.mjs` | webgpu | `pass:true` — no “already mapped”; canvas changes after wheel |
| `selfcheck_editor.mjs` | webgl2 via `?force=webgl` | calibration + texture `pass:true`, byte-exact |
| `clippy -p map-engine-render` (wasm, `-D warnings`) | — | clean |
| `cargo check` native + wasm, `trunk build --release` | — | clean |

**Note:** GPU readback self-check forces WebGL2 (`?force=webgl`, mirrors React `WgpuCanvas`) —
polled `map_read_4` only resolves on WebGL2 headless; Dawn hangs.

## Files (worktree)

- `crates/map-engine-render/src/engine.rs` — `poll()` + `disable_frame_timing()`
- `apps/website-leptos/Cargo.toml` — web-sys features + Location
- `apps/website-leptos/src/mission_editor.rs` — loop + wheel + resize + `__selfChecks` + `?force=webgl`
- `.ai/artifacts/t159_gates/driver/selfcheck_editor.mjs` — new

## Follow-up (not fixed here)

**Latent GpuTimer err-path:** unmaps only on `res.is_ok()`, clears `in_flight` unconditionally —
will bite when HUD/timer returns. Track as a dedicated ticket (see registry idea / T-159 note).

## Next

Camera interaction (pan / pick) → **T-159.16** doc host (`WasmMissionDoc`).
