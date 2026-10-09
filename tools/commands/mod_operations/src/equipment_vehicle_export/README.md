# Equipment and vehicle export validation and publication

Checks a source-only equipment and vehicle generation that the `tbd-export`
[Workbench](/documentation/glossary/n_to_z.md#workbench) plugins wrote, and publishes a valid one as
an immutable, sealed copy with a `current.json` pointer. It serves
`cargo xtask mod validate-equipment-vehicle-export` and
`cargo xtask mod publish-equipment-vehicle-export`.

## Contents

```text
tools/commands/mod_operations/src/equipment_vehicle_export/
├── files.rs                       safe paths under a root, SHA-256 digests, JSON reads that refuse duplicate keys
├── gameplay_receipt.rs            the gameplay publication receipt: fact bytes by section, diagnostic inventory, schema check
├── graph.rs                       per-resource record and snapshot checks: identity, facts, links, cycles
├── mod.rs                         the module tree, the validation report and the two command entry points
├── publication.rs                 the locked, staged, hash-verified publication and the pointer swap
├── relationships.rs               reference, type and class-hierarchy checks across resources
├── tests/                         unit tests for validation, publication, recovery and tampering
├── unversioned_export_archive.rs  journalled move of the unversioned export folders aside at first publication
├── upload_bundle.rs               the `archives/<generation id>.tar.gz` upload bundle of a published gameplay generation
└── validation.rs                  the generation walk against the export schema and every census check
```

## How it works

The Workbench plugins write a generation under
`$profile:TBD_Export/equipment_vehicle_exports/generations/<generation id>/`
(`mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Generation/TBD_SourceExportGeneration.c`).

- Validate: `validation::validate` checks `generation.json` and every record and snapshot against
  `contracts/definitions/equipment-vehicle-export.schema.json`, compiled in with
  `include_str!`. The generation must be `completed`, of scope `complete`, with Workbench reader
  verification `passed`. Resource ids, names and output paths must be unique, the discovery
  census non-empty and fully exported, and the folder must hold no unlisted file and no symlink.
  `graph` and `relationships` check each resource's facts, links, references, types and class
  hierarchy. The command prints the report as JSON on stdout and a one-line summary on stderr.
- Publish: the input must sit in `equipment_vehicle_exports/generations/`. Under a publication
  lock the command validates again, copies every listed file into a private staging folder,
  checks each copy against its validated digest, writes `manifest.json` (schema version 2) and
  renames the staging folder to `published/<generation id>/`. It then replaces `current.json`
  through a temporary file, keeping the previous pointer until the swap is durable. An id that is
  already published must match byte for byte. The first publication moves the export root's
  `equipment/` and `vehicles/` folders into `unversioned_exports/<generation id>/` under a
  durable journal (`.unversioned-export-archive.json`), and every run first recovers an
  interrupted move.

## Boundaries

- Depends on: `contracts/definitions/equipment-vehicle-export.schema.json`; the `jsonschema`,
  `serde_json`, `content_digest` and `walkdir` crates.
- Used by: `tools/commands/mod_operations/src/mod_dispatch.rs` (`validate_command`,
  `publish_command`).
- Rules: a generation that is partial or unverified never publishes, and a failed validation
  leaves the current generation untouched (`partial_and_unverified_exports_cannot_publish` and
  `failed_validation_keeps_the_current_generation_byte_for_byte` in `tests/validation.rs`);
  published bytes are sealed by their hashes (`publication_is_sealed_retryable_and_preserves_old_data`,
  `finalized_hashes_detect_consistent_but_tampered_facts`); paths never escape the root or follow
  a symlink (`path_traversal_and_symlinks_fail`).

## Related documentation

- [Equipment and vehicle export evidence](/documentation/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/README.md)
  — recorded validation and publication runs.
