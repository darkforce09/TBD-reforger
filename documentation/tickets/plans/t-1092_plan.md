**Status:** live

# T-1092 — Plan

## Context

Execution document for [T-1092](/documentation/tickets/specs/t1092_mod_script_modularisation.md): the sub-agent roster and every launch prompt. Progress, amendments and the shared-helper index live in the [progress checkpoint](/documentation/mod/script_modularisation_progress_checkpoint.md).


The mod scripts break two project laws.

**Law 7: files may not exceed 500 lines.** 52 `.c` files exceed it:
- 36 in tbd-framework
- 15 in tbd-export
- 1 vendored file in tbd-emcp

The largest is `TBD_SpawnManager.c` at 4231 lines. No gate catches this, because `file-length` walks only `.rs` files (`node_and_file_limits/repository_access.rs:70`).

**Law 8: comment standard.** The framework audit covered 172 files and 67.5k lines. It found:
- 169 files with no `/** */` header
- 113 files containing non-ASCII characters
- about 950 comment lines citing tickets and about 650 containing history words
- 2,522 `//----` separators
- 42% of methods and 62% of fields undocumented
- `@replicated` present on only 1 of 7 `[RplProp]` fields
- `@route` present on none of the 12 HTTP call sites

**Outcome:**
- Every mod `.c` file is at or under 500 lines, split by responsibility.
- Duplicated code is merged into shared helpers.
- Every file meets the Enfusion comment standard.
- Two CI gates lock this in: `file-length` now also covers `.c`, and a new `verify enfusion-comments` checks comments.
- The work runs as pre-written sub-agent prompts, tuned for token cost and wall-clock time.

### Operator decisions (2026-09-26)

- **File header:** the standard's `/** */` block.
- **Scope:** all three addons, in this order:
  1. framework
  2. tbd-emcp
  3. tbd-export, only once the other session has committed its export work
- **Comment gate:** the full rule set, wired into CI.
- **Duplicate code:** consolidate everything, including merging the 8 tick drivers into one ordered heartbeat.
- **Vendored file:** split it and record in its README that it diverges from upstream.
- **Separators:** remove all of them.
- **tbd-export compile:** the operator compiles in Workbench.
- **Execution:** each phase runs as sub-agents in slices, from prompts written in advance.


## Approach

### Efficiency design

The main costs are output tokens and serial waiting. Six measures reduce them.

1. **Move code, never retype it.** Agents move code between files with shell line-range extraction:
   - `sed -n 'A,Bp' src >> dst`, then `sed -i 'A,Bd' src`, working from the bottom of the file upward.
   - They write only the seams by hand: the new class shell, back-references, and call sites.
   - For 67k lines of code, this is the difference between minutes and hours of generation.
2. **The gate is the to-do list.** P1 ships `verify enfusion-comments` first. Writers fix the findings it reports for their own folder. They do not audit files by reading them.
3. **A mechanical sweep runs before any writer.** A throwaway scratchpad script handles every change that needs no judgement:
   - transliterate non-ASCII characters
   - delete separator and decorative banner lines
   - strip bare ticket-id parentheticals

   Writers then deal only with prose.
4. **Each file is touched once.** P2 creates the shared helpers as new files only. Each P3 slice replaces the duplicates inside its own folders during its split, so no file is read twice by two phases.
5. **Parallel waves over disjoint folders.** Slices in a wave own separate folders and keep every public member that other folders call. Where a clean split would change an external caller, the slice leaves a forwarder and logs it, and the closing run removes it. This lets up to 5 agents run at once.
6. **Small prompts.** Each agent receives:
   - a short base brief
   - the compact standard card (the gate enforces it, so agents do not read the standards documents)
   - only its slice parameters

   Agents read only the files they own, plus a grep for callers. Reports are a table of at most 150 words.

### Mechanics

**Orchestrator (the main session).** It:
- launches agents
- spot-checks diffs
- commits each slice by pathspec
- runs the compile after each wave
- updates the checkpoint and memory

It writes no product code.

**Agents.**
- All agents run on `opus`.
- Each agent stops softly at 350k tokens. Unfinished work continues through SendMessage to the same agent (context intact) or as a `<id>b` relaunch.

**Compile inside a parallel wave.**
- An agent treats its own part as clean when the compile error lines name none of its files.
- After the wave, the orchestrator runs `hcargo xtask mod compile` itself.
- A failure goes back to the responsible agent through SendMessage.

**Commits.**
- Commits go directly to main, with conventional messages ending in the Co-Authored-By line.
- `CLAUDE.md`, `.gitignore` and `Cargo.lock` are dirty from another session. Commits that touch them use a temporary index.

**Program files.** The ticket tree is exempt from the 500-line limit.

| File | Contents |
|---|---|
| [Spec](/documentation/tickets/specs/t1092_mod_script_modularisation.md) | Problem, goal, locked decisions, acceptance |
| This plan | Efficiency design, mechanics, roster and every launch prompt |
| [Progress checkpoint](/documentation/mod/script_modularisation_progress_checkpoint.md) | Roster status, frozen class names, amendments, shared-helper index, forwarders, tick-order baseline, leftovers, ticket batch |

**Tickets.**
- T-1092 is the program ticket "Modularise, document and gate the mod scripts".
- Its children:
  - .1 gates and sweep
  - .2 shared foundations
  - .3 framework decomposition
  - .4 framework pin and CI
  - .5 tbd-emcp
  - .6 tbd-export

