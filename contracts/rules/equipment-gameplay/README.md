# Equipment gameplay selection policy

The reviewed decision for every native class, property and type the
[Workbench](/documentation/glossary/n_to_z.md#workbench) equipment export meets: which classes
and component branches enter the published gameplay dataset, which of their fields it keeps, and
why each other field stays out. The xtask equipment commands read it, and the export addon runs
the selection tables generated from it.

## Contents

```text
contracts/rules/equipment-gameplay/
├── attachment/           weapon attachment types, attachment slots, ejectors and magazine wells
├── communications/       radio components and transceivers
├── excluded/             classes no gameplay consumer reads: presentation, engine infrastructure, AI
├── explosive/            explosion damage, fragmentation and impulse effects, charges and mines
├── identity/             item UI info, faction affiliation and editable-entity descriptors
├── inventory/            inventory item components and item attribute collections
├── magazine/             magazine components and ammunition type configuration
├── medical_effects/      consumable medical items and their effects
├── native-matching.json  which accepting slot property matches which providing item class
├── physics/              bounding volumes, rigid bodies and physics geometry
├── policy.json           the manifest: policy version, class file list, baseline census, rejections
├── projectile/           projectiles, ballistic tables, triggers and projectile movement
├── protection/           damage managers, hit zones, damage effects and game materials
├── sights/               sights, optics, fields of view, reticles and zeroing
├── storage/              equipment and inventory storage components and their slots
├── structure/            entity and vehicle topology and authored installation transforms
├── utility/              deployable items and parts, resources, and handheld gadgets
├── vehicle_systems/      compartments, drivetrains, fuel, rotors, landing gear and locks
├── visuals/              preview render attributes and mesh objects
└── weapon/               weapons, muzzles, fire modes, recoil and aim modifiers
```

## How it works

`policy.json` lists the class files in `class_files`, and `Policy::load` in
`tools/commands/mod_operations/src/equipment_gameplay/policy.rs` reads them in that order. Each
file is a JSON array of class rows; a row names a native class, its ancestor types, the section it
belongs to and its rules, and each rule gives a set of `{property, native_type}` fields one
disposition, the section that owns them, whether a reference is followed, and the reason:

| Disposition | What the published gameplay dataset holds |
|---|---|
| `retain_value` | the field's value |
| `retain_relationship` | the reference the field makes to another class, without its configuration |
| `traverse_required_container` | the container, walked only to reach retained branches below it |
| `exclude` | nothing; the reason records why |

A class whose section is `excluded` is not selected: only retained component branches enter the
published graph, and an excluded target of a required relationship keeps its identity and its
exclusion reason, never its values. The loader refuses a policy whose version is not 1, a class
file path that climbs out of the folder, an unknown disposition, an empty reason, a class or a
(class, property, native type) field listed twice, and a field count that differs from
`baseline_field_combinations`, the census of the diagnostic generation the policy was reviewed
against. The policy's digest is the SHA-256 of `policy.json` followed by every class file in list
order; a gameplay generation records it, and validation refuses a generation that records another.
`unknown_fields` and `unknown_classes` are `reject_publication`: a field or class the policy does
not review fails the generation instead of passing unreviewed.

`native-matching.json` is separate from the selection: each rule names an accepting class and
property (an attachment slot's `AttachmentType`, a muzzle's `MagazineWell`) and the providing
class whose native type must equal or inherit the accepted one.

## Format

- Encoding: UTF-8 JSON. Class files are named `<section>/classes_NN.json`, with a two-digit
  sequence number within the section folder.
- Schema: no JSON Schema; the shapes are the `Class`, `Rule` and `Field` structures of `policy.rs`
  and the `Rule` structure of
  `crates/api/api_equipment_datasets/src/queries/native_matches.rs`.
  The published generation the policy selects follows the schemas in
  `contracts/definitions/equipment-gameplay/`.
- Adding a file: put the rows in the section's folder, list the file in `class_files`, update
  `baseline_field_combinations` when the census changes, then run
  `cargo xtask mod generate-equipment-gameplay-policy` and commit the regenerated tables with it.

## Producers and consumers

- Producers: people, reviewing every class and field a complete diagnostic generation reports.
- Consumers:
  - `cargo xtask mod generate-equipment-gameplay-policy`, which writes the selection tables under
    `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated/`
    (one folder per section, and `TBD_GameplayPolicyGenerated.c` with the digest), and with
    `--check` refuses drift between the policy and those tables;
  - `cargo xtask mod project-equipment-gameplay`, which projects a complete diagnostic generation
    through the policy into a gameplay generation;
  - `cargo xtask mod validate-equipment-vehicle-export` and `publish-equipment-vehicle-export`,
    which hold every field of a gameplay generation to its rule and its recorded digest to the
    policy's;
  - the [API](/documentation/glossary/a_to_f.md#api)'s equipment data viewer, which embeds
    `native-matching.json` at compile time for its native-match listing.

## Boundaries

- Depends on: the native class, property and type names of the Enfusion classes the Workbench
  equipment export reads, as a complete diagnostic generation reports them.
- Used by: the xtask equipment gameplay and equipment export commands in
  `tools/commands/mod_operations/src/`, the generated selection tables and
  `TBD_GameplaySelectionPolicy` in the export addon, and the API's equipment data viewer
  services.
- Rules: every (class, property, native type) field appears in exactly one rule and every class
  in exactly one row (`Policy::load`); a policy edit ships with its regenerated tables
  (`cargo xtask mod generate-equipment-gameplay-policy --check`); a published gameplay generation
  records the digest of the policy that selected it.

## Related documentation

- [Equipment gameplay contracts](/contracts/definitions/equipment-gameplay/README.md) — the
  schemas of the gameplay generation this policy selects.
