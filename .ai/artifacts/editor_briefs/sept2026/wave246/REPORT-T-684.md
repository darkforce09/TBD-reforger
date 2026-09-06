# REPORT-T-684

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-684
slice/T-684
```

First command this session: `pwd && git branch --show-current` matched that path and branch. Work stayed in this worktree.

## defect_verified_on_main

Verified on this worktree at `475aec391` (slice base, before any T-684 commits) — that is main as of wave 246 dispatch.

| claim | path:line | command |
|---|---|---|
| No Enfusion `missionParams` reader | (identifier absent) | `rg -n "missionParams\|MissionParam" apps/mod --glob '*.c'` — zero hits |
| `TBD_MissionParams.c` does not exist | (file absent) | `ls apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionParams.c` — No such file |
| `TBD_MissionDocumentStruct` has no `missionParams` member | `TBD_MissionLoader.c:310-360` (struct ended at `environment`) | `rg -n "class TBD_MissionDocumentStruct" apps/mod --glob '*.c'` — present; no `missionParams` field |
| T-706 already declared the wire key | `packages/tbd-schema/schema/mission.schema.json:125` (`missionParams` array, `$defs/missionParam`) | `rg -n '"missionParams"' packages/tbd-schema/schema/mission.schema.json` |
| UNREAD_WIRE_FIELDS baseline 0 | `xtask/src/schema_gates.rs:2630-2636` `expected: 0, ticket: "T-684"` | left untouched |

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionParams.c` | NEW (401 lines) | Struct bind (`name`, `titleKey`, `values`, `displays`); `Get(symbol)` / `Has(symbol)` / `Count()`; launch file `$profile:TBD_MissionParams.json`; fail-closed unknown symbol |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionParams.c` | NEW | Byte-identical twin |
| same, `TBD_MissionParamStruct.authoredDefault` | 65 | Wire key `default` cannot be an Enforce member (keyword; compile probe RED "Syntax error" / "Unexpected scope"). Second `JsonLoadContext` pass `ReadValue("default", authored)` at 227 — key is a STRING. Index join with the primary parse. |
| same, `Resolve` / `FillAuthoredDefaults` / `LoadLaunchSelections` | 100 / 200 / 355 | Allocate-on-absent then `Count()`; server-config selections else authored default if it is in `values[]` |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 364, 1103 | `missionParams` on `TBD_MissionDocumentStruct`; `TBD_MissionParams.Resolve()` after `TBD_EnvironmentReader.Apply()` |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | 364, 1103 | Same code |

Launch-selection surface (honest): **server config** `$profile:TBD_MissionParams.json` `{ "selections": [ { "name": "<symbol>", "value": <int> } ] }`, else the authored default. There is no lobby.

Not touched: `packages/tbd-schema/**`, `flatten.rs`, `extensions.rs`, `xtask/src/schema_gates.rs`, `.ai/tickets/`, T-678 / T-680 owns.

## perturbation

Broke the loader hook the compile guards: renamed `class TBD_MissionParams` to `class TBD_MissionParamsX` in the framework copy only. RED output VERBATIM:

```
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Backend/TBD_MissionLoader.c:1103: Can't find variable 'TBD_MissionParams'
------------------------------------------------------------
1 error(s) in TBD sources, 11 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:324: Error in parameters
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:333: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:335: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Syntax error
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetCountEntitiesInTrigger.c:10: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetLastFinishedTaskLayer.c:11: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:25: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:27: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:452: Can't find class Tuple2
  … 1 more
```

Restored by copying the untouched export twin over the framework file, `touch` both `TBD_MissionParams.c` copies and both `TBD_MissionLoader.c` copies.

**restored_green:** `cargo xtask mod compile` → `OK: compiled clean` / `Module: Game; loaded 5747x files; 11371x classes` / `0 warning(s) in TBD sources`.

Separate compile probe (not left in the tree): `int default = ABSENT` as a class member → `TBD_MissionParams.c:2: Syntax error` / `TBD_MissionParams.c:3: Unexpected scope`. That is why the wire key is read as a string.

No cargo tests were added (owns are Enfusion `.c` only).

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-684`:

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

  gate verdict FAIL @ 57c35e19d021 recorded: .ai/artifacts/verdicts/T-684.json
SLICE GATE: FAIL
```

The only failing step was `schema`. Height-labels SKIP is the worktree LFS-pointer DEM (environmental, as briefed). Schema validate's unread-fields check (`cargo xtask schema validate`):

```
T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):
  FAIL  a 1.3 wire field gained a reader — update mission.schema.json
        'missionParams' now has 8 mod identifier(s) (baseline 0) — if T-684 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'clean' identifier, re-pin the baseline here on purpose
```

This is the T-682 / T-674.2 shape: the slice gate went RED **because the ticket succeeded**. `JsonLoadContext` binds by field name, so the identifier `missionParams` IS the contract. Brief owns-list + "Do not touch packages/tbd-schema" + "Do NOT edit xtask/src/schema_gates.rs" forbids the two files the gate's own message names. Did not widen.

`fmt (changed)` and `clippy (changed crates)` PASS. Diff is Enfusion `.c` only.

## mod_compile_verdict

```
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)
OK: compiled clean
    Module: Game; loaded 5747x files; 11371x classes
    Compiling Game scripts took: 853.049000 ms
    0 warning(s) in TBD sources
```

EnfusionMCP: 19 `.c` files present in the worktree, not committed.

## files_outside_owns

[] — no extra files edited.

Required for `SLICE GATE: PASS` (not edited; command centre):

- `xtask/src/schema_gates.rs` — retire the `missionParams` `UNREAD_WIRE_FIELDS` row (baseline was 0 — retire, do not re-pin)
- `packages/tbd-schema/schema/mission.schema.json` — drop "NOTHING reads it on any shipped build" on `missionParams` / `$defs/missionParam` (brief forbade this file)

## found_not_fixed

| path:line | repro |
|---|---|
| `xtask/src/schema_gates.rs:2630` (`UNREAD_WIRE_FIELDS` `missionParams` expected 0) | `cargo xtask schema validate` after this reader: `missionParams` 8 identifiers. Same command the slice gate runs. |
| `packages/tbd-schema/schema/mission.schema.json:125` and `:1037` | Still says nothing reads `missionParams` on any shipped build. That sentence is now false. |
| `crates/map-engine-core/src/mission/flatten.rs` | Zero `missionParams` hits. Editor flatten does not emit the array. Brief forbade this file (T-946.35). Hand-staged 1.3 JSON / golden `schema-1_3-wire-fields.json` still reach the reader. |

## deviations

- Brief asked for `SLICE GATE: PASS`. Gate is FAIL on schema unread-fields **because the reader landed**. Owns list + "do not touch packages/tbd-schema" + "Do NOT edit schema_gates.rs" is the T-682 instruction; followed that over the PASS line.
- Plan/spec still say `params[]`. Bound **`missionParams[]`** as the command-centre note required.
- Wire key `default` cannot be an Enforce member. Recovered with `ReadValue("default", authored)` on a second `JsonLoadContext` pass (string key, compile-probed `StartArray(name, count)` + `StartObject("")`). Primary pass still binds the other keys by member name.
- No editor UI (not in owns). No lobby.

## commits

- `57c35e19d02167b35d9862dfc06cccf5bf9e7d29` T-684: Enfusion reader — missionParams[] and Get(symbol)

HEAD `57c35e19d`. Worktree clean. No push.

## manual_checklist

- Dedicated-server boot `packages/tbd-schema/golden-missions/schema-1_3-wire-fields.json` (authored `time_of_day` default `1`, values `[0,1,2]`). Confirm log `[TBD][MissionParams] symbol='time_of_day' value=1 source=default` and that `TBD_MissionParams.Get("time_of_day")` is `1` (headless compile cannot prove this).
- Same mission, write `$profile:TBD_MissionParams.json` as `{ "selections": [ { "name": "time_of_day", "value": 2 } ] }`, reboot **without re-baking**. Confirm `value=2 source=launch`.
- `TBD_MissionParams.Get("no_such_symbol")` logs `unknown symbol -- fail closed` and returns `0`; `Has("no_such_symbol")` is false.
- Dedicated-server boot a 1.1/1.2 mission with no `missionParams` key. Confirm no `[TBD][MissionParams] symbol=` lines and boot unchanged.
- Optional: `displays` length != `values` length logs `labels will not pair` and Get still returns the integer.

## twins_confirmed

| path | on disk |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionParams.c` | yes (13689 bytes, ASCII) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionParams.c` | yes (13689 bytes, `cmp` identical) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (edited) |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_MissionLoader.c` | yes (edited; new hunks ASCII and code-identical) |
