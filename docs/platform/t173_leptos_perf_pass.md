# T-173 — Leptos SPA + Mission Creator performance pass

**Status:** SHIPPED @ tag **T-173** / `dddf3158` · **Branch:** `main`  
**Depends on:** T-172 (shipped)  
**Verify:** [`.ai/artifacts/t173_verify_log.md`](../../.ai/artifacts/t173_verify_log.md) · inventory [`.ai/artifacts/t173_inventory.md`](../../.ai/artifacts/t173_inventory.md)  
**Scope shipped:** `apps/website/frontend/**`, `crates/map-engine-*`, Makefile serve path. **Not** `apps/mod/`. **Not** T-170.

**No silent deferrals.** Soft “later / optional / fold forward” forbidden unless the operator explicitly says `defer X` / `skip X`.

## Why

T-172 restored dead UI and many MC fidelity gaps, but operator review 2026-07-18 says **performance is still unacceptable**:

> pan/zoom stutter “unbearable”; glyphs/roads thrash on load/unload; Mission Library scroll still laggy with delayed catch-up; dossier side-panel open animation incredibly laggy; tree guide lines not continuous + hard to see; Mission Settings render prefs from before the rewrite are missing; fences/railings missing when zoomed in; building icons upside down (building fills OK).

Also: local `make leptos` is `trunk serve` **without** `--release` (debug wasm). Gates use `trunk build --release`. That compile/profile gap is a first-class suspect — measure it, then fix tooling **and** real engine/UI jank that remains in release.

**Operator 2026-07-18 (after `trunk serve --release`):** release “definitely made a big improvement” but is **“nowhere near good enough”** / **“not as good as it was before”** (pre-rewrite React MC). Later same session: acceptance is **better than pre-rewrite**, not “near” / “as good as”. → **P8 is necessary, not sufficient.** Phases 2–5 must beat the React baseline on the **release** serve path; do not mark P1–P5 done because debug→release alone felt better.

**Evidence:** [`.ai/artifacts/t173_operator_screens/01_mc_stutter_tree_lines.png`](../../.ai/artifacts/t173_operator_screens/01_mc_stutter_tree_lines.png) (post-T-172 MC: CUR Z live, glyphs on map, dashed hard-to-see tree lines).

## Goal

Smoothness on the **day-to-day release path** that is **better than the pre-rewrite React editor** (operator bar — not “near”):

1. Mission Creator pan/zoom **smoother than React**; residency (glyphs/roads/chunks) less thrashy than React  
2. Mission Library scroll + dossier sheet **snappier than React**  
3. Mission Settings restores pre-rewrite **render prefs** (map style / grid / hillshade / world-layer toggles — wire to live host, not placeholder copy)  
4. Outliner + Asset Browser guide lines: **continuous** vertical stems + readable contrast  
5. Documented local serve story so `make leptos` is not silently a debug-wasm trap (or an explicit `make leptos-release` that becomes the operator default)  
6. Fence + railing markers visible when zoomed in (T-152.15 parity)  
7. Building **icons/badges** upright (building OBB fills already correct)

## Acceptance matrix (MUST)

| ID | Bug | Notes |
|----|-----|-------|
| P1 | MC pan stutter | Profile release build; fix GPU/CPU/residency causes — **smoother than React baseline** |
| P2 | MC zoom stutter | Same; include zoom-band forest/glyph transitions — **smoother than React baseline** |
| P3 | Glyph / road load-unload thrash | Residency / upload / drain path — no visible rag-dolling; **less thrash than React** |
| P4 | Mission Library scroll lag / delayed catch-up | After T-172 A3 CSS; remaining causes — **smoother than React library** |
| P5 | Dossier / side-panel open animation lag | Sheet enter must not hitch; **snappier than React sheet** |
| P6 | Missing Mission Settings render prefs | `eden_chrome.rs` still says map style/grid/hillshade/world toggles “arrive with the terrain render host” — restore vs React T-091.2 / Mission Settings |
| P7 | Tree guide lines discontinuous + low contrast | Continuous stem through open folders; higher-contrast Aegis-legal color |
| P8 | Dev serve profile honesty | Class-R: compare `trunk serve` (debug) vs release serve FPS/hitch; make the day-to-day path release-quality (change `make leptos`, add `leptos-release`, or Trunk release-by-default — pick one, document, prove) |
| P9 | Fences / railings missing on zoom-in | Operator: fence markers + railing markers should appear when zoomed in (React/T-152.15 fence/pier strip lane). Find why Leptos host never draws them (not wired, wrong zoom gate, or strip lane off) and restore |
| P10 | Building icons upside down | Operator: building **glyphs/badges** are inverted; building footprint/OBB rotation looks correct. Fix icon/badge upright (V-flip / atlas UV / instance yaw) without breaking OBB fills |

## Phases

### Phase 0 — Profile inventory → `.ai/artifacts/t173_inventory.md`

1. **Build-profile A/B** — same mission, same camera path: debug `trunk serve` vs `trunk serve --release` / release dist served by API. Record FPS, hitch ms, wasm size. Quote which path the operator was on.  
2. Chrome Performance / wasm samples for: pan, zoom, library scroll, sheet open.  
3. Engine suspects: chunk residency, forest retint uploads, road/glyph lanes, `on_camera_changed`, atlas, rAF storms, Leptos signal fan-out.  
4. Shell suspects: mission grid DOM size, Sheet animation + backdrop-blur, still-fixed backgrounds on cards.  
5. Mission Settings React vs Leptos control gap table (exact toggles to restore).  
6. Tree guide-line CSS/DOM plan (continuous stem).  
7. Fence/railing/pier strip lane status vs T-152.15 (wired? zoom gate? toggle?).  
8. Building-badge upright: atlas UV / instance transform vs OBB (file:line).

