# T-939.7 — Plan

## Context
outliner.rs:39 VIRTUAL_SLOT_THRESHOLD=50 gates only virtual_tree; vehicles_panel.rs:231-276 `.map().collect_view()` over
every row; the outliner re-flattens per render.

## Approach
1. Verify on main: test counting flatten calls across two selection changes shows two; paste the red.
2. outliner.rs: memo keyed on the document version.
3. vehicles_panel.rs: virtual list above the threshold.
4. Perturbation: drop the memo key → call-count test red; restore, touch, green.

## Risks
- Memo key must change on every doc edit, including hidden-layer toggles.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.7`
