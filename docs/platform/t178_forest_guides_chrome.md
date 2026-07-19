# T-178 — Forest load consistency + YouTube guides polish + Outliner label

**Status:** `ready` · **Executor:** Claude Code *(operator may run in a **fresh Cursor chat on Grok 4.5** — Claude tokens exhausted; same prompt)* · **Branch:** `main`  
**Depends on:** T-177 (shipped)  
**Scope:** `apps/website/frontend/**`, `crates/map-engine-*` if forest GPU/shader needs it. Prefer **one coherent canopy draw** over visible 512 m progressive tiles. **Not** `apps/mod/`. **Not** T-071.1 CRUD.  
**Evidence:** [`.ai/artifacts/t178_operator_screens/`](../../.ai/artifacts/t178_operator_screens/) · UX audit [`.ai/artifacts/t178_mc_ux_audit.md`](../../.ai/artifacts/t178_mc_ux_audit.md)

**No silent deferrals** of A1–A4. Soft “shader later / fold forward” forbidden unless operator says `defer X`.

## Why

Post T-177 eye-pass 2026-07-19:

1. **Forest loading glitched / inconsistent** — sometimes patchy rectangular tiles (512 m progressive push); seams/opacity differ across chunks. Operator: improve beyond live piecemeal calc — consider **shader / single coherent canopy** (investigate; ship what kills the tiling glitch).  
2. **Remove “Outliner”** word from left panel — keep **Editor Layers** only.  
3. **Guide lines have gaps** — not coherent like YouTube; elbows don’t form a continuous spine.  
4. **Click the guide line** to expand/collapse that branch (YouTube behavior) — today guides are `pointer-events-none`; only chevron toggles.

## Leads

| ID | Lead |
|----|------|
| A1 | `forest_mass.rs` — `push_composite` with partial `present`; `CHUNK_SIZE_M=512`; `CAMERA_GESTURE` skips forest during pan → stale/patchy until settle. Explore: full-island precompose, sticky full mesh until ready, or GPU density texture/shader path — **pick and ship** one approach that looks continuous. |
| A2 | `eden_chrome.rs` ~1128 — literal `"Outliner"` above `"Editor Layers"`. |
| A3 | `guide_spans` elbow `h-1/2` + last-child drops tail → vertical gaps between rows. Need continuous spine like YouTube ref. |
| A4 | Guides `pointer-events-none`; wire click on spine/elbow → same toggle as chevron for that folder depth. |

## Acceptance (MUST)

| ID | Done when |
|----|-----------|
| A1 | Pan/zoom island: forest canopy looks **consistent** (no obvious missing/half-loaded 512 m rectangles under normal use). Document approach in verify log (shader / prebake / settle policy). |
| A2 | Left dock has no “Outliner” label — **Editor Layers** (or equivalent single title) only. |
| A3 | Tree connectors continuous (no dashed gaps between elbows); matches YouTube coherence intent. |
| A4 | Clicking the guide line for a branch toggles expand/collapse (chevron still works). |

## Phases

0. Inventory → `t178_inventory.md` (forest path A/B options + guide geometry).  
1. A1 forest consistency.  
2. A2 label.  
3. A3–A4 guides.  
4. Gates + verify + tag **T-178**.

## Locked

1. Fix A1–A4 completely.  
2. Do not reintroduce 32 m landcover forest wash (T-176).  
3. Do not expand into T-071.1.  
4. `apps/mod/**` OFF LIMITS.  
5. Day-to-day = `make leptos` (release).  
6. No silent deferrals.

## Claude / Grok prompt — T-178 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first.

Implement **T-178** — Forest load consistency + YouTube guides polish + Outliner label.

═══ PREFLIGHT ═══
  git pull --ff-only
  make db-up
  ./scripts/ticket brief T-178

═══ READ ═══
  1. .ai/artifacts/t178_claude_code_handoff.md
  2. docs/platform/t178_forest_guides_chrome.md
  3. .ai/artifacts/t178_operator_screens/ + t178_mc_ux_audit.md
  4. apps/website/frontend/src/world_assets/forest_mass.rs
  5. apps/website/frontend/src/world_assets/mod.rs (CAMERA_GESTURE)
  6. apps/website/frontend/src/eden_chrome.rs (guide_spans, Outliner label)
  7. apps/website/frontend/src/outliner.rs (FlatRow.ancestors)
  8. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  Forest canopy loads inconsistently (chunky/patchy tiles). Remove left
  "Outliner" label. Guide lines have gaps — make continuous YouTube-style;
  clicking a guide line toggles expand/collapse.

═══ SHIPPED ═══
  T-177 @ e97a01c6 — keep ORBAT Manager, grab cursor, menu z-index.
  T-176 — no landcover forest wash return.

═══ LANGUAGE GATE ═══
  Rust owns forest geometry/GPU. Leptos = chrome/host. Prefer coherent canopy
  draw (shader / density texture / full mesh) over visible progressive tiles.

═══ LOCKED ═══
  - Fix A1–A4 completely
  - No 32m landcover wash
  - Not T-071.1
  - apps/mod/** OFF LIMITS
  - No silent deferrals

═══ DO ═══
  1. Phase 0 inventory (forest approach choice)
  2. A1 forest consistency
  3. A2 remove Outliner label
  4. A3 continuous guides + A4 click-to-toggle on guides
  5. make leptos-gates + make ci-local; t178_verify_log.md
  6. Commit on main T-178: · tag T-178 · push
  7. Cursor doc list

═══ DO NOT ═══
  - Edit docs/** / registry / CLAUDE sync markers
  - Touch apps/mod/**
  - Leave patchy forest as "good enough"
  - Leave gapped guides

═══ VERIFY ═══
  make leptos-gates
  make ci-local
  t178_verify_log.md: A1–A4 PASS + forest approach note + manual screens

═══ RETURN ═══
  - tag T-178 @ sha
  - inventory + verify
  - matrix
  - Cursor doc list
  - ASK if blocked
```
