# T-151.9 — Claude Code handoff (Deck flip + retirement)

**Spec (wins on conflict):**
[`t151_9_deck_retirement.md`](../../docs/specs/Mission_Creator_Architecture/t151_9_deck_retirement.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** `tbd-reforger-wgpu-spike/` @ `ec59d10e` (tag **T-151.8.1**) — **never `main`**.

## LANGUAGE GATE (D5)

Rust already owns cull / density / slots / camera. This slice is mostly **deletion + default
flip**. Do **not** invent new TS engine controllers. See
`.ai/tickets/CLAUDE_CODE_PROMPT.md` §T-151 language gate and
`.cursor/rules/no-silent-deferrals.mdc`.

## Operator note

W8 + W8.1 shipped cull/ladder/compute. Next is the F4 analog: **wgpu default**, then **delete
Deck runtime**. Do not ship “flip only / delete later” unless the human explicitly defers.

## CURRENT STATE

| Piece | Status |
|-------|--------|
| `WgpuTacticalMap` | Production path behind `VITE_MC_ENGINE=wgpu` / `?engine=wgpu` |
| `TacticalMap` (Deck) | Still default when flag unset |
| World/slots/cull | Rust + thin TS |
| Deck layers / workers | Still in tree — **delete this slice** |
| deck.gl npm | **dependencies** — move to **devDependencies** (oracle) |

## What you are building

1. Default Mission Creator → wgpu.
2. Delete Deck runtime modules + workers; retarget tests.
3. Bundle delta + E2E notes in verify log.
4. Tag **T-151.9**.

## Do not

- Edit docs/registry/CLAUDE (verify log OK).
- Defer Deck deletion after flip without user say-so.
- Grow `wgpuSlots.ts` past 60 / add fat controllers.
- Start W10 (T-069/T-070/…).

## Key files

| Concern | Path |
|---------|------|
| Engine flag | `features/mission-creator/MissionCreatorPage.tsx` |
| Deck mount | `features/tactical-map/TacticalMap.tsx` |
| wgpu mount | `features/tactical-map/WgpuTacticalMap.tsx` |
| Deck layers | `features/tactical-map/layers/*` |
| World Deck | `features/tactical-map/worldmap/useWorldMapLayers.ts`, `*Layer.ts` |
| Workers | `worldObjects.worker` (+ related) |
| Thin slots | `wgpu/wgpuSlots.ts` |

## Gotchas

- Camera/ortho parity tests may still import Deck math — keep those under **dev** oracle paths.
- Non-editor pages must not break (route-table smoke).
- Sticky empty mid-hydration / world loader behavior must survive (W4.1 lessons).

## Return

- SHA + tag **T-151.9**
- `.ai/artifacts/t151_9_verify_log.md`
- **Ready for Cursor doc sync.**
