# Equipment and vehicle source export

The exporter reads installed Workbench configuration. It preserves native values, explicit empty values, enums, units when documented, effective inheritance, authored overrides, component instances, and gameplay references. It does not calculate inventory grids, loaded mass, tracer ratios, horsepower, compatibility lists, or vehicle classifications.

## Complete export

1. Open `TBD_Export` in Workbench. Cold restart after adding script files; reload scripts after editing existing files.
2. Run **Plugins → TBD → Export Equipment and Vehicles**.
3. Find the completed generation under `$profile:TBD_Export/equipment_vehicle_exports/generations/<generation_id>/`.
4. Run `cargo xtask mod validate-equipment-vehicle-export --input <generation_directory>`.
5. Run `cargo xtask mod publish-equipment-vehicle-export --input <generation_directory>`.

The validator prints a readable result to stderr and a JSON report to stdout. Publication repeats validation, copies and verifies an immutable bundle in `published/<generation_id>/`, writes a SHA-256 manifest, then replaces `current.json` atomically. Consumers resolve the directory named by that pointer. They must not select the newest staging directory by timestamp.

On first publication, the old sibling `equipment/` and `vehicles/` directories move to `equipment_vehicle_exports/legacy/<generation_id>/`. Other exports remain in place. A durable archive journal supports recovery after interruption. Previous published generations remain available for rollback. Publishing a previously validated staging generation selects it again without rewriting its sealed content.

## Records and source evidence

`generation.json` contains the source resource index, equipment and vehicle identity indexes, discovery census, environment, extraction errors, reader verification, and native type hierarchy. `records/<addon>/<identifier_prefix>/` holds organized records; `sources/` holds matching typed snapshots.

Identity is the native resource GUID, or the exact resource name when a GUID is unavailable. Component native container identifiers remain available alongside structural paths that distinguish repeated installations. Each capability is an array of component/configuration instances, each linked to a source node. Muzzles, compartments, pockets, wheels, effects, and attachment installations keep their separate identities and order.

Native container identifiers are scoped to the loaded configuration. Workbench can generate new identifiers for unauthored editor containers after a script reload. Structural paths preserve their context; resource GUIDs remain the catalog identity. Repeatability checks compare the same loaded source configuration, without treating changing native identifiers as authored gameplay changes.

An organized fact is the same typed fact as its source snapshot. Its source names the resource, node, property, and native read method. `origin` distinguishes declared values, inherited values, engine defaults, and native getters. Values of zero, false, empty text, empty arrays, and null object references are retained. Source property spelling is unchanged; organized field names use snake_case. `AmmoTemplate` is exposed as `default_projectile`. The distinct native `Trigger Offset` vector uses `trigger_offset_vector3` to avoid colliding with the scalar `TriggerOffset`.

Source snapshots include effective containers and separately identified ancestor containers. Gameplay configurations receive one resource record each, and links resolve within the bundle. Models, textures, audio, and other binary assets remain external resource references. Parent resources are linked through `BaseContainer.GetResourceName`. A source reference with that method points to node metadata; references read with `BaseContainer.Get` point to native properties.

Vectors retain native coordinate order (`VECTOR2`: x/y; `VECTOR3`: x/y/z); colors retain r/g/b/a. Object arrays retain order, null entries, and explicit empty overrides. Shared containers receive distinct effective and ancestor views so inheritance evidence cannot replace an effective installation. Prefabs, configuration resources, game materials, and readable ragdoll configurations are closed gameplay dependencies.

## Names, units, and absence

Original name text and localization keys remain in source facts. English names are resolved by `WidgetManager.Translate` with `en_us` active, restoring the prior locale afterwards. Unresolved names remain unavailable.

No numeric unit conversions take place. Inventory weight, volume, and dimensions carry the units documented by `ItemPhysicalAttributes`: kg, cm3, and cm. Other units remain null unless documented by a supported source. FOV values retain native values without an assumed angle unit. Inventory size enums are separate from dimensions and volume.

Facts support `present`, `not_present`, `not_applicable`, `unavailable`, and `error`. Enumerated source properties must be read or fail extraction. An unread property cannot be marked unavailable to bypass validation. Optional capability blocks only exist when native configuration provides that capability. No synthetic empty capability blocks are created.

## Diagnostics and automation

Actions under **TBD Diagnostics** use the same pipeline for selected resource sets. Their `scope` is `diagnostic`, and the publisher rejects them.

The `EMCP_WB_SourceExport` handler supports `verify`, `start`, `step`, `status`, and `diagnostic` (with an explicit `resources` array). A start performs discovery and reader verification. Each step captures one resource and queues gameplay dependencies; repeat until completed. The response reports extraction errors and the generation directory. `equipment_vehicle_exports/progress.json` is a transient progress file outside the generation.

Reader verification checks native overrides and ordered object relationships on the installed M997, M16/M203 and IIFS backpack; isolated fixtures exercise component removal, zero/false/empty values, Unicode and large arrays, and explicit traversal failure. The fixtures are never part of the catalog. `verify_resources` checks an explicit resource list through the same native getters, and `discovery` inspects the discovery classification of that list. Rust regression tests cover provenance, reference closure, repeated installations, status handling, schemas, publication integrity, and rollback.

The native type hierarchy uses script declarations as ancestor candidates and verifies relationships with `TypeName.IsInherited`. This includes addon-defined classes. Native container classes that have no script TypeName remain explicitly unavailable for that reflection operation; their source properties are still exported.

An optional `$TBD_Export:exporter_revision.txt` build stamp is recorded verbatim. Without it, the revision is unavailable. Game build comes from `Game.GetBuildVersion`; addon identity/order comes from `GameProject.GetLoadedAddons`. Unexposed addon versions remain unavailable. No version numbers are guessed.

The authoritative contract is `contracts_v2/definitions/equipment-vehicle-export.schema.json`. Implementation acceptance evidence and the field inventory live under `docs/verification/equipment-vehicle-export/`.
