**Status:** live

# Equipment and vehicle exporter implementation acceptance

**Complete:** all 24 findings have a recorded disposition and passing evidence. The fresh generation validates and is published as current. The 160 original files are archived and match their audit hashes.

The exporter uses one source reader and schema version 2. It preserves native values and relationships, keeps inheritance evidence separate, and rejects extraction failures before publication. It performs no inventory-grid calculation, unit conversion, inferred ammunition classification, compatibility expansion, or gameplay simulation.

## Evidence

- [Old-to-new field inventory](field-mapping/inventory.json): all 706 observed field paths have a disposition; grouped files contain each decision and its audit finding numbers.
- [Workbench cases](workbench-native-acceptance.json): 32 installed resources checked against native primitive getters and ordered object relationships.
- [Compile evidence](workbench-compilation.json): successful WorkbenchGame module load after the required initial cold restart and subsequent script reloads.
- [Code hashes](source-code-manifest.json): source reader, validator/publisher, tests and contract used for acceptance.
- [Original file hashes](legacy-input-manifest.json): all 160 old files match the original audit.
- [Original prefab references](legacy-resource-inventory.json): 1,336 exact resource names, including catalog resources and referenced dependencies.

[Publication and archive proof](publication.json) records the current pointer, sealed manifest, and verified original files.

## Accepted bundle

Generation `6A6E9885CF2F83F9`, installed game build `1.8.0.13`: **3,265 resources, 580,993 source nodes, 6,282,833 facts, zero extraction or validation errors**. The indexes contain 1,327 equipment resources and the exact original set of 219 vehicles. All 1,336 audited prefab references resolve. The 910 resources shared with the diagnostic generation produce 1,820 byte-identical record/snapshot files, excluding run metadata.

The bundle contains about 4.0 GB of records and source snapshots. The complete export takes approximately 16 minutes on this installation. Native string joining reduces the measured M997 snapshot operation from approximately 5.8 to 4.1 seconds; preserving separate effective and ancestor views adds source evidence compared with earlier trial runs.

[Validation summary](validation-summary.json), [coverage](coverage-summary.json), [repeatability](repeatability.json), and [reader fixtures](source-reader-verification.json) record the acceptance checks. All 21 Rust regression tests pass. The [32 native Workbench cases](workbench-native-acceptance.json) pass against the final scripts.

## Concrete before/after examples

Before values come from the audited files. After values come from the installed game configuration, checked against Workbench getters. Old exports do not identify their producing build/revision.

- [IIFS backpack](before-after/iifs_inventory_storage.json): remove the invented 4 × 75 grid. Keep native volume, dimensions, size enum and storage restrictions separately.
- [M997](before-after/m997_mass_cargo_fuel.json): keep the child cargo weight limit of 870 and volume limit of 100000, alongside native fuel, engine and buoyancy fields. [Catalog mass](before-after/m997_catalog_mass.json) points to `RigidBody.Mass = 2791`, replacing the inherited generic value of 1.
- [M16 fire modes](before-after/m16_fire_modes.json): preserve Safe, Single and Burst configurations instead of labeling every mode Safety.
- [Terminal tracer magazine](before-after/terminal_magazine.json): retain the ordered mapping and ammunition configuration; remove calculated ratios and loaded weights.
- [AN/PRC-68](before-after/radio_transceivers.json): retain each transceiver's range and frequency; 1300 is a native range value, not a wattage specification.
- [Saline](before-after/saline_effect.json) and [morphine](before-after/morphine_effect.json): retain concrete effect classes and separate amounts, speeds and durations, including legitimate zero values.
- [M252 mortar](before-after/mortar_default_projectile.json): keep the default projectile relationship and source mass fields without substituting a family or destroyed-debris mass.

## Findings and corrections

### 01. Invented wearable grids

Remove derived grids; retain ItemVolume, ItemDimensions and the native inventory size enum separately.

### 02. Incomplete storage

Use the same source-backed storage instances for wearables, containers and vehicles, including pockets, slots, MaxCumulativeVolume and MaxItemSize.

### 03. Ancestor overrides

Read effective values once and preserve ancestors as separate evidence; catalog indexes reference that same record.

### 04. Ammunition path classification

Remove path substring classification; preserve native projectile configurations and magazine mappings.

### 05. Guessed caliber and designation

Remove guessed caliber spellings and Standard fallbacks; retain authored values, native magazine wells and configuration relationships.

### 06. Incorrect fire modes

Export configured BaseFireMode instances, UIName, burst/salvo settings and RoundsPerMinute; use no uninitialized script instance getters.

### 07. Duplicated installations

Keep effective and ancestor graphs separate; preserve ordered native instances and shared configuration links.

### 08. Invented drivetrain figures