### Phase 1 — Serve path (P8)

Ship a default local path that is **release-quality** (or make the difference impossible to miss). Update Makefile / README / DEV_RUNBOOK return list for Cursor if prose-only.

### Phase 2 — Shell perf (P4/P5)

Mission Library scroll + dossier animation. Fix root causes from inventory (virtualize, defer heavy work off open frame, reduce blur/filter cost, etc.).

### Phase 3 — MC engine perf (P1/P2/P3)

Pan/zoom/residency. Prefer Rust engine policy (`map-engine-*`); thin Leptos host. Prove with smokes + operator-path FPS notes in verify log.

### Phase 4 — Settings + tree polish (P6/P7)

Wire Mission Settings render prefs to the live map host. Continuous readable tree guides.

### Phase 5 — Cartographic fidelity residuals (P9/P10)

Restore fence + railing (and pier strips if the same lane) visibility on zoom-in per T-152.15 contract. Fix upside-down building badges/icons (upright in screen space; OBB fills stay as-is).

### Phase 6 — Verify + ship

`make leptos-gates` + `make ci-local`; `.ai/artifacts/t173_verify_log.md`; tag **T-173**.

## Locked

1. Operator closed T-172 as functionally good enough — **do not reopen A/B matrix** unless a T-172 regression appears.  
2. Performance bar: **strictly better than the pre-rewrite React editor** on pan/zoom/residency + Mission Library scroll/sheet — proven on the operator’s day-to-day **release** path. “Near” / “parity” / “good enough vs React” is **FAIL**.  
3. No three.js. No mod. No inventing Out-of-scope for P1–P10.  
4. If a render pref needs a new engine API, add it — don’t leave the placeholder sentence.  
5. P9/P10 are fidelity bugs found in the same operator pass — **in this ticket**, not a later cartography program.

## Claude Code prompt — T-173 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first.

Implement **T-173** — Leptos SPA + Mission Creator performance + fidelity residuals
(P1–P10 + Phase 0 discoveries).

═══ PREFLIGHT ═══
  git pull --ff-only
  make db-up
  ./scripts/ticket brief T-173

═══ READ ═══
  1. .ai/artifacts/t173_claude_code_handoff.md
  2. docs/platform/t173_leptos_perf_pass.md
  3. .ai/artifacts/t173_operator_screens/01_mc_stutter_tree_lines.png
  4. apps/website/frontend/src/eden_chrome.rs (MissionSettings placeholder)
  5. apps/website/frontend/src/world_assets/world_host.rs (glyph/badge lanes)
  6. docs/specs/Mission_Creator_Architecture/t152_15_fence_pier_bridge_visibility.md
  7. Makefile leptos vs leptos-build targets
  8. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  T-172 fixed functional bugs; operator still sees unbearable MC pan/zoom
  stutter, glyph/road thrash, Mission Library scroll + sheet animation lag,
  missing Mission Settings render prefs, weak discontinuous tree guides,
  missing fence/railing markers on zoom-in, and upside-down building icons
  (OBB fills OK). make leptos is trunk serve without --release — measure first.

═══ SHIPPED ═══
  T-172 @ e08884f4 (tag T-172) — functional bash; residual = this ticket.
  T-152.15 fence/pier strips shipped in React-era engine — reinstate on Leptos host.

═══ LOCKED ═══
  - Fix P1–P10 + every inventory H-row from Phase 0
  - Prove operator day-to-day path is release after P8
  - Perf bar: BETTER than pre-rewrite React (not near / not parity)
  - Restore real Mission Settings render prefs (no placeholder copy)
  - Continuous high-contrast tree guide lines
  - Fences + railings visible on zoom-in; building badges upright
  - apps/mod/** OFF LIMITS
  - No silent deferrals

═══ DO ═══
  1. Phase 0 inventory + build-profile A/B numbers
  2. Phase 1 serve path (P8)
  3. Phase 2 shell perf (P4/P5)
  4. Phase 3 MC engine perf (P1/P2/P3)
  5. Phase 4 settings + tree lines (P6/P7)
  6. Phase 5 fences/railings + upright building badges (P9/P10)
  7. make leptos-gates + make ci-local; t173_verify_log.md
  8. Commit on main T-173: · tag T-173 · push
  9. Cursor doc list if needed

═══ DO NOT ═══
  - Edit docs/** / registry / CLAUDE sync markers
  - Touch apps/mod/**
  - Call residuals “good enough” without fixing P1–P10
  - Leave make leptos as a silent debug-wasm trap without documenting
    an operator-default release path
  - Flip building OBB fills when fixing badge uprightness

═══ VERIFY ═══
  make leptos-gates
  make ci-local
  t173_verify_log.md: P1–P10 PASS with before/after numbers on the
  operator serve path; inventory Found-by-hunt non-empty if hunt finds more

═══ RETURN ═══
  - tag T-173 @ sha
  - inventory + verify paths
  - P/H matrix pass table
  - Cursor doc list
  - ASK if blocked
```
