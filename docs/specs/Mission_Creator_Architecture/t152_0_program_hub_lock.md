# T-152.0 — Program hub lock (docs)

**Ticket:** T-152 · **Slice:** T-152.0  
**Status:** **ready** (docs pass)  
**Executor:** cursor-docs  
**Authority:** [`t152_map_cartographic_fidelity_program.md`](t152_map_cartographic_fidelity_program.md)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · **Branch:** `ticket/T-152`

---

## In one sentence

Freeze the T-152 cartographic fidelity program hub, **all** slice specs **.0–.10**, registry rows, and advance gates so Grok 4.5 code slices do not guess agent split, worktree, or sequential policy.

---

## Problem

T-151 shipped wgpu world vectors + placeholder glyphs but **no program ticket** covers Reforger icon art, fence/pier/bridge fidelity, airfield symbology, height markers, or town/road labels. Without a locked hub, work would scatter into ad-hoc T-090.x slices or Claude `./scripts/ticket run` — wrong agent and wrong branch.

---

## Goal

1. Program hub [`t152_map_cartographic_fidelity_program.md`](t152_map_cartographic_fidelity_program.md) — agent split, problem table, slice ladder .0–.10, workbench matrix (plan-authoritative).
2. Slice specs **all authored in this pass:** `.0`–`.10`.
3. Registry **T-152** parent + `slice_plan` for **T-152.0–.10** with correct `spec` paths.
4. Cross-links: T-090 hub Related + ROADMAP pointer + `.ai/artifacts/t152_1_grok_code_handoff.md`.
5. `./scripts/ticket sync` + `./scripts/ticket check` PASS for T-152.

---

## Out of scope

- Application code, wasm, SVG art, Workbench plugins (start at T-152.1+)
- `./scripts/ticket run`

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **Implementing agent = Grok 4.5**; registry `executor: claude-code` on code slices is enum compatibility only | Operator directive |
| L2 | **Worktree** `TBD-T-152` / branch `ticket/T-152`; parallel with T-068 on `main` | Isolation |
| L3 | **Sequential advance**: every Gn PASS before next slice | No silent deferrals |
| L4 | Code prompts titled **§Grok Code prompt** with **LANGUAGE GATE** on wgpu slices | T-151 D5 |
| L5 | Placeholder glyphs **must** be replaced for `LANDMARK_SET` in .2 | P4 |
| L6 | **All** `.0–.10` specs land in the T-152.0 docs pass | Approved plan |

---

## Tasks (Cursor)

| # | Path | Action |
|---|------|--------|
| 1 | `t152_map_cartographic_fidelity_program.md` | Program hub |
| 2 | `t152_0` … `t152_10_*.md` | All eleven slice specs |
| 3 | `.ai/tickets/registry.json` | T-152 + slice_plan .0–.10 |
| 4 | `t090_091_map_terrain_program.md` Related | Link T-152 |
| 5 | `ROADMAP.md` | T-152 pointer |
| 6 | `.ai/artifacts/t152_1_grok_code_handoff.md` | First code handoff |
| 7 | Generated ticket views | Via `./scripts/ticket sync` only |

---

## Mathematical acceptance matrix

| ID | Predicate | Pass condition |
|----|-----------|----------------|
| **G1** | Hub exists | `test -f …/t152_map_cartographic_fidelity_program.md` |
| **G2** | All slice specs `.0`–`.10` exist | Eleven plan-authoritative filenames present |
| **G3** | Registry parent | `jq` selects `id=="T-152"` |
| **G4** | Slice plan complete | `slices` length = 11; every `slice_plan.*.spec` exists |
| **G5** | Ticket sync + check | `./scripts/ticket sync`; `./scripts/ticket check` exit 0 (fix T-147–149 hygiene if blocking) |
| **G6** | Agent split documented | Hub contains "Grok 4.5" and forbids `./scripts/ticket run` |
| **G7** | Cross-links | T-090 Related + ROADMAP mention T-152; handoff exists |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
HUB=docs/specs/Mission_Creator_Architecture
test -f $HUB/t152_map_cartographic_fidelity_program.md
for f in t152_0_program_hub_lock.md t152_1_wgpu_text_lane.md t152_2_reforger_icon_art.md \
  t152_3_wire_landmark_glyphs.md t152_4_fence_pier_bridge.md t152_5_airfield_symbology.md \
  t152_6_locations_export.md t152_7_height_markers.md t152_8_town_labels.md \
  t152_9_road_names.md t152_10_e2e_cartographic_gate.md; do test -f $HUB/$f || exit 1; done
jq -e '.tickets[] | select(.id=="T-152") | .slices | length == 11' .ai/tickets/registry.json
test -f .ai/artifacts/t152_1_grok_code_handoff.md
grep -q 'T-152' docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md
grep -q 'T-152' docs/specs/Mission_Creator_Architecture/ROADMAP.md
./scripts/ticket sync
./scripts/ticket check
```

---

## Manual checklist

| ID | Check | Pass |
|----|-------|------|
| M1 | Operator confirms Grok 4.5 owns code (not Claude Code) | ☐ |
| M2 | Worktree path matches hub absolute path | ☐ |

---

## Documentation sync

After G1–G7 PASS: registry `T-152.0 → shipped`; hub **Active slice → T-152.1**; commit prefix **`T-152.0:`** · tag **`T-152.0`** (doc-only).

---

## Related

- [`t152_map_cartographic_fidelity_program.md`](t152_map_cartographic_fidelity_program.md)
- [`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md)
- [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md)
