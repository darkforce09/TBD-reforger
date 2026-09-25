# Object registry alias check

The body of `cargo xtask verify object-registry-aliases`: a census proving that every object the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s Objects palette can offer has a
row in the [mod](/documentation_v2/glossary.md#mod)'s spawn registry, so a placed object is never
skipped at mission load.

## Contents

```text
tools_v2/xtask/src/verifications/registry/
├── mod.rs                      the module tree
├── object_registry_aliases.rs  the census: inputs, mirror pins, alias derivation, fail-fast verdicts
└── tests/                      unit tests against the real files, perturbed copies and missing inputs
```

## How it works

The palette derives an object's alias from its [Workbench](/documentation_v2/glossary.md#workbench) resource name and display name
(`derive_object_alias` in `apps/website/map-engine/src/data/store/operations/assets.rs`), and the
mod's `SpawnMissionEntities` looks that alias up in `apps/mod/tbd-framework/Data/registry.json`.
Nothing joins the two ends at compile time, so the check recomputes the alias independently:

1. All three inputs must exist: the Workbench catalog
   (`contracts_v2/catalogs/registry-items.workbench.json`), the mod registry and the map-engine
   source.
2. The map-engine source must still hold `pub fn derive_object_alias` and the hand-wired
   `comp:checkpoint_small` alias, so the copy here cannot outlive the code it mirrors.
3. The eligible items are those of `kind` `crate` or `other` that are not abstract. For each one
   the check derives the `prop:` or `comp:` alias and requires a registry row with that alias
   whose `guid` equals the item's `resource_name`, and checks both against the schema shapes.

The verdict is fail-fast, in this order: the eligible count must equal `ELIGIBLE_EXACT` (333), the
registry must hold at least `PROP_FLOOR` (289) `prop:` and `COMP_FLOOR` (45) `comp:` rows and the
`comp:checkpoint_small` row, then no alias may be missing, no `guid` may differ, and no shape may
break. A pass prints `PASS: Objects palette aliases — eligible=… prop=… comp=…`.

Exit codes: 0 pass; 1 a failed census or a lost mirror pin; 2 a check that did not run: a
missing input (`FAIL: missing <path>`), an input that could not be read or has the wrong
structure, or a pattern that did not compile.

## Boundaries

- Depends on: `developer_tools::repository_layout::registry_items_catalog_path`;
  `verification_core` (`Pattern`, `Verdict`, `Finding`, `NotRun`, `gate::require`); `serde_json`.
- Used by: `tools_v2/xtask/src/commands/verify/dispatch.rs`; the platform [wave](/documentation_v2/glossary.md#wave) gate's
  `VERIFY_STEPS` (`tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`), which prints the
  last 15 lines of a failure.
- Rules:
  - The derivation here is a hand copy on purpose, never an import of the map-engine code, and it
    must match it (`the_mirror_matches_the_frontend` in
    `tests/object_registry_aliases/tests.rs`).
  - Perturbing the registry turns the pass red (`perturbing_the_registry_turns_the_pass_red`), and
    inputs that were never examined never read as a pass
    (`inputs_that_were_never_examined_do_not_read_as_pass`).
  - A re-exported Workbench catalog changes `ELIGIBLE_EXACT`, which is re-pinned in the same change.
