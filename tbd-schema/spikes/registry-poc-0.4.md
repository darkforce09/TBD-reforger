# Registry POC 0.4 — five aliases end to end

**Status:** code ready; verify in Workbench on a dedicated server.

## Goal

Prove that Mission JSON registry aliases resolve to spawnable vanilla prefabs inside Enfusion.

## Registry file

[`registry.vanilla-poc.json`](../registry/registry.vanilla-poc.json) maps the five POC aliases to **vanilla** resource names copied from the CRF reference mod (not CRF-owned prefabs).

| Alias | Vanilla resource |
|---|---|
| `preset:us_army_82nd` | `Character_US_Rifleman.et` (stand-in until TBD-Content faction preset) |
| `kit:us_rifleman` | `Character_US_Rifleman.et` |
| `kit:us_sl` | `Character_US_GL.et` (replace with `Character_US_SL` once GUID confirmed in Workbench) |
| `veh:m151_mg` | `M151A2_M2HB.et` |
| `comp:checkpoint_small` | `E_Sandbag_Barricade_US_04.et` |

Shipped inside the mod at `tbd-framework/Data/registry.json` (copy of the POC file).

## Enfusion components

In `tbd-framework/Scripts/Game/TBD/`:

- `TBD_Registry.c` — loads registry JSON, resolves alias → resource name
- `TBD_RegistryPocComponent.c` — on dedicated server init, spawns all five aliases in a row and logs results

## Workbench verification checklist

1. Open `tbd-framework/addon.gproj` in Workbench.
2. Add `TBD_RegistryPocComponent` to the scenario game mode entity (or a test world).
3. Run dedicated server / local MP host.
4. Confirm log lines: `[TBD] Registry POC spawned kit:us_rifleman …` (five lines, no errors).
5. Replace `kit:us_sl` GUID with the Workbench “Copy Resource GUID” value for `Character_US_SL.et`.

## Pass criteria

- All five aliases resolve (no `unknown alias` errors).
- At least one character and one vehicle entity exist in the world after init.
- Golden mission entity block (`veh:m151_mg`, `comp:checkpoint_small`) uses the same registry file.
