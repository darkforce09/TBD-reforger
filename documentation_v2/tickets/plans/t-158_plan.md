# T-158 — Plan

## Context
The shell carries two Settings entries, a disabled History button and a left-dock Assets tab that duplicates the
right dock (T-818). Packs after T-142 (toolbelt/attributes polish) and T-939.x on the same files.

## Approach
1. wasm inventory test listing top-strip buttons and left-dock tabs — pins today's set (red once we change it).
2. `panels/top_strip.rs`: wire History to the versions dialog or remove it; keep one Settings gear.
3. `panels/dock_left.rs`: remove the Assets tab and the dock Settings duplicate; confirm search is in the right dock.
4. Update the inventory test to the final set; perturbation: re-add Assets → red; restore, `touch`, green.

## Risks
- Hotkeys bound to removed buttons; grep commands_hotkeys.rs and keep the bindings on the surviving control.

## Verification
- `cargo xtask mk ci-local-leptos` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-158`
