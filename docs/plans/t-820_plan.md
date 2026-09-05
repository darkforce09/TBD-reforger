# T-820 — Plan

## Context

Wave-203 MINOR: with no current modpack (`/registry` probe 404), two headless pages showed the generic "request failed" with chips visible instead of the named cause at `dock_right.rs:749` ("No modpack is configured…"). Possibly a T-809 restructure regression or a headless AuthStore timing artifact (`let Some(auth) else return` in the re-probe Effect).

## Approach

1. Reproduce in a real browser with `is_current = false` first; if headless-only, close with evidence and no code.
2. If real: `dock_right.rs` — keep the 404 cause through the re-probe Effect (do not early-return before the cause lands), hide chips on failure.
3. Restore the modpack: Retry repopulates the tree.

## Risks

- Auth timing differs between headless and browser; the repro decides the path — say which in the report.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-820`
