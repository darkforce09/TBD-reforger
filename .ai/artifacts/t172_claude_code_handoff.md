# T-172 — Claude Code handoff (Leptos bug bash)

**Start on `main` after T-171.**  
**Do not touch `apps/mod/`.**  
**Do not edit docs/registry/CLAUDE sync markers** — return a Cursor list only if prose needs updating after ship.

## One-line

Fix the operator-reported Leptos shell + Mission Creator regressions — **and actively hunt for more bugs** (Phase 0 + mid-pass), inventory them as `H1…Hn`, and fix **all** of them in **one** ticket. Operator list = seed, not ceiling.

## Authority

1. [`docs/platform/t172_leptos_bug_bash.md`](../../docs/platform/t172_leptos_bug_bash.md) — matrix + phases + copy-paste prompt  
2. This handoff  
3. [`.ai/artifacts/t172_operator_screens/`](t172_operator_screens/) — screens `01`–`05`

## Confirmed code leads (recon)

| Bug | Lead |
|-----|------|
| A1 user menu dead | `apps/website/frontend/src/layout.rs` — avatar button; comment says dropdown is “a follow-up” |
| A2/A8 sticky nav + breadcrumb | same file — `use_location().pathname.get()` once; comment admits reactive follow-up |
| A9 narrow sidebar pop-out dead | same file — `SidebarMobileToggle` button has no open state / drawer; `Sidebar` is `hidden lg:flex` only |
| A4–A6 dead list select | `wiki.rs` / `vehicles.rs` / `modpacks.rs` — selected = first mock, no click→state |
| B2 no Z | `eden_chrome.rs` — CUR “X/Y only” by scope comment |
| B10 2D Arsenal | `arsenal.rs` — SVG `paper_doll`; React T-154 used `DollEngine` |
| B3/B4 map | `world_assets/*` + mission entity/icon lanes from T-166 |

## Execution order

1. **Phase 0 — HUNT first** (spec §C methods — stubs grep, every nav route click-through, narrow viewport, MC smoke, screen-`05` parity, console/network). Write `t172_inventory.md` with operator A/B leads **and** a non-empty **Found-by-hunt** table (`H1…Hn`). Empty hunt table = incomplete.  
2. **Phase 1** — shell: reactive nav, user menu, narrow-viewport sidebar drawer (A9), wiki/vehicles/modpacks selection, dossier loading UX + shell H-rows.  
3. **Phase 2** — scroll + MC lag root causes + perf H-rows.  
4. **Phase 3** — slots visible, forest translucent, CUR Z, tree collapse/icons/lines, 3D Arsenal, chrome gaps + MC H-rows.  
5. Mid-pass finds → append H-rows → fix before tag.  
6. **Phase 4** — gates; verify log with Found-by-hunt section; tag **T-172**; push.

## Hard rules

- No silent deferrals ([`.cursor/rules/no-silent-deferrals.mdc`](../../.cursor/rules/no-silent-deferrals.mdc)).  
- Inventory discoveries are in-scope. If blocked → **ASK**, do not invent Out-of-scope.  
- 3D Arsenal is mandatory (operator rejected SVG-only).  
- T-069 markers / T-070 vehicles **placement** stay their tickets; chrome tabs/buttons visible in screen `05` must exist in Leptos (stub OK only if React also stubbed that control).

## Return

- Tag `T-172` @ sha  
- Inventory + verify log paths  
- Matrix A/B + **H** rows → PASS/FAIL  
- Count: operator-seeded vs found-by-hunt  
- Cursor doc list (if any)  
- ASK rows (if any)
