# T-127 — Fable audit Mission Creator UX fixes

**Ticket:** T-127 · **Executor:** claude-code · **Status:** **ACTIVE** (T-126 shipped @ `4a47688e`)  
**Git tag on ship:** **T-127**  
**Handoff:** [`.ai/artifacts/t127_claude_code_handoff.md`](../../.ai/artifacts/t127_claude_code_handoff.md) · **Send-off:** [`.ai/artifacts/t127_SEND_TO_CLAUDE.md`](../../.ai/artifacts/t127_SEND_TO_CLAUDE.md)  
**Authority:** [`.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) §2 Frontend · §4 UX · [`FABLE_5_AUDIT_PROGRAM.md`](FABLE_5_AUDIT_PROGRAM.md)

---

## In one sentence

Fix **Mission Creator trust and feedback** gaps from Fable 5 — conflict server-load persistence, export errors, silent Map view, and destructive layer delete.

---

## Problem

| ID | Finding | Severity |
|----|---------|----------|
| U1 | `resolveConflict('server')` uses blocking `hydrateMissionDoc` + `INIT_ORIGIN` — no IDB persist → same conflict next boot | MED |
| U2 | `exportJson` async fire-and-forget — compile failure = unhandled rejection, no user feedback | MED |
| U3 | `basemapView==='map'` + no map tiles → silent grid-only canvas, no degraded toast | MED |
| U4 | `EditorLayersSection` folder delete — subtree wipe with no confirm (T-037 destructive op) | MED |
| U5 | `events.tsx` slot errors flattened — optional: surface backend 409 reason strings | LOW |

---

## Goal

1. **U1** — Server conflict resolution: use chunked/progress hydrate at scale OR ensure post-hydrate **LOCAL_ORIGIN flush** to v2 IDB; clear warm session marker appropriately; verify second boot does not re-prompt same conflict.
2. **U2** — `exportJson`: try/catch, toast or save-phase error state on compile failure; fix return type to `Promise<void>`.
3. **U3** — Until T-090.1.1: coerce persisted `'map'` → `'satellite'` on read **OR** call existing degraded basemap toast when map pyramid absent; `computeLod` must not silently return none without feedback.
4. **U4** — Confirm dialog (shared `Dialog`) before `removeEditorLayer` on folders with children; mention subtree count if cheap.
5. **U5** (stretch) — Map distinct ORBAT 409 messages in registration toast when backend error differs.

---

## Out of scope

- Building map tiles — **T-090.1.1**
- Full mission archive/delete — future
- Security fixes — **T-126**

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| U3 | Prefer coerce `'map'`→`'satellite'` in `basemapView.ts` read + one-time localStorage fix; toast if user explicitly picks Map in settings before T-090.1.1 ships |
| U1 | Must persist adopted server state to IDB — mirror save-after-hydrate pattern from load path |
| U4 | Use shared Aegis `Dialog`, not `window.confirm` |
| Executor | claude-code only |

---

## Tasks

1. `useMissionEditor.ts` — `resolveConflict('server')` IDB + hydrate path.
2. `useMissionEditor.ts` + `TopCommandStrip.tsx` — export error surfacing.
3. `basemapView.ts` + `useTerrainBasemapLayer.ts` — U3 coerce/toast.
4. `EditorLayersSection.tsx` — delete confirm.
5. `pages/events.tsx` — U5 if time.

---

## Verify

```bash
cd apps/website/frontend && npm run build && npm run lint
```

**Manual U1:** force conflict → Load server → reload tab → no repeat prompt.  
**Manual U3:** set localStorage `tbd-mc-basemap-view=map` → satellite or toast, not blank grid.  
**Manual U4:** delete folder with slots → confirm required.

---

## Claude Code prompt — T-127 (copy-paste)

Extract: `./scripts/ticket prompt T-127`

```
Read CLAUDE.md first.

Implement **T-127** — Fable MC UX audit fixes (U1–U4, U5 stretch).

═══ PREFLIGHT ═══
  git pull
  ./scripts/ticket brief T-127

═══ READ ═══
  1. .ai/artifacts/t127_claude_code_handoff.md
  2. docs/platform/t127_mc_ux_audit_fixes.md
  3. .ai/artifacts/fable_5_omni_audit_report.md  (§2 Frontend, §4 UX)
  4. features/mission-creator/hooks/useMissionEditor.ts
  5. features/tactical-map/state/basemapView.ts
  6. features/tactical-map/layers/useTerrainBasemapLayer.ts
  7. features/mission-creator/layout/LeftOutliner/EditorLayersSection.tsx

═══ PROBLEM ═══
  Fable audit: conflict "load server" doesn't persist to IDB; export fails silently;
  Map basemap preference renders empty grid; folder delete destroys subtree without confirm.

═══ SHIPPED (do not reopen) ═══
  - T-062.2 editor session · T-062.1 IDB v2 · T-060 conflict dialog shell

═══ LOCKED ═══
  - U1 IDB persist after server adopt required
  - U3 coerce map→satellite until T-090.1.1 (see basemapView comment)
  - U4 Aegis Dialog confirm
  - No docs/registry edits

═══ DO ═══
  1. U1 — resolveConflict server path + IDB persist + manual note
  2. U2 — exportJson error UX
  3. U3 — basemap coerce or degraded toast
  4. U4 — layer folder delete confirm
  5. U5 stretch — events 409 message mapping
  6. .ai/artifacts/t127_verify_log.md
  7. Tag **T-127** · prefix **T-127:**

═══ VERIFY ═══
  cd apps/website/frontend && npm run build && npm run lint

═══ RETURN ═══
  - SHA + tag T-127 · verify log · **Ready for Cursor doc sync.**
```
