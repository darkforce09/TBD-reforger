# REPORT T-936.3 — Editable radio nets and frequencies

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-936.3
slice/T-936.3
```

First actions matched the brief. `export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. Shared-cache `--list` can serve a foreign binary (measured: a `--exact` run of `a_duplicate_frequency_is_refused` ran 0 tests / 984 filtered). Perturbation and full lib tests used `/home/Samuel/.cache/tbd-target-T-936.3`.

## defect_verified_on_main

The worktree is `slice/T-936.3` at merge-base + T-942 packing, i.e. the code `main` had for these paths. Two failing tests were added first and run before the fix.

1. **Authored `radioPlan` is ignored; flatten always derives (T-203).**
   - claim: a payload carrying `radioPlan.nets[0].freqMHz = 41` still emits the four derived ORBAT nets at 30.0 / 30.5 / 31.0 / 31.5.
   - path: `crates/map-engine-core/src/mission/flatten.rs` (`derive_radio_plan` always assigned at the `ModMissionDocument` literal; `EditorPayload` had no `radioPlan` field).
   - command: `cargo test -p map-engine-core --all-features --lib -- an_authored_radio_plan_reaches_the_wire_unchanged -- --exact --nocapture`
   - red (verbatim):
     ```
     assertion `left == right` failed: authored plan is one net, not the derived ORBAT plan: [{"id":"net:blufor_cmd","label":"US Army Command","freqMHz":30.0,"faction":"blufor","range":"long"},{"id":"net:opfor_cmd","label":"Soviet VDV Command","freqMHz":30.5,"faction":"opfor","range":"long"},{"id":"net:blufor_alpha","label":"Alpha","freqMHz":31.0,"faction":"blufor"},{"id":"net:opfor_grom","label":"Grom","freqMHz":31.5,"faction":"opfor"}]
       left: Some(4)
      right: Some(1)
     ```

2. **`tasks[]` dropped at `authored_blocks_root` (T-946.35).**
   - claim: a payload with `tasks: [{id: t-pri, …}]` compiles to a document with no `tasks` key (`Null`).
   - path: `flatten.rs:authored_blocks_root` copied only `winConditions`.
   - command: `cargo test -p map-engine-core --all-features --lib -- authored_tasks_survive_flatten_to_mod_document -- --exact --nocapture`
   - red (verbatim):
     ```
       left: Null
      right: "t-pri"
     ```

## changes

| path | line | why |
|---|---|---|
| `crates/map-engine-core/src/mission/radio_plan.rs` | NEW (parse/validate, `refuse_duplicate_frequency` :67, tests) | Authored-net model; freq 30..=512; duplicate freq/id; net cap 32; net id `^net:[a-z0-9_]+$`. |
| `crates/map-engine-core/src/mission/mod.rs` | 17 | Register `radio_plan`. |
| `crates/map-engine-core/src/mission/extensions.rs` | 75–87, DOCUMENT_OWNED | AUTHORED_BLOCKS row `radioPlan`; DOCUMENT_OWNED so the typed `ModMissionDocument.radio_plan` is not emitted twice. Parse arm + tests `len()==3`. |
| `crates/map-engine-core/src/mission/flatten.rs` | 1400–1404, 1414–1425 | `EditorPayload.tasks` + `radioPlan`; `authored_blocks_root` inserts both. |
| `crates/map-engine-core/src/mission/flatten.rs` | 2632–2658, call site | `resolve_radio_plan`: authored → `mod_plan_from_authored` (id/label/freq/faction/range unchanged); else `derive_radio_plan`. |
| `crates/map-engine-core/src/mission/flatten.rs` | tests after `radio_plan_label_is_capped_at_the_mod_limit` | Authored radio pass-through; T-946.35 tasks survive flatten. |
| `apps/website/frontend/src/editor/panels/radio_panel.rs` | NEW | Net list, faction assignment, range, undoable ops, Reset-to-derived (`radioPlan: null`). Duplicate/out-of-range refused with a message. |
| `apps/website/frontend/src/editor/panels/mod.rs` | 37 | Register `radio_panel`. |

