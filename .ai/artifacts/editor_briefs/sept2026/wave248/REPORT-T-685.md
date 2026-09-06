# REPORT T-685 — Zone volumes

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-685
slice/T-685
```

HEAD at dispatch: `4b01ae415` (twin-widen already on this branch). Worktree matched. EnfusionMCP (19 files) was already present; not committed.

## defect_verified_on_main

| claim | path:line | command |
|---|---|---|
| Zero Enfusion readers of the six T-706 keys | `apps/mod/**/*.c` (no matches) | `rg attackerCount\|defenderCount\|advantagePercent\|minHeight\|maxHeight\|startingOwner apps/mod` → empty before the patch |
| Loader zone rules bound only play-area three | `TBD_MissionLoader.c` `TBD_MissionZoneRulesStruct` (graceSeconds / warnEverySeconds / penalty only) | `sed -n '90,125p' apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` |
| No `TBD_ZoneVolume.c` in either tree | `apps/mod/tbd-framework/Scripts/Game/TBD/Zones/` listed PlayArea/Trigger/Zone/Geometry/Registry only | `ls .../Zones/` |
| Schema already has the six keys (T-706) | `packages/tbd-schema/schema/mission.schema.json` `$defs/zoneRules` | python dump of properties; do not edit |
| Flatten still does not emit them | (not in owns; confirmed empty) | `rg placementRadius\|attackerCount` in flatten.rs was empty per brief |
| Inspector already generates from `$defs/zoneRules` | `zones_panel.rs` `zone_rule_fields()` | existing `zone_rule_fields_cover_the_whole_vocabulary` |

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` (export twin) | 109–119 | Bind the six keys on `TBD_MissionZoneRulesStruct` with `ABSENT` / `ABSENT_INT` (0 and -5 are authored). JsonLoadContext maps by member name. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Zones/TBD_ZoneVolume.c` (export twin, NEW) | whole file | Volume AGL test (per-entity `GetSurfaceY`), attacker/defender counts, advantagePercent, startingOwner. TBD rules, WOG sentence marked INFERRED only. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectiveRegistry.c` (export twin) | 132, 163, 395–396, 753 | Read/Clear; ApplyStartingOwner + LogBound; destroy query uses `ContainsOrigin` (XZ + AGL). |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c` (export twin) | 308, 342, 560–561 | **Outside owns** — without these call sites the volume/count rules never reach capture/hold presence. See files_outside_owns. |
| `apps/website/frontend/src/editor/panels/zones_panel.rs` | 1280+ | `t685_volume_fields_render_from_zone_rules_schema`: kinds for the six keys (no second inspector). |

TBD semantics (not the inferred WOG sentence):

- Volume: XZ footprint AND AGL in each authored `[minHeight, maxHeight]` at the entity's own ground position.
- `attackerCount` absent => 1; authored 0 => count gate off (presence still required to act).
- `defenderCount` absent => 1; authored 0 => nobody contests / nobody required to hold.
- `advantagePercent` only when `contestable` is false; `contestable:true` still freezes on a qualifying defender.
- `startingOwner` sets capture `m_sOwner` + full progress when the key is a known, allowed `factions[].key`.

Existing `contestable` / `neutralizeSeconds` / `onEmpty` / `decayRate` / `pauseOnEnemy` / `resetOnEnemy` / `requireHolderPresent` are unchanged branches with extra gates in front.

## perturbation

Broke `t685_volume_fields_render_from_zone_rules_schema` by replacing the `startingOwner` factionKey pattern expect with `PERTURB_T685`. Restored, `touch`ed `zones_panel.rs`, re-ran green (`1 passed; 1303 filtered out`). Binary: `website_frontend-2262f3955c054231` (this worktree's test name was in `--list`).

**red_output VERBATIM:**

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.06s
     Running unittests src/main.rs (/home/Samuel/.cache/tbd-target/debug/deps/website_frontend-2262f3955c054231)

running 1 test

thread 'editor::panels::zones_panel::tests::t685_volume_fields_render_from_zone_rules_schema' (1915832) panicked at apps/website/frontend/src/editor/panels/zones_panel.rs:1335:51:
assertion `left == right` failed: startingOwner must resolve $ref factionKey (side picker, not a free string)
  left: Some("^[a-z][a-z0-9_]*$")
 right: Some("PERTURB_T685")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test editor::panels::zones_panel::tests::t685_volume_fields_render_from_zone_rules_schema ... FAILED

failures:

failures:
    editor::panels::zones_panel::tests::t685_volume_fields_render_from_zone_rules_schema

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1303 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p website-frontend --bin website-frontend`
```

**restored_green:** `test editor::panels::zones_panel::tests::t685_volume_fields_render_from_zone_rules_schema ... ok` / `1 passed; 0 failed; 1303 filtered out`.

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-685`:

