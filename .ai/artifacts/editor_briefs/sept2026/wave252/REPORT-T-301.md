# REPORT-T-301

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-301
slice/T-301
```

Verified first: `pwd && git branch --show-current` → worktree path + `slice/T-301`. EnfusionMCP already 19 files; no copy. Dispatch HEAD: `b59b99116`.

## defect_verified_on_main

Ticket `:519` is **not** stale on this tree. Live `BuildKit` is still `TBD_BriefingData.c:519` (pre-slice). It listed seven gear fields; `launcher` / `handgun` / `throwable` were absent from the file.

**claim:** a slot with a launcher, handgun, or throwable never shows those weapons on the briefing kit list.

**path:line:** `apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_BriefingData.c:519-533` (and the export twin). `AddKitLine` for Primary, Optic, Magazine, Uniform, Vest, Helmet, Backpack only.

**command (pre-edit, both trees):**

```
rg -n "AddKitLine|launcher|handgun|throwable|pants|boots|handwear" \
  apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_BriefingData.c \
  apps/mod/tbd-export/Scripts/Game/TBD/UI/TBD_BriefingData.c
```

Paste (framework + export identical AddKitLine set):

```
:527:			AddKitLine(payload, "Primary", gear.primary);
:528:			AddKitLine(payload, "Optic", gear.optic);
:529:			AddKitLine(payload, "Magazine", gear.magazine);
:530:			AddKitLine(payload, "Uniform", gear.uniform);
:531:			AddKitLine(payload, "Vest", gear.vest);
:532:			AddKitLine(payload, "Helmet", gear.helmet);
:533:			AddKitLine(payload, "Backpack", gear.backpack);
:553:	protected static void AddKitLine(...)
```

Zero hits for `launcher`, `handgun`, `throwable`, `pants`, `boots`, `handwear` in either file. Struct fields exist at `TBD_SlotGearStruct` (`TBD_MissionSlotStruct.c:16-18` launcher/handgun/throwable; `:22-24` pants/boots/handwear). Empty skip is `AddKitLine` returning on `resource.IsEmpty()`.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_BriefingData.c` | header `:520-522`; rows `:532-534` | After Primary: Launcher, Handgun, Throwable via existing `AddKitLine` empty-skip. Header documents row order and that pants/boots/handwear stay hidden. |
| `apps/mod/tbd-export/Scripts/Game/TBD/UI/TBD_BriefingData.c` | same CODE (ASCII hyphen in the older header line) | Lockstep twin. |

No new payload fields. No source-scan test (brief: script only, no new files; owns are these two `.c` only). Did not edit schema, flatten, tickets, or docs.

Live row order after ship: Primary, Launcher, Handgun, Throwable, Optic, Magazine, Uniform, Vest, Helmet, Backpack, then Cargo.

## perturbation

No source-scan test added. Broke the new identifier: `gear.launcher` → `gear.launcherT301PERTURB` in both trees, then `cargo xtask mod compile`.

**red_output VERBATIM:**

```
FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/UI/TBD_BriefingData.c:532: Can't find variable 'launcherT301PERTURB'
------------------------------------------------------------
1 error(s) in TBD sources, 2 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:452: Can't find class Tuple2
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:517: Can't find class Tuple2
```

**restored_green:** restored `gear.launcher` in both trees. Same command:

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11434x classes
    Compiling Game scripts took: 854.562000 ms
    0 warning(s) in TBD sources
```

## gate_verdict_tail

`cargo xtask schema validate` → **All contracts valid.**

`cargo xtask platform wave gate --slice T-301` (no `skip:`):

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

  gate verdict PASS @ b59b99116a09 recorded: .ai/artifacts/verdicts/T-301.json
SLICE GATE: PASS
```

Verdict SHA is pre-commit dispatch HEAD. Rust/fmt/clippy steps do not compile Enfusion `.c`; load-bearing proof is `cargo xtask mod compile` on the patched sources (green above). Code commit after the gate: `fe635220a`.

map-engine-core `--all-features` tests not run: this slice did not touch that crate. Wave gate `cargo check` / clippy (changed crates) PASS.

## mod_compile_verdict

Touched `.c` → `cargo xtask mod compile` (EnfusionMCP 19 already present, gitignored, not committed). Restored green as quoted in perturbation.

## files_outside_owns

[]

This report path is required by the slice brief; it is not application code.

## found_not_fixed

[]

Pants/boots/handwear remain unlisted (deliberate). Attachments array is still not a kit row (T-310 / not this ticket).

## deviations

[] vs required ship. Brief claimed ticket `:519` is stale; on this tree `BuildKit` was still `:519` (now `:523` after the header comment).

## commits

- `fe635220a443f52afb70496b11fba710935a29da` — `T-301: list launcher, handgun, throwable on briefing kit`

## manual_checklist

- Boot a mission whose slot carries rifle + launcher + handgun + throwable; the briefing kit list shows all four, in that weapon order, before optic/magazine/wear.
- A slot with only a primary shows no empty Launcher / Handgun / Throwable rows.
- Pants, boots, and handwear still do not appear.
- Kit list still scrolls if the extra rows overflow the panel.

## twins_confirmed

| relative path | framework | export |
|---|---|---|
| `Scripts/Game/TBD/UI/TBD_BriefingData.c` | on disk | on disk; CODE match for the new rows (export keeps ASCII `-` in the pre-existing header sentence) |

No other `.c` files edited.