Keep native engine, rotor, transmission and suspension fields; remove RPM multipliers, wheel-count drivetrain guesses and derived power.

### 09. Incomplete mounted weapons

Traverse nested components and child entities using the common weapon reader; retain actual turret settings.

### 10. Radio quantities mislabeled

Keep individual transceivers and their native range/frequency configuration; remove invented watts, channel counts and encryption booleans.

### 11. Fuel and water properties

Preserve actual fuel, buoyancy and water-propulsion properties and spellings; export no synthesized amphibious status.

### 12. Synthetic defaults

Remove guessed stack limits, capacities, roles, empty cargo lists and utility classifications. Native zero/false/empty values remain present.

### 13. Mixed measurements

Separate inventory attributes from physics bodies and projectile configuration; remove calculated loaded weights and supposed physical bounding boxes.

### 14. Tracer statistics

Preserve ordered AmmoMapping, MaxAmmo, configuration references and authored tracer settings; do not expand ratios or inferred functions.

### 15. Attachment type lists

Read slot/default-installation rules and native object constraints; expand declared ancestor candidates and verify relationships with TypeName.IsInherited.

### 16. Names and identity

Use native resource GUIDs or exact-name fallback; preserve original names and resolve English with WidgetManager.Translate.

### 17. Guessed selectable or abstract status

Remove filename-derived claims; retain native flags and technical templates as source records.

### 18. Stale optics and aliases

Close gameplay references within one generation and retire independent legacy views on successful publication.

### 19. Inconsistent contract and provenance

Use schema version 2, common source facts, coordinated indexes, environment metadata, immutable publication and hashed manifests.

### 20. Damage and protection

Keep hit zones, material/configuration references, geometry references and native damage settings; remove fabricated protection and thickness ratings.

### 21. Medical and utility semantics

Keep concrete effect classes, amounts, regeneration speeds, durations and utility configuration without altering authored values.

### 22. Unread defaults and guessed units

Enumerate typed source properties and fail on unsupported/failed reads; preserve native FOV, ballistic and effect data without unit guesses.

### 23. Mortar relationships and mass

Expose AmmoTemplate as default_projectile; retain separate compatibility constraints and native mass fields without debris/family substitutes.

### 24. Vehicle stations and systems

Preserve child crew stations, role/access/door relationships and electrical, instrumentation, communications, audio and damage configuration.

## Reading the new data

Read `equipment_vehicle_exports/current.json`, then the referenced published directory. `generation.json` indexes the records and source snapshots. Equipment and vehicle indexes refer to the same resource identities. Each capability contains separate component/configuration instances; each fact points to its original native property.

`ItemVolume`, `ItemDimensions`, and inventory `m_Size` remain separate values. They do not supply a calculated two-dimensional grid in this export. A vehicle storage compartment uses the same source structure as a backpack storage compartment.

The old directories move into the generation-specific `legacy/` archive only after a complete generation validates. Partial diagnostics and failed exports cannot become current. Existing published generations remain available for rollback; unrelated exports remain in place.

## Genuine source/API limitations

- There are 17 unresolved nonempty localization entries; [their exact source locations and keys](unresolved-localization.json) are recorded. Another 9,856 unavailable English entries have empty source names, mainly technical/action UI containers. These are name-entry counts, not missing inventory resources. Original text/keys remain intact.
- Native C++ container classes without a script TypeName cannot expose a verified script inheritance hierarchy. Their properties are still captured; this limitation applies only to type reflection.
- Units remain explicitly unknown unless supported by source documentation. Inventory weight, volume and dimensions use documented kg, cm3 and cm; no conversion is applied.
- GameProject exposes addon identity and order, but no supported loaded-addon version getter. An exporter revision is unavailable when no build stamp is installed; the acceptance code hashes are recorded separately.
- Workbench generates identifiers for some unauthored editor containers when scripts reload. Their exact native identifiers are retained with structural context. Resource GUID identity is separate; deterministic repeat checks use the same loaded native configuration.
- Binary model, texture, sound and animation resources remain external references. Readable prefabs, configurations, game materials and ragdoll configurations are included as gameplay source records.

## Verification commands

```sh
cargo test -p xtask equipment_vehicle_export
cargo xtask mod validate-equipment-vehicle-export --input <generation_directory>
cargo xtask mod publish-equipment-vehicle-export --input <generation_directory>
```

The publisher reruns input validation, checks every copied file against the validated SHA-256 hashes, writes a finalized manifest, and atomically changes the current pointer. Byte identity preserves the completed schema and graph checks without parsing unchanged copies again. Rust tests cover corruption, stale files, incomplete discovery, omitted relationships, changed source values, conflicting identities, partial exports, interrupted archival, and preservation of the previous generation.
