# T-127 — Claude Code handoff (MC UX)

**Ticket:** T-127 · **After:** T-126 shipped  
**Spec:** [`docs/platform/t127_mc_ux_audit_fixes.md`](../../docs/platform/t127_mc_ux_audit_fixes.md)

---

## U1 — Conflict server load (critical UX)

`resolveConflict('server')` in `useMissionEditor.ts`:

- Today: `hydrateMissionDoc(md, conflict)` under default origin — **IDB v2 only listens on LOCAL_ORIGIN** (`useMissionEditor.ts` persist hook).
- Fix: after hydrate, trigger LOCAL_ORIGIN meta flush OR call existing persist flush API so adopted server doc survives cold boot.
- Prefer chunked hydrate if conflict payload is large (reuse `hydrateMissionDocWithProgress` if available).

---

## U2 — Export errors

Wrap `compileMission` in try/catch; toast via existing MC error pattern; fix `exportJson` typing.

---

## U3 — Map view silent failure

`packages/map-assets/everon/` has **no** `tiles/map/` — manifest advertises it.

Until T-090.1.1: in `basemapView.ts` `read()`, never return `'map'` from localStorage (coerce to `'satellite'`). Optionally one-shot migration clearing bad key.

---

## U4 — Layer delete confirm

`EditorLayersSection.tsx` — confirm before `removeEditorLayer` when folder has child slots/subfolders.

---

## Return

Tag **T-127** · `.ai/artifacts/t127_verify_log.md`
