# REPORT — T-212 · Typed per-side objectives with attributes · wave 256

> Transcribed by the command centre from the slice agent's returned text: its harness blocks
> subagents from writing report `.md` files. Content verbatim; every claim independently re-checked
> before acceptance (see the verification note at the end).

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-212
slice/T-212
```

The brief is **not present in the worktree** — `.ai/artifacts/` is gitignored, so nothing under it was
ever committed. It was read from the main checkout **read-only**; no file in the main checkout was
written.

## defect_verified

**1 — the spine has no reader in the lane that owns objectives.** Counted with the gate's own
`count_mod_readers` semantics (comments and string literals stripped) over
`apps/mod/tbd-framework/Scripts/Game/TBD/Objectives`:

| `$defs/objective` property | tree-wide | Objectives lane |
|---|---|---|
| `id` | 206 | 10 |
| `type` | 54 | 1 |
| `side` | 59 | **0** |
| `zoneId` | 45 | 4 |
| `label` | 111 | **0** |
| `framing` | **0** | **0** |
| `lock` | 6 (all `TBD_VehicleState.c`) | **0** |
| `autoLose` | **0** | **0** |
| `variantId` | 18 | **0** |
| `title` / `text` | 32 / 91 | 3 / 24 |

Six of eleven had no identifier at all in the lane. The tree-wide column is why the assertion is
lane-scoped: `id` 206 and `label` 111 are unrelated subsystems and would make a whole-tree check pass
vacuously. The counting replica was validated against the live pins first — it reproduces
`objectives` 13, `framing` 0, `autoLose` 0, `lock` 6 exactly.

**2 — it is structural.** `TBD_MissionDocumentStruct` (`TBD_MissionLoader.c:425-458`) declares no
`objectives` field, and `:380` says so outright: *"objectives[] / editorTriggers[] are NOT on
TBD_MissionDocumentStruct; their readers re-parse GetRawJson()"*. `JsonLoadContext` binds by member
name, so an unspelled key is invisible at runtime.

**3 — a runnable RED, committed first (`32b978a71`)**, verbatim on the pre-change tree:

```
thread '...objective_spine_is_read_in_the_objectives_lane' panicked at xtask/src/schema_gates.rs:4389:9:
$defs/objective properties with NO identifier under .../apps/mod/tbd-framework/Scripts/Game/TBD/Objectives: ["side", "label", "framing", "lock", "autoLose", "variantId"]
JsonLoadContext binds by member name, so an unspelled property is unreadable. Either the reader lost a field, or the schema grew one T-212's reader has not taken up yet.
```

Baseline `cargo xtask mod compile` before any edit: `OK: compiled clean / 5761x files / 11484x
classes / 0 warning(s)` — matches the brief.

## changes

### `TBD_ObjectiveRegistry.c` (both trees) — the reader

- `TBD_ObjectiveFramingSideStruct` (`title`, `text`), `TBD_ObjectiveFramingStruct` (`attacker`,
  `defender`), `TBD_ObjectiveEntityStruct` (the nine-property spine), `TBD_ObjectiveEntityDocStruct`
  (declares `objectives` and nothing else).
- `TBD_ObjectiveEntityReader` — the **third** typed `JsonLoadContext` pass over `GetRawJson()`, after
  `TBD_ObjectiveRulesReader` and `TBD_TriggerRuntime`, for the same two documented reasons.
- Rows join zones by **`zoneId`, never by index**. `TBD_ObjectiveRulesReader` joins two passes over
  the *same* array where index equality is a parser property; this joins two *different* arrays.
- T-654 variant gating on `GetActiveVariantIds()`, which the loader explicitly requires of
  `GetRawJson()` readers. Null (no `variants[]`) = everything runs.
- `WarnDuplicateZoneIds()` refuses the FNF v4 shape the schema bans: two rows for one `zoneId` warns
  quoting *ONE ENTITY, TWO FRAMINGS*; first row wins deterministically.
- `ReportUnclaimed()` names every row that bound to nothing — that "carried on the wire and does
  nothing" state is the ticket's whole subject.
- `BindTypedEntity` + `ApplyFraming`/`ApplyTypedLabel`/`CheckTypedKind`/`CheckTypedSide`/
  `CheckAutoLose`/`SeedHolderFromSide`, called from `Prepare()` **before** the kind-specific rules.
- `LogTyped()`, `ReportTypedCoverage()`; `Clear()`/`Build()` wired. `BuildBoardForFaction` emits the
  framed side's task text as a continuation line.

### `TBD_Objective.c` (both trees) — the per-side runtime

`enum TBD_EObjectiveRole {NEUTRAL, ATTACKER, DEFENDER}`; eleven typed fields; `RoleOf`, `HasFraming`,
`TitleFor`, `TaskTextFor`; `BoardLine` titles per side. **No existing signature changed**, so every
call site outside the owns compiles unmodified.

**Decisions (the shape the ticket asked to be recorded):**

| Question | Decision |
|---|---|
| Typed row vs zone | **Overlay.** `Build()` still walks prepared zones only; untyped behaves exactly as before. |
| Type disagreement | **The zone wins**, named at load. A task that doesn't match the rules applied is a lie to the player. |
| Which side is which | `capture`/`destroy` → `side` attacks; `hold`/`defend` → `side` defends. Untyped falls back to kind. A third faction reads the attacker framing. |
| `side` vs `zones[].faction` | Both kept, different claims. Faction = ownership restriction; side = who the task is for. Difference is a note. |
| `side` seeding a holder | `HOLD_UNTIL` only (empty faction = INERT). **Excluded for CAPTURE**, where empty means "anyone may own". |
| Identity | `objectives[].id` in `m_sEntityId`, apart from the zone id. Stable, never positional. |

### `xtask/src/schema_gates.rs`

`objectives` re-pinned **13 → 16**; `framing` (0→10) and `autoLose` (0→2) **retired** (see
deviations); `unread_gate_fires_when_a_reader_appears` rebuilt on `objectives` and now bidirectional;
two new test modules at the **bottom** of the file.

### `packages/tbd-schema/schema/mission.schema.json` — PROSE ONLY

Five descriptions said "NOTHING reads this on any shipped build" — now false, and the gate's own
failure text asks for this edit. Proven prose-only two ways: `git diff -U0` shows 5 `+` / 5 `−` lines,
**every one a `"description":` key**; and both revisions parsed with every `description` deleted
recursively are **identical**. `cargo xtask schema validate` → `All contracts valid.`

## perturbation (RED verbatim)

**P1 — the new spine test.** `string autoLose;` → `string autoLoseZZ;`, both trees.
```
thread '...objective_spine_is_read_in_the_objectives_lane' panicked at xtask/src/schema_gates.rs:4436:9:
$defs/objective properties with NO identifier under .../apps/mod/tbd-framework/Scripts/Game/TBD/Objectives: ["autoLose"]
JsonLoadContext binds by member name, so an unspelled property is unreadable. Either the reader lost a field, or the schema grew one T-212's reader has not taken up yet.
```
Restored + `touch` → `test result: ok. 2 passed; 0 failed`.

**P2 — `mirror_lockstep`.** One `static const string TASK_PERTURBATION` line in the **framework copy
only**.
```
FAIL: tbd-framework and tbd-export are not in lockstep
      (scripts compared as code + string literals, with the ASCII rule's punctuation
       folded away; every other shared path compared byte-for-byte)
  Scripts/Game/TBD/Objectives/TBD_ObjectiveRegistry.c: the two copies differ in code or in a string literal
      The engine compiles the tbd-framework copy (T-946.23) and the mirror ships too,
      so a divergence here is code that no gate reads. Make the two copies agree.