### Roster

| Order | Id(s) | Parallel | What |
|---|---|---|---|
| 0 | orchestrator | — | Program files, tickets, checkpoint skeleton, and the frozen class-name list: every `TBD_*` class referenced from `.et`, `.layout`, `.conf` or `.gproj` (found with one grep) |
| 1 | P1-1, P1-2 | ∥ | Gates: `file-length` learns `.c`; new `verify enfusion-comments` |
| 2 | P1-3 | — | Mechanical sweep of the framework and tbd-emcp, with before and after gate counts |
| 3 | P2-1 | — | Shared helpers as new files, plus two sed-safe repo-wide replacements |
| 4 | P2-2 | — | Tick-order baseline, then heartbeat merge and the DebriefScreen fold (world-boot) |
| 5 | Wave A: P3-1…P3-5 | ∥ 5 | Spawning · Mission/Loaders · Zones · Gamemode/Objectives · API |
| 6 | Wave B: P3-6…P3-10 | ∥ 5 | Mission/Data+Ingestion · Loadouts+AI · Audio+Markers+Radio · Gamemode/Orchestrator+Stages · Session/Briefing |
| 7 | Wave C: P3-11…P3-14 | ∥ 4 | Session/Lobby · Session/Spectator+Players+PostGame · Session/Admin+MissionSelector · UI (all) + Core |
| 8 | P3-C | — | Closing run: zero findings, forwarders removed, leftovers, world-boot |
| 9 | P4-1 | — | Pin the framework in both gates, wire CI, update the standards and CLAUDE.md |
| 10 | OP-1 | operator | Two-client playtest |
| 11 | P5-1 | — | tbd-emcp split and comments, then pin (operator compiles in Workbench) |
| 12 | Wave D: P6-1…P6-3, then Wave E: P6-4…P6-6 | ∥ 3 each | tbd-export slices, after the other session commits (one operator Workbench compile per wave) |
| 13 | P6-C | — | Pin tbd-export, `ci-local`, close the tickets |

Wave A's Mission/Loaders slice creates `Mission/Data/Document/`. The Mission/Data slice runs in Wave B, so the two never overlap.

---

### Prompts (copied verbatim at launch; additions go in the amendments table)

#### B0: base brief (prefix for every agent)

```text
Program T-1092 in /run/media/system/Disk_2/Projects/TBD-Reforger. You are slice {ID}. The
orchestrator commits; you never commit, push, branch, stash, reset or restore.
- Cargo: always /home/Samuel/.cache/tbd-bin/hcargo (e.g. `hcargo xtask mod compile`). One command
  per Bash call; aim for 30 s or less per command.
- Touch only your owned paths. Another session has uncommitted work in tbd-export, CLAUDE.md,
  .gitignore and Cargo.lock; leave it alone unless your slice names it.
- Never edit .rdb, .meta, .et, .layout, .conf or .gproj. Never rename a class on the frozen list
  in documentation_v2/mod/script_modularisation_progress_checkpoint.md.
- Behaviour, JSON keys, RPC names, [Attribute] fields, RplProp fields and component class names do
  not change, unless your slice names a reconciliation.
- Token discipline: read each owned file once, by line ranges; never re-read a file you just edited;
  do not read the standards documents (the card below is the standard); grep, not read, outside
  your paths.
- Soft stop at 350k tokens: leave the tree compiling and list what remains.
- Report: a table of at most 150 words: files (final line counts), gate exit codes, forwarders left,
  reconciliations, leftovers, bugs noticed (file:line).
```

#### CARD: Enfusion standard (appended for P1-2, P1-3, P2 and every writer)

```text
Every .c file you create or touch must satisfy all of these; `hcargo xtask verify enfusion-comments
--path <p>` checks them.
1  ASCII only, including string literals ("--" and "->").
2  Header first:
   /**
    * @file <exact name>.c
    * @brief <one line>
    *
    * Role: <responsibility>  Position: <who feeds it, who consumes it>
    * State: <owned state + owning machine, or "none">  Invariants: <guarantees, failure modes>
    */
3  //! banner directly above every class, modded class, enum and method (above any attributes).
   Method banner = contract: what it does, params, return value, failure behaviour.
4  Trailing //!< on every field and enum member: unit, default or JSON key.
5  //! @authority server|client|owner on methods that depend on where they run, and on every method
   that calls Rpc(.
   //! @rpc <Reliable|Unreliable> <Server|Owner|Broadcast> directly above [RplRpc], matching it.
   //! @replicated <prop> directly above [RplProp].
6  //! @route <METHOD> <path> on methods that call TBD_GameRuntimeHttp.Post/Get or RestContext.
   //! @contract <schema>#<pointer> on every *Struct class. The Struct suffix is only for
   schema-backed DTOs; internal wire classes are named *Wire.
7  [Attribute] has desc:; [ComponentEditorProps] has description:.
8  Present tense and context-free. Not allowed: ticket ids, dates, wave/slice/lane words, history
   ("previously", "used to", "no longer", "reworked", "split out of", "ported", "legacy"),
   TODO/FIXME/HACK, commented-out code, separator or banner lines, cross-repo file:line
   references (use symbol names).
9  File name = its primary type. One primary type per file; a small private companion type may
   share it.
```

#### SPLIT RULES (appended for writers)

