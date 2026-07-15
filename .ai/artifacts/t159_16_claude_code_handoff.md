# T-159.16 — Claude Code handoff (MissionDoc host)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Branch:** `t-159-leptos-ui` · **Baseline:** `ebcabe1d` / tag `T-159.15.2`  
**Spec:** [`docs/platform/t159_16_mission_doc_host.md`](../../docs/platform/t159_16_mission_doc_host.md)

## Context

Pages done. MC camera: create → loop → wheel → pan. Next = own **MissionDoc** in Leptos so
later slices can persist, pick, and save. React oracle: `WasmMissionDoc` shell +
`useMissionDoc` attach/`apply_update` boot.

## File map (expected)

| Path | Action |
|------|--------|
| `apps/website-leptos/src/mission_doc.rs` (or similar) | Create — lifecycle shell |
| `apps/website-leptos/src/mission_editor.rs` | Mount/dispose doc |
| `crates/map-engine-*` | Only if needed to expose MissionDoc in same wasm |
| `.ai/artifacts/t159_gates/driver/smoke_doc_*.mjs` | New Class R smoke |
| `.ai/artifacts/t159_16_verify_log.md` | Create |

## Return

Tag **T-159.16** + verify log + ready for Cursor → **T-159.17**.
