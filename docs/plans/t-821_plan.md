# T-821 — Plan

## Context

Wave-203 MINOR, pre-existing: `save_semver` is initialised statically to 0.1.0 (`mission_editor.rs:1012`) and only the dialog input changes it; `save_now` (`commands_hotkeys.rs:955`) never bumps it, so a second save 409s (versions immutable). T-789's auto-bump claim was refuted live.

## Approach

1. Verify on main: save 0.1.0, reopen → prefill 0.1.0 (red).
2. On dialog open read the mission's persisted latest version and prefill patch+1; alternatively bump `save_semver` on the save-success arm — pick one, pin it in the ticket.
3. Manual input still overrides; test both paths.

## Risks

- The latest version may not be in the DTO; fallback is the success-arm bump.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-821`
