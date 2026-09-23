# Equipment diagnostic actions

Equipment actions select resources for inspection and call the shared source exporter in `../EquipmentVehicleExport/`. Every diagnostic uses the same reader, identity, snapshots, capability fields, dependency traversal, and schema as the complete export.

Use **Export Equipment and Vehicles** under **TBD** for a complete generation. Actions under **TBD Diagnostics** create partial generations that the publisher rejects. They do not write independently maintained equipment catalogs or summaries.

See [the shared exporter](../EquipmentVehicleExport/README.md) for export and validation commands.
