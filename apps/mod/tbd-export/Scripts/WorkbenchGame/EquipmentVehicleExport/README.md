# Equipment and vehicle source export

The [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) exporter that captures the installed
equipment, vehicles and their gameplay dependencies as source-backed records: every native value
as the loaded configuration holds it, with its inheritance, its authored overrides and the method
that read it. It computes nothing: no inventory grids, loaded mass, tracer ratios, horsepower,
compatibility lists or vehicle classes. `cargo xtask` validates a finished generation and publishes
it as one immutable bundle.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/
├── Capabilities/   organized records: facts by capability, field names, English names, type hierarchy
├── Discovery/      the prefab search that classes each loaded prefab as equipment, vehicle or neither
├── Generation/     one generation's run: queue, records, snapshots, environment, `generation.json`
├── Plugins/        the "Export Equipment and Vehicles" menu entry and the shared diagnostic runner
├── Serialization/  the JSON encoding and checked file writes every part uses
├── Source/         the source reader: containers into typed facts, nodes and references
└── Verification/   reader checks before every run, and the `EMCP_WB_SourceExport` Net API handler
```

## How it works

```text
menu "Export Equipment and Vehicles" (Plugins/)   or   cargo xtask mcp wbcall EMCP_WB_SourceExport (Verification/)
        │
        ▼
Generation/: reader verification (Verification/) ─▶ discovery census (Discovery/) ─▶ queue
        │  per resource: Source/ reads the container tree ─▶ Capabilities/ builds the record
        │                 gameplay references (.et .conf .gamemat .ragdoll) join the queue
        ▼
$profile:TBD_Export/equipment_vehicle_exports/generations/<generation id>/
   generation.json · records/<addon>/<id prefix>/<id>.json · sources/<same path>
        │
        ▼
cargo xtask mod validate-equipment-vehicle-export ─▶ publish-equipment-vehicle-export
        ▼
equipment_vehicle_exports/published/<generation id>/ + manifest.json, current.json ─▶ it
```

A resource's identity is its resource GUID, or its exact resource name when it has none. Each
resource has a source snapshot, which holds its effective containers and, separately identified,
their ancestors, and an organized record, whose facts are the same typed facts under snake_case
field names, grouped into capabilities that each list their installations in order. Every fact
names its resource, node, property and native read method, and its origin: declared, inherited,
engine default or native getter. Zero, false, empty text, empty arrays and null references are
kept as values; units appear only where the engine documents them, and nothing is converted.
Gameplay configurations each get a record of their own, so every gameplay link resolves inside
the bundle; models, textures, audio and other assets stay external references. Each child README
holds the detail.

Diagnostic actions, in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/` and
`apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/`, run the same pipeline over a selected
set; their generations have scope `diagnostic`, and the publisher rejects them.

### Complete export

1. Open `apps/mod/tbd-export/addon.gproj` in Workbench. Cold-restart Workbench after adding a
   script file; reload scripts after editing one.
2. Run Plugins → TBD → Export Equipment and Vehicles, or drive it from outside with the
   `start` action of `EMCP_WB_SourceExport` and `step` until it reports `completed`.
3. Find the generation under `$profile:TBD_Export/equipment_vehicle_exports/generations/<generation id>/`.
4. Validate and publish it:

```bash
cargo xtask mod validate-equipment-vehicle-export --input <generation_directory>
cargo xtask mod publish-equipment-vehicle-export --input <generation_directory>
```

The validator prints a readable summary on stderr and the JSON report on stdout. Publication
validates again under a lock, copies and hash-checks every file into `published/<generation id>/`,
writes a SHA-256 `manifest.json`, and replaces `current.json` atomically. Consumers read the
directory `current.json` names, never the newest staging folder. The first publication moves the
unversioned `equipment/` and `vehicles/` export folders beside `equipment_vehicle_exports/` into
`equipment_vehicle_exports/legacy/<generation id>/` under a durable journal, recovered on the
next run after an interruption; other exports stay in place. Earlier published generations stay
for rollback, and publishing an already published id again selects it without rewriting it.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's `WorkbenchPlugin`, `NetApiHandler` and resource search; the engine's
  `BaseContainer` reflection, `TypeName`, `WidgetManager`, `GameProject`, `JsonSaveContext` and
  `FileIO`; the loaded addons' prefabs and configs; nothing from `tbd-framework`.
- Used by: the diagnostic plugins in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`
  and `apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/`, which call
  `TBD_SourceDiagnosticExport`; `cargo xtask mcp wbcall`, which reaches `EMCP_WB_SourceExport`; the
  validation and publication commands in `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/`,
  which read the generation files.
- Rules: the exporter writes source facts only and every consumer derives its own values from
  them; every enumerated property is read or fails the extraction, and an unread property cannot
  be passed off as unavailable (`a_readable_property_cannot_be_dismissed_as_unavailable`); only a
  complete, verified generation publishes (`partial_and_unverified_exports_cannot_publish`); the
  published bytes are sealed by their hashes
  (`finalized_hashes_detect_consistent_but_tampered_facts`); all three tests are in
  `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`, which also
  covers provenance, reference closure, repeated installations, statuses, publication and
  rollback. The schema `contracts_v2/definitions/equipment-vehicle-export.schema.json` is the
  contract for every file a generation writes. The scripts compile only when Workbench loads
  `tbd-export`; `cargo xtask mod compile` compiles the framework addon alone
  (`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs`).

## Related documentation

- [Export validation and publication](/tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/README.md)
  — every check the validator runs and the publication steps.
- [Equipment and vehicle export documentation](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md)
  — the exporter's documentation index.
- [Acceptance evidence](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/README.md)
  — the frozen acceptance record, the field mapping and the before-and-after examples.
- [MCP commands](/tools_v2/xtask/src/commands/mcp/README.md) — `cargo xtask mcp wbcall`.