```text
- EnfScript has no partial classes. [Attribute], [RplProp], [RplRpc], engine overrides and
  GetInstance stay on the component or entity.
- Extracted logic goes into `class X : Managed` helpers owned by `ref`, with the parent passed to the
  constructor (CallLater helpers are cancelled in the parent's OnDelete), or into static utility
  classes.
- MOVE CODE WITH SHELL, NEVER RETYPE IT: `sed -n 'A,Bp' src >> dst`, then delete the range, working
  from the bottom up. Hand-write only headers, class shells, back-references and call-site edits.
- Public members called from outside your owned folders keep their class and signature. If a clean
  split would change them, leave a one-line forwarder and list it (the closing run removes it).
- A file that becomes 3 or more files gets a subfolder. Aim for 400 lines or fewer.
- Moved whole files: git mv. New subfolder: README.md following
  documentation_v2/standards/templates/readme_mod_scripts.md. Update README Contents in every folder
  you change.
- xtask pins on paths you move (grep tools_v2 for the old path) are yours; update the pin and its
  test.
DONE (in order):
  hcargo xtask mod compile (your files absent from errors)
  -> hcargo xtask verify enfusion-comments --path <each owned folder> = 0
  -> wc -l shows every owned .c file at 500 lines or fewer
  -> hcargo xtask verify readme-coverage --path <f>
  -> hcargo xtask verify link-check --path <f>
  -> hcargo test -p xtask <filter> if you edited a verification.
```

#### P1-1: `file-length` learns `.c` (B0 only)

```text
Owned: tools_v2/xtask/src/verifications/language_bans/node_and_file_limits.rs and
node_and_file_limits/**, tools_v2/xtask/src/tests/node_free_tests.rs, their READMEs,
language_bans/README.md, documentation_v2/standards/coding_standards/file_size_and_complexity.md.
1. The walk keeps .rs and .c (rename walk_rust_sources to walk_length_gated_sources). Output:
   "scanned N source file(s) (R .rs, C .c)". The vacuous-walk refusal says "source files".
2. is_test_file accepts a *_tests stem for .c as well.
3. Add MOD_SCRIPT_ROOTS = the three `mod/<addon>/Scripts` roots. Never pin all of mod:
   crf_framework and vanilla_reference are gitignored references. Do not add them to
   FILE_LENGTH_PINS yet; P4-1, P5-1 and P6-C each add one.
4. Tests: .c 500 passes / 501 fails; *_tests.c holds to 1000; the mixed count appears in output.
   The existing anti-vacuity tests still pass.
5. Docs describe the .c walk and the per-addon pin schedule.
Gates: hcargo test -p xtask node_free; hcargo xtask verify file-length (still 0); hcargo clippy -p
xtask.
```

#### P1-2: `verify enfusion-comments` (B0 + CARD)

```text
Owned: NEW tools_v2/xtask/src/verifications/mod_scripts/enfusion_comments/ (one file per rule
family, each 500 lines or fewer); mod_scripts/{mod.rs,README.md};
mod_scripts/tests/enfusion_comments_tests.rs; commands/verify/{cli.rs,dispatch.rs,README.md};
NEW documentation_v2/standards/templates/enfusion_script_header.md (the CARD header plus a worked
example); templates/README.md.
Build `cargo xtask verify enfusion-comments [--path <dir|file>]`:
- One rule id per CARD item (ECM-1..ECM-9). Output `<rule> <path>:<line> <msg>`, then per-rule
  counts. Exit 0 clean, 1 findings, 2 did-not-run.
- Reuse the comment stripping in mod_scripts/destroy_target_diagnostics/strip_c_comments.rs; move
  it to a shared module rather than copying it.
- Walk with verification_core::scan::walk_files, .c only. Default roots: a pinned-roots const,
  empty now (later slices fill it). --path narrows to anything under mod.
- Fail closed: a missing root or an empty walk is did-not-run.
- Detection must be deterministic:
  - Text inside string literals is not a comment.
  - A method is a class-body signature line followed by {.
  - A field is a class-body declaration ending in ;.
  - "Directly above" skips attribute lines.
- ECM-8 lexicon, comments only:
  - T-[0-9]{3,}(\.[0-9]+)*, ENF-[0-9]+, 20[0-9]{2}-[0-9]{2}-[0-9]{2}
  - whole words, case-insensitive: previously, formerly, "used to", "no longer", reworked,
    rewritten, "split out of", ported, legacy, TODO, FIXME, HACK, wave, slice, lane
  - separator lines
  If tools_v2 has an existing prose-lexicon list (grep "formerly"), share it.
- Tests: one pass fixture and one fail fixture per rule, plus --path, a missing root and an empty
  walk.
- Finally run it with --path mod/tbd-framework/Scripts. Put per-rule counts in your report.
Do not touch CI wiring.
Gates: hcargo test -p xtask enfusion_comments; hcargo clippy -p xtask.
```

#### P1-3: Mechanical sweep (B0 + CARD)

```text
Owned: every .c file under mod/tbd-framework/Scripts and mod/tbd-emcp/Scripts, for
mechanical edits only. Write ONE throwaway script in your scratchpad (never in the repo). Run it on
both trees, then review the diff by sampling.
The script does exactly these things:
(a) Transliterates non-ASCII: — – to "--", → to "->", ← to "<-", … to "...", · to "-", " " to ",
    ' ' to '. Any other non-ASCII character is listed, not guessed.
(b) Deletes lines that are only a separator or decorative banner:
    //-{4,}, //={4,}, lines of box-drawing characters,
    `// ── Title ──` or `//! -- Title --` section titles.
