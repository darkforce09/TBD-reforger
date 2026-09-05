# T-300 — Plan

## Context
The shared CARGO_TARGET_DIR (wave/mod.rs:241) let a worktree-built website-api binary satisfy a main-checkout
`make api` in wave 1; any run lane can repeat that during an 8-wide wave. Per-worktree targets cost 44 GB each.

## Approach
1. `xtask/src/wave/mod.rs`: add `Ctx::run_target_dir()` = `<CARGO_TARGET_DIR>/run-main`; run lanes export it and
   write `tbd-built-from` (sha + checkout path) beside the binary after a successful build.
2. `xtask/src/platform_preflight.rs` step 4: read the stamp, compare with `git rev-parse HEAD`, red with the fix
   (`cargo clean` in that target) on mismatch.
3. Perturbation: write a wrong sha into the stamp; preflight goes red; restore, `touch`, green.
## Risks
- A run lane that bypasses Ctx keeps the old behaviour; grep every `CARGO_TARGET_DIR` export in xtask/src/wave.
- Fallback: preflight-only stamp check without the second target still catches the incident class.

## Verification
- `cargo test -p xtask wave:: platform_preflight`
- `cargo xtask platform preflight`
- `cargo xtask platform wave gate --slice T-300`
