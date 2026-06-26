# T-068.6 — Phase 1 E2E human gate

**Ticket:** T-068 · **Slice:** T-068.6  
**Status:** Spec ready — code pending  
**Executor:** human  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Manual golden path proves dumb Virtual Arsenal works web → file → mod before Phase 2 starts.

---

## Problem

No signed-off proof that registry + palette + loadout export + mod equip chain works end-to-end.

---

## Goal

Human runs checklist with **objective PASS/FAIL per row**; signs off Phase 1 only when **100% PASS**. Registry stays `ready` (do **not** `./scripts/ticket done T-068`).

Optional: git tag commit note "T-068 Phase 1" — full **T-068** tag @ T-068.11 only.

---

## Preconditions (hard gate)

Before starting T-068.6, confirm **verify paste blocks exist** in Docs & Tickets chat for:

- T-068.0.1, T-068.2, T-068.3, T-068.4, T-068.5 (required)
- T-068.1 (required if Workbench export used in E2E; optional if dev seed only — state which in sign-off)

If any required slice lacks a PASS verify paste → **do not start T-068.6**.

Stack:

```bash
make db-up && make api && make web
curl -sf http://localhost:8080/healthz | jq -e '.status == "ok"'
```

Dev-login: `http://localhost:8080/api/v1/auth/dev-login?role=mission_maker`

---

## E2E checklist (all must PASS)

| ID | Step | Pass condition | Evidence required |
|----|------|----------------|-----------------|
| E1 | Editor boot | `/missions/:id/edit` loads; no error overlay | Screenshot or URL + load time |
| E2 | Registry network | DevTools Network: `GET /api/v1/registry` **200** | Status + response byte size |
| E3 | Factions tree | NATO + CSAT from API; not static mock | Tree root labels pasted |
| E4 | Search `medic` | Exactly one visible Medic row path | Pasted filter string + visible label |
| E5 | Search `nato` | NATO subtree visible | Pasted observation |
| E6 | Drag place | Character placed on map | OBJ count +1 |
| E7 | `assetId` | Placed slot uses GUID `resource_name` (paste value) | Exact string |
| E8 | Arsenal tab | **Stub gone** — 4 **enabled** dropdowns + **enabled** download (not “Loadout Forge soon”) | Screenshot showing dropdowns + no stub copy |
| E9 | Download | `/tmp/loadout-export.json` passes jq gate from T-068.4 spec | jq command outputs |
| E10 | Profile copy | `TBD_LoadoutTest.json` at documented profile path | `ls -la` + `sha256sum` |
| E11 | Mod spawn | Dev server console shows 4 equip OK lines (T-068.5 A2–A5) | Log excerpt |
| E12 | Perf smoke | Pan/zoom 10s on mission with ≥200 slots (or largest available) — no freeze | FPS counter ≥55 or subjective "no stall" with slot count noted |

---

## Verification gate (mandatory)

### Sign-off format (paste to Cursor)

```markdown
## T-068.6 verify — PASS | FAIL
**Phase 1 E2E gate**
**Date:**
**Data source:** dev seed only | workbench import (state which)

### Checklist
| ID | Result | Evidence |
|----|--------|----------|
| E1 | PASS | … |
… E12 … |

**Phase 2 approved to start:** YES | NO
```

**Advance to T-068.7 only when:** header says **PASS**, E1–E12 all **PASS**, and **Phase 2 approved: YES**.

---

## Out of scope

- Phase 2 worker/UI
- `./scripts/ticket done T-068`

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Gate | Zero-fail checklist; no waivers without new ticket |
| Ticket status | Remains `ready` through Phase 2 |

---

## Depends on / Unblocks

- **Depends on:** T-068.1–T-068.5 verify pastes + E1–E12
- **Unblocks:** T-068.7 (after approval)

---

## Documentation sync (Cursor)

After sign-off paste: program hub Phase 1 acceptance note; optional CLAUDE §Status Phase 1 milestone line.
