# T-152.13 — Readable text atlas (font fidelity)

**Ticket:** T-152 · **Slice:** T-152.13 (remediation ladder #2)
**Status:** `queued`
**Executor:** **claude-code** (Claude Code)
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §7.3 (A11, S4)
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.13`**
**Depends on:** T-152.12 (upright, alive text lane)
**Blocks:** T-152.16, T-152.17 (readability acceptance)

## In one sentence

Replace the procedural 8×8 px numerals-first bitmap font with a readable baked atlas — larger cells, real lowercase, punctuation, accent folding — while keeping the 20 B text-instance format and the Rust-owned bake path.

---

## Problem

Audit §7.3: `bake_ascii_atlas_rgba` builds a 16×6-cell, **8×8 px** atlas (128×48 total) of 5×7 stroke patterns — "enough for height numerals" (`crates/map-engine-render/src/text_layout.rs:164-179`). Lowercase is folded to uppercase (`:204-208`); only `0-9`, `A-Z`, space, and `m` have real glyphs (`:209-222`); **every other character renders an O-shaped blob** (`:223`). Town names like "Saint-Philippe" (hyphen) or any punctuation are unreadable regardless of orientation — operator S4's "hard to read even if orientation were correct".

---

## Goal

1. **New baked atlas:** cell ≥ **16×16 px** (32×32 preferred if wasm-size budget allows), coverage = full printable ASCII 32–126 **with distinct lowercase**, plus a documented accent-fold map (é→e …) for any non-ASCII in `locations.json` / `road-names.json`.
2. **Same contract:** glyph index scheme + 20 B instance format + `ensure_text_atlas()` entry unchanged so consumers (`useWgpuHeightLabels` / town / road hooks) need zero TS changes.
3. **No-blob gate:** every character actually used by committed label data maps to a real glyph, not the fallback.
4. Verify log `.ai/artifacts/t152_13_verify_log.md`.

---

## Out of scope

- SDF pipeline / runtime font rasterization (bake-time only; revisit only if bitmap at 16–32 px fails M1).
- Label placement, zoom bands, declutter (T-152.16/.17).
- Icon glyph atlas (`world-glyphs.webp`) — untouched.
- TS changes.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | Bake-time bitmap atlas stays in Rust (`text_layout.rs`) — embedded font table or `include_bytes!` raster, no network/asset fetch | Self-contained wasm; LANGUAGE GATE |
| L2 | Grid may grow (e.g. 16×8 cells @ 16–32 px); `vs_text` UV math reads grid dims from constants shared with the bake — **no hardcoded 16/6 left behind** | The .12 shader divides by 16.0/6.0 today |
| L3 | Lowercase rendered as lowercase (kill the uppercase fold) | Cartographic typography |
| L4 | Fallback glyph becomes a visually-obvious `□` (tofu), and the **no-blob gate** makes committed-data fallback hits a FAIL | Silent blob was the S4 trap |
| L5 | Wasm size delta logged; soft ceiling **+256 KiB** on `map_engine_wasm_bg.wasm` (raise only with justification in verify log) | Keep bundle sane |
| L6 | Commit `T-152.13:` · tag `T-152.13` · verify log | House convention |

---

## Tasks

1. Choose cell size + charset table; implement new bake (replace 5×7 stroke synth) with a real embedded pixel font.
2. Thread grid dims through `vs_text` UV (constants or uniform) — no magic 16/6.
3. Accent-fold map + charset audit script over `locations.json` + `road-names.json` + height numerals.
4. Update G-tests: atlas dims, glyph coverage, no-blob gate; keep .12's orientation + guard gates green.
5. Verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | Atlas cell ≥ 16×16 px; dims test updated and passing | Atlas |
| **G2** | ∀ ch ∈ printable ASCII 32–126: dedicated glyph (≠ fallback); lowercase ≠ uppercase raster for a–z | Coverage |
| **G3** | ∀ ch ∈ chars(committed `locations.json` names ∪ `road-names.json` names ∪ "0-9 m"): resolves to real glyph after accent fold — **0 fallback hits** | No-blob |
| **G4** | T-152.12 gates still green (WGSL guard, UV corners, GPU upright readback both backends) | Regression |
| **G5** | Wasm size delta ≤ +256 KiB (or justified in verify log) | Budget |
| **G6** | cargo fmt/clippy/tests + `make wasm` + FE test/build/lint exit 0; no `apps/**` diffs | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo test -p map-engine-render     # G1–G4
make wasm && ls -l apps/website/frontend/src/wasm/pkg/*_bg.wasm   # G5 vs prior size
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1:** Town labels at z=−2 readable at arm's length (Morton, Saint-Philippe with hyphen intact).
- **M2:** Height numerals crisp at z=0; road names legible along curves.

---

## Documentation sync (Cursor, after merge)

Registry `T-152.13 → shipped`; hub row; `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.13 (copy-paste)

Authority: this spec. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.13** — readable text atlas (font fidelity).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  git log --oneline -3   # expect T-152.12 shipped (upright text lane)
  make wasm

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_13_text_atlas_fidelity.md
  2. .ai/artifacts/t152_11_fidelity_audit_report.md §7.3
  3. crates/map-engine-render/src/text_layout.rs (bake + pack + tests)
  4. crates/map-engine-render/src/shader.wgsl (vs_text UV grid divisors)
  5. packages/map-assets/everon/{locations.json,road-names.json} (charset reality)

═══ PROBLEM ═══
  8×8 px 5×7 uppercase-only font; punctuation/diacritics → O-blob. Unreadable labels (operator S4).

═══ SHIPPED (do not reopen) ═══
  T-152.12 orientation + 16 B uniform — keep its gates green.

═══ LANGUAGE GATE ═══
  Rust/WGSL ONLY. Zero TS changes; 20 B instance format frozen.
  STOP IF: about to fetch fonts at runtime or touch apps/** — bake-time embedded data only.

═══ LOCKED ═══
  - Cell ≥16×16, full printable ASCII + lowercase + accent-fold map
  - Grid dims shared bake↔shader (no hardcoded 16/6 remnants)
  - Tofu fallback + zero fallback hits on committed label data (G3)
  - Wasm delta ≤ +256 KiB soft ceiling (log it)

═══ DO ═══
  1. New embedded pixel font bake in text_layout.rs
  2. UV grid constants threaded to vs_text
  3. Charset audit gate over committed label data
  4. Update atlas tests; keep .12 gates green
  5. Verify suite; .ai/artifacts/t152_13_verify_log.md; commit "T-152.13: ..."; tag T-152.13

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/**, generated TICKET_*.md
  - Change instance byte format or TS consumers
  - Touch icon atlas (world-glyphs) or label policy (bands/declutter)

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1–M2 per spec

═══ RETURN ═══
  - Commit SHA + tag; verify log path
  - Atlas dims + wasm size delta
  - G3 charset audit output (0 fallback hits)
```
