# T-163 — Merge integration: t-159-leptos-ui + t-161-ticket-xtask → main

**Status:** shipped · **Executor:** claude-code (solo session, operator-approved plan) ·
**Verify:** [`.ai/artifacts/t163_merge_verify_log.md`](../../.ai/artifacts/t163_merge_verify_log.md)

## What

Land both finished worktree branches on `main` with mathematically pinned conflict resolution,
then reconcile the seams the two programs opened against each other:

1. **Merge 1** — `t-159-leptos-ui` (Leptos SPA rewrite + React deletion, T-159.15–.29.3) @
   `df181120`. Conflict-free by construction (merge-base = pre-merge main tip; `merge-tree`
   dry-run exit 0).
2. **Merge 2** — `t-161-ticket-xtask` (ticket CLI → Rust xtask T-161 + Python eradication T-162)
   @ `3cec57ee`. Exactly the 6 dry-run-predicted conflicts resolved: `registry.json` (main base +
   t-161 delta), `Makefile` (t-159 body + t-161's verify-no-python 3-hunk delta), `scripts/ticket`
   (xtask wrapper wins), `scripts/lib/ticket_registry.py` (modify/delete — deletion wins),
   `Cargo.lock` (minimal re-resolve), `docs/TICKET_REGISTRY.md` (regenerated in-merge via
   `ticket sync`). Also restored the T-152.5 (`074086d8`) `compose_roads` caller in
   `map-engine-wasm` — broken on **all three heads**; the merge's `cargo check --workspace` gate
   was the first native compile of that crate since.
3. **This commit (T-163)** — integration: the t-159 edits that died with the Python (deleted by
   T-162) re-landed in their xtask ports — dead-React-reference purge across `xtask/src/cmds.rs`
   (8 lines) + `check.rs` scan roots (2 lines), closure-gated by
   `git grep -cE 'apps/website/frontend|ci-local-frontend' -- xtask/` = 0; `verify-monorepo`
   V13 `web`→`leptos`; ticket template stale-command fixes; registry truth (T-159 → shipped,
   T-154 gets its real `order` so the CLAUDE.md "Latest shipped" headline computes, T-163 row);
   `.ai/artifacts/worktrees/README.md` rewritten post-cleanup.

## Residual (unchanged by this work)

- **Prod default flip** — operator-gated (T-159 §HELD): `SPA_DIST_DIR`, OAuth origin, staging soak.
- Pre-existing reds ledger: 1 dangling `@contract` citation (`TBD_LoadoutEquipComponent.c`, T-068
  lane, mod-executor-gated); `ticket check` field debt on T-147/148/149 (accepted A4′ set);
  `scripts/map-assets/verify-t152-cartographic.mjs` React-era wasm path (manual T-152 tool).
