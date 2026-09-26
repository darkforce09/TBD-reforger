**Status:** live

# Before and after examples

Nine frozen comparisons from the equipment and vehicle exporter's acceptance: for one resource
each, the value the original export carried and the source-backed value the accepted generation
`6A6E9885CF2F83F9` holds, checked against [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)'s
native getters.

## Contents

```text
documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/before-after/
├── iifs_inventory_storage.json     the IIFS field pack: native volume, dimensions and storage, no grid
├── m16_fire_modes.json             the M16A2: its configured Safe, Single and Burst fire modes
├── m997_catalog_mass.json          the M997 ambulance: the rigid body mass behind the catalog mass
├── m997_mass_cargo_fuel.json       the M997 ambulance: cargo limits, fuel, engine and buoyancy fields
├── morphine_effect.json            the morphine injector: its effect class, amounts and durations
├── mortar_default_projectile.json  the M252 mortar: its default projectile and source mass fields
├── radio_transceivers.json         the AN/PRC-68: each transceiver's range and frequency
├── saline_effect.json              the US saline bag: its effect class, amounts and durations
└── terminal_magazine.json          the STANAG magazine with five closing tracers: its ammunition mapping
```

## How it works

Each file holds `generation_id`, `resource_name`, `before` (the audited original values) and
`after` (the generation's source facts). The
[acceptance report](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/implementation_acceptance.md)
says what each comparison proves. The files are frozen and never regenerated.

## Code

- [Equipment and vehicle source exporter](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/)
  — the reader that produced the `after` values.

## Boundaries

- Depends on: the accepted generation and the audited original export.
- Used by: the acceptance report in the parent folder, which links each file.
- Rules: the files stay as recorded; a new comparison belongs to a new acceptance run.
