# TBD Framework

Greenfield Enfusion game mode for the TBD Reforger platform. **TBD-owned code only.**

## Coalition / CRF — do not use in Workbench

| Folder | Role | Open in Workbench? |
|---|---|---|
| **`tbd-framework/`** (this mod) | Production TBD framework | **Yes** |
| **`Tbd_framework/`** | CRF reference (read patterns in Cursor only) | **No** — 60+ Coalition workshop deps |

The “Missing Addon Dependencies” dialog appears when you open `Tbd_framework/addon.gproj`. That is expected: it is not part of the TBD product. **Remove it from the Workbench project list.**

This mod depends on **vanilla Arma Reforger only** (`58D0FB3206B6F859` in `addon.gproj`). No Coalition GUIDs.

**Reference only:** you may read CRF patterns in `Tbd_framework/` in the editor — never copy verbatim, never add CRF as a dependency.

## Workbench setup

1. Fix base game path (Linux/Proton): `bash scripts/setup-workbench-linux.sh`, then locate  
   `~/ArmaReforger-Base/data/ArmaReforger.gproj`
2. **+ Add Project** → **Add Existing** → select **`tbd-framework/addon.gproj`** (not `Tbd_framework`)
3. Open **TBD_Framework** in the launcher

- Backend config from `$profile:TBD_BackendConfig.json`
- Mission loader: REST `GET /api/missions/{id}/compiled` → file fallback → `$profile/missions/{id}.json`
- Registry alias resolution + POC spawner component
- Game stage enum + manager stub (`LOADING → … → DEBRIEF`)
- Radio bridge hook stubs (partner VOIP wires later)

## Workbench setup

1. Add this addon to a scenario (Everon test world).
2. Attach `TBD_FrameworkManager` to the game mode entity.
3. Optional: attach `TBD_RegistryPocComponent` to verify registry aliases spawn.
4. Copy [`Data/backend.example.json`](Data/backend.example.json) to `$profile:TBD_BackendConfig.json` and set `backendUrl`, `serverToken`, `missionId`, `eventId`.

## Server profile layout

```
$profile/
  TBD_BackendConfig.json
  missions/
    msn_8f3a2c.json    # cached after successful REST fetch
```

## Registry

Shipped at `Data/registry.json` (vanilla POC GUIDs). Replace with TBD-Content export in Phase 1+.

See [`tbd-schema/spikes/registry-poc-0.4.md`](../tbd-schema/spikes/registry-poc-0.4.md).
