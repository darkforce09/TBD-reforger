# T-164 — post-merge green sweep — verify log

**Executor:** claude-code (operator-approved plan, 2026-07-17). **Base:** `0be52f9d` (T-163).
**Result: PASS locally; CI watch results appended below after push.**

## CI-red fixes (5 causes, local proof)

| Cause (job) | Fix | Local proof |
|-------------|-----|-------------|
| dangling `@contract` (schema + contracts) | `TBD_LoadoutEquipComponent.c:28` → `#/$defs/gear` (root `properties` died at T-068.10.4; `$defs/gear` is version-agnostic, matches all sibling citations + the generated `loadout.rs::Gear`) | `verify-contract-citations.mjs` **exit 0 — "All @contract citations resolve"** (first time) |
| rust-backend fmt | `cargo fmt` in `apps/website` (factions drift, pre-existing) | `cargo fmt --check` clean |
| map-engine LFS | ci.yml map-engine job: selective `git lfs pull --include …everon-dem-16bit.png` (repo had **zero** `lfs:` config; test read a pointer) | test passes locally (real file); CI proof on push |
| editorconfig | `sap-seam-metrics.mjs` comment re-flow; `scripts/map-assets/vendor/` added to excludes | targeted `editorconfig-checker` on both: **clean** (repo-wide local run is noisy only from untracked `target/` dirs absent in CI) |
| website-leptos tailwind | `Trunk.toml` `[tools] tailwindcss = "4.3.2"` (aegis.css is v4 syntax; CI auto-downloaded 3.3.5; local silently used the PATH v4.3.2 binary) | `trunk build --release` ✅ with the pin, log shows `tailwindcss v4.3.2` |

## Gate debt

- **`./scripts/ticket check` → `check OK`, exit 0 — first time in repo history.** T-147 `[MAP,ORBAT]/[ui]`,
  T-148 `[MAP]/[ui,state]`, T-149 `[MAP]/[ui,perf]`; prose-polluted T-145 → `[SHELL,DATA]/[api,infra,compiler]`,
  T-151 → `[MAP]/[perf,ui]`. T-164 row (order 1610), `next_id` 165. `sync` ×2 fixed-point.
- **`verify-t152-cartographic.mjs` → OK, exit 0.** Three wasm-dependent steps (its own size guard +
  `verify-town-labels` + `verify-road-names`) hard-failed on the deleted React pkg path → all three
  now retire-skip (the math is pinned by `make wasm-ci` cargo tests); the tool's real T-152 checks
  still run and pass.

## Doc truth closures (grep-verified)

- CLAUDE.md operational sections: **0** Go/GORM/golangci/gofmt refs before §Status (backend = Axum +
  sqlx, real `src/bin/api.rs` layout, cargo semantics, actual ci.yml step list). §Status history intact.
- Makefile: `go mod tidy` target **gone**; comments cargo-truthful; parses.
- DEV_RUNBOOK: **0** dead refs (`make web`→`make leptos`, npm→cargo equivalents, psql mock-seed recipe).
- CODING_STANDARDS: T-164 cutover banner; §2 Go + §3 TS/React marked **RETIRED** with live
  equivalents; §11 replay block = the real `make ci-local` sequence.
- `docs/website/frontend/**`: **0 files** reference `apps/website/frontend` (23 page docs +
  README/ROADMAP/THEME/template/shell/auth re-pointed at `apps/website-leptos/src/*`; stitch refs
  marked git-history).
- Leptos README ("only frontend since T-159.29.3") + aegis.css provenance comment.
- `t159_leptos_full_migration_inventory.md` rescued into `.ai/artifacts/` (was scratchpad-only).

## Tag reconcile

`T-090.1.2.6`: remote pointed at superseded WIP `3de3d22f` (ancestor); force-pushed to the shipped
commit the registry cites, `b958e3b4`. Revert path: `git push --force origin 3de3d22f:refs/tags/T-090.1.2.6`.

## CI watch (final — run 29576884048 @ `801e7cf9`)

**ci.yml: completed SUCCESS — all 5 jobs green:**

| Job | Conclusion |
|-----|-----------|
| rust-backend (Rust 1.95 + Postgres 18) | success |
| map-engine (wasm-ci) | success |
| editorconfig (FMT-2) | success |
| schema + citations | success |
| website-leptos (Leptos SPA) | success |

**contracts.yml: completed SUCCESS** (citations + codegen-drift). tbd-schema compatibility test:
success. **First fully green CI on `main` since the T-145/T-159 cutovers** (origin CI had last run
45 commits before the merges).
