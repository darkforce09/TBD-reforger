# Equipment and vehicle export verification and Net API

The checks that prove the source reader reads what the engine holds before any generation starts,
and the Net API handler that lets tools outside [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
drive an export, step it, and probe single resources.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Verification/
├── EMCP_WB_SourceExport.c          the Net API handler with its request and response
└── TBD_SourceReaderVerification.c  reader checks on isolated fixtures and installed prefabs
```

## How it works

### Reader verification

`TBD_SourceReaderVerification.Run` runs at the start of every generation, and a failure stops it:

- isolated fixtures, never exported: 2,001 strings with quotes, escapes, newlines and Unicode
  survive `TBD_SourceExportJson` and a reload; a `TBD_SourceReaderFixture` container keeps explicit
  `0`, `false`, empty text and an empty array as `declared` values; a capture started past the depth
  limit fails loudly; a child container of the installed M16A2 that removes every inherited
  component reads as an explicit empty `components` array;
- installed prefabs, the M997 ambulance, the M16A2 with M203 and the IIFS field pack: every fact of
  every node matches a direct native read, every object link matches the engine's order, and at
  least one child override differs from its ancestor.

`RunResources` runs the installed-prefab check over any resource list without the override
requirement. `Json` gives `passed` or `failed` with the list of checks, which `generation.json`
records.

### Net API handler

`EMCP_WB_SourceExport` is a `NetApiHandler`; `cargo xtask mcp wbcall EMCP_WB_SourceExport '<json>'`
calls it with a request of `action`, `resource` and `resources`, and gets back `status`,
`snapshot` (a directory or file), `error_count` and up to 20 `errors`.

| Action | What it does | Writes |
|---|---|---|
| `start` | starts a complete generation | the generation directory |
| `diagnostic` | starts a diagnostic generation over `resources` | the generation directory |
| `step` | captures the next queued resource; repeat until `completed` | the generation directory |
| `status` | reports the running generation | nothing |
| `verify` | runs the reader verification | `source_reader_probes/verification.json` |
| `verify_resources` | runs the installed-prefab check over `resources` | `source_reader_probes/verification.json` |
| `discovery` | classes `resources` as discovery would | `source_reader_probes/discovery.json` |
| `hierarchy` | captures `resource` and writes its type hierarchy | `source_reader_probes/hierarchy.json` |
| `read_script` | copies the script file named by `resource` | `source_reader_probes/native_script.c` |
| any other | captures `resource` as a source snapshot | `source_reader_probes/latest.json` |

The probe files sit under `$profile:TBD_Export/`, outside every generation.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`, the reader in
  `Source/`, the generation in `Generation/`, the discovery in `Discovery/` and
  `TBD_SourceExportJson` in `Serialization/`; the engine's `NetApiHandler`, `JsonApiStruct`,
  `BaseContainerTools` and `JsonLoadContext`; the installed vanilla prefabs the checks name.
- Used by: `TBD_SourceExportGeneration` in `Generation/`, which runs the verification; over the Net
  API, `cargo xtask mcp wbcall` (`tools_v2/xtask/src/commands/mcp/`). `cargo xtask verify no-crf-leak`
  scans this folder with the rest of the addon and reports the backpack prefab GUID the checks name
  (`tools_v2/xtask/src/verifications/licensing/README.md`).
- Rules: a generation whose reader verification did not pass never publishes
  (`partial_and_unverified_exports_cannot_publish` in
  `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`); the handler
  answers only while Workbench has `apps/mod/tbd-export/addon.gproj` open, the one project that
  compiles these scripts. The Unicode fixture string is the one non-ASCII literal, and it is
  deliberate.

## Related documentation

- [Acceptance evidence](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/README.md)
  — the recorded verification, Workbench acceptance and repeatability runs.
- [Workbench MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md) — the Net API
  and the calls that reach this handler.
