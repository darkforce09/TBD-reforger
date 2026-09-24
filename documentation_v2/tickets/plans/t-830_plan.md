# T-830 — Plan

## Context

Wave-204 eye-pass: 16 px outliner rows with always-visible icon clusters (eye, camera, rename, delete, drop chip) squeeze truncated names at 240 px. Eden shows row tools on hover only. `outliner_tree.rs:158` pins the `h-4` recipe and windowing; T-825 (design session, untouched here) may absorb this if it lands first.

## Approach

1. `outliner_tree.rs`: raise row height/padding; render the tool cluster on hover-or-active; give the name the freed width.
2. Keep the T-803 drop-target, T-809 vehicle rows and the windowing threshold (50) intact — existing tests must pass.
3. Screenshot before/after at 240 px.

## Risks

- Hover-only tools hurt touch/keyboard reach; keep tools visible on the active row.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-830`
