# T-092 — Post-merge verify handoff (Claude Code on `main`)

**Merged:** `ticket/T-092` → `main` @ **`a73224f2`** (rebased onto T-090.1.1 `6e06e679`)  
**Slices:** T-092.1 spawn policy · T-092.2 flatten + `GET /api/v1/missions/:id/compiled`  
**Tags:** not yet — tag **T-092.1** / **T-092.2** only after this verify pass (or PENDING wb_play accepted)

---

## Your job

Run **automated gates** on repo root **`main`**. Then **Workbench MCP** spawn verify (now safe — mod code is on `main` gproj). Update verify logs. Tag if PASS (or PENDING wb_play with operator sign-off).

---

## 1. Automated (no Workbench)

```bash
cd /home/Samuel/Projects/TBD-Reforger

make db-up
cd packages/tbd-schema && npm run validate

make test-it

cd apps/website/frontend && npm run build && npm run lint && npm test
```

**API smoke** (`make api` in another terminal, or use running API):

```bash
# dev-login in browser OR curl dev-login; save a mission with slots in MC first
curl -sS -H "X-Service-Token: $SERVICE_TOKEN" \
  "http://localhost:8080/api/v1/missions/{MISSION_ID}/compiled" | jq .schemaVersion,.slots|length

# expect 401 without token:
curl -sS -o /dev/null -w "%{http_code}\n" \
  "http://localhost:8080/api/v1/missions/{MISSION_ID}/compiled"
```

Note: first **Save Version** after create must bump semver (auto-seeded 0.1.0).

Log results in:
- `.ai/artifacts/t092_1_verify_log.md` (update PENDING → PASS where applicable)
- `.ai/artifacts/t092_2_verify_log.md`

---

## 2. Workbench MCP (spawn + capsule offset)

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
bash scripts/mod/tbd-spawn-verify.sh
bash scripts/mod/mcp-wb-logs.sh '\[TBD\]\[Spawn\]'
```

**T-092.1 M1–M4:** feet-on-ground, headingDeg, measure **`CAPSULE_GROUND_OFFSET_M`** (replace `0.0` placeholder in `TBD_SpawnManager.c` if needed — one commit, re-run).

If Workbench unreachable: leave rows **PENDING**; still tag if operator accepts automated-only ship.

---

## 3. Tag + return

```bash
git tag T-092.1 <commit-for-.1>   # first slice commit on main after merge base, or b46e8020 equivalent
git tag T-092.2 a73224f2          # merge tip if .2 is tip
```

Return: SHAs, tags, updated verify logs — **"Ready for Cursor doc sync T-092"**.

**Do not edit** `docs/**`, `.ai/tickets/registry.json`, CLAUDE status — Cursor owns doc sync after tags.

---

## Key files (already on `main`)

| Area | Path |
|------|------|
| Schema 1.2 + `y` | `packages/tbd-schema/schema/mission.schema.json` |
| Mod spawn | `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` |
| Mod loader | `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` |
| Flatten TS | `apps/website/frontend/.../compiler/flattenModDocument.ts` |
| Flatten Go | `apps/website/internal/services/mission_compile.go` |
| Route | `apps/website/internal/handlers/missions_compiled.go` |
| Kit aliases | `packages/tbd-schema/registry/kit-aliases.json` |

Spec hub: `docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md`
