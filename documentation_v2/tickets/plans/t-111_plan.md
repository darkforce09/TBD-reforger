# T-111 — Plan

## Context
Program wrapper for T-067.1. T-067 shipped bulk paste and 512 m chunk scaffolding; the deferred §Deferred item of
t067_spatial_chunks.md is memory-bounded residency at 1M objects, re-anchored on the Rust WorldResidency.

## Approach
1. T-067.1 (after T-935.13): world/eviction.rs byte-budget policy, residency.rs call sites, world_host.rs cache
   eviction + reload.
2. Program closes when T-067.1 ships; no further slices planned.

## Risks
- If T-935.13 changes chunk byte accounting late, T-067.1 re-measures before wiring the budget.

## Verification
- T-067.1's verify block; `cargo xtask ticket check` for the program row.
