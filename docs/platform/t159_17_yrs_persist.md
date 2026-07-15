# T-159.17 — yrsPersist (IDB) + warm editor session

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.16** @ `f2cd6178`

## Problem

MissionDoc lives in the Leptos editor but is **ephemeral** — reload loses the seed/local edits.
React durability is v3 whole-blob IDB (`yrsPersist.ts`, DB `tbd-mission-yrs`) + warm
`sessionStorage` marker (`editorSession.ts`) so same-tab return can skip a cold multi-MB GET.

## Locked decisions

| # | Decision |
|---|----------|
| P1 | Port **v3 contract**: DB name `tbd-mission-yrs`, store `doc-state`, key = mission id, value = `encode_state()` bytes. No v1/v2 migration. |
| P2 | Implement in Rust/`web-sys` or a small `idb` Rust crate — **not** npm `idb`. Same semantic as React: `saveState` / `loadState` / `clearState`. |
| P3 | **Debounced + serialized** writer (IDLE-gated; longer debounce OK); `getBytes` at write time; cancel if doc disposed. Flush on `visibilitychange` / `pagehide` when hidden. |
| P4 | On editor mount: `loadState(missionId)` → `apply_update` (INIT) if present, else keep .16 seed (or empty). Class R: save → reload page → encode matches prior (or slot_count + encode len stable). |
| P5 | **Warm session:** `sessionStorage` key `tbd-editor-session` — same JSON shape as React (`missionId`, `readyAt`, `slotCount`, `currentSemver`) + 24h TTL helpers. Mark ready after doc load; smoke may assert read/mark/clear. |
| P6 | Mission id for smoke: fixed fixture id (e.g. `t159-17-smoke`) when route has no `:id` yet, **or** wire `:id` from router if already available — document choice in verify log. |
| P7 | **Out of scope:** server hydrate / conflict GET, full mutator port, autosave dirty from UI edits (beyond debounce of encode after seed/load), outliner, save version POST, pick/marquee. |
| P8 | Keep 15.x–16 smokes green; no GpuTimer; no `unproject_xy`. |

## Do

1. `yrs_persist` module (load/save/clear + debounced writer + hide flush).
2. Wire into mission editor / mission_doc host boot.
3. `editor_session` warm marker helpers.
4. Smoke: persist round-trip across reload (or two navigations); warm session unit/smoke.
5. `.ai/artifacts/t159_17_verify_log.md` · tag **T-159.17**.

## Claude Code prompt — T-159.17 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.17** — yrsPersist (IDB) + warm editor session.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current
  git rev-parse --short HEAD  # expect f2cd6178 (T-159.16) or later

═══ READ ═══
  1. .ai/artifacts/t159_17_claude_code_handoff.md
  2. docs/platform/t159_17_yrs_persist.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_16_verify_log.md
  5. apps/website-leptos/src/mission_doc.rs
  6. apps/website-leptos/src/mission_editor.rs
  7. apps/website/frontend/src/features/mission-creator/persistence/yrsPersist.ts
  8. apps/website/frontend/src/features/mission-creator/hooks/editorSession.ts
  9. apps/website/frontend/src/features/mission-creator/hooks/useMissionDoc.ts

═══ PROBLEM ═══
  MissionDoc is in-memory only. Need React-parity v3 IDB whole-blob persist + warm
  sessionStorage marker so reload keeps local doc. Mutator port still deferred.

═══ SHIPPED ═══
  T-159.16 @ f2cd6178 — MissionDocCore host, smoke_doc_editor, seed 8 slots
  T-159.15.x camera stack

═══ LOCKED ═══
  - DB tbd-mission-yrs / doc-state / encode_state blob
  - Debounced serialized writer + hide flush
  - Warm session key tbd-editor-session, 24h TTL, React JSON shape
  - Class R: persist across reload
  - No server hydrate, full ydoc port, save POST, pick UI
  - Keep prior smokes; no GpuTimer / unproject_xy

═══ DO ═══
  1. yrs_persist + editor_session modules; wire boot
  2. Smoke persist round-trip (+ warm session check)
  3. .ai/artifacts/t159_17_verify_log.md
  4. Commit T-159.17: · tag T-159.17
     Co-Authored-By trailer: match prior worktree commits (Claude Opus harness style)

═══ DO NOT ═══
  - Edit docs/** or registry
  - Break smoke_doc_editor / pan / wheel / selfcheck
  - Port all mutators or Eden chrome

═══ VERIFY ═══
  Prior 15.x–16 smokes pass
  New persist smoke: reload → same encode/slot_count
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - SHA + tag T-159.17
  - verify log
  - Ready for Cursor → T-159.18 (select / LMB tools) or as hub directs
```
