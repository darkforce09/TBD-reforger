# T-164 — Post-merge green sweep: CI green + registry hygiene + doc truth

**Status:** shipped · **Executor:** claude-code (operator-approved plan) ·
**Verify:** [`.ai/artifacts/t164_verify_log.md`](../../.ai/artifacts/t164_verify_log.md)

## Why

The T-163 merge push surfaced real GitHub CI: **6 red jobs across 5 causes** (verified from run
logs, not prediction), plus known gate debt, a diverged remote tag, one broken manual tool, and
operational doc rot left behind by the T-145 (Go→Rust) and T-159 (React→Leptos) cutovers.

## What shipped

### CI green (all 5 causes)

| Cause | Fix |
|-------|-----|
| dangling `@contract` (schema + contracts jobs) | `TBD_LoadoutEquipComponent.c:28` → `#/$defs/gear` (root `properties` died at the T-068.10.4 `oneOf` restructure; comment-only mod edit, flagged in plan) |
| rust-backend `cargo fmt --check` | `cargo fmt` in `apps/website` (pre-existing drift, factions files) |
| map-engine "Invalid PNG signature" | CI had **no LFS anywhere**; the DEM test read a pointer. `ci.yml` map-engine job gains a selective `git lfs pull --include …everon-dem-16bit.png` (72 MB, not the 206 MB satellite) |
| editorconfig | `sap-seam-metrics.mjs` comment re-flowed (odd padding); `scripts/map-assets/vendor/` excluded (vendored `bcdec.h` — never edit vendor) |
| website-leptos trunk/tailwind | `aegis.css` is Tailwind **v4** syntax; local trunk used the PATH v4.3.2 binary while CI auto-downloaded 3.3.5 → `[tools] tailwindcss = "4.3.2"` pinned in `Trunk.toml` |

### Gate debt → `./scripts/ticket check` **exit 0** (first time ever)

T-147/148/149 got real `surfaces`/`impact`; the two prose-polluted rows (T-145/T-151) recoded to
the registry vocabulary; T-164 row registered.

### Also

- Remote tag `T-090.1.2.6` force-pushed from the superseded WIP commit (`3de3d22f`) to the shipped
  commit the registry cites (`b958e3b4`).
- `verify-t152-cartographic.mjs` wasm size-guard → retired-skip (pkg died with React; crates gated
  by `make wasm-ci`).
- **Doc truth:** CLAUDE.md operational sections (Rust backend, real layout, cargo semantics, real
  ci.yml step list); Makefile `tidy` (Go) target deleted + comments; DEV_RUNBOOK all runnable-as-
  written; CODING_STANDARDS §2/§3 + Go/npm gate rows marked **RETIRED** with the live cargo/Leptos
  replay block; all 23 page docs + hub/shell/auth/template docs re-pointed at
  `apps/website-leptos/src/*` (closure: 0 `apps/website/frontend` refs under `docs/website/frontend/`);
  Leptos README + aegis.css provenance comments.
- `t159_leptos_full_migration_inventory.md` (the audit that drove the finish program; only copy)
  committed to `.ai/artifacts/`.
