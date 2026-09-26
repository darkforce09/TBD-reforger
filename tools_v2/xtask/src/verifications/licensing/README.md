# Upstream code leak gate

The licensing check `cargo xtask verify no-crf-leak`: the upstream frameworks that sit beside the
[mod](/documentation_v2/glossary/g_to_m.md#mod) as read-only references (the Coalition Reforger
Framework, under the Arma Public License, and PlayableSelector, which carries no licence) are read
and cited, never copied, so neither their identifiers nor their asset GUIDs may reach the addons
this repository ships.

## Contents

```text
tools_v2/xtask/src/verifications/licensing/
├── mod.rs                  the module tree
├── tests/                  unit tests for each arm, the wordings, symlinked lanes and missing trees
├── upstream_code_leaks/    the gate body: the identifier and GUID arms and the vanilla probe
└── upstream_code_leaks.rs  the lanes, patterns and output log; re-exports `verify_crf_leak`
```

## How it works

The gate reads files on disk, tracked or not, in five lanes: our code in
`apps/mod/tbd-framework/` and `apps/mod/tbd-export/`; the framework reference in
`apps/mod/crf_framework/` and the PlayableSelector reference in `apps/mod/playable_selector/`
(both gitignored, so absent on a fresh clone; with no in-repository PlayableSelector folder,
`TBD_PS_ORACLE` names one); and the vanilla game paks under the Steam install in `$HOME`. It runs
four arms in order:

```text
CRF_ identifiers ─▶ PS_ identifiers ─▶ CRF asset GUIDs ─▶ PlayableSelector asset GUIDs
   (our code, comments and EnfusionMCP skipped)    (UI/ and Prefabs/ of each reference)
```

1. An identifier arm prints every line of our code where the prefix follows a non-identifier
   character, up to 20 lines, after dropping comment lines; `EnfusionMCP` folders are skipped.
2. A GUID arm collects every `{16 hex}` GUID under the reference's `UI/` and `Prefabs/` folders,
   following symlinks, intersects them with the GUIDs in our code, and drops each shared GUID that
   a vanilla `.pak` also contains. What remains fails. A reference that is absent prints `SKIP`,
   never `OK`. Without a local game install every shared GUID is reported.

Any hit prints a closing note that names the mod design document and the slice workflow runbook.
A cold run can take several minutes, because a GUID that is not in vanilla is searched through
every pak.

On the committed tree, with a local game install and no PlayableSelector folder, the gate exits 1:

- the `CRF_` arm reports the text `@CRF_Framework` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/TBD_MissionSelectorData.c`
  (inside a trailing `//!<` comment, which the line filter does not treat as a comment) and in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/TBD_MissionSelectorMock.c` (a string);
- the framework GUID arm reports four GUIDs: the two that
  `apps/mod/tbd-framework/Data/registry.json` references at lines 1382 and 1762, the
  `robotomono_msdf_28.fnt` font GUID that the layouts under
  `apps/mod/tbd-framework/UI/layouts/Session/` use, and a backpack prefab GUID in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Verification/TBD_SourceReaderVerification.c`.

## Public surface

- `upstream_code_leaks::verify_crf_leak`: the entry of `cargo xtask verify no-crf-leak`, taking the
  checkout root. Exit codes: 0 `no-oracle-leak: PASS`; 1 a leak in any arm; 2 our code trees could
  not be read or a pattern did not compile.

## Boundaries

- Depends on: `verification_core` (`scan`, `proc::Run`, `Verdict`, `NotRun`, `Pattern`); `regex`;
  the `grep` binary; `crate::core::repository_layout::documentation` (`MOD_DESIGN`,
  `SLICE_WORKFLOW_RUNBOOK`).
- Used by: `tools_v2/xtask/src/commands/verify/dispatch.rs`; people following the mod slice
  workflow runbook, which runs the gate before a slice lands. The mod [wave](/documentation_v2/glossary/n_to_z.md#wave) driver names a
  `no-crf-leak` step, but that step runs `make verify-no-crf-leak`
  (`tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs`), not this command.
- Rules:
  - Three different wordings stay distinct: `OK (none)`, `OK (nothing to compare)` and `SKIP`
    (`the_three_no_finding_wordings_stay_distinct` in `tests/upstream_code_leaks/tests.rs`).
  - A GUID is exempt only when a vanilla pak holds it
    (`vanilla_paks_exempt_a_shared_guid_and_only_a_shared_guid`), and symlinked reference lanes
    are followed (`asset_dirs_descend_through_symlinked_lanes`).
  - The command keeps the name `no-crf-leak`, which the slice workflow runbook cites, although it
    covers both references.
  - Tests that change `PATH` keep `/usr/bin` on it, so this gate's `grep` still resolves
    (`tools_v2/xtask/src/core/test_environment.rs`).

## Related documentation

- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the reference lanes a
  slice worktree links.
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the design authority the
  failure text cites.
