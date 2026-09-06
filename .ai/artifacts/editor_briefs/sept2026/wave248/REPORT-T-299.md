# REPORT T-299 — Single-faction compile ships a phantom opfor

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-299
slice/T-299
```

Verified before any edit: `pwd && git branch --show-current` → worktree path + `slice/T-299`. HEAD at dispatch: `4b01ae415` (twin-widen already on the branch).

## defect_verified_on_main

Verified on this slice HEAD (`4b01ae415`, the widened main) **before** deleting the pad. Ticket/plan line numbers (`flatten.rs:2470-2472`) were stale; the pad was at `flatten.rs:3597-3612`.

- **claim:** a one-faction editor graph compiles with a stub `opfor` padded into `factions[]` so briefing/ORBAT show a side nobody can join.
- **path:line:** `crates/map-engine-core/src/mission/flatten.rs:3597-3612` (`// Schema requires ≥ 2 factions; pad a stub opposing faction for single-faction drafts.`). Schema `properties.factions.minItems` was `2` at `packages/tbd-schema/schema/mission.schema.json:70`.
- **command:** added `mission::flatten::tests::single_faction_compile_does_not_pad_a_phantom_opfor` (asserts compiled `factions` keys `["blufor"]`), then:

```
cargo test -p map-engine-core --all-features --lib \
  mission::flatten::tests::single_faction_compile_does_not_pad_a_phantom_opfor -- --exact --nocapture
```

`--list` showed that one test (1 test, 0 benchmarks) on binary `map_engine_core-374f4772eaee4d5d` compiled from this worktree. Failure (pad still present):

```
assertion `left == right` failed: phantom side padded into factions[]: ["blufor", "opfor"]
  left: ["blufor", "opfor"]
 right: ["blufor"]
```

## changes

| path | line | why |
|---|---|---|
| `packages/tbd-schema/schema/mission.schema.json` | 70 | Surgical `factions.minItems` `2` → `1`. Other `minItems: 2` (`:184` playerRange, `:577`) left alone. File stayed 1433 lines (no `json.dumps` rewrite). |
| `crates/map-engine-core/src/mission/flatten.rs` | was 3597-3612; production now continues at 3596 `let terrain = …` | Deleted the pad. One-faction compile emits exactly the authored sides. |
| `crates/map-engine-core/src/mission/flatten.rs` | 3508-3510 | Radio harvest comment no longer refers to a stub faction that is not invented. |
| `crates/map-engine-core/src/mission/flatten.rs` | 3598+ | `faction_eliminated` still only when `sides_holding_slots >= 2`. Not reintroduced as an unconditional second side. |
| `crates/map-engine-core/src/mission/flatten.rs` | 5647 `single_faction_compile_does_not_pad_a_phantom_opfor` | Class-R: one faction, ORBAT/briefings have no `opfor`, wire `factions.len()==1`, no `faction_eliminated`. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c` | CheckFactions close ~340 | Dropped `declared.Count() == 1` warning (`mission.schema.json expects at least two`). Empty-side warning kept at 769. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionValidator.c` | same relative close ~340 | Twin of the framework edit (ASCII hyphen already in this tree). Empty-side warning kept at 769. |

Two-faction Class-R: `compiler_shaped_golden_is_a_fresh_emitter_output` **ok** (byte-identical). `cargo test -p map-engine-core --all-features mission::flatten` → **91 passed, 0 failed, 2 ignored** (manual dump/regen only; no `skip:`).

`cargo xtask schema validate` → **All contracts valid.** `schema-codegen` not run (`mission.schema.json` is not a typify target).

## perturbation

Restored the pad block at `flatten.rs:3596`, rebuilt (`Compiling map-engine-core`), ran the same exact test.

**red_output VERBATIM:**

```
   Compiling map-engine-core v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-299/crates/map-engine-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.33s
     Running unittests src/lib.rs (/home/Samuel/.cache/tbd-target/debug/deps/map_engine_core-374f4772eaee4d5d)

running 1 test

thread 'mission::flatten::tests::single_faction_compile_does_not_pad_a_phantom_opfor' (1872724) panicked at crates/map-engine-core/src/mission/flatten.rs:5681:9:
assertion `left == right` failed: phantom side padded into factions[]: ["blufor", "opfor"]
  left: ["blufor", "opfor"]
 right: ["blufor"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test mission::flatten::tests::single_faction_compile_does_not_pad_a_phantom_opfor ... FAILED

failures:

failures:
    mission::flatten::tests::single_faction_compile_does_not_pad_a_phantom_opfor

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 996 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p map-engine-core --lib`
```

**restored_green:** `git checkout -- crates/map-engine-core/src/mission/flatten.rs` then `touch crates/map-engine-core/src/mission/flatten.rs` (stale-mtime trap). Rebuilt (`Compiling map-engine-core` again) and the same test **ok**. Tree back to commit `159b25a83`.

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-299`:

```
  T-278 catalogue drift    PASS
  db_migrate claim body    PASS
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

  gate verdict PASS @ 159b25a83a4b recorded: .ai/artifacts/verdicts/T-299.json
SLICE GATE: PASS
```

## mod_compile_verdict

Touched `.c` → ran `cargo xtask mod compile` (EnfusionMCP 19 files already present, gitignored, not committed):

```
OK: compiled clean
    Module: Game; loaded 5750x files; 11397x classes
    Compiling Game scripts took: 856.682000 ms
    0 warning(s) in TBD sources
```

## files_outside_owns

[]

## found_not_fixed

[]

Grep of `apps/mod/**/*.c` for `factions[1]`, `GetFactionAt`, `factionKeys[1]`: no hits. Briefing/ORBAT iterate declared factions; with the pad gone they do not invent a second side. The remaining `factions/1` notes in `cargo xtask schema validate` kit-alias output are two-faction golden `/factions/1/presetId` paths, not a hard-coded second side.

## deviations

[] vs this brief.

Ticket `verify` named `cargo xtask ci schema-validate` and `cargo xtask ci schema-codegen`. Obeyed the command-center brief: `cargo xtask schema validate` only; no codegen.

## commits

- `159b25a83a4b8cb4d168bf1320af85da305a6c3f` — `T-299: stop padding a phantom opposing faction.`

## manual_checklist

- Boot a one-faction mission on a dedicated server; briefing screen lists one side only.
- Same boot: ORBAT / slotting UI lists one side only (no empty `opfor`/`blufor` phantom).
- `#tbd validate` on that mission does **not** warn `only one faction is declared; mission.schema.json expects at least two`.
- `#tbd validate` on a **declared** empty side (author listed a faction with zero slots) still warns `declared but has no slots — nobody can play this side`.
- Boot a two-faction mission; briefing and ORBAT still show both sides.

## twins_confirmed

| relative path | framework | export |
|---|---|---|
| `Scripts/Game/TBD/Backend/TBD_MissionValidator.c` | `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c` on disk | `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionValidator.c` on disk |

No other `.c` files edited.
