# T-817 — Plan

## Context

Wave-201 NIT-2: grid labels recompute on cursor movement plus a ~1 Hz zoom heartbeat, so a wheel zoom with a stationary pointer waits up to ~1.4 s. `toolbelt.rs:40` documents `m_per_px = 2^(−deck_zoom)` — the camera signal already ticks per frame.

## Approach

1. Verify on main: scripted wheel zoom with a stationary pointer shows a stale-label window > 100 ms (red).
2. `toolbelt.rs`: subscribe the label memo to the camera `m_per_px` signal edge; keep the T-793 position-quantised `For` key.
3. Assert labels within 2 px of the CUR-unproject oracle within one settled frame; pan case unchanged.

## Risks

- Per-frame recompute must stay cheap (memo keyed on quantised m_per_px), or FPS drops.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-817`