```
Restored + `touch` → `OK: compiled clean / 5761x files / 11492x classes / 0 warning(s)`.

**P3 — the re-pinned `objectives` baseline.** One extra identifier, count 16 → 17.
```
thread '...all_1_3_fields_are_unread_on_the_live_tree' panicked at xtask/src/schema_gates.rs:2724:9:
a 1.3 wire field is no longer unread:
  [
    "'objectives' now has 17 mod identifier(s) (baseline 16) — if T-212 landed the reader, DROP the field's \"no reader on any shipped build\" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing '13 are TBD_ObjectivesComponent's own objective-list field, unrelated to the mission-doc objectives[] array; 3 are T-212's TBD_ObjectiveEntityReader binding that array' identifier, re-pin the baseline here on purpose",
  ]
```
Restored + `touch` → `unread_wire_field_tests`: `5 passed; 0 failed`; counts back to 16 / 10 / 2.

*Finding from the first P3 attempt:* naming the extra member `objectives_perturbation` did **not**
move the count — `count_mod_readers` matches `\bobjectives\b` and `_` is a word character. Not a
defect (the gate measures JSON-bindable member names), but a future re-pin must not be trusted on a
suffixed identifier.

**P4 — the staged-golden binding proof.** `string zoneId;` → `string zoneIdZZ;`, both trees.
```
thread '...the_staged_1_3_golden_objectives_row_binds_to_the_reader' panicked at xtask/src/schema_gates.rs:4568:9:
the staged 1.3 golden authors objectives[] keys the reader declares no member for: ["zoneId"]
JsonLoadContext binds by member name, so those keys are invisible at runtime — the document says one thing and the round does another.
```
Restored + `touch` → `test result: ok. 3 passed; 0 failed`.

## gate_verdict_tail

```
═══ slice gate T-212 ═══
touch_workspace: invalidated 678 workspace .rs file(s) and 134 include_str!/include_bytes! input(s) across 9 member(s)
  cargo check              PASS
  wasm32 (frontend)        PASS
  fmt (changed)            PASS
  clippy (changed crates)  PASS
  test (frontend, changed) PASS
  schema                   PASS
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

  gate verdict PASS @ 1e65fc1e263a recorded: .ai/artifacts/verdicts/T-212.json