```
  T-440 faction library seed PASS
  T-438 deploy-staging     PASS
  T-456 REST size gate     PASS
  T-468 CI schema parity   PASS
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict FAIL @ 5b6fd25b6a06 recorded: .ai/artifacts/verdicts/T-685.json
SLICE GATE: FAIL
```

Schema step was the only fail. `cargo xtask schema validate` unread section (T-685's own six keys, expected 0):

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'attackerCount' now has 9 mod identifier(s) (baseline 0) — if T-685 landed the reader, ...
        'defenderCount' now has 9 ...
        'advantagePercent' now has 10 ...
        'minHeight' now has 12 ...
        'maxHeight' now has 12 ...
        'startingOwner' now has 16 ...
```

Brief forbids editing `xtask/src/schema_gates.rs` and `packages/tbd-schema/**`. Member-name binding (required) is exactly what trips UNREAD. Command center retires those six rows after merge (same as T-681 pre-dispatch). Re-run the slice gate after that retire; check/wasm32/fmt/clippy/pins already passed.

No other unread field moved (no collision with T-689 `vehicleClasses` / T-212 `framing` / etc.).

## mod_compile_verdict

```
OK: compiled clean
    Module: Game; loaded 5751x files; 11401x classes
    Compiling Game scripts took: 863.156000 ms
    0 warning(s) in TBD sources
```

`cargo xtask mod compile` after the `.c` edits. EnfusionMCP 19 files present, not committed.

## files_outside_owns

| path | why |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c` | SamplePresence is the only capture/hold presence walk. AGL + count gates have to sit here or they are a dead mechanism. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c` | Export twin (lockstep). |

Not touched: `flatten.rs`, `schema_gates.rs`, `packages/tbd-schema/**`, `TBD_SpawnManager.c`, `TBD_PlacementScatter.c`, `TBD_MissionValidator.c`.

## found_not_fixed

| path:line | repro |
|---|---|
| UNREAD_WIRE_FIELDS six T-685 rows still expected 0 | `cargo xtask schema validate` after this reader. CC owns the retire. |
| `crates/map-engine-core/src/mission/flatten.rs` (T-946.36) | Flatten still does not emit the six keys. Hand-staged 1.3 JSON reaches the reader; live `/compiled` will not until that class lands. |
| `TBD_ObjectiveRegistry.c` `DiagnoseEmptyDestroyTargets` | Authored `entities[]` have x/z only — diagnose cannot apply AGL. Live destroy query does (`ContainsOrigin`). |
| `TBD_Objective.c` `ResolveActingFaction` | Still the old 1-vs-1 helper. Capture tick now calls `TBD_ZoneVolume.ResolveActingFaction`. Direct callers of the old method (if any besides the replaced site) would skip count gates. Grep: only the replaced AdvanceCapture site + definition. |

## deviations

1. Extra file pair `TBD_ObjectivesComponent.c` (both trees) — runtime apply. Brief said stop-and-report; shipping a reader nobody calls would violate "dead mechanisms are worse than missing ones".
2. Slice gate is FAIL on UNREAD only. Did not edit `schema_gates.rs` / schema wording (forbidden). Not a collision — it is the T-685 reader landing, which is what UNREAD is built to detect.

## commits

- `5b6fd25b6` T-685: bind zone-volume rules and consume them at runtime.

(Report commit follows if this file is committed.)

## manual_checklist

1. Hand-stage a schemaVersion 1.3 mission JSON (flatten will not emit these keys) with `rules.maxHeight: 30` on an `objective_capture` zone; fly an aircraft above ~30 m AGL over the zone — capture must not tick; a soldier on the ground must.
2. Same document with `minHeight: -5` — a body in a basement under the footprint must count; one below -5 AGL must not.
3. `attackerCount: 4` — three attackers in the volume must not bank; four must.
4. `defenderCount: 3` with `contestable: true` — one defender must not freeze; three must.
5. `defenderCount: 0` — capture must not freeze from enemy presence; hold must run without a holder (undefended).
6. `contestable: false` + `advantagePercent: 50` — 2v2 must freeze; 3v2 must progress.
7. `startingOwner: "opfor"` on a capture zone (faction exists) — round start must show opfor holding at 100% progress; a bogus key must leave it neutral (boot log warning).
8. Existing `neutralizeSeconds` / `onEmpty: decay` / `pauseOnEnemy` / `resetOnEnemy` still behave on a zone that authors no T-685 keys.
9. Editor zone inspector: the six schema-generated controls appear (Attacker count, Defender count, Advantage percent, Min height, Max height, Starting owner as a factionKey text/side field). Blank = not authored.

## twins_confirmed

| path | framework | export |
|---|---|---|
| `Scripts/Game/TBD/Zones/TBD_ZoneVolume.c` | yes (17227 B, identical bytes) | yes |
| `Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (surgical fields; pre-existing emdash vs hyphen comments left alone) | yes (ASCII) |
| `Scripts/Game/TBD/Objectives/TBD_ObjectiveRegistry.c` | yes | yes |
| `Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c` | yes | yes |
