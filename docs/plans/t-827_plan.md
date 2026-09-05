# T-827 — Plan

## Context

Wave-204 MINOR: the validation chip (`top_strip.rs:63`, T-798 geometry) passes 5.31:1 by plate-calc but measures 4.01–4.34:1 live-effective over the backdrop-blur strip glass on four map backgrounds — under the F-36 4.5:1 target.

## Approach

1. Verify on main with the four-sample screenshot method (red numbers pasted).
2. `top_strip.rs`: raise plate alpha or add a solid pill behind the count; keep the error text colour.
3. Re-measure live on the darkest and lightest backgrounds; plate-calc stays ≥ 4.5:1.

## Risks

- A solid pill changes chip geometry; keep the T-798 dimensions.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-827`