SLICE GATE: PASS
```

`cargo xtask mod compile` — final, after every restore:
```
OK: compiled clean
    Module: Game; loaded 5761x files; 11492x classes
    Compiling Game scripts took: 925.888000 ms
    0 warning(s) in TBD sources
```
Baseline was 11484 classes; +8 for the new enum and five structs. Files unchanged at 5761 (no new
`.c` file). `mirror_lockstep` runs unconditionally after a clean compile and did not fire;
`ascii_check_export` did not fire, and `grep -P '[^\x00-\x7F]'` over both export files returns 0.

## files_outside_owns

**[]** — none. All four commits touch exactly the six owned paths. `TBD_ObjectivesComponent.c` was
**not touched in either tree**. The 19 gitignored `EnfusionMCP` `.c` files were copied in per the
brief and are not committed (`apps/mod/.gitignore:28`).

## found_not_fixed

1. **The per-side title does not reach the live HUD.** `TBD_ObjectivesComponent.c:823
   titles.Insert(objective.DisplayName());` and `:836 barLabel = objective.DisplayName();`. Changing
   `:823` to `TitleFor(factionKey)` puts the per-side title on the HUD — `factionKey` is resolved at
   `:815` and already passed to `StatusText` at `:824`. **That file is T-946.55's owns this wave**;
   follow-on is one line per tree.
2. **The board path this slice made per-side has no consumer today.** `grep -rn
   "BuildBoardForPlayer" --include='*.c' apps/mod/` returns **2 hits, both the definition**
   (`TBD_ObjectivesComponent.c:738` in each tree) — no caller. Per-side text reaches the seam and the
   load log; it reaches a *player* only once item 1 lands.
3. **`objectives[]` still does not reach `/compiled`.** `ModMissionDocument` in `flatten.rs` has no
   such field (0 hits in the struct; the word appears once in the file, a doc comment at `:1411`).
   That is **T-946.36**, out of owns. Proven against the hand-staged golden instead — the T-685
   precedent. No live wire is claimed.
4. **`autoLose` validated and reported, not enacted.** Acting on it ends a round, and that authority
   is `TBD_FrameworkManager.TickWinConditions` — outside owns. A dangling key is blanked.
5. **`lock` parsed, counted, reported, deliberately not enforced.** `_Lock` says "at round start" and
   the corpus has no evidence of what unlocks one (`wmt_main` absent, wog.md:88-95). A permanent
   exclusion would be a different parameter wearing this name and would silently make a
   single-objective mission unwinnable. The board stays truthful; the boot log states it.
6. **Variant gating cannot distinguish deselected from dangling.** `IsVariantRowIncluded` is
   `protected` (`TBD_MissionLoader.c:1500`), another lane. Excluded rows are counted together and
   logged, with the limit stated in code.
7. **No SPA authoring UI for `objectives[]`** — the ticket's own follow-on; the shape it needs is now
   decided and recorded above.

## deviations

**One, from the brief's literal wording.** The brief and acceptance say re-pin all three rows.
`objectives` was re-pinned 13 → 16 as asked. **`framing` and `autoLose` were RETIRED instead**,
because re-pinning would make the table assert something untrue:

- `UNREAD_WIRE_FIELDS` documents that a non-zero baseline means *"these hits are a pre-existing
  UNRELATED identifier"*, and `nonzero_baselines_explain_the_pre_existing_identifier`
  (`schema_gates.rs:2787`) **enforces that in the `why` wording**. For `framing` 10 and `autoLose` 2
  every hit is T-212's own reader, so any `why` satisfying the test would be false.
- The repo has made this call twelve times: T-674 retired `callsign`/`tag` for exactly this reason
  ("smaller than a table that lies"); T-676/677/678/679/680/681/685/689/705/654/682/684 all retired
  clean-0 rows on landing a reader. `objectives` re-pins because its 13 genuinely *are* unrelated —
  the `seats`/`size`/`gadgets` class.
- The gate's own failure text offers both: *"remove/repin its UNREAD_WIRE_FIELDS row"*.
- The tripwire is inverted and strengthened, not lost: `t212_objective_spine_tests` asserts the
  **complement** — every `$defs/objective` property must KEEP a reader in the objectives lane — so
  deleting the reader is a red rather than a silent regression back to a dead container.

Consequence handled: `unread_gate_fires_when_a_reader_appears` was keyed on `framing` and no clean-0
row survives, so the fire-once proof was rebuilt on `objectives` and now runs in both directions (a
tree *at* the baseline must not be reported; one more identifier must trip it) — strictly stronger
than what it replaces.

No other deviations. No work deferred. No subagents, no ship, no merge, no push.

## commits

```
1e65fc1e2  T-212: prove the staged 1.3 golden reaches the reader
33baa8c7e  T-212: re-pin the wire gate and drop the "no reader" prose
1b3643d8b  T-212: TBD_ObjectiveRegistry reads typed per-side objectives
32b978a71  T-212: pin the typed-objective spine as READ — failing test first
```

Branch `slice/T-212`, working tree clean. Gate verdict recorded at `1e65fc1e263a`.

## manual_checklist

1. **The ticket's acceptance: an objective reads differently to attacker and defender in game.** Load
   a hand-staged 1.3 document (start from `golden-missions/schema-1_3-wire-fields.json`), join as
   `blufor` and as `opfor`, confirm the board titles differ ("Seize the hilltop" vs "Hold the
   hilltop") with the framed side's task text on the next line. **Blocked until `found_not_fixed`
   item 1 lands** if checked via the HUD rather than the board seam.
2. **Boot log (no live round needed).** Confirm `[TBD][ObjTyped] typed rows=1 bound=1 framed=1
   locked=0 autoLose=1`, one `objectiveTyped` line and two `objectiveFraming` lines — those strings
   come off a parsed document, which is the proof a compile cannot give.
3. **Regression:** run any pre-1.3 golden; capture/destroy/hold and the board must be unchanged.
4. **Duplicate-row refusal:** add a second `objectives[]` row with the same `zoneId`; expect one
   warning quoting the schema's ban, first row winning.
5. **Unclaimed-row report:** point a row at a non-existent `zoneId`; expect it named as inert.
6. **Variant gating:** with a `variants[]` registry present, a row whose `variantId` is not active
   must be excluded and counted.

---

## Command-centre verification (not the agent's words)

Re-checked independently before acceptance, 2026-09-08:

- `files_outside_owns []` — **true**. Six owned paths, nothing else. `TBD_ObjectivesComponent.c`
  touched **0** times in either tree, as required (it is T-946.55's this wave).
- Both Enfusion twins changed symmetrically: `TBD_Objective.c` +176 each, `TBD_ObjectiveRegistry.c`
  +717 each.
- **`mission.schema.json` prose-only claim — verified.** Every changed line in that file's diff is a
  `"description"` key; the count of changed non-description lines is **0**.
- **The deviation is legitimate.** `nonzero_baselines_explain_the_pre_existing_identifier` genuinely
  exists (`schema_gates.rs:2834`) and does enforce the `why` wording, so re-pinning `framing` and
  `autoLose` would have required a false justification string. Confirmed on the slice tree: only the
  `objectives` row survives in `UNREAD_WIRE_FIELDS`, and the live counts are `objectives` 16,
  `framing` 10, `autoLose` 2 — matching the report exactly.
