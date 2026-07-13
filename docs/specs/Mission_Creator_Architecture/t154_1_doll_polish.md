# T-154.1 — Doll polish (contrast, rotation direction, hover + callout)

**Ticket:** T-154 · **Slice:** T-154.1 · **Status:** shipped ·
**Executor:** claude-code (Mode D session, operator screenshot review of T-154.0) ·
**Verify:** [`.ai/artifacts/t154_1_verify_log.md`](../../../.ai/artifacts/t154_1_verify_log.md)

## In one sentence

Operator review fixes on the 3D doll: **lighter backdrop** (dark gear was unreadable on the
dark clear color), **drag turns the character** (not the camera — the old sign felt
inverted), and **hover + naming**: hovering a part lifts its tone and shows a cursor
tooltip with the part's name; the ACTIVE part carries a pinned name chip with a leader
line that tracks the anchor while rotating. Pitch/zoom explicitly declined; mesh fidelity
explained and deferred.

## Shipped shape

- **Core** (`doll/mod.rs`): `CLEAR_COLOR` → mid-light grey-blue `[0.52, 0.56, 0.63]`
  (unorm8 tie-safe 133/143/161 — the self-check background probe derives from the
  constant, no probe edits); `state_color(state, hovered)` — hover lifts ×1.22 clamped,
  tie-safe for all three states; region anchors: `anchor_world` (first instance's model
  translation) + `anchor_px` (projected via `view_proj_gl`, `None` behind camera).
  Tests: hover lift distinct per state; every region's anchor projects inside an 800×600
  viewport at yaw 0; anchor projection agrees with the `transform_vector` path to 1e-9 px.
- **Render**: `doll_pack::pack_instances(states, hover)` — hovered region colors through
  the lifted palette (native test: hover flip rewrites exactly the hovered region's
  colors). `DollEngine`: `rotate` sign **negated** (drag left → soldier turns left);
  `set_hover(region)` (no-op when unchanged, tiny repack + dirty); `anchor_px(region) ->
  Vec<f64>` (`[]` = hidden); self-check passes `hover = -1`.
- **TS** (`SoldierModel3D.tsx`, 264 LOC — growth is the allowed DOM-label lane):
  `catalogByName` prop for names; pointer-move (not dragging) → `pick_region` →
  `set_hover` + cursor tooltip chip "{Region} — {item | empty}"; drag start clears hover;
  active callout chip + 1px leader line positioned every frame in the existing rAF loop by
  direct style mutation from `engine.anchor_px(activeIdx)` (zero per-frame React renders;
  hidden when the anchor is behind the camera). `ArsenalTab` passes `catalogByName`.

## Gates

`make wasm-ci` exit 0 (core **73**: +hover/anchor/projection-agreement; render **29**:
+hover-flip) · `make wasm` → **4,217,176 B** (+1,104 over T-154.0) · vitest **358/358** ·
build + `tsc --noEmit` clean · lint = pre-existing `router.tsx` only ·
`verify-wgpu-gpu.mjs` exit 0 — **36/36 probes, allPass true** incl. `doll` with the new
background bytes · operator visual at the Mode D pause.
