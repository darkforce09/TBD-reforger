# T-173 — Claude Code handoff (perf pass)

**Start on `main` after T-172 tag.**  
**Do not touch `apps/mod/`.**  
**Do not edit docs/registry** — return a Cursor list for prose if needed.

## Operator word (closes T-172 residual into this ticket)

> “For this ticket [T-172], I think it's good enough, but the mission creator is still way too slow… We need to make the performance good… Mission Library scrolling is still very laggy… side panel animation incredibly laggy… settings … missing … lines for the folders are not a continuous line … hard to see … I heard something about … how it's compiled”

**After `trunk serve --release` (same day):**

> “That definitely made a big improvement, but it's nowhere near good enough. Nowhere near good enough. At least, it's not as good as it was before.”

**Bar clarification (same day):**

> “Not near, they needs to be better than pre-rewrite.”

→ Fix P8 so day-to-day is release, then keep hunting/fixing on **release** until pan/zoom/library/**sheet are strictly better than the pre-rewrite React editor**. “Near” / parity = FAIL. Do not stop at “release is better than debug.”

## One-line

Make day-to-day Leptos + Mission Creator **feel fast**, restore Mission Settings render prefs, continuous readable tree guides, **show fences/railings on zoom-in**, and **upright building badges** (fills already OK).

## Authority

1. [`docs/platform/t173_leptos_perf_pass.md`](../../docs/platform/t173_leptos_perf_pass.md)  
2. This handoff  
3. Screen: [`.ai/artifacts/t173_operator_screens/01_mc_stutter_tree_lines.png`](t173_operator_screens/01_mc_stutter_tree_lines.png)  
4. Prior: [`.ai/artifacts/t172_verify_log.md`](t172_verify_log.md)

## Confirmed leads

| Item | Lead |
|------|------|
| P8 debug serve | `Makefile` `leptos:` → `trunk serve` (no `--release`); `leptos-build` / gates use `--release`; `index.html` `data-wasm-opt="z"` applies on release |
| P6 missing settings | `eden_chrome.rs` MissionSettingsDialog — placeholder: map style/grid/hillshade/world toggles “arrive with the terrain render host” |
| P1–P3 MC jank | `mission_editor.rs` + `world_assets/*` + `map-engine-render` residency/upload; screenshot shows `glyph 14605` @ ~192 FPS readout with operator-reported hitch |
| P4/P5 library | `missions.rs` grid + Sheet animations (`animate-sheet-in` / backdrop-blur) |
| P7 tree lines | `eden_chrome.rs` border-l guide spans from T-172 B7 — discontinuous + low contrast |
| P9 fences/railings missing | T-152.15 contract; engine strip lanes exist — check Leptos `world_host` / residency zoom gates / toggles never uploading fence+railing strips |
| P10 building icons upside down | `world_badge_glyphs` / icon lane 2 upload in `world_host.rs` — upright/V-flip vs OBB; do not “fix” by rotating building fills |

## Execution

Phase 0 inventory (build A/B numbers required) → P8 serve path → shell perf → MC engine → settings + tree lines → **fences + upright badges (P9/P10)** → gates + verify log → tag **T-173**.

## Return

Tag @ sha · inventory + verify · P1–P10 (+ H) pass table · Cursor doc list · ASK if blocked.
