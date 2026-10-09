**Status:** live

# Export addon Workbench exporter documentation

The deeper documents of the export addon's [Workbench](/documentation/glossary/n_to_z.md#workbench)
exporters: the map exporters that feed the terrain data, the acceptance evidence of the
equipment and vehicle source exporter, and the ballistics oracle.

## Contents

```text
documentation/mod/tbd-export/Scripts/WorkbenchGame/
├── BallisticsOracle/        the ballistics oracle feature doc
├── EquipmentVehicleExport/  the equipment and vehicle exporter's acceptance evidence
└── MapExport/               the map export feature doc and the terrain export runbook
```

## How it works

The folders mirror `mod/tbd-export/Scripts/WorkbenchGame/`, one per exporter family that has
documents beyond its code READMEs. The equipment and vehicle diagnostics and the registry item
plugin have no documents here: their code READMEs cover them.

| Exporter family | Live entry point | Documents |
|---|---|---|
| Map export | the Net API handler `EMCP_WB_TbdBlueprint` through `cargo xtask mcp wbcall`; no menu entries | [map export](/documentation/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md), [terrain export runbook](/documentation/mod/tbd-export/Scripts/WorkbenchGame/MapExport/terrain_export_runbook.md) |
| Equipment and vehicle source export | the menu entry "Export Equipment and Vehicles" and the Net API handler `EMCP_WB_SourceExport` | [acceptance evidence](/documentation/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/README.md) |
| Ballistics oracle | the menu entry "Ballistics Oracle", then playing the export world | [ballistics oracle](/documentation/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/ballistics_oracle.md) |
| Registry items | none: the plugin's menu entry is commented out | none |

## Code

- [Workbench scripts](/mod/tbd-export/Scripts/WorkbenchGame/) — every exporter of the addon;
  the scripts compile only inside Workbench.

## Boundaries

- Depends on: the code READMEs under `mod/tbd-export/Scripts/WorkbenchGame/`, which hold each
  exporter's folder detail.
- Used by: the [export addon index](/documentation/mod/tbd-export/README.md).
- Rules: a document sits in the folder mirroring the exporter it covers; evidence folders are
  named `verification_evidence/` and stay frozen.
