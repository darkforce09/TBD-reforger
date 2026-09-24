# T-068.10.8 — Arsenal UX pass 2 (no-scroll modal, A3 slot rail, clean doll)

**Ticket:** T-068 · **Slice:** T-068.10.8 · **Status:** shipped ·
**Executor:** claude-code (Mode D session, operator screenshot review of .10.7) ·
**Verify:** [`.ai/artifacts/t068_10_8_verify_log.md`](../../../.ai/artifacts/t068_10_8_verify_log.md)

## In one sentence

Operator review fixes on the .10.7 paper-doll: the Arsenal now opens with **zero
scrolling** (modal pinned to the popup height, panes scroll internally), the left panel
gains the **Arma 3 Virtual Arsenal vertical slot rail** (icon per pickable region, click =
select — same handler as clicking the doll), and the doll itself is **text-free** (the
label soup was the "looks a little fucked" offender) with names moved to the rail
tooltips, list header, context column, and a one-line caption under the doll.

## Shipped shape

- **No-scroll:** `AttributesModal` Arsenal className → `h-[85vh] max-w-7xl` (height ==
  the shared DialogContent's `max-h-[85vh]` → deterministic body height) + the tab
  wrapper goes `h-full min-h-0` on Arsenal only; `ArsenalTab` root `h-full` flex column,
  grid `h-[72vh]` → `min-h-0 flex-1`. Dialog-body `overflow-y-auto` stays as a
  tiny-viewport fallback.
- **Slot rail:** `RAIL_REGIONS` in `arsenalDollModel.ts` — all 14 pickable keys, flat,
  A3 order (weapons + rifle attachments first, then wear head-to-toe), completeness
  vitest-asserted against `EMPTY_PICKS` like `DOLL_REGIONS`. `SlotRail.tsx` renders the
  44px icon column (lucide icons, equipped dot, active highlight, tooltip/aria =
  "Region — item | empty"); grid is now `[44px | 260px | 1fr | 240px]`
  (rail | list | doll | context). Rail and doll share `activeKey` — two-way sync for free.
- **Clean doll:** `SoldierSilhouette` drops every `<text>` label + the label plumbing;
  hotspot geometry, 3 states, `<title>` tooltips, aria, keyboard handling unchanged.
  New `DollCaption` line under the doll: `{active region} — {item | empty}`.

## Out of scope → T-154

Operator call (2026-07-13, via question at the .10.7 pause): the doll's next fidelity step
is a **basic rotatable 3D model** — implemented in **Rust/wgpu** as its own slice, NOT
three.js (one GPU stack; D5 language gate: Rust owns engine policy, TS stays dumb UI).
Registry row **T-154** (`idea`): extend `map_engine_wasm` with a 3D perspective pipeline
(WGSL, depth, lighting), primitive soldier mesh, ray picking, modal canvas lifecycle;
replaces the SVG silhouette. The region/state contract to build against is already pure in
`arsenalDollModel.ts`.

## Gates

vitest **354/354** (+ RAIL_REGIONS completeness) · build + `tsc --noEmit` clean · lint =
pre-existing `router.tsx` only · operator visual at the Mode D pause (checklist in the
verify log — the headline assert: Arsenal opens fully visible at ~1080p, no scrollbar).
