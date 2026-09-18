# Missions Contract (`missions/contract/`)

Canonical schema types and semantic validation engines enforcing mission, scenario, and loadout contract specifications.

---

## 1. Structure & Submodules

### `generated/`
- **Source of Truth**: Generated directly from canonical JSON schemas in `packages/tbd-schema/schema/*.json` via `cargo xtask ci schema-codegen`.
- **Key Modules**:
  - `mission.rs`: Strongly typed mission root structure, metadata, terrain coordinates, and scenario rules.
  - `loadout.rs`: Unit equipment manifests, inventory slots, ammo allocations, and weapon configurations.
  - `orbat.rs`: Faction structures, company/platoon/squad hierarchies, role requirements, and vehicle crew assignments.
- **Invariants**:
  - Never edit generated files by hand. All schema alterations originate in `packages/tbd-schema/schema/*.json`.

### `validation.rs` (<450 LOC)
- **Purpose**: Semantic and structural validation of scenario documents beyond pure JSON schema typing.
- **Key Validations**:
  - **Coordinate Bounds**: Verifies all entity, marker, and objective coordinates lie within the target terrain's metric extents.
  - **Entity Reference Integrity**: Ensures vehicle crew slots point to defined squad members and loadouts match declared weapon registries.
  - **ORBAT Completeness**: Validates that every playable side has at least one command echelon and spawn point.
  - **Asset Existence**: Validates that referenced prefab GUIDs exist in the canonical prefab registry.
