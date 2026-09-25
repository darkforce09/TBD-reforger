# Mod script checks

Checks over the [EnfScript](/documentation_v2/glossary.md#enfscript) sources and `.layout` files of
the `tbd-framework` [mod](/documentation_v2/glossary.md#mod): source pins that stop false comments
and a size bypass from returning, the structural gate for UI layouts, and the two
[Workbench](/documentation_v2/glossary.md#workbench) spawn checks behind `cargo xtask mod`.

## Contents

```text
tools_v2/xtask/src/verifications/mod_scripts/
├── destroy_target_diagnostics/            the destroy-target audit body: live pins, RED proofs, comment stripping
├── destroy_target_diagnostics.rs          the destroy-target targets, banned phrasings and pinned signatures
├── mission_rest_size_limits.rs            `verify mission-rest-size-limits`: the 8 MiB mission ceiling pins
├── mod.rs                                 the module tree
├── player_identity_comments.rs            `verify player-identity-comments`: bans and pins on `TBD_PlayerIdentity.c`
├── results_reporter_identity_comments.rs  `verify results-reporter-identity-comments`: the same for `TBD_ResultsReporter.c`
├── spawn_determinism.rs                   `mod spawn-determinism`: preflight, offline selftest, log normalising
├── spawn_determinism_live.rs              the live Workbench runs of spawn-determinism and their comparison
├── spawn_verification.rs                  `mod spawn-verify`: a 25 s Workbench play, judged by `mcp wb-logs`
├── tests/                                 unit tests for every check and the layout parser
├── ui_layout_parser/                      the parser's record, keyword and conversion helpers
├── ui_layout_parser.rs                    `Analyzer`: arms C1 to C4 and C6 over one `.layout` text
└── ui_layouts.rs                          `verify ui-layouts`: walks the layouts, runs the parser and arm C5
```

## How it works

Every check reads committed files under `apps/mod/tbd-framework/` from the checkout root and
prints its own report. The static checks share one discipline: a live pin, then RED proofs that
perturb the source text in memory and must fail, so a check that can no longer fail is itself a
failure. None of them writes a file.

| Check | Reads | Holds |
|---|---|---|
| `mission-rest-size-limits` | `TBD_MissionLoader.c`, `TBD_MissionArtifactVerification.c`, `TBD_MissionArtifactCache.c` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/` | `IsMissionBodyWithinCap(` is called before `ParseMissionJson(` in comment-stripped code, its body compares `Length() <= MISSION_FILE_MAX_BYTES`, and the verification and cache refuse oversized documents |
| `player-identity-comments` | `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_PlayerIdentity.c` | no comment claims `#tbd link` is unimplemented; the truth pins stay |
| `results-reporter-identity-comments` | `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_ResultsReporter.c` | the same, for the identity the reporter sends |
| `destroy-target-diagnostics` | the objective sources, the mission validator and `mission.schema.json` | no diagnostic claims `entities[]` go unspawned (see its folder's README) |
| `ui-layouts` | the `.layout` files directly in `apps/mod/tbd-framework/UI/layouts/` (not its subfolders), and every file under `apps/mod/tbd-framework/Scripts/Game/TBD/UI/` | C1 brace balance, C2 attested slot classes, C3 frame slot geometry, C4 container children declare a slot, C5 every widget name a script looks up exists, C6 a container child's slot sets its alignment |

The `ui-layouts` walk does not descend: every committed layout sits in a subfolder of
`apps/mod/tbd-framework/UI/layouts/` (`Common/`, `Hud/` or `Session/`), so the check finds
no file and exits 1 with `FAIL: no .layout files under …`.

`spawn_determinism.rs` and `spawn_verification.rs` are not `verify` verbs: `cargo xtask mod`
dispatches `spawn-determinism` and `spawn-verify` to them, and both drive a running Workbench.
spawn-determinism first checks that the Workbench Net API listens on `ENFUSION_WORKBENCH_PORT`
(5775 by default), then compares the normalised spawn and equip log lines of several plays of one
world; spawn-verify plays the open world through `cargo xtask mcp call` and returns the verdict of
`cargo xtask mcp wb-logs`. `--selftest` runs either offline.

Exit codes of the `verify` checks: 0 held; 1 a violation, a missing file, or a RED proof that
passed; 2 a check that did not run (a RED perturbation that could not be set up, a missing layout
folder).

## Public surface

- `mission_rest_size_limits::verify_mission_rest_size_limits`,
  `player_identity_comments::verify_player_identity_comments`,
  `results_reporter_identity_comments::verify_results_reporter_identity_comments`,
  `destroy_target_diagnostics::verify_destroy_target_diagnostics` and
  `ui_layouts::verify_ui_layouts`: the five `cargo xtask verify` entries, each taking the checkout
  root.
- `spawn_determinism::run` and `spawn_verification::run`: the bodies of
  `cargo xtask mod spawn-determinism` and `cargo xtask mod spawn-verify`.

## Boundaries

- Depends on: `verification_core` (`Pattern`, `Verdict`, `NotRun`, `gate`, `scan`, `proc`);
  `regex`; `developer_tools::repository_layout` for the mission schema path;
  `crate::core::repository_root` and `crate::core::repository_layout::documentation` (the
  spawn-determinism runbook); `cargo xtask mcp` subprocesses and `ss` for the spawn checks.
- Used by:
  - `tools_v2/xtask/src/commands/verify/dispatch.rs` and
    `tools_v2/xtask/src/commands/mod_ops/dispatch.rs`;
  - the `ci-local` step `verify-mission-rest-size-limits` in
    `tools_v2/xtask/src/commands/ci/task_definitions.rs`, and the `mod-gates-hosted` job of
    `.github/workflows/ci.yml`;
  - the platform [wave](/documentation_v2/glossary.md#wave) gate's `VERIFY_STEPS`
    (`tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`): mission-rest-size-limits,
    destroy-target-diagnostics and both identity-comment checks;
  - the mod wave gate (`tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs`), which
    runs `verify ui-layouts`.
- Rules:
  - Every ban and pin must fail on its perturbed copy (`every_ban_is_discriminating` and
    `every_pin_is_discriminating` in `tests/player_identity_comments/tests.rs`,
    `every_red_arm_turns_the_live_shape_red` in `tests/mission_rest_size_limits/tests.rs`).
  - A missing target file never reads as a pass (`a_missing_target_does_not_read_as_pass`).
  - The wave gates print the last 15 lines of a failed step, so the printed report is part of each
    check's contract.

## Related documentation

- [Spawn determinism](/documentation_v2/runbooks/spawn_determinism.md) — running the Workbench
  spawn checks.
- [Mod command group](/tools_v2/xtask/src/commands/mod_ops/README.md) — the `mod spawn-determinism`
  and `mod spawn-verify` commands.
