# Missed-items handoff — post-T-159/T-165 residuals (setup 2026-07-18)

Operator's list of folded/incomplete work, verified against the tree @ `476f1ddf` and
minted as registry tickets **T-166..T-170** (`./scripts/ticket brief T-16x` for any row).
Start a new chat per item (or per pair); every claim below is file:line-verified.

## 0. "This tree still has React" — FALSE, verified

`apps/website/frontend` does not exist; `git ls-files apps/website/frontend` = 0; the only
branch is `main`, the only worktree is this one; deletion landed at T-159.29.3 and merged
via T-163. Nothing to merge or check out. (If some other machine/editor still shows React,
it is a stale checkout there — `git fetch && git reset --hard origin/main`.)

## 1. T-166 (ready) — Leptos editor full map

**State:** editor renders hillshade + grid only. The fold is documented at
`apps/website-leptos/src/world_assets.rs:11` — deferred: unified satellite
(`everon-sat.tbd-sat`) + world objects (`map-engine-core::world` parser + residency).
**Key fact:** every render pipeline already exists and is byte-gated — T-151 shipped the
basemap lane (W1), world parser (W2), residency + building draws (W3), vector layers
roads/forest/contours (W4), glyph atlas trees/props (W5) in the engine crates
(`crates/map-engine-*`). This ticket is HOST WIRING in `world_assets.rs` /
`mission_editor.rs` (fetch → decode → upload → draw-list), not render code.
**Specs:** `t151_1_basemap_lane.md` … `t151_5_glyph_atlas.md`; asset contracts in
`packages/map-assets/everon/manifest.json` (unified sat = LFS, 206 MB — CI pulls
selectively, keep it out of CI fetches).
**Gates:** extend `gate smoke hillshade`-style CDP smokes per lane (`tools/tbd-tools`
`smokes.rs` — T-165.6 harness); `window.__mapAssets` bridge pattern already exists.

## 2. T-167 (ready) — Leptos smart Arsenal port

**State:** `apps/website-leptos/src/arsenal.rs:7-11` — "dumb Forge" tier only; folded:
compat optic/magazine edge rows, clickable paper-doll, weight/validation, Faction Manager.
**Key fact:** the SMART Arsenal shipped in React before the rewrite (T-068.10.2–.10.8:
picker UX, SlotLoadoutV2, expanded modal, paper-doll Mode D + pass 2) and the compat
backend is LIVE (`GET /registry/compat`, 1,880 items / 4,012 edges in DB via T-150/T-068.9).
This is a UI re-port against working endpoints; `SlotLoadoutV2` persistence is unchanged
(`editor_ops::set_loadout`), so panels add without doc/schema changes.
**Specs:** `t068_10_smart_forge_ui.md`, `t068_10_3_forge_picker_ux.md`,
`t068_10_6_arsenal_expanded_modal.md`, `t068_10_7_arsenal_paper_doll.md`,
`t068_10_8_arsenal_ux_pass2.md`. React reference code is in git history pre-`T-159.29.3`
(e.g. `git show T-159.29.2:apps/website/frontend/src/features/mission-creator/ArsenalTab.tsx`).
**Related:** T-068.11 (compiled mod loadout block) is PARKED by operator "until Arsenal
proper" — this ticket is that prerequisite. T-068.12 (player equip) follows.
**Gates:** `gate smoke arsenal` exists (R1–R5 on the dumb tier) — extend for compat rows +
paper-doll.

## 3. T-168 (ready) — ORBAT tree in the left dock

**State:** `apps/website-leptos/src/eden_chrome.rs:360` — "Scope (O7): ORBAT stays a stub
header; no reparent DnD, rename, delete, or virtualization." Only the header text renders.
**Target:** live squads/slots tree from the doc (squad grouping, click-select,
dbl-click→Attributes — the SEL-ORBAT-DBL-001 contract the smokes already assert for the
map path). React reference: `OrbatSection.tsx` (history), behaviors T-037/T-054.
**Boundary:** the dock tree is browse/select; squad MANAGEMENT is T-071's ORBAT Manager
modal (separate queued ticket, operator map-first-gated).
**Gates:** extend `gate smoke undo`'s a6_docksMounted + a new orbat-tree smoke.

## 4. T-169 (queued) — VirtualOutliner at mission scale

**State:** `apps/website-leptos/src/outliner.rs` renders plain rows (fine at seed scale;
DOM-chokes on big missions). React parity target: T-064 — `@tanstack/react-virtual` +
`flattenOutliner.ts` segment-index flatten, `VIRTUAL_SLOT_THRESHOLD=50`, proven @ ~367k.
**Approach:** port the flatten semantics + a hand-rolled window (scroll container +
spacer + visible-slice render) — no JS dep equivalents needed in Leptos.
**Spec:** `t064_virtualized_outliner.md`; references `VirtualOutliner.tsx`/`TreeRow.tsx`
in history.

## 5. T-170 (queued, executor: human) — prod default flip

**State:** Leptos is NOT the prod default yet. Operator-held: set `SPA_DIST_DIR` to the
trunk release dist in the real prod env, move the Discord OAuth redirect origin to the
prod SPA origin, staging soak, flip, keep rollback. claude-code's share: prepare env/config
diffs + a runbook section + verify the soak gates (`gate r-auth` / `gate smoke mutations`
against staging). Runbook: `docs/website/DEV_RUNBOOK.md`.

## Suggested order

T-166 → T-167 (unparks T-068.11 → .12) → T-168 → T-169; T-170 whenever the operator wants
the flip (independent). T-071/T-090.6 remain the other open lanes (unchanged).

## Tooling ground rules for the new chats (post-T-165)

Everything is Rust: gates via `cargo run -p tbd-tools --bin gate|world|map` and
`cargo xtask …`; `make leptos-gates` = editor smokes + V-suite; **no Node anywhere**
(`make verify-no-node` enforces; enfusion-mcp under `scripts/mod` is the only floor).
Direct-on-main, tags `T-16x[.y]`, `Co-Authored-By: Claude Fable 5`, registry via
`./scripts/ticket sync|check`.
