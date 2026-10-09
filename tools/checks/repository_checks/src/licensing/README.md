# Upstream code leak gate

The licensing check `cargo xtask verify no-crf-leak`: the upstream frameworks that sit beside the
[mod](/documentation/glossary/g_to_m.md#mod) as read-only references (the Coalition Reforger
Framework, under the Arma Public License, and PlayableSelector, which carries no licence) are read
and cited, never copied, so neither their identifiers nor their asset GUIDs may reach the addons
this repository ships.

## Contents

```text
tools/checks/repository_checks/src/licensing/
├── mod.rs                  the module tree
├── tests/                  unit tests for each step, the wordings, symlinked lanes and missing trees
├── upstream_code_leaks/    the gate body: the identifier step, the GUID step and the vanilla probe
└── upstream_code_leaks.rs  the lanes, patterns and output log; re-exports `verify_crf_leak`
```

## How it works

The gate reads files on disk, tracked or not: our code in `apps/mod/tbd-framework/` and
`apps/mod/tbd-export/`; the `crf_framework` and `playable_selector` lanes of
`apps/mod/References/` (gitignored, so filled per machine as
[its README](/apps/mod/References/README.md) describes; a non-empty `TBD_PS_ORACLE` names another
PlayableSelector checkout); and the vanilla game paks under the Steam install in `$HOME`. A lane
that is absent or holds no `UI/` or `Prefabs/` folder stops the run with exit 2 before any check.
Then it runs two steps:

```text
CRF_ identifiers ─▶ PS_ identifiers ─▶ asset GUIDs of both lanes, one vanilla pass
   (our code; comments and EnfusionMCP skipped)   (UI/ and Prefabs/ of each lane)
```

1. The identifier step prints every line of our code where the prefix follows a character that
   is neither an identifier character nor `@`, up to 20 lines, after dropping comment-only lines;
   `EnfusionMCP` folders are skipped. A `@`-prefixed Workshop mod name such as `@CRF_Framework` is
   a dependency reference, not code.
2. The GUID step collects every `{16 hex}` GUID under each lane's `UI/` and `Prefabs/` folders,
   following symlinks, intersects them with the GUIDs in our code, and asks the vanilla paks once
   about all shared GUIDs: one `grep` per pak, in parallel, lists the pak's runs of 16 or more
   uppercase hex digits, and a GUID inside any run is vanilla. Each lane's remaining GUIDs fail,
   each printed with the `path:line` of every reference in our code. Without a local game install
   every shared GUID is reported.

Any finding prints a closing note that names the mod design document and the slice workflow
runbook. A run reads the ~25 GB of paks once; on the development machine it takes about 8 s, bound
by disk reads.

With the lanes filled and a local game install, the gate exits 1 on four CRF GUIDs: the two that
`apps/mod/tbd-framework/Data/registry.json` references, the `robotomono_msdf_28.fnt` font GUID
that the layouts under `apps/mod/tbd-framework/UI/layouts/` use, and a backpack prefab GUID in
`apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Verification/TBD_SourceReaderVerification.c`.
Each is a licence decision (re-author from vanilla or record an attribution), never an exemption.

## Public surface

- `upstream_code_leaks::verify_crf_leak`: the entry of `cargo xtask verify no-crf-leak`, taking the
  checkout root. Exit codes: 0 `no-oracle-leak: PASS`; 1 a finding in any step; 2 a lane, our code
  trees or a pak could not be read, or a pattern did not compile.

## Boundaries

- Depends on: `verification_core` (`scan`, `Verdict`, `NotRun`, `Pattern`); `process_runner::Run`; `regex`;
  the `grep` binary; `repository_layout` (the lane paths and the documents the
  closing note names).
- Used by: `tools/xtask/src/commands/verify/dispatch.rs`; the mod
  [wave](/documentation/glossary/n_to_z.md#wave) gate's `no-crf-leak` step
  (`tools/commands/mod_operations/src/wave_execution/execution.rs`); people following the mod slice
  workflow runbook, which runs the gate before a slice lands.
- Rules:
  - A lane the gate cannot compare against is exit 2 naming it, never a pass
    (`a_lane_the_gate_cannot_compare_against_is_did_not_run` in `tests/upstream_code_leaks/tests.rs`).
  - A Workshop mod name is not an identifier leak, a real symbol still is
    (`a_workshop_mod_name_is_not_an_identifier_leak`).
  - A GUID is exempt only when a vanilla pak holds it, and an unreadable pak is exit 2
    (`vanilla_paks_exempt_a_shared_guid_and_only_a_shared_guid`); the one vanilla pass answers
    every GUID as a `grep -qla` per GUID does
    (`one_vanilla_pass_answers_every_guid_as_a_grep_per_guid_does`); symlinked lanes are followed
    (`asset_dirs_descend_through_symlinked_lanes`).
  - The command keeps the name `no-crf-leak`, which the slice workflow runbook and the wave gate
    cite, although it covers both lanes.
  - Tests that change `PATH` keep `/usr/bin` on it, so this gate's `grep` still resolves
    (`tools/foundation/process_runner/src/search_path.rs`).

## Related documentation

- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the reference lanes a
  slice worktree links.
- [Mod design](/documentation/apps/mod/tbd-framework/mod_design.md) — the design authority the
  failure text cites.
