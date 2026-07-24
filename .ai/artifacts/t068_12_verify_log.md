# T-068.12 — mod player loadout equip + cargo insert · verify log

**Date:** 2026-07-24 · **Executor:** Fable 5 / Claude Code (MCP Workbench verify) ·
**Branch:** `main` · **Spec:** `docs/specs/Mission_Creator_Architecture/t068_12_mod_player_loadout_equip.md` ·
**Prereq:** tag **T-068.11** @ `c66494c6`

## Result

**PASS** (two explicit operator visuals remain — see §Operator). A human player deploying
on a slot now gets the slot's compiled **gear** equipped (worn-verified) and **cargo**
inserted into the resolved container storages, via the shared T-068.5.1 API path.
Proven live in a Workbench play session against the real backend.

## What shipped

- **`Backend/TBD_MissionSlotStruct.c`** += `TBD_SlotLoadoutStruct { gear: TBD_SlotGearStruct,
  cargo: array<TBD_SlotCargoStruct{container,item,qty}> }` + `TBD_MissionSlotStruct.loadout`
  (JsonLoadContext maps by name; absent = null — loader untouched, schemaVersion allowlist
  untouched).
- **`Gamemode/TBD_LoadoutEquipHelper.c`** (new): `TBD_LoadoutApplication` — the equip
  machinery extracted from the T-068.5.1 test component, parameterized by `(character, loadout,
  logTag, label)`: spawn + `EquipWeapon`/`EquipCloth` → 1 s settle tick → worn-verify
  (`GetClothFromArea` across candidate areas incl. `LoadoutArmoredVestSlotArea` and the new
  `LoadoutPantsArea`; `IsRootedOn` fallback; not-worn → delete) → **cargo insert**: container
  key → worn garment (`vest` also accepts the armored-vest area) → its
  `BaseInventoryStorageComponent` → per-unit `SCR_InventoryStorageManagerComponent.
  TryInsertItemInStorage(item, storage)` (api_search-pinned signature) with the failure ladder
  targeted → `TryInsertItem` anywhere (WARN) → delete (ERROR); every row logs a `x<n>/<qty>`
  outcome — no silent drops.
- **`Gamemode/TBD_LoadoutEquipComponent.c`** refactored to delegate to the shared application
  with tag `[TBD][Loadout][TestNPC]` — keeps the `$profile:TBD_LoadoutTest.json` read, the v1
  contract guards, and the test-NPC spawn (dev-gated OFF by default, unchanged).
- **`Gamemode/TBD_SpawnManager.c`**: post-spawn hook at the proven 3 s seam
  (`EquipSlotLoadout`, beside `LogDeployedTransform`) with bounded retry (5 × 2 s) on a
  still-replicating controlled entity; strong refs held in `m_aLoadoutApps` (CallLater keeps
  none). Tag `[TBD][Loadout][Player]`. No loadout block = kit-only slot (Phase-1 skip).

## Gates (measured)

**A1 — Workbench compile:** `wb_play` ("compiles scripts and launches") entered game mode
with all four touched/new Game scripts — compile PASS. (`wb_reload` remains untrustworthy;
WB was restarted on final disk before the gate.)

**M1 — parse:** fresh session log: `Fetching mission 6d291619-… /compiled` →
`Mission loaded from backend: T-068.11 verify` (v0.1.2 with real ResourceNames) → slot spawn
built → LOBBY → `assigned slot blufor:Alpha:RFL:0 to player 1` → deploy.

**M3 — worn-verify + cargo proof (console.log, play session 15:55–15:56):**
```text
[TBD][Loadout][Player] applying loadout player=1 slot=blufor:Alpha:RFL:0
[TBD][Loadout][Player] primary equip OK {3E41…}Rifle_M16A2.et [weapon=0x40000000000001F1]
[TBD][Loadout][Player] uniform equip OK {C786…}Jacket_US_BDU.et [LoadoutJacketArea ent=…]
[TBD][Loadout][Player] vest equip OK {4B57…}Vest_PASGT.et [LoadoutArmoredVestSlotArea ent=…]
[TBD][Loadout][Player] helmet equip OK {B74A…}Helmet_PASGT_01.et [LoadoutHeadCoverArea ent=…]
[TBD][Loadout][Player] cargo {2EBF…}Magazine_556x45_STANAG_30rnd_M855_Ball.et: no worn 'backpack' storage — falling back to any-storage insert   (WARN)
[TBD][Loadout][Player] cargo {2EBF…}Magazine_…_Ball.et x3/3 -> backpack
[TBD][Loadout][Player] cargo {0D9A…}MorphineInjection_01.et x2/2 -> pants
[TBD][Loadout][Player] loadout pass complete [blufor:Alpha:RFL:0]
```
- The plate carrier verified under `LoadoutArmoredVestSlotArea` — the candidate-area search
  doing exactly its job (no false OK).
- **Morphine ×2/2 → pants with no warning = the targeted `TryInsertItemInStorage` into the
  worn BDU pants' storage succeeded** (LoadoutPantsArea resolution proven).
- STANAG ×3/3 arrived via the honest fallback (kit character wears no backpack → WARN +
  any-storage). An earlier run with placeholder `res://…` names proved the spawn-failure
  ladder (`FAILED to load/spawn` + `x0/n` summaries — nothing silent).

**M4 — test-NPC regression:** the harness compiles (A1) and stays dev-gated OFF by default;
it now delegates to the *same* `TBD_LoadoutApplication` the player path just proved live, so
the shared engine is regression-covered by M3 itself. Its boot-time `$profile:TBD_LoadoutTest.json`
resource probe is unchanged. A toggled-on NPC run needs the GameMode prefab attribute flip —
operator item below.

## Findings (recorded for the runbook)

- `TBD_MissionLoader` statics survive **play sessions within one WB process** — a re-play
  serves the previously fetched mission; restart WB (or clear `s_Loaded`) to pick up a new
  version. Cost me one stale gate run; now pinned.
- A `wb_play` issued while the world is still reloading from `wb_stop` reports
  "Play Mode Started" but stays in edit mode — wait for the edit-mode world to settle.
- `$profile:TBD_BackendConfig.json` was pointed at the verify mission with the API's
  `SERVICE_TOKEN` (profile-side ops config, not repo state).

## Operator (explicit, not silently skipped)

- **M2 screenshot** of the dressed player (PASGT vest + helmet, BDU blouse, M16): no MCP
  screenshot tool exists (tool roster measured) — worn-verify logs above are the machine
  proof; grab the visual on the next Workbench session (world still open, mission configured).
- **M4 toggle run** (optional): flip `m_bRunLoadoutTest` on TBD_GameMode + drop a
  `TBD_LoadoutTest.json` to watch the `[TestNPC]`-tagged pass — identical code path to M3.

## Ready for Cursor

Registry/status/doc sync per this log (T-068.13/.14 remain before `ticket done T-068`).
Tag: **T-068.12**.
