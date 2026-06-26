> **Superseded for spawn (2026-06-14):** `TBD_RegistryPocComponent` is no longer on `TBD_GameMode.et`. Production spawn uses `TBD_SpawnManager` + mission `slots[]` (schema 1.1).

# Registry POC 0.4 — five aliases end to end

**Status:** GREEN — verified on Linux dedicated server (2026-06-13).

## Goal

Prove that Mission JSON registry aliases resolve to spawnable vanilla prefabs inside Enfusion.

## Registry file

[`registry.vanilla-poc.json`](../registry/registry.vanilla-poc.json) maps five POC aliases to **vanilla** resource names.

| Alias | Vanilla resource |
|---|---|
| `preset:us_army_82nd` | `Character_US_Rifleman.et` (stand-in until TBD-Content faction preset) |
| `kit:us_rifleman` | `Character_US_Rifleman.et` |
| `kit:us_sl` | `Character_US_GL.et` (replace with `Character_US_SL` once GUID confirmed in Workbench) |
| `veh:m151_mg` | `M151A2_M2HB.et` |
| `comp:checkpoint_small` | `E_Sandbag_Barricade_US_04.et` |

Shipped inside the mod at `tbd-framework/Data/registry.json`.

## Enfusion components

In `tbd-framework/Scripts/Game/TBD/`:

- `TBD_Registry.c` — loads `$TBD_Framework:Data/registry.json`, falls back to `$profile:TBD_Registry.json`
- `TBD_RegistryPocComponent.c` — on init, spawns all five aliases in a row and logs results

Attached via `TBD_GameMode.et` prefab on the dev scenario.

## Verification (done)

```bash
bash scripts/setup-server-profile.sh
bash scripts/run-dev-server.sh
```

Log output (`.local-test-profile/logs/.../console.log`):

```
[TBD] Registry loaded (5 aliases).
[TBD] Registry POC spawned kit:us_rifleman at <0, 1, 0>
[TBD] Registry POC spawned comp:checkpoint_small at <8, 1, 0>
[TBD] Registry POC spawned kit:us_sl at <16, 1, 0>
[TBD] Registry POC spawned preset:us_army_82nd at <24, 1, 0>
[TBD] Registry POC spawned veh:m151_mg at <32, 1, 0>
```

## Remaining cleanup

- Move spawn origin to Eden land coords (currently cosmetic — water at 0,0)
- Confirm `kit:us_sl` GUID via Workbench “Copy Resource GUID” for `Character_US_SL.et`

## Pass criteria

- [x] All five aliases resolve (no `unknown alias` errors)
- [x] Character + vehicle entities exist after init
- [x] Golden mission entity block uses the same registry file
