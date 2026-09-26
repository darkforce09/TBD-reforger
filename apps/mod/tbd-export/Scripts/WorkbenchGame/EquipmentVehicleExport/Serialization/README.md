# Equipment and vehicle export JSON writing

The JSON encoding every part of the equipment and vehicle source exporter writes through: quoted
strings, scalars at full native precision, typed arrays, JSON Pointer segments and checked file
writes. It runs in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) whenever a generation,
a probe or a verification writes a file.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Serialization/
└── TBD_SourceExportJson.c  string, scalar and array encoding, joins and checked file writes
```

## How it works

`TBD_SourceExportJson` is a static helper; the exporter builds each document as a string and hands
it to `Write`.

- Strings of up to 512 characters and every float go through the engine's own `JsonSaveContext`,
  so escaping matches the native serializer and scalars keep their native precision
  (`SetMaxDecimalPlaces(324)`). A longer string is escaped by hand (quotes, backslashes and control
  characters), because the native serializer caps its output length.
- `Scalars`, `Integers`, `Booleans`, `Strings` and `VectorValue` write typed arrays; `Join` joins
  with the engine's `string.Join`, which allocates the final buffer once for large snapshots.
- `Nullable` writes `null` for an empty string; `PointerPart` escapes `~` and `/` for the node ids,
  which are JSON Pointer paths.
- `Write` opens the file, writes the whole string and reports failure unless every character was
  written.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the engine's `JsonSaveContext`, `FileIO` and `FileHandle`.
- Used by: every sibling folder of `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`
  that writes JSON: `Source/`, `Capabilities/`, `Generation/` and `Verification/`.
- Rules: every value the exporter writes passes through these encoders, never through string
  concatenation of raw text, so one escaping rule holds across records, snapshots and
  `generation.json`; the reader verification in `Verification/` round-trips 2,001 escaped and
  Unicode strings through `Strings` on every export start. The scripts compile only when Workbench
  loads `tbd-export`; `cargo xtask mod compile` compiles the framework addon alone
  (`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs`).

## Related documentation

- [Export contract](/contracts_v2/definitions/equipment-vehicle-export.schema.json) — the schema
  the written documents follow.
