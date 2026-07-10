# T-150 verify log — Universal Enfusion registry + compat export

**Date:** 2026-07-10 · **Executor:** Claude Code (Fable 5) · **Branch:** `main` @ repo root ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t150_universal_registry_export.md`

## Result

**PASS** — universal scanner shipped; curated allowlist retired. Vanilla (+TBD framework)
Workbench session export:

- **Items: 1,880** across 3 loaded addons (`core` 0 · `ArmaReforger` 1,880 · `TBD_Framework` 0 —
  its single prefab `TBD_GameMode.et` carries no item signal, correctly skipped).
- **Edges: 4,012** — `character_default_loadout` 2,746 · `mag_in_weapon` 545 ·
  `optic_on_weapon` 362 · `attachment_on_weapon` 241 · `mag_in_vehicle_weapon` 118.
- Kind histogram (post-reclassification, from committed file): character 354, crate 276,
  **other 252**, vehicle 218, magazine 127, gear_primary 125, ammo 101, gear_uniform 100,
  gear_helmet 92, vehicle_weapon 77, gear_vest 46, gear_backpack 43, optic 30, attachment 26,
  gear_launcher 9, gear_handgun 4.
- Scan stats: seen 15,320 `.et` · skippedDeny 7,254 · noSignal 6,176 · failedLoad 10 ·
  droppedEndpoints 0 · **elapsed 717 ms**.
- Deterministic: two runs of the final build produce byte-identical envelopes modulo
  `generatedAt`.
- `npm run validate` ALL PASS incl. new strict referential-integrity check over all 4,012 edges.

Commits: schema v2 `e358777a` · mod scanner + workbench envelopes + this log = the **T-150**-tagged commit.

## Phase 1 — API spike (live, enfusion-mcp)

MCP preflight: `tbd-dev-bootstrap.sh` → `wb_connect` OK → `mod_validate` 5/5 passed
(gproj/prefabs/configs/references/naming; EnfusionMCP handler-path warnings pre-existing).

### Enumeration APIs (all live-confirmed via `api_search`)

| API | Signature (live api_search) | Status |
|---|---|---|
| `Workbench.SearchResources` | `static proto bool SearchResources(WorkbenchSearchResourcesCallback callback, array<string> fileExtensions=null, array<string> searchStrArray=null, string rootPath=string.Empty, bool recursive=true)` — "rootPath must be in 'exact path' format e.g. `$addonName:Prefabs`" | PROVEN |
| `GameProject.GetLoadedAddons` | `static proto void GetLoadedAddons(notnull out array<string> addonGUIDs)` | PROVEN |
| `GameProject.GetAddonID/GetAddonTitle/IsVanillaAddon` | statics on `GameProject` (System group), GUID-keyed | PROVEN |
| Fallback `ResourceDatabase.SearchResources` | `static proto bool SearchResources(SearchResourcesFilter filter, SearchResourcesCallback callback)`; filter props `rootPath/fileExtensions/searchStr/tags/recursive` | PROVEN (unused rung) |

### Container var names (vanilla, via `game_read` of pak prefabs)

| Family | Evidence (vanilla prefab) | Vars |
|---|---|---|
| weapon → magwell | `Prefabs/Weapons/Rifles/M16/Rifle_M16A2_base.et` | `MuzzleComponent { MagazineWell MagazineWellStanag556 {} , MagazineTemplate "{D8F2CA92583B23D3}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_M856_Last_5Tracer.et" }` |
| weapon → attachment slots | same file | `AttachmentSlotComponent { AttachmentSlot InventoryStorageSlot optics {...}, AttachmentType AttachmentOpticsCarryHandle {} }` (+ Handguard `AttachmentHandGuardM16`, Bayonet `AttachmentBayonetM9`, Muzzle `AttachmentMuzzle556_45` nested inside `MuzzleComponent.components`) |
| magazine → magwell | `Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_Base.et` | `MagazineComponent { MagazineWell MagazineWellStanag556 {}, MaxAmmo 30, AmmoConfig "{...}Configs/Weapons/Ammo/Ammo_556x45.conf" }` |
| attachment item → type | `Prefabs/Weapons/Attachments/Optics/Optic_1P29/Optic_1P29_base.et` | `InventoryItemComponent { Attributes SCR_ItemAttributeCollection { ItemDisplayName UIInfo { Name "#AR-Item_1P29_Name" }, CustomAttributes { WeaponAttachmentAttributes { AttachmentType AttachmentOpticsDovetailAK {} } } } }` + `SCR_2DPIPSightsComponent` |
| cloth → area | `Prefabs/Characters/Vests/Vest_PASGT/Vest_PASGT_base.et` (+ CRF corpus) | `BaseLoadoutClothComponent { AreaType Loadout*Area {} }` — AreaType often lives on an ANCESTOR copy → ancestry walk mandatory |
| vehicle → weapon chain | `M1025_armed_M2HB.et` → roof → gun mount | `SlotManagerComponent.Slots[RegisteringComponentSlotInfo Roof].Prefab → VehParts/M1025_roof_M2HB.et → Slots[Turret].Prefab → VehParts/M1025_gun_mount_M2HB.et → WeaponSlotComponent { WeaponTemplate "{E517E6CCC1DF5737}Prefabs/Weapons/HeavyWeapons/HMG_M2HB_pintle_M1025.et" }` |
| character → default loadout | `Character_US_Rifleman.et` | `BaseLoadoutManagerComponent { Slots { LoadoutSlotInfo ArmoredVest { Prefab "{4B57C11AA5161760}...Vest_PASGT.et" } ... } }`; root class `SCR_ChimeraCharacter` |

Offline cross-check: 775 text `.et` prefabs in `apps/mod/crf_framework` (reference-only) show the
same var shapes (`MagazineTemplate` ×52 files, `AttachmentSlotComponent` ×89, `WeaponTemplate`
×25, `LoadoutManagerComponent` ×47, `AreaType` ×17).

### Compile/run loop

- `wb_reload {"target":"scripts"}` does **not** recompile (known: `t092_1_verify_log.md`).
  Automated compile path = restart Workbench with the project:
  `steam -applaunch 1874910 -gproj 'Z:\var\home\Samuel\Projects\TBD-Reforger\apps\mod\tbd-framework\addon.gproj'`
  (`-gproj` skips the project-picker — BIKI Startup Parameters; picker-stuck launches never open
  the Net API port). Fresh launch compiles all addon scripts and registers plugin classes.
- Remote trigger proven: `wb_execute_action {"menuPath":"Plugins,TBD,Export TBD Registry Items"}`
  → old curated plugin ran ("Wrote 21 items"); flat `Plugins,<name>` form returns false.

## Phase 2 — Schema v2 (commit 1)

- `registry-items.schema.json`: kind enum → 16 locked v2 values; optional `addons[]`;
  `resource_name` pattern widened to allow spaces/parens/apostrophes in paths (real mod dirs like
  `Machine Guns/` exist — old pattern would reject them).
- NEW `registry-compat.schema.json` (envelope v1): `edges[] {from_node, to_node, edge_type,
  evidence?}`; direction: **from_node = item that goes in/on, to_node = host**.
- Fixtures: `registry-items.sample.json` v2 showcase (17 items, every kind);
  `registry-compat.sample.json` (8 edges, every edge_type).
- `validate.mjs`: compat schema wired + strict referential-integrity check (edge endpoints must
  exist in paired items envelope).
- Codegen rerun (`make schema-codegen`, idempotent): `registryItems.ts` + `registry_items.rs`
  regenerated; hand-written `RegistryItemKind` union widened.
- Gates: `npm run validate` PASS · frontend build PASS · vitest **285/285** · `cargo check` PASS.
  - Frontend vitest/build initially failed on a **stale gitignored wasm pkg** (predates T-151.3
    exports) — rebuilt via `make wasm`; unrelated to T-150.
  - `npm run lint`: 1 pre-existing error on clean HEAD (`router.tsx` react-refresh
    `only-export-components`) — reproduced with T-150 changes stashed; local eslint drift, not
    this ticket. Untouched.

Commit 1: `e358777a` — schema v2 + codegen.

## Phase 3/4 — Scanner plugin + export run

Plugin: `TBD_RegistryItemsExportPlugin.c` rewritten in place (class + menu name kept:
`Plugins,TBD,Export TBD Registry Items`); `BuildCuratedRows()`/`AddRow`/`TBD_RegistryItemRow`
**deleted** — no curated path/GUID list remains. New `TBD_RegistryScan.c` helper classes
(scanner/classifier/edge builder). Denylist prefilter constants `{/Structures/, /Rocks/, /Trees/,
/Debris/, /Foliage/}` (skip-without-load); `/Props/` deliberately NOT hard-skipped (crates live
there — spec force-include clause), component classification drops inert props instead.

### Compile loop (what it actually took)

1. First compile (operator Script Editor screenshot) surfaced 3 real EnfScript defects, fixed:
   - `vanilla` is a **reserved keyword** (modded-class super call) → member renamed `isVanilla`;
   - `string.Format` accepts **max 9 params** → DONE log line split;
   - **strong-ref containers can't be method arguments** (`array<ref T>` param rejected) →
     DeriveEdges output memberised (`m_Edges`/`m_EdgeHistogram`);
   - (also hoisted a method-local `static const` array to class scope).
   The `ResourceManage…`/`ScriptEditor/SCR_…`/`commentsToAdd` errors in the same list were
   module-failure cascade (blank Addon column) — gone after the fixes.
2. **A failed WorkbenchGame compile silently kills the EnfusionMCP bridge** (same module):
   every net call returns "not existing Net API function 'EMCP_WB_*'". Deceptive — the port
   stays open. Recovery = fix scripts + restart Workbench.
3. Run 2: scan+derive worked, **file write failed** (`FileHandle.Write wrote=0`, partial
   deleted): `FileHandle` stored as a plain class member is a **weak ref** — collected right
   after `Open()` returned (locals hold strong refs implicitly; members need `ref`). Fixed:
   `protected ref FileHandle m_Handle`.
4. Run 3 (fixed build): full success in **792 ms**; run 4 on the final committed code (log
   histogram moved post-reclassification): **717 ms**, identical envelopes.

Ops note: `pkill -f ArmaReforgerWorkbench` matches the calling shell's own cmdline (exit 144,
kills itself before/after the target) — use `pkill -f '[A]rmaReforgerWorkbench'`.

## Edge families — status

| Family | Status | Notes |
|---|---|---|
| mag_in_weapon | **SHIPPED — 545** | magwell class match + MagazineTemplate direct refs (e.g. `Magazine_556x45_STANAG_30rnd_M855_Ball.et → Rifle_M16A2.et` evidence `MagazineWellStanag556`) |
| optic_on_weapon / attachment_on_weapon | **SHIPPED — 362 / 241** | slot `AttachmentType` × item `WeaponAttachmentAttributes.AttachmentType` (exact match or item-derives-slot via `ToType()/IsInherited`) |
| mag_in_vehicle_weapon | **SHIPPED — 118** | vehicle → SlotManager chain (depth ≤4) → `WeaponTemplate` → magwell; turret-referenced weapons reclassified `vehicle_weapon` (77) — e.g. `Box_762x51_M60_100rnd → MG_M60_Mounted` evidence `MagazineWellM60` |
| character_default_loadout | **SHIPPED — 2,746** | `LoadoutSlotInfo.Prefab` |
| ammo_in_vehicle_weapon | **OPEN — 0 edges** | Implemented (vehicle-weapon `MagazineTemplate` targets of kind `ammo`), but the vanilla set exposes no such pairing — every vehicle-weapon template target is a box **magazine** (already covered by mag_in_vehicle_weapon). Not invented; family fires automatically when a modset provides shell-style templates. |
| **ammo_in_mag** | **OPEN** | Vanilla magazines link ammo via `AmmoConfig "*.conf"` (a config, not an item prefab) — no engine-readable mag→ammo-prefab edge exists in container data. Family ships EMPTY; no edges invented. Revisit iff a runtime API (`MuzzleComponent.GetDefaultMagazineOrProjectileName` on spawned entities) is sanctioned in a later slice. |
| gear_in_slot (vest/uniform capacity) | **OPEN (by locked decision)** | Not in the v1 edge enum — T-150 spec §Locked decisions defers it; cloth→LoadoutAreaType captured as item kinds instead. |

## Scale claim (mod-agnostic proof)

The export is a pure function of `GameProject.GetLoadedAddons()` — the run enumerated **3
addons dynamically** (`core`, `ArmaReforger`, `TBD_Framework`; per-addon counts in the run log
above, scan set recorded in both envelopes' `addons[]`). There is no prefab/GUID/path list
anywhere in the plugin (`BuildCuratedRows` deleted; denylist = generic dressing-dir constants).

**Procedure to include a new Workshop mod (zero code changes):**
1. Add the mod to the Workbench project (Resource Browser → enable addon, or `addon.gproj`
   dependency) and restart Workbench (`steam -applaunch 1874910 -gproj '<gproj>'`).
2. `wb_execute_action {"menuPath":"Plugins,TBD,Export TBD Registry Items"}`.
3. The new addon appears in `addons[]`; its matching prefabs/edges join the envelopes
   (per-addon count printed per run). Copy the two `$profile:` files into
   `packages/tbd-schema/registry/` and re-run `npm run validate`.

Executed this pass with the three addons above (vanilla-only pass sanctioned by spec Task 6);
`apps/mod/crf_framework` (reference-only, 775 text prefabs) is the natural second addon for the
next live proof — its `.et` corpus already cross-validated every container shape offline.

## A1–A5 gate (T-068.8)

| ID | Check | Result |
|----|-------|--------|
| A1 | Export exists at documented path | PASS — `packages/tbd-schema/registry/registry-{items,compat}.workbench.json` (from `$profile:TBD_RegistryItems.json` / `$profile:TBD_RegistryCompat.json`) |
| A2 | Schema validates | PASS — `npm run validate` (both envelopes + referential integrity 4,012 edges) |
| A3 | ≥50 edges | PASS — 4,012 |
| A4 | 3 sampled edges match ResourceName regex | PASS — 3/3 random sample |
| A5 | Every edge_type in locked enum | PASS — enum enforced by ajv; histogram = 5 shipped types |

Spec §Verify extras: ≥1 `mag_in_weapon` (545) PASS · vehicle edge present (`mag_in_vehicle_weapon`
118) PASS.

## Commands + outputs

```bash
bash scripts/mod/tbd-dev-bootstrap.sh          # wb_connect OK, mod_validate 5/5 passed
bash scripts/mod/mcp-call.sh wb_execute_action '{"menuPath":"Plugins,TBD,Export TBD Registry Items"}'
# [TBD][RegistryExport] DONE items=1880 edges=4012 addons=3 elapsedMs=717
cd packages/tbd-schema && npm run validate     # All contracts valid.
python3 - <<'EOF'
import json,collections
i=json.load(open('registry/registry-items.workbench.json'))
c=json.load(open('registry/registry-compat.workbench.json'))
print('items',len(i['items']),dict(collections.Counter(x['kind'] for x in i['items'])))
print('edges',len(c['edges']),dict(collections.Counter(e['edge_type'] for e in c['edges'])))
EOF
# items 1880 {'vehicle':218,'vehicle_weapon':77,'character':354,'gear_helmet':92,'crate':276,
#             'other':252,'ammo':101,'attachment':26,'gear_primary':125,'optic':30,'gear_handgun':4,
#             'gear_uniform':100,'magazine':127,'gear_vest':46,'gear_backpack':43,'gear_launcher':9}
# edges 4012 {'character_default_loadout':2746,'mag_in_weapon':545,'optic_on_weapon':362,
#             'attachment_on_weapon':241,'mag_in_vehicle_weapon':118}
```

Frontend build PASS · vitest 285/285 · `cargo check` PASS · codegen-drift clean ·
`mod_validate` 5/5 (pre-existing warnings only: EnfusionMCP handler-path notes + a spurious
"Missing expected directory: Scripts/Game" — the dir exists; validator quirk, present before
T-150). Pre-existing local-only lint error (`router.tsx` react-refresh) reproduced on clean
HEAD — untouched.

## Known limitations (explicit, not silent)

- `ammo_in_mag` OPEN (`.conf`-only linkage) and `ammo_in_vehicle_weapon` 0-edge on vanilla —
  see Edge families table.
- Display names for prefabs whose UIInfo `Name` is a localisation key (`#AR-...`) fall back to
  humanised file stems (e.g. "Rifle M16A2") — raw keys are not shipped; proper localisation is a
  web-side concern (T-068.9+).
- Grenades/throwables classify as `gear_primary` (they carry `WeaponComponent`); mines/charges
  without item signal may land in `other` (252 total) — acceptable v2 escape hatch, count
  reported per locked decision.
- `/Foliage/` added to the denylist constants beyond the spec's example list (same static-junk
  class); `/Props/` deliberately NOT hard-skipped so crates/arsenal boxes survive (force-include
  clause) — inert props drop via the no-component-signal rule instead.
