# T-092 — Claude Code handoff (spawn policy + mod compile)

**Ticket:** T-092 · **Branch:** `ticket/T-092` (both slices, sequential)  
**Worktree:** `.ai/artifacts/worktrees/TBD-T-092` (parallel with **T-090.1.1** on `main`)  
**Hub:** [`docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md`](../../docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md)

---

## Slice order (same worktree — do not split branches)

```text
T-092.1  mod spawn height + yaw + schema y     → tag T-092.1
  ↓
T-092.2  flatten + GET /compiled + mod path   → tag T-092.2
```

**T-092.0** (spawn contract docs) is **shipped** — read only, do not re-spec.

---

## T-092.1 — Mod spawn policy

**Spec:** [`t092_1_mod_spawn_policy.md`](../../docs/specs/Mission_Creator_Architecture/t092_1_mod_spawn_policy.md)

### What you ship

1. Optional `y` on `$defs/slot` in `packages/tbd-schema/schema/mission.schema.json` → **schemaVersion "1.2"** when present.
2. `TBD_MissionSlotStruct.c` — optional `float y`; JSON parse.
3. `TBD_SpawnManager.c` spawn policy:

```text
spawnY = slot.y if finite else GetSurfaceY(x, z)
spawnY += CAPSULE_GROUND_OFFSET_M   // measure in wb_play — NOT guessed
heading = slot.headingDeg
```

4. Log line: `[TBD][Spawn] slot=… Y=… jsonY=… surfaceY=… delta=… heading=…`
5. Warn if `|jsonY - GetSurfaceY| > MAX_Y_DELTA_M` (start 2.0 m).

### Key files

| File | Role |
|------|------|
| `packages/tbd-schema/schema/mission.schema.json` | Optional `y`, 1.2 bump |
| `packages/tbd-schema/golden-missions/*.json` | Must still validate without `y` |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c` | Parse `y` |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | Height + yaw policy |
| `.ai/artifacts/t092_1_verify_log.md` | M1–M4 + S1–S5 table |

### Verify (.1)

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
cd packages/tbd-schema && npm run validate
cd apps/website && go build ./...
# wb_play: 3 elevations + headingDeg M1–M4
```

**Out of scope @ .1:** compiler flatten, `/compiled` API, slot id format change.

---

## T-092.2 — Mod compile + /compiled API

**Spec:** [`t092_2_mod_compile_route.md`](../../docs/specs/Mission_Creator_Architecture/t092_2_mod_compile_route.md)  
**Blocked until:** T-092.1 tagged.

### What you ship

1. **`flattenEditorToModDocument(snapshot)`** in frontend compiler (or shared module) → native 1.1/1.2 document matching [`bridgehead-at-levie.json`](../../packages/tbd-schema/golden-missions/bridgehead-at-levie.json).
2. Deterministic slot id: `{faction}:{groupCallsign}:{role}:{index}` (align `flatten-orbat-slots.mjs`).
3. `assetId` ResourceName → `kit:` alias (registry / T-068 mapping — document table).
4. `orbat` **map** builder from editor factions/squads.
5. Go **`GET /api/v1/missions/:id/compiled`** — `RequireServiceToken` — body = mod document (**not** `buildMissionDoc` wrapper).
6. Fix `TBD_MissionLoader.c` → `/api/v1/missions/{id}/compiled`.
7. DEV_RUNBOOK curl example.

### Key files

| File | Role |
|------|------|
| `apps/website/frontend/src/features/mission-creator/compiler/compile.ts` | Flatten hook |
| `apps/website/internal/handlers/missions.go` (or new) | `/compiled` handler |
| `apps/website/internal/handlers/handlers.go` | Route registration |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | API path fix |
| `packages/tbd-schema/scripts/flatten-orbat-slots.mjs` | Id naming reference |
| `.ai/artifacts/t092_2_verify_log.md` | S1–S6 + API smoke |

### Verify (.2)

```bash
cd packages/tbd-schema && npm run validate
make test-it
cd apps/website/frontend && npm run build && npm run lint
curl -sS -H "X-Service-Token: $SERVICE_TOKEN" \
  http://localhost:8080/api/v1/missions/{id}/compiled | jq .schemaVersion
```

**Unblocks when ALL PASS:** T-071, T-068 Phase 2 resume path.

---

## Do not

| Forbidden | Why |
|-----------|-----|
| Edit `docs/**`, `.ai/tickets/registry.json`, CLAUDE markers | Cursor doc sync after merge |
| Touch `packages/map-assets/**`, basemap FE, `scripts/map-assets/` | **T-090.1.1** on `main` |
| Change Version POST `json_payload` shape (editor superset) | Locked — flatten is separate artifact |
| Ship `/compiled` before .1 spawn policy | Registry order |
| Guess Enfusion APIs | `tbd-dev-bootstrap.sh` + enfusion-mcp |

**Low merge risk:** `packages/tbd-schema/schema/mission.schema.json` if main also touches schema — rebase worktree onto `main` before merge.

---

## Preflight (worktree)

```bash
cd .ai/artifacts/worktrees/TBD-T-092
git fetch origin 2>/dev/null; git rebase main   # pick up T-090.1.1 setup commits
./scripts/ticket brief T-092
bash scripts/mod/tbd-dev-bootstrap.sh           # before .1 wb_play verify
```

---

## Return contract

| Slice | Tag | Artifacts |
|-------|-----|-----------|
| T-092.1 | **T-092.1** | `t092_1_verify_log.md` + capsule offset measurement note |
| T-092.2 | **T-092.2** | `t092_2_verify_log.md` + curl smoke output |

Merge **ticket/T-092** → `main` once both tagged. Cursor doc sync on `main` after merge.