`compile.rs` was not edited (generic `copy_authored_blocks`). No schema change. No `.c`.

## perturbation

Dropped the `refuse_duplicate_frequency(&out, net.freq_mhz, index)?;` call in `radio_plan::parse`. Rebuilt in the private target dir (shared cache had served a binary that did not contain the test).

**red_output VERBATIM:**

```
thread 'mission::radio_plan::tests::a_duplicate_frequency_is_refused' (1582041) panicked at crates/map-engine-core/src/mission/radio_plan.rs:395:10:
duplicate frequency: AuthoredRadioPlan { nets: [AuthoredNet { id: "net:a", label: "A", freq_mhz: 41.0, faction: None, range: None }, AuthoredNet { id: "net:b", label: "B", freq_mhz: 41.0, faction: None, range: None }] }
```

Restored the call, `touch crates/map-engine-core/src/mission/radio_plan.rs`, re-ran: **restored_green** (`ok`. Compiling line present).

## gate_verdict_tail

```
  db_migrate persist       PASS
  T-439 objects aliases    PASS
  T-444 wiki seed          PASS
  T-440 faction library seed PASS
  T-438 deploy-staging     PASS
  T-456 REST size gate     PASS
  T-468 CI schema parity   PASS
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ ebaf06e3693e recorded: .ai/artifacts/verdicts/T-936.3.json
SLICE GATE: PASS
```

Waited on the gate lock held by slice T-133; then check / wasm32 / fmt / clippy / schema all PASS.

## mod_compile_verdict

no .c touched

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `apps/website/frontend/src/editor/panels/settings_modal.rs` | Radio panel is registered and tested but not mounted. T-946.33 owns that one line (`{radio_panel(ctrl)}`), same as T-936.1/.2. Not in this slice's owns. |
| flatten `missionParams` / group AI / vehicle lock-fuel-ammo | Did not fall out of the radio/tasks insert. T-946.36. Compiling a 1.3 payload that authors those keys still omits them from `/compiled`. |
| `crates/map-engine-core/src/mission/tasks.rs` comment (~522) | Still says flatten must not grow a `tasks` field (T-936.2 did not own flatten.rs). Stale; T-133 owns the file. Behaviour is fixed here. |
| `dem::peaks::tests::everon_peaks_max_above_350` | `Decode("Invalid PNG signature.")` in the slice worktree — LFS pointer / missing `packages/map-assets` payload. Brief: environmental; not chased. `--list` 986; run 983 passed + 1 this fail + 2 ignored. |

## deviations

[] — `radioPlan` is DOCUMENT_OWNED (typed field already on `ModMissionDocument`) rather than riding the carrier, so the derived JSON key stays in the same slot and Class-R goldens stay byte-identical. Ticket said "add the radioPlan row + validator"; the withhold list is the existing two-destinations rule.

Ticket verify lines `mk ci-local-leptos` / `mk leptos-gates` were ignored per brief.

## commits

- `ebaf06e3693e1fb551299cd3c9463fe85249bcfb` — T-936.3: authored radioPlan pass-through and restore tasks[] on flatten.

## manual_checklist

- After T-946.33 mounts `{radio_panel(ctrl)}` in Mission Settings: add two nets, set distinct frequencies, save, reload — the same nets are in the bag; `/compiled` `radioPlan.nets` matches (not 30.0+0.5×i).
- Set two nets to the same MHz — the panel shows the collision sentence and does not commit.
- Set a frequency below 30 or above 512 — the panel shows the range sentence and does not commit.
- Click **Reset to derived** — `radioPlan` clears; `/compiled` matches today's T-203 allocation for that ORBAT.
- Dedicated server: an authored `freqMHz: 41` net is the channel `TBD_RadioPlan` serves (not 30.0).

## twins_confirmed

no .c touched — no twin paths.
