# Agent commit checklist

**Use on every feature commit.** Sync docs **in the same commit** as code — never merge stale docs.

**Authority ladder:** running code → [`CLAUDE.md`](../CLAUDE.md) §Status → domain **ROADMAP.md** → supporting docs → archive.

---

## Before you code

| Domain | Start here |
|--------|------------|
| **Any frontend work** | [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) → surface spec in [`frontend/docs/INDEX.md`](../frontend/docs/INDEX.md) |
| **Mission Creator** | [`Design_Docs/Mission_Creator_Architecture/ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) → [`agent_execution.md`](../Design_Docs/Mission_Creator_Architecture/agent_execution.md) Decisions log for UX locks |
| **Backend / API** | [`docs/backend/ROADMAP.md`](backend/ROADMAP.md) |
| **Tag naming** | [`docs/TAGS.md`](TAGS.md) |

If a spec exists for the task (e.g. [`t048_library_create_dialog.md`](../Design_Docs/Mission_Creator_Architecture/t048_library_create_dialog.md)), read it before editing code.

---

## Same-commit sync table

| What changed | Update these |
|--------------|--------------|
| **Shipped milestone** | [`CLAUDE.md`](../CLAUDE.md) §Status — new **T-0xx** bullet; bump `"latest feature work"` line |
| **New or removed route** | [`frontend/src/router.tsx`](../frontend/src/router.tsx) + matching [`frontend/docs/pages/*.md`](../frontend/docs/pages/) + row in [`frontend/docs/INDEX.md`](../frontend/docs/INDEX.md) + [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) shipped table |
| **UI surface (no route change)** | Relevant page spec **Element Inventory** + **`Live source:`** path |
| **Nav / sidebar change** | [`frontend/src/config/navigation.ts`](../frontend/src/config/navigation.ts) + [`frontend/docs/shell/sidebar.md`](../frontend/docs/shell/sidebar.md) |
| **API / model change** | `internal/models/` JSON tags + matching `frontend/src/types/`; note handler if behavior changed |
| **Mission Creator UX lock** | [`agent_execution.md`](../Design_Docs/Mission_Creator_Architecture/agent_execution.md) **Decisions log** row |
| **Mission Creator new capability** | [`feature_inventory.md`](../Design_Docs/Mission_Creator_Architecture/feature_inventory.md) FEDS row |
| **Eden parity gap closed** | [`eden/gap_analysis.md`](../Design_Docs/Mission_Creator_Architecture/eden/gap_analysis.md) P-item + table row |
| **Frontend surface (MC route)** | [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) recently shipped + [`frontend/docs/pages/mission-editor.md`](../frontend/docs/pages/mission-editor.md) milestones |
| **New T-0xx tag** | [`docs/TAGS.md`](TAGS.md) row |
| **Deferred / blocked (not shipped)** | **FD-0xx** in [`frontend/docs/TRACKING.md`](../frontend/docs/TRACKING.md) or **BE-0xx** in backend ROADMAP — **never** T-0xx |
| **Doc-only reorg** | Own T-0xx commit; §Status note if authority changed |

---

## Mission Creator triggers (quick)

| Trigger | Doc |
|---------|-----|
| Layout / interaction / chrome decision | `agent_execution.md` Decisions log |
| New user-facing editor feature | `feature_inventory.md` |
| Engineering contract (schema, compiler, workers) | `engineering_plan.md` |
| Eden UI parity | `eden/gap_analysis.md` + maybe `eden/ui_anatomy.md` |
| Shipped git milestone | [`CLAUDE.md`](../CLAUDE.md) §Status + rows above (MC ROADMAP, frontend ROADMAP, TAGS, mission-editor, gap_analysis, feature_inventory, agent_execution as applicable) |

Shell phases PRE-3.5–9 are **done** (T-033–T-040). **Open work = the Eden P0–P2 backlog** ([`eden/gap_analysis.md`](../Design_Docs/Mission_Creator_Architecture/eden/gap_analysis.md)) per MC ROADMAP **§Current strategy** (Eden-first); Track A map tiles/DEM deferred until after.

### Mission Creator slice workflow

Eden slices ship as **T-053+** following the **spec → code → same-commit docs** pattern proven by T-048..T-055:

1. **Spec** — write or locate the `t0xx_*.md` spec under `Design_Docs/Mission_Creator_Architecture/` for the gap_analysis item.
2. **Code** — implement; `cd frontend && npm run build && npm run lint`.
3. **Docs (same commit)** — sync [`CLAUDE.md`](../CLAUDE.md) §Status + MC [`ROADMAP.md`](../Design_Docs/Mission_Creator_Architecture/ROADMAP.md) + [`docs/frontend/ROADMAP.md`](frontend/ROADMAP.md) + [`docs/TAGS.md`](TAGS.md) + relevant [`frontend/docs/pages/mission-editor.md`](../frontend/docs/pages/mission-editor.md) + the closed [`eden/gap_analysis.md`](../Design_Docs/Mission_Creator_Architecture/eden/gap_analysis.md) item + [`feature_inventory.md`](../Design_Docs/Mission_Creator_Architecture/feature_inventory.md) FEDS row + [`agent_execution.md`](../Design_Docs/Mission_Creator_Architecture/agent_execution.md) Decisions log.

---

## Never update

- `Design_Docs/**/code.html`, `screen.png` mockups (archive)
- `frontend/src/stitch-exports/**` (archive)
- `artifacts/eden-wiki/*.md` (generated — re-run scraper instead)
- Historical T-0xx bullets in CLAUDE (commit archaeology)

Live UI authority: `frontend/src/pages/` + `frontend/src/features/`.

---

## Verify before commit

```bash
cd frontend && npm run build && npm run lint
# Backend changes:
make test-it   # when API/DB touched
```

---

## Commit conventions

- Commit directly to **`main`** (no feature branches).
- Tag messages **T-0xx** at start.
- End with `Co-Authored-By:` trailer when using AI.
- **Do not commit** unless the user explicitly asks.

---

## Examples

**T-048 (library create dialog):** code + `mission-library.md` + sidebar.md + INDEX + both ROADMAPs + CLAUDE §Status + t048 spec status → `planned` → `shipped`.

**Internal refactor (no UX/API change):** code only; no doc updates unless `Live source:` path moves.

**Blocked registry work:** FD-0xx in TRACKING; do **not** add T-0xx until shipped.
