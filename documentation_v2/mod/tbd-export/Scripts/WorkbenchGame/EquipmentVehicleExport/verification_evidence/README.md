**Status:** live

# Equipment and vehicle export acceptance evidence

The frozen record that accepted the export addon's equipment and vehicle source exporter: the 24
audit findings and their dispositions, the old-to-new field mapping, the
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) cases, the hashes of the code and of the
original files, and concrete before-and-after examples. Developers changing the exporter, its
validator or its contract read it to see what was proven and how.

## Contents

```text
documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/
├── acceptance-cases.json            the 32 installed resources the native Workbench cases select
├── audit-dispositions.json          the 24 audit findings, each with its disposition, cases and evidence
├── before-after/                    old and new values of nine resources the audit named
├── coverage-summary.json            the accepted generation's resource, index and discovery counts
├── field-mapping/                   the disposition of every old field path, grouped by subject
├── implementation_acceptance.md     the acceptance report: the findings, the bundle, the limitations
├── legacy-input-manifest.json       the 160 original export files with their sizes and SHA-256 hashes
├── legacy-resource-inventory/       the 1,336 original prefab references, split by GUID prefix
├── legacy-resource-inventory.json   the reference inventory's source, count and part files
├── publication.json                 the publication proof: current pointer, sealed manifest, verified originals
├── repeatability.json               the byte comparison of the resources two generations share
├── source-code-manifest.json        hashes of the reader, validator, publisher, tests and contract
├── source-reader-verification.json  the reader fixture checks and their status
├── unresolved-localization.json     the 17 nonempty names with no English text, by node and key
├── validation-summary.json          the validator's verdict, counts, errors and hashed files
├── workbench-compilation.json       the WorkbenchGame module load after the cold restart
└── workbench-native-acceptance.json  the 32 native getter cases and their results
```

## How it works

Read [implementation_acceptance.md](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/implementation_acceptance.md)
first: it states the accepted bundle (generation `6A6E9885CF2F83F9`, game build `1.8.0.13`), walks
the 24 findings and links each JSON file that proves one. The JSON files fall into four groups:

| Group | Files | What it proves |
|---|---|---|
| Audit | `audit-dispositions.json`, `field-mapping/` | every finding and every old field path has a recorded decision |
| Originals | `legacy-input-manifest.json`, `legacy-resource-inventory.json`, `legacy-resource-inventory/` | the archived original export matches the audit, and every prefab it referenced resolves |
| Acceptance runs | `acceptance-cases.json`, `workbench-native-acceptance.json`, `workbench-compilation.json`, `source-reader-verification.json`, `source-code-manifest.json` | the final scripts compile and read native values correctly, for the hashed code |
| Bundle | `coverage-summary.json`, `validation-summary.json`, `repeatability.json`, `publication.json`, `before-after/`, `unresolved-localization.json` | the published generation is complete, valid, repeatable and sealed |

Every file here is frozen: its content is never reworded or regenerated. The evidence file names
keep their hyphenated spelling. A new acceptance run adds new evidence beside a new report.

## Code

- [Equipment and vehicle source exporter](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/)
  — the Workbench scripts the Workbench cases and code hashes cover.
- [Validation and publication](/tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/) —
  the validator and publisher the summaries and the publication proof come from.

## Boundaries

- Depends on: the exporter and the xtask commands above at the hashed revision, and the contract
  `contracts_v2/definitions/equipment-vehicle-export.schema.json`.
- Used by: the exporter README in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`
  and the validation README in `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/`.
- Rules: nothing here is edited after acceptance; only the folder indexes and the report's links
  change.

## Related documentation

- [Equipment and vehicle export documentation](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md)
  — how a generation is exported, validated and published.
