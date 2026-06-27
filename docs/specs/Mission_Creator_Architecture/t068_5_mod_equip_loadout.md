# T-068.5 — Mod equip from loadout-export JSON

**Ticket:** T-068 · **Slice:** T-068.5  
**Status:** **shipped** @ `21ec91e` (git tag **T-068.5**)  
**Executor:** claude-code (**enfusion-mcp required** for compile/reload/play verify)  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Mod reads `$profile:TBD_LoadoutTest.json` and equips exact ResourceNames on a **non-player test NPC** @ 6400/6400 — **not** the joining human player.

---

## Phase 1 boundary (critical)

| Receives web-export kit | Does **not** |
|-------------------------|--------------|
| **`TBD_LoadoutEquipComponent`** — spawns `Character_US_Base` test **NPC/AI entity** @ game-mode coords and dresses it from profile JSON | **Human player** — `TBD_SpawnManager.DeployPlayer` uses mission slot **kit aliases** (`TBD_Registry.Resolve`), not `TBD_LoadoutTest.json` |

Visual wear on the test NPC is proven @ **T-068.5.1** (`b233b11`). Player/slot loadout from the editor = **T-068.11**.

---

## Shipped @ T-068.5

**Mod path:** [`TBD_LoadoutEquipComponent.c`](../../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c) wired on [`TBD_GameMode.et`](../../../apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et).

| Piece | Detail |
|-------|--------|
| Input | `$profile:TBD_LoadoutTest.json` (manual copy of web `loadout-export.json`) |
| Subject | **Non-player test NPC** (not DeployPlayer / not human player) |
| Base body | `{520EC961A090BBD5}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Base.et` (empty US — no baked kit) |
| Spawn | `@ 6400/6400` (TBD_Dev_POC game mode coords; Y from `GetSurfaceY`) |
| Clothing equip (scaffold) | `TryInsertItem(PURPOSE_LOADOUT_PROXY)` — **false OK / naked mesh**; fixed in **T-068.5.1** |
| Primary equip (scaffold) | `EquipWeapon(item)` — fixed in **T-068.5.1** with worn-verify |
| Wear fix | **T-068.5.1** @ `b233b11` — `EquipCloth` + deferred `GetClothFromArea` / `GetCurrentWeapon` |
| Logs | `[TBD][Loadout] …` — full `{GUID}…et` strings, no `kit:` aliases |

**Workbench ops note:** a brand-new `.c` file requires **Workbench cold restart** (not just `wb_reload`) before the class registers.

---

## Problem

Downloaded loadout JSON had no in-game consumer.

---

## Goal

1. Enfusion component reads JSON matching `loadout-export.schema.json`.
2. On dev `wb_play`: apply primary/uniform/vest/helmet via exact `ResourceName` APIs — no alias layer.
3. Log each equip success/failure to console.
4. Document profile path in mod README / this spec.

---

## Out of scope

- Mission `json_payload` compiler path (T-068.11)
- Compat validation
- Equipping onto **human player** or mission-slot **`DeployPlayer`** bodies (test NPC @ 6400 only in Phase 1)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Phase 1 input file | `$profile:TBD_LoadoutTest.json` (manual copy of T-068.4 download) |
| Identity | Full ResourceName strings only |
| Host | `TBD_LoadoutEquipComponent` on `TBD_GameMode.et` |
| Base character | `Character_US_Base.et` @ 6400 (empty US body for A7 proof) |
| Canonical test JSON | T-068.4 verify artifact — four slots, no nulls |

---

## Verify

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
bash scripts/mod/mcp-call.sh wb_connect '{}'

# Canonical JSON (T-068.4 verify artifact):
cp /tmp/loadout-export.json "$PROFILE/TBD_LoadoutTest.json"
# Workbench $profile: → Proton pfx …/ArmaReforgerWorkbench/profile/ (paste exact path in verify)

bash scripts/mod/mcp-call.sh wb_reload '{"scope":"scripts"}'
bash scripts/mod/mcp-call.sh wb_play '{}'
sleep 5
bash scripts/mod/mcp-wb-logs.sh | grep -E '\[TBD\].*Loadout|Loadout equip' | tail -20
bash scripts/mod/mcp-call.sh wb_stop '{}'
sha256sum "$PROFILE/TBD_LoadoutTest.json"
```

---

## Verification gate (mandatory)

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | File read | Console log contains `Loadout`/`TBD_LoadoutTest` loaded — **no** parse error |
| A2 | Primary | Log line equipping **primary** ResourceName from JSON — **OK** |
| A3 | Uniform | Log line for **uniform** — **OK** |
| A4 | Vest | Log line for **vest** — **OK** |
| A5 | Helmet | Log line for **helmet** — **OK** (canonical JSON has all four) |
| A6 | No alias | Logged strings contain `{GUID}` form — **not** `kit:` aliases |
| A7 | Visual | **Test NPC** @ ~6400/6400 shows worn kit — **screenshot required** (T-068.5.1). **Not** the human player. |

**Verified @ `21ec91e`:** A1–A6 PASS via MCP `wb_play` (log-only — **A7 false pass**, naked mesh).

**Verified @ `b233b11` (T-068.5.1):** A0–A9 PASS including visual screenshot on test NPC.

---

## Depends on / Unblocks

- **Depends on:** T-068.0.1, T-068.4
- **Unblocks:** T-068.6 (shipped @ 2026-06-27)

---

## Documentation sync (Cursor)

**Done @ T-068.5 ship (`21ec91e`)** and **T-068.5.1 / T-068.6 doc pass** @ 2026-06-27: mod README, `CLAUDE-CODE-START.md`, program hub, MC ROADMAP, E11 screenshot gate, `active_slice` → **T-068.7**.

---

## Next slice

**T-068.7** — compat matrix spec ([`t068_7_compat_matrix_spec.md`](t068_7_compat_matrix_spec.md), cursor-docs). Phase 1 E2E gate **T-068.6 PASS** @ 2026-06-27.
