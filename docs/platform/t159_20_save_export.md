# T-159.20 — Save Version + Export (compile from MissionDoc)

**Parent:** [`t159_leptos_ui_program.md`](t159_leptos_ui_program.md) · **Executor:** claude-code ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/` · **Baseline:** **T-159.19** @ `f444b878`

## Problem

Editor can edit/persist locally but cannot **Save Version** (POST) or **Export** download.
React: `compile.ts` / worker → `MissionPayload` → `buildVersionBlob` → POST
`/api/v1/missions/:id/versions` (save) or file download (export). Leptos needs a Rust-side
compile from `MissionDocCore` (+ auth client already used by pages).

## Locked decisions

| # | Decision |
|---|----------|
| E1 | Implement **compile** in Rust from `MissionDocCore` / SoA → JSON matching React `MissionPayload` shape (schema / `compile.ts` oracle). Prefer pure Rust in `website-leptos` or `map-engine-core` — **no** Comlink worker required for .20 if sync compile of seed-scale docs is fine; document if you add a worker later for 360k. |
| E2 | **Export:** build payload → trigger browser download (`mission-export.json` or React naming). Class R: exported JSON validates against schema or deep-equals React compile of same golden doc (fixture). |
| E3 | **Save Version:** POST body `{ semver, notes, json_payload }` (or exact React blob shape from `buildVersionBlob`) to `/api/v1/missions/:id/versions` with Bearer from existing auth store; handle 201 / 409 / 413. Use fixed smoke mission id + admin/dev-login if needed (document). |
| E4 | Minimal UI on editor: **Save** + **Export** controls (chromeless is fine — not full TopCommandStrip). |
| E5 | After successful save: flush IDB optional; mark dirty false if you track dirty. |
| E6 | **Out of scope:** full TopCommandStrip / undo UI / conflict dialog / 360k progress UX polish / compiled mod `/compiled` (T-068.11). |
| E7 | Keep all **7** editor smokes green; marquee smoke stays `?force=webgl`. |

## Do

1. Rust compile MissionDoc → payload (+ tests/goldens).
2. Export download + Save POST wired to `:id`.
3. Smoke(s): export Class R; save 201 against local API if available (or document BLOCKED + unit test body).
4. `.ai/artifacts/t159_20_verify_log.md` · tag **T-159.20**.

## Claude Code prompt — T-159.20 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at .ai/artifacts/worktrees/TBD-T-159/ (NOT main).

Implement **T-159.20** — Save Version + Export (compile from MissionDoc).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
  test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-159"
  git status --porcelain
  git branch --show-current
  git rev-parse --short HEAD  # expect f444b878 (T-159.19) or later

═══ READ ═══
  1. .ai/artifacts/t159_20_claude_code_handoff.md
  2. docs/platform/t159_20_save_export.md
  3. docs/platform/t159_leptos_ui_program.md
  4. .ai/artifacts/t159_19_verify_log.md
  5. apps/website-leptos/src/mission_doc.rs
  6. apps/website-leptos/src/mission_editor.rs
  7. apps/website/frontend/src/features/mission-creator/compiler/compile.ts
  8. apps/website/frontend/src/features/mission-creator/hooks/useMissionEditor.ts  # saveVersion / export
  9. packages/tbd-schema/schema/mission-editor-payload.schema.json (or equivalent)
  10. Existing Leptos auth / API client modules (pages)

═══ PROBLEM ═══
  Local edit+persist works; no Save Version POST or Export download from Leptos MissionDoc.

═══ SHIPPED ═══
  T-159.19 @ f444b878 — marquee/move + edit-driven persist
  T-159.18 select; .17 IDB; .16 doc host

═══ LOCKED ═══
  - Rust compile from MissionDocCore → MissionPayload parity with compile.ts
  - Export download + Save POST /missions/:id/versions with auth
  - Minimal Save/Export UI; not full TopCommandStrip
  - Keep 7 editor smokes; marquee gate ?force=webgl
  - No T-068.11 compiled mod / conflict UI / 360k worker required

═══ DO ═══
  1. Compile module + Export + Save
  2. Smoke/golden Class R (+ live POST if API up)
  3. .ai/artifacts/t159_20_verify_log.md
  4. Commit T-159.20: · tag T-159.20 (Opus harness trailer)

═══ DO NOT ═══
  - Edit docs/** or registry
  - Break editor-marquee-drag-smoke or other editor gates
  - Port full Eden chrome

═══ VERIFY ═══
  Prior 7 smokes pass
  Export/save gates green (or BLOCKED documented with unit Class R)
  cargo check wasm32 + trunk build --release

═══ RETURN ═══
  - SHA + tag T-159.20
  - verify log
  - Ready for Cursor → T-159.21 (Eden chrome / undo — hub will specify)
```
