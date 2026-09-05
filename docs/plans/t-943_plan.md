# T-943 — Plan

## Context

`cargo xtask platform wave push` hung for ten minutes on 2026-09-04 while pushing a 28-commit range
that includes T-090.12.2's 1,691 BLAS sidecars. The LFS guard in `xtask/src/wave/push.rs` writes the
whole per-commit path list into `git check-attr`'s stdin and only then reads stdout, so once the
child's output exceeds the 64 KB pipe buffer both sides block. The guard was written for the
container that lacks git-lfs; this host has git-lfs 3.7.1, so the guard's refusal is also wrong here.

## Approach

1. In `lfs_paths_in_range`, move the stdin `write_all` onto a scoped thread that drops the handle
   when done; keep `wait_with_output` on the caller. Add a test that pipes a synthetic >64 KB list
   through a stub `check-attr` (a tiny Rust test helper, no shell) and asserts completion.
2. In `cmd_push`, probe `git lfs version`. Present → `git push origin main` with hooks, no guard,
   and print "git-lfs present: normal push". Absent → existing guard + `--no-verify`, unchanged text.
3. Perturbation: restore the inline `write_all`, run the new test, capture the timeout red, restore,
   `touch` the file, re-run green.

## Risks

- The thread must not outlive a failed spawn; use `std::thread::scope`.
- A normal push runs the pre-push hook, which needs git-lfs on PATH — that is exactly the branch
  where we detected it, so no regression in the container.

## Verification

- `cargo test -p xtask wave::push`
- `cargo xtask platform wave push` on this host over a range with LFS content
- `cargo xtask platform wave gate --slice T-943`
