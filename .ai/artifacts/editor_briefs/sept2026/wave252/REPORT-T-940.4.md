# REPORT T-940.4 — Nested telemetry counters

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-940.4
slice/T-940.4
```

First actions matched the brief. EnfusionMCP count was already 19. `export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` for xtask/schema/mod compile. Test binaries used `/home/Samuel/.cache/tbd-target-T-940.4`. Gate lock wait is serialisation, not a hang.

## defect_verified_on_main

Worktree is `slice/T-940.4` at merge-base + T-942 packing (pre-slice HEAD `b59b99116`). Before the fold:

- `TBD_ResultsReporter.c` `BuildPlayerRow` emitted only `arma_id` / `role_played` / `deaths` / `source_event_id` (no nested `counters`).
- `PlayerStatInput` discarded top-level `deaths` (unknown field) and 400'd any other top-level counter key via `legacy_counter_key`.
- `tests/telemetry.rs` `the_shipping_mod_payload_is_accepted_verbatim` pinned the loss: A1 `deaths` stored `NULL`, assert `"top-level deaths is ignored; absent counters store NULL (T-397)"`.

That is the drop: a 200 on the shipping payload wrote identity-core-only and no scoreline.

## changes

| path | why |
|---|---|
| `apps/website/api/src/handlers/telemetry/telemetry.rs` | Capture flat counter keys as `Option` values. `effective_counters()` uses nested when present; otherwise `fold_flat_counters()` builds a complete scoreline (unsent numerics 0, `is_command` false, `command_win` NULL). Identity-only (neither shape) still writes no counters. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_ResultsReporter.c` | `BuildPlayerRow` emits nested `counters` (zeros for unmeasured fields, `command_win` null) and keeps flat `deaths` for one release. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_ResultsReporter.c` | ASCII twin of the emit. |
| `apps/website/api/tests/deployments_combat.rs` | Flat-payload golden (kills=17…); both-shapes-equal (flat == nested; conflicting leftover flat ignored); reporter deaths-only == nested equivalent. |
| `apps/website/api/tests/telemetry.rs` | **Outside owns.** T-393 goldens that pinned the drop/400 now assert the fold (shipping deaths stored; complete flat body 200). Partial nested `counters` still 400. Identity-only re-ingest still not a write. |

## perturbation

Skipped the kills fold (`kills: 0` instead of `self.kills.unwrap_or(0)`). Rebuilt in the private target dir. Ran `flat_counter_payload_stores_the_scoreline` against `t9404_it`.

**red_output VERBATIM:**

```
thread 'flat_counter_payload_stores_the_scoreline' (3200367) panicked at apps/website/api/tests/deployments_combat.rs:416:5:
assertion `left == right` failed: flat kills/deaths/… must land, not NULL/0-drop
  left: (0, 3, 1, 842, 4, true, Some(true))
 right: (17, 3, 1, 842, 4, true, Some(true))
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test flat_counter_payload_stores_the_scoreline ... FAILED
```

Restored `self.kills.unwrap_or(0)`, `touch` + rebuild: **restored_green** (`flat_counter_payload_stores_the_scoreline ... ok`; unit `flat_kills_fold_into_nested ... ok`).

## gate_verdict_tail

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 9825640aadaf recorded: .ai/artifacts/verdicts/T-940.4.json
SLICE GATE: PASS
```

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11434x classes
    Compiling Game scripts took: 1725.545000 ms
    0 warning(s) in TBD sources
```

## files_outside_owns

- `apps/website/api/tests/telemetry.rs` — T-393 IT that asserted shipping deaths stay NULL and a complete flat body 400s. Those asserts *are* the defect. Leaving them would fail `cargo xtask db test-it`. No wave sibling owns this file. Partial nested `counters` 400 and identity-only "not a write" are unchanged.

## found_not_fixed

None in scope. The reporter still cannot measure kills / team_kills / longest_kill_m / vehicles_destroyed / is_command — those nested fields are honest zeros / false / null (ONE LIFE only knows deaths). End / Debrief screens were not touched (T-941.3). Event-array ingest is T-940.13.

## deviations

Edited `tests/telemetry.rs` (see files_outside_owns). No schema / flatten / End-screen edits. No allowlist extension.

## commits

- `9825640aa` `T-940.4: fold flat telemetry counters into nested ingest`
- report commit (this file) after gate

## manual_checklist

- Run a dedicated-server match with the patched mod.
- Confirm the POSTed JSON has a nested `counters` object **and** a top-level `deaths` key.
- Confirm `GET /api/v1/me/deployments` shows the stored deaths (and zeros for unmeasured counters) for a linked player.

## twins_confirmed

Yes. `BuildPlayerRow` nested `counters` emit is byte-identical on framework and export. Export header uses ASCII `--` in place of the framework em-dash; that is the existing ASCII twin rule, not a logic drift.
