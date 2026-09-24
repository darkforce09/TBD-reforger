# T-927 — Plan

## Context
Wave-210 eye-pass: double-clicking docks, the Attributes overlay or the top strip fires the map
canvas dblclick paths (place picker / Attributes-on-entity). pointerdown is stopped on chrome; native
dblclick still bubbles to the canvas handler in `mission_editor.rs`.

## Approach
1. Gate the canvas dblclick handler on the event target being the canvas element (not a chrome ancestor).
2. Stop dblclick propagation at the three chrome roots (dock_right, top_strip, attributes_modal).
3. Smoke: dblclick on each chrome surface produces no picker; canvas dblclick still places / opens.

## Risks
Shared files with T-939.x/T-142/T-158 — later wave. Do not break entity dblclick-to-Attributes.

## Verification
`cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`; `cargo xtask platform wave gate --slice T-927`.
