# T-069 — Claude Code handoff (Markers on map)

**Spec (wins on conflict):**
[`t069_markers_on_map.md`](../../docs/specs/Mission_Creator_Architecture/t069_markers_on_map.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
§W10 · **Working tree:** `tbd-reforger-wgpu-spike/` @ tag **T-151.9** — **never `main`**.

## LANGUAGE GATE (D5)

Rust owns marker geometry / GPU / pick. TypeScript = React + pointer + thin wasm calls.
Do **not** invent Deck layers or fat `*Controller` files. See
`.ai/tickets/CLAUDE_CODE_PROMPT.md` §T-151 language gate and
`.cursor/rules/no-silent-deferrals.mdc`.

## Operator note

T-151.9 retired Deck. W10 starts with **markers**. Minimum ship: **icon + line**. Other
`MapMarker` kinds only if they fit; otherwise list explicit deferrals in the verify log.

## CURRENT STATE

| Piece | Status |
|-------|--------|
| wgpu Mission Creator | Default (T-151.9) |
| `MapMarker` schema + `markers` map | Present; no UI/GPU yet |
| Slots select/move | Shipped (W6–W7) |
| Deck runtime | Deleted |

## What you are building

1. Yrs marker CRUD + bindings.
2. wgpu render + pick for shipped kinds.
3. Place/select/move/delete UX.
4. Tag **T-069** + verify log.

## Do not

- Edit docs/registry/CLAUDE (verify log OK).
- Reintroduce Deck.
- Grow `wgpuSlots.ts` past 60.
- Start T-070 / ruler / LoS.

## Key files

| Concern | Path |
|---------|------|
| Schema | `tactical-map/state/schema.ts` |
| Yrs / bindings | `tactical-map/state/ydoc.ts`, `bindings.ts` |
| wgpu mount | `WgpuTacticalMap.tsx`, `wgpu/*` |
| Select tool | `tools/useSelectTool.ts` |
| Palette stub | mission-creator RightInspector Markers tab |

## Gotchas

- Selection already allows `kind: 'marker'` — wire ids, do not redefine.
- Compiler/export must not drop `editor.markers` (or equivalent) on Save.
- Keep production Deck allowlist (3 oracle paths only).

## Return

- SHA + tag **T-069**
- `.ai/artifacts/t069_verify_log.md`
- **Ready for Cursor doc sync.**
