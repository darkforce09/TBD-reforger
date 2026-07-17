# T-163 — merge t-159 + t-161 → main — verify log

**Operator-approved plan executed 2026-07-17.** Executor: claude-code (solo session).
**Result: PASS — both branches on `main`, full battery green.**

## Anchors (rollback ledger)

| Ref | SHA |
|-----|-----|
| pre-merge main | `8c1a344a` |
| t-159-leptos-ui tip | `50bba633` (85 commits, tags T-159.15.1–.29.3) |
| t-161-ticket-xtask tip | `cac7b8fe` (tags T-161…T-162.4) |
| **Merge 1** (t-159, `--no-ff`, zero conflicts — merge-tree-predicted) | `df181120` |
| **Merge 2** (t-161, 6 predicted conflicts resolved) | `3cec57ee` |
| origin/main at start | `3c519257` (45 behind; not moved during the merge) |

## Merge-2 conflict resolutions (set matched dry-run `461e1342` exactly)

`scripts/lib/ticket_registry.py` modify/delete → deletion (T-162) · `scripts/ticket` → xtask
wrapper · `Makefile` → t-159 body + t-161's 3-hunk verify-no-python delta · `registry.json` →
main base + t-161 delta (T-161 shipped, T-162 appended, next_id 163; diff confined to those rows
+ one em-dash escape normalization) · `Cargo.lock` → t-159 side + minimal re-resolve ·
`docs/TICKET_REGISTRY.md` → regenerated in-merge via `ticket sync`. `Cargo.toml` auto-merged to
the 6-member union; `verify-monorepo-migration.sh` auto-merged with both sides' hunks (verified).

**In-merge find:** `cargo check --workspace` exposed `map-engine-wasm` broken on **all three
heads** — core's `compose_roads_mesh` went 3-arg at T-152.5 (`074086d8`, which also fixed the
caller) but the caller fix was lost in a later resolution; the orphaned crate was never compiled
natively again. Restored exactly the 074086d8 shape (method + call). `make wasm-ci` passes for
the first time since.

## T-163 integration (this commit)

- xtask dead-React purge: `cmds.rs` 8 lines + `check.rs` 2 scan roots. **Closure:**
  `git grep -cE 'apps/website/frontend|ci-local-frontend' -- xtask/` = **0** (was 10 lines).
- `verify-monorepo-migration.sh` V13 loop `web`→`leptos`.
- Templates: `CLAUDE_CODE_PROMPT.md` / `HANDOFF_TEMPLATE.md` / `SPEC_TEMPLATE.md` retired-command
  fixes (map-assets-link / React npm → `make ci-local-leptos`).
- Registry truth: **T-159 → shipped** (summary + active_slice cleared), **T-154 order 1515**
  (was null → sorted 9999 and squatted the "Latest shipped" headline), **T-163 row** (1600),
  next_id 164. `ticket sync` → CLAUDE.md headline now **"Latest shipped: T-163"**.
- `.ai/artifacts/worktrees/README.md` → post-cleanup truth.

## Gate battery (merged main, all numbers verified this run)

| # | Gate | Result |
|---|------|--------|
| 1 | `cargo fmt -p website-leptos --check` | clean |
| 2 | `cargo clippy -p website-leptos --target wasm32` | **12 = baseline** |
| 3 | `cargo test -p website-leptos` | **46 passed** |
| 4 | `cargo test -p map-engine-core --all-features` | **234 passed** (census pin exact) |
| 5 | `cargo clippy -p xtask -- -D warnings` | clean |
| 6 | `SQLX_OFFLINE=true cargo check -p reforger-backend` | clean |
| 7 | `make wasm-ci` | **PASS** (first green on any head) |
| 8 | `make test-it` (DB up) | 10 + 1 passed |
| 9 | `trunk build --release` | ✅ wasm **7,161,435 B** |
| 10 | `make leptos-gates` | **18 pass-markers** (selfcheck + 15 smokes + live gates) · **frozen V-suite 25/25** |
| 11 | `make verify-no-python` | PASS |
| 12 | `scripts/mod/mcp-call-selftest.sh` | **ALL PASS (19)** |
| 13 | `make schema-validate` | 0 FAIL/ERROR (t090 12/12) |
| 14 | `make schema-codegen` | zero drift |
| 15 | `ticket sync` ×2 | fixed-point (2nd run adds nothing beyond T-163-intended changes) |
| 16 | `./scripts/ticket check` | exit 1 · **exactly 6 ERROR lines** (T-147/148/149 field debt — accepted A4′ set; T-154 line gone after order fix) |
| 17 | closure greps | xtask dead refs **0** · `git ls-files apps/website/frontend` **0** |

## Pre-existing reds ledger (documented, unchanged)

1. `verify-contract-citations`: 1 dangling `@contract` (`TBD_LoadoutEquipComponent.c` — stale
   since T-068.10.4; mod scripts executor-gated; T-068 lane). GitHub CI `schema`/`contracts`
   citations steps will show this single red on push (main's checkout showed **3** findings
   pre-merge; the React deletion removed 2 — net improvement).
2. `ticket check` 6-line field debt (T-147/148/149) — A4′ Python-parity, preserved by design.
3. `scripts/map-assets/verify-t152-cartographic.mjs:126` — React-era wasm path; manual T-152
   tool, no Makefile/CI caller.

## Cleanup + push (final state)

- Worktrees `TBD-T-159` + `TBD-T-161` removed; branches `t-159-leptos-ui` + `t-161-ticket-xtask`
  deleted (fully merged). `git worktree list` = main only.
- Tags now reachable from main: 20× `T-159.*` + 12× `T-161*/T-162*` + `T-163`.
- Pushed `main --tags` to origin (`darkforce09/TBD-reforger`).

## HELD for operator (unchanged)

The prod default flip: `SPA_DIST_DIR` in prod env, `FRONTEND_URL`/`ALLOWED_ORIGINS`/Discord
redirect at the API origin, staging soak, real 142 MB save.
