# Destroy target diagnostics audit

The body of `cargo xtask verify destroy-target-diagnostics`: a check that the destroy-objective
diagnostics in the [mod](/documentation/glossary/g_to_m.md#mod) never claim that [mission](/documentation/glossary/g_to_m.md#mission) `entities[]`
go unspawned, and that the check itself still fails when such a claim returns. The parent file
`tools/checks/mod_script_checks/src/destroy_target_diagnostics.rs` holds the target
paths, the banned phrasings and the pinned signatures.

## Contents

```text
tools/checks/mod_script_checks/src/destroy_target_diagnostics/
├── source_audit.rs                   the entry: live scans and pins, then the four RED proofs and the verdict
└── source_pins_and_perturbations.rs  the source pins, the RED arms and the in-memory perturbations
```

## How it works

`verify_destroy_target_diagnostics` reads five files:
`Types/Destroy/TBD_ObjectiveDestroyTargets.c`, `Engine/Runtime/TBD_ObjectivesComponent.c` and
`Engine/Registry/TBD_ObjectiveRulesReader.c` under
`mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`, `TBD_MissionUnconsumedKeyCheck.c`
under `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/Validation/`, and
`contracts/definitions/mission.schema.json`. Then:

1. Each file is scanned for the exact phrasings in `EXACT_LIES` and the regex paraphrases in
   `PARAPHRASES`.
2. The destroy-target source, with `//` and `/* */` comments stripped, must still define
   `ArmDestroyTargets` and `DiagnoseEmptyDestroyTargets` with live return arms and the
   unresolved-alias `m_sInertReason` line.
3. Each other file must keep its truth pin, such as `SpawnMissionEntities`.
4. Four RED proofs perturb the destroy-target text in memory (a paraphrased claim, collapsed
   returns, a renamed function, a pin moved into a comment) and require the checks above to
   reject each.
   No file is written; the FAIL lines of a RED proof show a `/tmp/tmp.` display path.

Exit codes: 0 every live pin holds and every RED proof failed as expected; 1 a missing file, a
banned phrase, a lost pin or a RED proof that passed; 2 a RED perturbation that could not be set
up.

## Boundaries

- Depends on: the parent file's constants and `Paths`; `verification_core` (`Pattern`,
  `Verdict`, `gate`); `regex`; `repository_layout::definition_path` for the
  schema path.
- Used by: the parent file, which re-exports the entry to
  `tools/xtask/src/commands/verify/dispatch.rs`; the platform [wave](/documentation/glossary/n_to_z.md#wave) gate runs
  `verify destroy-target-diagnostics` as one of its `VERIFY_STEPS`
  (`tools/commands/platform_execution/src/wave_execution/gate.rs`).
- Rules: structural pins read comment-stripped source, so a pin kept only in a comment fails
  (`collapsed_returns_fail_registry_pins` in
  `tools/checks/mod_script_checks/src/tests/destroy_target_diagnostics/tests.rs`); the
  live tree passes (`live_tree_holds`); the output and the 0/1 status are what the wave gate
  prints and tests.