(c) Removes bare ticket parentheticals such as "(T-941.3)" or "(T-181.21, T-200)", and trailing
    "-- T-123" tags. A ticket id inside a sentence is LEFT for the writers.
(d) Converts a single-line `//! text` directly above a field declaration into a trailing
    `//!< text`, only when the result is 120 columns or fewer.
Do not touch anything else. Run the enfusion-comments gate before and after on both trees and
report per-rule counts. hcargo xtask mod compile must exit 0. Check that string literals changed
only by transliteration.
```

#### P2-1: Shared helpers (B0 + CARD)

```text
Owned: NEW helper files only (listed below), their folder READMEs, and the checkpoint section
"Shared helper index", plus the two repo-wide replacements at the end.
For each helper:
- grep the listed duplicates and read ONLY their method bodies (grep -n, then sed -n ranges).
- Write one helper to the CARD standard.
- Where the duplicates differ, add a parameter so every existing call site keeps its behaviour, and
  note it in the index.
Do not change other call sites; the P3 slices do that.
Helpers (paths under mod/tbd-framework/Scripts/Game/TBD/):
- Core/Characters/TBD_CharacterUtil.IsDead (IsBodyDead in TriggerRuntime, ObjectivesComponent,
  PlayArea and SpawnManager)
- Core/TBD_PlayerChat.Broadcast(tag, text) (FrameworkManager, Safestart, FleetPlayerActions); Tell
  already exists
- Core/Players/TBD_PlayerFaction.Of (ResolveFaction)
- Systems/Mission/Data/TBD_MissionFactionNames (ResolveFactionName in Briefing and Lobby)
- Core/Wire/TBD_WireCodec (the Field, Unmark, Sanitise, IsSet, Flag, Join and Record* codecs in
  Briefing, Lobby and AdminSnapshot; must be byte-identical)
- API/Http/TBD_BackendText (JsonEscape, DescribeBackend, UtcNowIso8601, Pad2)
- Core/Time/TBD_ClockText (the FormatClock and milestone copies)
- Core/TBD_Authority.IsClient/IsServer
- Systems/Mission/Ingestion/TBD_MissionJsonPass.LoadRoot (the second-pass JSON read in 12 files:
  ReadWire, EnsureParsed, Parse)
- Systems/Mission/Data/TBD_MissionVariants.IsActive (the variant checks in ObjectiveRegistry,
  TriggerRuntime and MissionLoader)
- Systems/Loadouts/TBD_LoadoutInventoryUtil (PrefabOf, CountGear, AreasForLabel, IsRootedOn,
  WeaponStorageOf, WeaponStorageHas)
- Systems/AI/TBD_AIGroupFactory.SpawnGroup
- Systems/AI/TBD_AIWireEnums.SpeedFromWire (two semantics; name both)
- Core/World/TBD_EntityQuery (the static s_Query* + OnQuery* callback pattern)
- TBD_MissionLoader.GetMissionId() (add it in place: a one-method edit)
- Core utilities for FactionExists, RoundToInt, WarnOnce and AnnounceOnce, each in a descriptively
  named Core file.
Repo-wide replacements done here (both sed-safe):
(1) `RplSession.Mode() == RplMode.Client` becomes TBD_Authority.IsClient() (79 sites), plus any
    exact server equivalents.
(2) The six `modded enum ChimeraMenuPreset` blocks move into UI/Core/TBD_MenuPresets.c. Preset names
    stay identical.
Index format, one row per helper: path | class.method signature | replaces (file:method) |
parameter notes.
Gates: hcargo xtask mod compile; hcargo xtask verify enfusion-comments on the new files only.
```

#### P2-2: Heartbeat (B0 + CARD)

```text
Owned: the 8 tick-driver `modded class SCR_BaseGameMode` blocks (Trigger, Audio, WinCondition,
Task, Weather, DynamicSpawner, GroupState, Waypoint: grep for them); NEW
Gamemode/Orchestrator/Heartbeat/TBD_RuntimeHeartbeat.c plus its README; the
`modded class TBD_ResultsReporter` block in Session/PostGame/UI/TBD_DebriefScreen.c and its target;
the checkpoint section "Tick-order baseline".
1. Baseline: record each driver's hooks, super-call position, CallLater interval and armed flag.
   Add temporary Prints, run hcargo xtask mod world-boot, and derive the real per-tick order from
   the log. Remove the Prints; the .c diff must be empty before step 2. Write the ordered list and
   its evidence lines into the checkpoint.
2. Write one heartbeat modded SCR_BaseGameMode that:
   - arms one CallLater loop
   - calls each runtime's static Tick in the recorded order, as an explicit documented list
   - keeps each runtime's own period using a per-runtime counter when intervals differ
   - moves the other hooks the drivers carried (for example the OnGameEnd resets), in the same
     order
   Delete the 8 driver blocks. Keep one permanent debug Print at arm time listing the order.
3. Fold FillScoreboard out of the modded TBD_ResultsReporter into its owner by responsibility
   (the reporter, or a debrief-side helper), and delete the modded block. Grep tools_v2 and
   documentation_v2 for the removed names and update them.
