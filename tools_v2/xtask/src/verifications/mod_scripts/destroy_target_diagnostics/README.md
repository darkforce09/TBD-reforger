# Destroy target diagnostics audit

The body of `cargo xtask verify destroy-target-diagnostics`: a check that the destroy-objective
diagnostics in the [mod](/documentation_v2/glossary.md#mod) never claim that [mission](/documentation_v2/glossary.md#mission) `entities[]`
go unspawned, and that the check itself still fails when such a claim returns. The parent file
`tools_v2/xtask/src/verifications/mod_scripts/destroy_target_diagnostics.rs` holds the target
paths, the banned phrasings and the pinned signatures.

## Contents

```text
tools_v2/xtask/src/verifications/mod_scripts/destroy_target_diagnostics/
├── source_audit.rs      the entry: live scans and pins, then the four RED proofs and the verdict
└── strip_c_comments.rs  comment stripping, the source pins, and the in-memory perturbations
```

## How it works

`verify_destroy_target_diagnostics` reads five files: `TBD_ObjectiveRegistry.c`,
`TBD_ObjectivesComponent.c` and `TBD_ObjectiveRules.c` under
`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`, `TBD_MissionValidator.c` under
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`, and
`contracts_v2/definitions/mission.schema.json`. Then:

1. Each file is scanned for the exact phrasings in `EXACT_LIES` and the regex paraphrases in
   `PARAPHRASES`.
2. The registry source, with `//` and `/* */` comments stripped, must still define
   `ArmDestroyTargets` and `DiagnoseEmptyDestroyTargets` with live return arms and the
   unresolved-alias `m_sInertReason` line.
3. Each other file must keep its truth pin, such as `SpawnMissionEntities`.
4. Four RED proofs perturb the registry text in memory (a paraphrased claim, collapsed returns, a
   renamed function, a pin moved into a comment) and require the checks above to reject each.
   No file is written; the FAIL lines of a RED proof show a `/tmp/tmp.` display path.

Exit codes: 0 every live pin holds and every RED proof failed as expected; 1 a missing file, a
banned phrase, a lost pin or a RED proof that passed; 2 a RED perturbation that could not be set
up.

## Boundaries

- Depends on: the parent file's constants and `Paths`; `verification_core` (`Pattern`,
  `Verdict`, `gate`); `regex`; `developer_tools::repository_layout::definition_path` for the
  schema path.
- Used by: the parent file, which re-exports the entry to
  `tools_v2/xtask/src/commands/verify/dispatch.rs`; the platform [wave](/documentation_v2/glossary.md#wave) gate runs
  `verify destroy-target-diagnostics` as one of its `VERIFY_STEPS`
  (`tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`).
- Rules: structural pins read comment-stripped source, so a pin kept only in a comment fails
  (`collapsed_returns_fail_registry_pins` in
  `tools_v2/xtask/src/verifications/mod_scripts/tests/destroy_target_diagnostics/tests.rs`); the
  live tree passes (`live_tree_holds`); the output and the 0/1 status are what the wave gate
  prints and tests.
