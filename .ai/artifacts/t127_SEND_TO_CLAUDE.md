# Send-off — T-127 (MC UX) ← **ONLY active work**

```bash
git pull
./scripts/ticket brief T-127
./scripts/ticket prompt T-127
```

| Doc | Path |
|-----|------|
| Handoff | [`.ai/artifacts/t127_claude_code_handoff.md`](t127_claude_code_handoff.md) |
| Spec | [`docs/platform/t127_mc_ux_audit_fixes.md`](../../docs/platform/t127_mc_ux_audit_fixes.md) |
| Living tracker | [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md) |
| Program | [`docs/platform/FABLE_5_AUDIT_PROGRAM.md`](../../docs/platform/FABLE_5_AUDIT_PROGRAM.md) |

**Prerequisite:** T-126 shipped @ `4a47688e` (tag **T-126**).

---

## Copy-paste prompt (same as `./scripts/ticket prompt T-127`)

```
Read CLAUDE.md first.

Implement **T-127** — Fable MC UX audit fixes (U1–U4, U5 stretch).

═══ PREFLIGHT ═══
  git pull
  ./scripts/ticket brief T-127

═══ READ ═══
  1. .ai/artifacts/t127_claude_code_handoff.md
  2. docs/platform/t127_mc_ux_audit_fixes.md
  3. .ai/artifacts/fable_5_omni_audit_report.md  (§2 Frontend, §4 UX — ACTIVE rows)
  4. features/mission-creator/hooks/useMissionEditor.ts
  5. features/mission-creator/persistence/{missionMetaStore,slotChunkStore}.ts
  6. features/tactical-map/state/basemapView.ts
  7. features/tactical-map/layers/useTerrainBasemapLayer.ts
  8. features/mission-creator/layout/LeftOutliner/EditorLayersSection.tsx

═══ PROBLEM ═══
  Fable audit: conflict "load server" doesn't persist to IDB; export fails silently;
  Map basemap preference renders empty grid; folder delete destroys subtree without confirm.

═══ SHIPPED (do not reopen) ═══
  - T-126 @ 4a47688e — security S1–S6
  - T-062.2 editor session · T-062.1 IDB v2 · T-060 conflict dialog shell

═══ LOCKED ═══
  - U1 IDB persist after server adopt required (saveMissionMetaFromDoc + saveSlotsFromDoc or flush)
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

**After ship:** tell Cursor **"doc sync for T-127"** → then Cursor ships **T-128**.