Gates: hcargo xtask mod compile; hcargo xtask mod world-boot (the order line matches the
baseline); enfusion-comments on the heartbeat file.
```

#### P3 writer prompt (B0 + CARD + SPLIT RULES + slice parameters)

```text
Owned folders: {FOLDERS} (every .c file and README.md, plus new subfolders you create).
Oversize files and target map: {MAP}. You may improve a boundary if a cleaner responsibility line
shows up; say why.
Duplicates to replace with shared helpers (see the checkpoint "Shared helper index"): {DUPES}.
Slice notes: {NOTES}.
Steps:
1. For each oversize file, list its types and method clusters with line ranges from one read.
2. Split it using shell moves.
3. Replace the listed duplicates.
4. Run enfusion-comments on your folders and fix every finding, including in files that were never
   oversize. Write banners from the code as it is now; never invent behaviour.
5. Update READMEs and pins.
6. Complete DONE and report.
```

**Slice parameters.** All paths are under `mod/tbd-framework/Scripts/Game/TBD/`.

**Wave A**

- **P3-1 Spawning** (`Systems/Spawning/`, 6974 lines)
  - **Map for `TBD_SpawnManager.c` (4231):**
    - `Manager/`: `TBD_SpawnTypes`, `TBD_ConnectionEpochs`, and a residual `TBD_SpawnManager` (under 400 lines) that exposes `GetSlots`, `GetIdentity` and `GetLives`.
    - `Slots/`: `TBD_SlotClaimBook`, `TBD_SlotRosterWire`, `TBD_SlotBodyMaterializer`, `TBD_SlotBodyDressing`, `TBD_SlotLoadoutSettle`.
    - `Identity/`: `TBD_SpawnIdentityKeys`, `TBD_SpawnIdentityGate`, `TBD_SpawnJoinAudit`.
    - `Deploy/`: `TBD_DeployWaves`, `TBD_DeployExecutor`, `TBD_WalkOnPlacement`, `TBD_PossessTicketLedger`.
    - `Lives/`: `TBD_OneLifeLedger`, `TBD_DeathRespawnFlow`, `TBD_SpawnDeparture`.
    - `Vehicles/`: `TBD_VehicleSpawnWire`, `TBD_VehicleSpawnDefaults`.
  - **Map for `TBD_DynamicSpawner.c` (600):** `Dynamic/`: `TBD_SpawnModuleStructs`, `TBD_DynamicSpawner`, `TBD_DynamicSpawnVolley`.
  - **Existing files:** go to `Deployment/`, `VanillaBridge/` (`TBD_SCR_*`) and `Client/`.
  - **Notes:**
    - First fold `TBD_SpawnManagerDeploymentAuthorization.c` (a same-addon modded class with 11 overrides) into the manager, keeping the order of effects of its super calls.
    - 52 external `GetInstance` callers must keep working.
    - Update the pins in `spawn_determinism*.rs` and `spawn_verification.rs`.
    - Run world-boot.
  - **Duplicates:** IsBodyDead, CurrentMissionId, JSON pass, EntityQuery, AIGroupFactory, ZoneRegistry.FindById, TriggerRuntime.HasFired.

- **P3-2 Mission/Loaders** (`Systems/Mission/Loaders/` plus new `Systems/Mission/Data/Document/`, 4816 lines)
  - **Map for `TBD_MissionLoader.c` (1844):**
    - Its 20 structs go to `Data/Document/`: `TBD_MissionDocumentStruct`, `TBD_MissionZoneStructs`, `TBD_MissionOrbatStructs`, `TBD_MissionBriefingStructs`, `TBD_MissionEntityStruct`, `TBD_MissionVariantStructs`.
    - `Loaders/Mission/`: `TBD_MissionLoader`, `TBD_MissionOrbatQuery`, `TBD_MissionWorldApplier`, `TBD_MissionVariantFilter` (with `ApplyVariantFilter` broken into one pass per kind over a context object), `TBD_MissionVariantSources`.
  - **Map for `TBD_MissionValidator.c` (1561):** `Loaders/Validation/`: `TBD_MissionValidator`, `TBD_MissionValidationFindings` (replaces the protected statics), `TBD_MissionStructureChecks`, `TBD_MissionSlotChecks`, `TBD_MissionWinConditionChecks`, `TBD_MissionUnconsumedKeyCheck`.
  - **Notes:**
    - Keep 167 external `TBD_MissionLoader.` call sites working.
    - Update the pins in `mission_rest_size_limits.rs`.
    - Every struct gets `@contract`.
  - **Duplicates:** MissionVariants, LoadoutInventoryUtil (CountGearRefs).

- **P3-3 Zones** (`Systems/Zones/`, 4360 lines)
  - **Map:**
    - `TBD_TriggerRuntime.c` (2172) goes to `Triggers/`: `TBD_TriggerWireStructs`, `TBD_Trigger`, `TBD_TriggerVocabulary`, `TBD_TriggerRuntime`, `TBD_TriggerCompiler`, `TBD_TriggerEffectValidator`, `TBD_TriggerPlayerSnapshot`, `TBD_TriggerConditions`, `TBD_TriggerEffects`, `TBD_TriggerWorldEffects`, `TBD_TriggerFlowEffects`, `TBD_TriggerSoundRelay` (the modded PC with its RPC).
    - `Registry/`: `TBD_ZoneRegistry`, `TBD_ZoneCompiler`.
    - `Volumes/`: `TBD_ZoneVolume`, `TBD_ZoneVolumeBounds`, `TBD_ZoneContestResolver`.
    - `PlayArea/`: `TBD_PlayAreaComponent`, `TBD_PlayAreaViolation`, `TBD_PlayAreaPenalties`.
  - **Notes:** the 89-line header of `TBD_TriggerRuntime.c` moves into `Triggers/README.md`.
  - **Duplicates:** IsDead, Tell, PlayerFaction, MissionVariants, EntityQuery, CurrentMissionId, JSON pass.

- **P3-4 Gamemode/Objectives** (`Gamemode/Objectives/`, 4399 lines)
  - **Map:**
    - `Registry/`: `TBD_ObjectiveEntityStructs`, `TBD_ObjectiveEntityReader`, `TBD_ObjectiveRegistry`, `TBD_ObjectiveRuleResolver`, `TBD_ObjectiveTypedBinder`, `TBD_ObjectiveDestroyTargets`, `TBD_ObjectiveEndConditions`.
    - `Runtime/`: `TBD_ObjectivesComponent`, `TBD_ObjectiveProgression`, `TBD_ObjectiveHudPublisher`.
    - `Model/`: `TBD_ObjectiveEnums`, `TBD_Objective`, `TBD_ObjectiveText`, `TBD_ObjectiveRules`.
    - `Tasks/`: `TBD_TaskTypes`, `TBD_TaskStateMachine`, `TBD_TaskSchedule`.
  - **Notes:** update the `destroy_target_diagnostics*` pins and their tests.
  - **Duplicates:** IsDead, Tell, PlayerFaction, MissionVariants, EntityQuery, CurrentMissionId, JSON pass.

- **P3-5 API** (`API/**`, 4261 lines)
  - **Map:**
    - `Http/`: BackendConfig, GameRuntimeHttp, GameRuntimeAnswer, BackendText.
    - `RuntimeSession/`: 6 files.
    - `Identity/`: PlayerIdentity, `TBD_IdentityLink`, `TBD_IdentityLinkConfirm`, `TBD_IdentityLinkPending`.
    - `Results/`: `TBD_ResultsReporter`, `TBD_ResultsPayload`.
  - **Notes:** update the pins in `player_identity_comments.rs` and `results_reporter_identity_comments.rs`, keeping their truth pins.
  - **Duplicates:** Tell, BackendText, Broadcast (FleetPlayerActions).

**Wave B**

- **P3-6 Mission/Data and Ingestion** (`Systems/Mission/Data/` excluding `Document/`, plus `Ingestion/`)
  - **Map:** `TBD_MissionVehicleStruct.c` (751) goes to `Data/Vehicles/`: `TBD_MissionVehicleStruct`, `TBD_MissionVehicleRoster`, `TBD_MissionVehicleCrewSeating`, plus `TBD_VehicleState` (moved).
  - **Duplicates:** JSON pass, EntityQuery, CurrentMissionId.

- **P3-7 Loadouts and AI** (`Systems/Loadouts/`, `Systems/AI/`)
  - **Map:**
    - `TBD_LoadoutEquipHelper.c` (1659) goes to `Loadouts/Application/`: `TBD_LoadoutApplication`, `TBD_LoadoutPendingItems`, `TBD_LoadoutGearPhase`, `TBD_LoadoutWeaponPhase`, `TBD_LoadoutCargoPhase`, `TBD_LoadoutWornAudit`. Each phase is a Managed object owned by a `ref`, with a back-reference.
    - `Preview/`: `TBD_LoadoutPreviewDresser`, `TBD_LoadoutPreviewMount`.
    - `AI/Waypoints/`: `TBD_WaypointWireStructs`, `TBD_WaypointRuntime`, `TBD_WaypointFactory`.
    - `AI/GroupState/`: `TBD_GroupStateWireStructs`, `TBD_GroupState`.
  - **Duplicates:** LoadoutInventoryUtil, AIGroupFactory, AIWireEnums, JSON pass, CurrentMissionId, EntityQuery.

- **P3-8 Audio, Markers and Radio** (`Systems/{Audio,Markers,Radio}/`)
  - **Map:**
    - `Audio/`: `TBD_AudioWireStructs`, `TBD_AudioSourceEntity`, `TBD_AudioEmitter`, `TBD_AudioPlayerController` (holds the 2 RPCs).
    - `Markers/Client/`: `TBD_MarkerClient`, `TBD_MarkerApplier`, `TBD_StyledMapMarker`.
    - `Markers/`: `TBD_MarkerService`, `TBD_MarkerWire`, `TBD_MarkerStyleCodec`, split out of `TBD_MarkerData`.
  - **Duplicates:** TriggerRuntime.HasFired, JSON pass, CurrentMissionId.

- **P3-9 Gamemode/Orchestrator and Stages** (`Gamemode/Orchestrator/` excluding `Heartbeat/`, plus `Gamemode/Stages/`)
  - **Map:**
    - `Orchestrator/Flow/`: `TBD_JipPolicy`, `TBD_MissionFlow`, `TBD_MissionFlowReport`.
    - `Orchestrator/Stage/`: `TBD_StageEnvironment`, `TBD_EndBanner`, `TBD_RoundClock`, `TBD_FactionElimination`, `TBD_LoadingGate`.
    - `Orchestrator/`: `TBD_FrameworkRollCall`, plus `TBD_FrameworkManager` (under 400 lines; all 6 RplProps stay on it with `@replicated`, and it calls `BumpMe` itself).
    - `Stages/Safestart/`: `TBD_SafestartManager`, `TBD_SafestartProtection`, `TBD_SafestartWatchdog`.
    - `Stages/WinConditions/`: `TBD_WinConditionStructs`, `TBD_WinConditionEvaluator`, `TBD_WinConditionModes`.
  - **Duplicates:** Broadcast, ClockText, ZoneRegistry.FindById.

- **P3-10 Session/Briefing** (`Session/Briefing/**`)
  - **Map:**
    - `Service/`: `TBD_BriefingService`, `TBD_BriefingWire`, `TBD_BriefingWireSelfCheck`, `TBD_BriefingText`.
    - `UI/`: `TBD_BriefingMapLauncher`, plus a slimmer `TBD_BriefingScreen`.
  - **Duplicates:** WireCodec, MissionFactionNames, CurrentMissionId.

**Wave C**

- **P3-11 Session/Lobby** (`Session/Lobby/**`)
  - **Map:**
    - `Service/`: `TBD_LobbyService`, `TBD_LobbyRosterWire`, `TBD_LobbyRosterWireSelfCheck`, plus the moved Catalog and Data files.
    - `PreSlot/`: the 2 PreSlot files.
    - `UI/Roster/`: `TBD_LobbySlotRow`, `TBD_LobbySquadCard`, `TBD_LobbyRosterPanel`.
  - **Notes:** fold `TBD_LobbyServiceDeploymentAuthorization.c` into `ApplyDeploy`.
  - **Duplicates:** WireCodec, MissionFactionNames.

- **P3-12 Session/Spectator, Players and PostGame** (`Session/{Spectator,Players,PostGame}/**`)
  - **Map:**
    - `Spectator/Host/`: `TBD_SpectatorHostPlayerController` (holds the RPC), `TBD_SpectatorHostRecord`, `TBD_SpectatorHost`, `TBD_SpectatorHostLifecycle`, `TBD_SpectatorHostFactory`.
    - `Spectator/Controller/`: `TBD_SpectatorController`, `TBD_SpectatorTargeting`, `TBD_SpectatorHostReporter`.
  - **Notes:** DebriefScreen and EndScreen currently have no method docs.

- **P3-13 Session/Admin and MissionSelector** (`Session/{Admin,MissionSelector}/**`)
  - **Map:**
    - `Admin/`: `TBD_AdminService` and `TBD_AdminSubcommands`; the enum moves into `TBD_AdminData`.
    - `Admin/UI/`: `TBD_AdminScreen`, `TBD_AdminScreenSections`.
    - `MissionSelector/UI/`: `TBD_MissionInspectorPanel`, `TBD_MissionInspectorCards`.
  - **Notes:**
    - Fold `TBD_AdminServiceDeploymentAuthorization.c` into the service.
    - Fix `TBD_MissionBrowser.c:167`: a plain comment sits between `@rpc` and the attribute.
  - **Duplicates:** WireCodec.

- **P3-14 UI and Core** (`UI/**`, `Core/**`, 8940 lines; about 7k of them are comment-only work)
  - **Map:**
    - `UI/Common/Dropdown/`: `TBD_DropdownItem`, `TBD_DropdownMenuBridge`, `TBD_DropdownComponent`, `TBD_DropdownMenu`.
    - `Common/Inputs/`: SearchBox, TabStrip, Chip.
    - `Common/Layout/`: Panel, Section, NumberedCard, KeyValueRow, ScrollList, Caption.
    - `Common/SessionChrome/`: TopBar, BottomBar.
    - `UI/Core/Theme/`: `TBD_UITheme`, `TBD_UIStateColours`, `TBD_UIThemeEnums`, UIIcons, UILayouts.
    - `UI/Core/Screens/`: MenuBase, MenuStack, ShellScreen, DockScreen, MenuPresets.
    - `UI/Core/Controls/`: UIButton, UIInteractive, UIScrollBar, ListBox, ListBoxRow.
  - **Notes:**
    - Add `@authority` to the `Rpc(` callers in `TBD_TaskHud` and `TBD_ObjectiveHud`.
    - Grep the `.layout` files for handler class names before moving anything.

#### P3-C: closing run (B0 + CARD + SPLIT RULES)

```text
Owned: mod/tbd-framework/Scripts, for fixes only.
1. Remove every forwarder listed in the checkpoint by updating its callers.
2. Clear the leftovers from roster rows P1-3..P3-14.
3. Grep every duplicate name from the shared helper index; none may survive.
4. hcargo xtask verify enfusion-comments --path mod/tbd-framework/Scripts = 0.
5. Every .c file at 500 lines or fewer.
6. hcargo xtask mod compile, then hcargo xtask mod world-boot (the order line matches the baseline).
7. Docs gates over mod/tbd-framework and documentation_v2/mod.
8. Collect the "bugs noticed" from every row into the checkpoint "Ticket batch".
```

#### P4-1: pin and CI (B0)

```text
Owned:
- FILE_LENGTH_PINS and the enfusion-comments roots (add mod/tbd-framework/Scripts to both)
- commands/ci/task_definitions.rs (verify-coding-standards gains enfusion-comments)
- .github/workflows/ci.yml (the language-gates step after verify-file-length)
- docs:
  - CLAUDE.md law 7 ("Rust source trees" becomes "Rust and the pinned mod Scripts roots")
  - CLAUDE.md law 8 (the tags are checked by verify enfusion-comments)
  - CLAUDE.md section 3 (list the command)
  - documentation_standards.md sections 3.1, 6 and 7
  - coding_standards/{file_size_and_complexity, enfusion_code_policy (lines 24-27 and 47-49),
    README (the SIZE-3 row), ci_gates}.md
  - commands/{ci,verify}/README.md
  - mod_design.md:208
CLAUDE.md carries another session's hunks: edit only your lines, and give the exact hunks in your
report.
Gates:
- hcargo xtask verify file-length
- hcargo xtask verify enfusion-comments
- hcargo test -p xtask
- hcargo xtask ci ci-local. It needs the podman shim: create <scratchpad>/bin/podman containing
  `exec distrobox-host-exec podman "$@"` and put it first on PATH.
Report any failure that is outside this program with evidence; do not fix it.
```

#### P5-1: tbd-emcp (P3 writer prompt)

- **Folders:** `mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/` (19 handlers).
- **Map:** split `EMCP_WB_ModifyEntity.c` (582) by operation family, keeping the NetAPI surface.
- **Notes:**
  - Keep the `EMCP_` class names, because the broker calls them by name.
  - The README records that the file derives from enfusion-mcp@0.6.1 and is no longer byte-identical.
  - Pin `mod/tbd-emcp/Scripts` in both gates.
  - Replace the compile step with "operator Workbench compile required".

#### P6: tbd-export (P3 writer prompt; launch only once `git status --short mod/tbd-export` is clean)

Before launch, the orchestrator runs P1-3's sweep script on tbd-export (as amendment P6-0), re-measures the file sizes and logs the final file sets. All paths are under `mod/tbd-export/Scripts/`.

| Wave | Slice | Folders | Map |
|---|---|---|---|
| D | P6-1 | `WorkbenchGame/` root, `Game/TBD/Export/` | Split `TBD_RegistryScan.c` (1794) into `WorkbenchGame/Registry/` by scan walk, classification, serialisation and report. Split `TBD_RoadExportComponent.c` (744) into sampling, classification and writer helpers. |
| D | P6-2 | `MapExport/Terrain/Roads/**` | The 6 road exporters (615–726 lines each) share one shape. Extract `Roads/Shared/` (base, sampler, writer) and leave only class-specific rules in each exporter. |
| D | P6-3 | `MapExport/Terrain/{Water,DEM,Satellite}`, `MapExport/Vegetation/**` | Shared water and vegetation helpers. Lakes 567, Ponds 601, Rocks 576, Trees 522. |
| E | P6-4 | `MapExport/{Core,Locations,Objects,Plugins,Registry}/**` | Split the Buildings trace and architect files (847, 745, 611) into `Trace/` and `Architect/`. |
| E | P6-5 | `EquipmentExport/**` | Comments, plus any files over 500 lines. Fix the false "under 500 lines" claim in two READMEs. |
| E | P6-6 | `EquipmentVehicleExport/**`, `VehicleExport/**` | Comments, plus any files over 500 lines. |

- **P6-C** uses the P3-C prompt for tbd-export, plus these steps:
  - pin `mod/tbd-export/Scripts` in both gates
  - run `hcargo xtask ci ci-local`
  - close T-1092 and its children, then `ticket sync`


## Risks

- **Heartbeat order drift.** Merging the 8 tick drivers could change the per-tick order. P2-2 records the real order from a world-boot log before changing anything, the heartbeat encodes that order as an explicit list, and every later world-boot compares its arm-time order line with the baseline. OP-1 checks the behaviour in a live two-client session.
- **Parallel compile interference.** In a wave, one agent's half-finished split can break another agent's compile. Each agent judges only error lines that name its own files; the orchestrator compiles after the wave and routes any failure back to the responsible agent by SendMessage.
- **Cross-folder API churn.** A split that changes a member other folders call would collide with a parallel slice. Slices keep those members and leave logged forwarders; P3-C removes them serially.
- **Resource references.** Prefabs, layouts and configs reference class names. The frozen list in the checkpoint names every one; no slice renames them.
- **Another session's uncommitted work.** It sits in tbd-export, tools_v2/xtask (mod_ops), CLAUDE.md, .gitignore and Cargo.lock. Agents never touch those paths unless named; the orchestrator commits by pathspec and uses a temporary index for shared files. If that work breaks the xtask build, the affected slice reports it instead of fixing it. tbd-export waits until the other session commits.
- **Workbench-only compile.** tbd-emcp and tbd-export compile only in Workbench; the operator compiles after P5-1 and after waves D and E.


| Check | When | Pass |
|---|---|---|
| `hcargo xtask mod compile` | each P2 slice; orchestrator after each wave; P3-C | exit 0 |
| `hcargo xtask mod world-boot` | P2-2, P3-1, P3-C, P4-1 | exit 0, and the heartbeat order line matches the baseline |
| `hcargo xtask verify enfusion-comments` | every slice from P1-3 on | 0 findings on owned paths; the whole framework after P3-C |
| `hcargo xtask verify file-length` | P1-1, P4-1, P5-1, P6-C | exit 0, with the `.c` count shown |
| `hcargo test -p xtask` | P1 and any slice that edits a verification | pass |
| `readme-coverage` and `link-check` | every writer slice | pass on owned paths |
| `hcargo xtask ci ci-local` | P4-1, P6-C | pass |
| Two-client playtest (operator) | OP-1 | spawn, lobby, briefing, objectives, safestart, triggers, spectator and results behave as before |
| Workbench compile (operator) | P5-1, after waves D and E | clean |
