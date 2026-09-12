# Systems/Mission

The mission data ingestion and schema pipeline.

---

## Subdirectories

| Subdirectory | Responsibility |
|---|---|
| **`Data/`** | Strongly-typed Enforce structs mapping to the mission JSON schema (`SlotStruct`, `VehicleStruct`, `Params`, `State`). |
| **`Loaders/`** | Network and file loaders that fetch compiled mission JSON from the backend API or local disk, validate against schema rules, and fetch rosters. |
| **`Ingestion/`** | Runtime systems that read world environments, spawn scattered props/trees, and configure dynamic weather from mission parameters. |

---

## Technical Contracts

1. **Server-Authoritative:**
   Clients do not hold or parse the full mission document. The authority parses the mission, validates schema requirements via `TBD_MissionValidator`, and distributes state through specialized subsystem managers (`Spawning`, `Objectives`, `Zones`).
2. **Sentinel-Based Presence:**
   Enfusion's `JsonLoadContext` allocates nested ref objects even for absent JSON keys. Struct fields use explicit scalar sentinels (`ABSENT`, `UNSET`) rather than null checks.
