# Missions Services (`missions/services/`)

Internal business logic for scenario compilation, prefab registry imports, and Enfusion game server injection.

---

## 1. Domain Services

### `mission_compiler.rs` (<450 LOC)
- **Purpose**: Transforms authored scenario documents (Yjs/JSON) into validated runtime mission payloads.
- **Key Functions**:
  - `compile_mission_scenario(doc: &ScenarioDocument) -> Result<CompiledMissionPackage, CompileError>`: Validates entities, evaluates triggers/scripts, serializes entity hierarchy, and builds world header descriptors.
  - `extract_mission_summary(raw_bytes: &[u8]) -> Result<MissionHeaderSummary, CompileError>`: Fast-path inspection of uploaded mission bundles without full world inflation.
- **Invariants**:
  - Pure deterministic compilation. Output depends strictly on input document and canonical schema version.

### `registry_importer.rs` (<400 LOC)
- **Purpose**: Processes Workbench export dumps (`prefabs.json`, `vehicles.json`, `equipment.json`) and reconciles the canonical prefab registry database tables.
- **Key Functions**:
  - `sync_workbench_catalog(pool: &PgPool, catalog: WorkbenchCatalogDump) -> Result<ImportSummary, AppError>`: Upserts prefab GUIDs, display names, categories, inventory capacities, and slots.
- **Invariants**:
  - Idempotent execution. Retains custom user annotations while updating game asset paths and mesh references.

### `game_agent_bridge.rs` (<350 LOC)
- **Purpose**: Bridge client for transmitting compiled scenario payloads to dedicated server instances and triggering live reloads.
- **Key Functions**:
  - `deploy_mission_to_server(client: &reqwest::Client, server_url: &str, secret: &str, mission: &CompiledMissionPackage) -> Result<DeploymentAck, BridgeError>`: Authenticated push of mission assets to the server's local storage daemon.
