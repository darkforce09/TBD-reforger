# T-936 — Mission logic the audit found missing

Owner: command center. Source: master audit S1 (2026-09-04), every claim verified against main
@ 072988d57 (see README.md in this directory). Scope: TBD-Reforger only. Executor: claude-code,
including the Enfusion `.c` scripts (gate `cargo xtask mod compile`; in-game behaviour on the human checklist).

## 0. Operator deferral (the only one in this program)
The in-editor "Play scenario" dry run — `deferred by operator 2026-09-04`. No slice builds it.

## 1. Related existing tickets — referenced, never re-minted
T-675/.1/.2 vehicles · T-676 triggers (T-936.2 consumes its events) · T-677 waypoints · T-678 group AI ·
T-674/.1/.2 slot identity · T-673 markers (T-936.7 builds on it) · T-212 objectives · T-680–T-705 ·
T-299 phantom opfor · T-290/T-291 dead fields · T-310 attachments · T-257 undo scope · T-190 second tab ·
T-295 realtime collab · T-132 visual diff.

## 2. Verified anchors (2026-09-04)
| Finding | Anchor | Verdict |
|---|---|---|
| win/loss hardcoded | flatten.rs:2573-2574 `mode: "attrition"`, :2582 `end_on`; :2249 winConditions synthesized | PARTIAL — schema already requires it (:8, $defs :1132), mode is a free string |
| radio synthesized | flatten.rs:1927-1960 derive_radio_plan; :1876 "no radio UI, no radioPlan in payload" | TRUE — $defs/radioPlan and /net (:481) exist; Radio/TBD_RadioPlan.c parses |
| tasks | schema :1208 `task` is only an enum value; no state machine in core or mod | TRUE |
| weather | schema :190 environment.weatherPreset static; panels/env.rs edits it | TRUE (no timeline) |
| audio, spawn modules, tactical graphics | nothing in schema, core, editor or mod | TRUE |
| dry run | — | deferred by operator |

## 3. Architecture — AUTHORED_BLOCKS passthrough (lands in T-936.1)
Pipeline: editor doc → `mission/compile.rs:156 compile_payload` → payload → `mission/flatten.rs` →
`ModMission` (validated by mission.schema.json, top level `additionalProperties: false`) → framework scripts.
Flatten synthesizes everything the editor cannot author (:2249). To avoid seven slices each editing
compile.rs and flatten.rs (both heavily contested), T-936.1 adds `crates/map-engine-core/src/mission/extensions.rs`:
- `AUTHORED_BLOCKS: &[AuthoredBlock { key, validate }]` — compile_payload copies each listed key verbatim from the doc;
- `ExtensionBlocks` struct carried on `ModMission` with `#[serde(flatten)]`; absent blocks serialize to nothing,
  so the Class-R byte-parity fixtures stay identical;
- later slices add one field + one validator here and declare the key in mission.schema.json — nothing else.
The payload schema top level is deliberately open (its own :50 rationale); T-936.1 declares `winConditions`
there and documents the convention. Wave packing: every slice owns mission.schema.json, mission/mod.rs and
extensions.rs, so the chain .1 → .2 → .4 → .5 → .6 → .7 (and .3 after .1) is one slice per wave.

## 4. Slices
| Slice | Block | Core model | Editor | Runtime script | Deps |
|---|---|---|---|---|---|
| T-936.1 | `winConditions.mode` enum attrition\|objective\|extraction\|vip\|timeout + params | win_conditions.rs, extensions.rs | panels/win_conditions_card.rs | Gamemode/TBD_WinConditionEvaluator.c | — |
| T-936.2 | `tasks[]` tier + state | tasks.rs | panels/tasks_panel.rs | Objectives/TBD_TaskStateMachine.c, UI/TBD_TaskHud.c | T-936.1, T-676 |
| T-936.3 | `radioPlan` (existing $defs) authored | radio_plan.rs | panels/radio_panel.rs | existing TBD_RadioPlan.c | T-936.1 |
| T-936.4 | `weatherTimeline.keyframes[]` | weather.rs | panels/weather_timeline.rs | Gamemode/TBD_WeatherRuntime.c | T-936.2 |
| T-936.5 | `audio.{emitters[],musicCues[]}` | audio.rs | panels/audio_emitters.rs | Gamemode/TBD_AudioEmitter.c | T-936.4 |
| T-936.6 | `spawnModules[]` wave\|garrison | spawn_modules.rs | panels/spawn_modules.rs | Gamemode/TBD_DynamicSpawner.c | T-936.5 |
| T-936.7 | `tacticalGraphics[]` 4 kinds | tactical_graphics.rs | canvas/tactical_graphics.rs | — (planning map) | T-673, T-936.6 |

### 4.1 winConditions (T-936.1)
```json
{ "mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "s-12" }
```
Params by mode: objective → none (registry endOn), extraction → `extractionZoneId`, vip → `vipSlotId`,
timeout → `timeoutMinutes`, attrition → none. Absent block ⇒ today's `attrition` + derived `end_on`.
`TBD_WinConditionEvaluator.c` evaluates extraction (all living players of the faction inside the zone) and
vip (VIP dead ⇒ loss for owner; VIP extracted ⇒ win); objective/timeout map onto the registry's endOn triggers.

### 4.2 tasks (T-936.2)
Item `{id, title, tier: primary|secondary|optional, state: assigned|succeeded|failed, triggerId?, markerId?, description?}`.
Transitions: assigned→succeeded, assigned→failed; anything else is logged and ignored (server authoritative).
HUD: one marker per assigned task through the existing marker icon path.

### 4.3 radioPlan (T-936.3)
No schema change. `radio_plan.rs` validates freqMHz ≥ 30 (schema floor), duplicates, and the net cap
flatten.rs:1392 already enforces. flatten.rs:1927-1960 derives only when the authored block is absent.

### 4.4 weatherTimeline (T-936.4)
`{keyframes: [{atMinutes, weatherPreset, windDirDeg?, fog?}]}`, strictly increasing `atMinutes`; preset
vocabulary shared with environment.weatherPreset. Runtime applies each keyframe via the world weather manager.

### 4.5 audio (T-936.5)
`emitters[] {id, x, z, y?, sound, radiusM > 0, loop, triggerId?}`; `musicCues[] {id, event, track}` with
event ∈ mission_start|task_succeeded|task_failed|mission_end. One sound source per emitter in-game.

### 4.6 spawnModules (T-936.6)
`{id, kind: wave|garrison, factionKey, groupTemplate, x/z XOR zoneId, count, intervalSeconds?, maxAlive?, triggerId?}`.
Wave: spawn `count` on interval or trigger while alive < maxAlive. Garrison: spawn once, hold. Cleanup on end.

### 4.7 tacticalGraphics (T-936.7)
`{id, kind: phase_line|boundary|axis_of_advance|curved_arrow, points: [[x,z]…] (≥2), label?, sideKey?, style?}`.
Style reuses T-673's color/fill vocabulary. Canvas draws through the existing overlay layer (no render lane).

## 5. Rules every slice encodes
Verify the defect on main first (red pasted verbatim); perturbation proof with `touch` after restore;
`cargo test -p map-engine-core --all-features` only; no `git add -A`, no `git stash`, no `cargo xtask ci ci-local`;
`skip:` = FAIL; no .py/.sh/.mjs committed; file-length allowlists never grow (new code → new files);
Class-R parity tests scrub their own source; agents never merge/push/change status. Report schema:
pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns ·
found_not_fixed · deviations · commits.

## Claude Code prompt — T-936.1

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-936.1 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3-§4.1; docs/plans/t-936_1_plan.md; flatten.rs:2240-2260,2565-2590; compile.rs:156-230; mission.schema.json:1-100,1132-1152; TBD_ObjectiveRegistry.c:1-60,230-250; panels/mod.rs.
═══ PROBLEM ═══
flatten.rs:2573-2582 hardcodes mode "attrition"; the editor never authors winConditions and the payload never carries it.
Authors cannot choose a win rule; the mod only knows endOn triggers.
═══ SHIPPED ═══
$defs/winConditions (:1132), TBD_ObjectiveRegistry.c endOn triggers, Class-R parity suite. Do not redefine them.
═══ LANGUAGE GATE ═══
Rust (map-engine-core, Leptos), JSON Schema, Enforce Script (.c) compiled by `cargo xtask mod compile`.
═══ LOCKED ═══
- Absent block ⇒ byte-identical output to today (parity fixtures are the proof).
- extensions.rs is the ONLY passthrough; compile.rs and flatten.rs get one call site each.
- Evaluator ends the mission exactly once; objective/timeout reuse registry endOn triggers.
═══ DO ═══
1. Verify on main: payload with winConditions {mode: vip} flattens to attrition — paste the red.
2. Schema enum + params + golden + payload-schema declaration; `cargo xtask ci schema-validate`.
3. extensions.rs + win_conditions.rs (tests); wire compile.rs:156 and flatten.rs:2573-2582; register in mission/mod.rs.
4. win_conditions_card.rs mounted from panels/mod.rs, undoable ops; TBD_WinConditionEvaluator.c; mod compile.
5. Perturbation: invert the timeout comparison → red; restore, touch, green.
═══ DO NOT ═══
No dry run; no edits to TBD_ObjectiveRegistry.c; no files outside owns; no allowlist growth.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask ci schema-validate ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-936.1
═══ MANUAL ═══
Checklist: vip mission ends once on VIP death; extraction ends once when all players are in the zone.
═══ RETURN ═══
Report schema per spec §5. Ready for Cursor doc sync.
```

## Claude Code prompt — T-936.2

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-936.2 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3, §4.2; docs/plans/t-936_2_plan.md; mission/extensions.rs; T-676's trigger runtime API; TBD_ObjectiveRegistry.c:1-60; TBD_MarkerIcons.c (tbd-export) for the HUD icon path.
═══ PROBLEM ═══
No task state machine exists in schema, core, editor or mod; objectives fire endOn triggers but nothing tracks
primary/secondary/optional tasks or shows them on the HUD.
═══ SHIPPED ═══
T-936.1 (extensions.rs), T-676 (trigger runtime + completion events). Do not redefine either.
═══ LANGUAGE GATE ═══
Rust, JSON Schema, Enforce Script (.c).
═══ LOCKED ═══
- Transitions: assigned→succeeded|failed only; illegal ones logged and ignored; server authoritative.
- Tasks observe endOn triggers, never fire them (T-212 owns objectives).
- HUD marker only while assigned.
═══ DO ═══
1. Verify on main: payload with tasks flattens without them — paste the red.
2. Schema tasks[] + golden; tasks.rs (transition tests) registered in mission/mod.rs and extensions.rs.
3. tasks_panel.rs in panels/mod.rs, undoable; TBD_TaskStateMachine.c subscribing to T-676 completions; TBD_TaskHud.c.
4. Perturbation: legalise succeeded→assigned → red; restore, touch, green.
═══ DO NOT ═══
No compile.rs/flatten.rs edits; no objective logic; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask ci schema-validate ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-936.2
═══ MANUAL ═══
Checklist: three tasks reach one terminal state each; HUD hides completed tasks on all clients.
═══ RETURN ═══
Report schema per spec §5. Ready for Cursor doc sync.
```

## Claude Code prompt — T-936.3

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-936.3 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3, §4.3; docs/plans/t-936_3_plan.md; flatten.rs:185-215,1380-1430,1870-1960; mission.schema.json $defs/radioPlan,/net (:481); mission/extensions.rs; Radio/TBD_RadioPlan.c parse.
═══ PROBLEM ═══
Every mission's nets are synthesized (base + step × index); authors cannot see or set frequencies and the
payload has no radioPlan at all (flatten.rs:1876).
═══ SHIPPED ═══
$defs/radioPlan + $defs/net, derive_radio_plan (T-203), TBD_RadioPlan.c, T-936.1 extensions.rs.
═══ LANGUAGE GATE ═══
Rust (map-engine-core, Leptos).
═══ LOCKED ═══
- No schema change; the existing $defs are the contract.
- derive_radio_plan runs only when no authored block exists; default output byte-identical.
- Validation: freqMHz ≥ 30, no duplicate frequency, net cap of flatten.rs:1392.
═══ DO ═══
1. Verify on main: payload with radioPlan flattens to the derived plan — paste the red.
2. radio_plan.rs (tests) in mission/mod.rs; register radioPlan in extensions.rs.
3. Gate derive_radio_plan at flatten.rs:1927-1960; pass authored nets through.
4. radio_panel.rs in panels/mod.rs: nets, assignments, Reset-to-derived; undoable.
5. Perturbation: drop the duplicate check → red; restore, touch, green.
═══ DO NOT ═══
No RadioManagerEntity/world edits; no mod script edits; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-936.3
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per spec §5. Ready for Cursor doc sync.
```

## Claude Code prompt — T-936.4

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-936.4 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3, §4.4; docs/plans/t-936_4_plan.md; mission.schema.json:180-200; panels/env.rs:1-60,300-330; mission/extensions.rs; Gamemode/TBD_FrameworkManager.c stage hooks.
═══ PROBLEM ═══
environment.weatherPreset is one static value; weather never changes during a mission and no script drives it.
═══ SHIPPED ═══
environment block + panels/env.rs, T-936.1 extensions.rs, T-936.2 tasks (unrelated, packs earlier).
═══ LANGUAGE GATE ═══
Rust, JSON Schema, Enforce Script (.c).
═══ LOCKED ═══
- keyframes strictly increasing atMinutes; preset vocabulary shared with environment.weatherPreset.
- Server applies keyframes; clients follow replication; every transition logged.
- Absent block ⇒ byte-identical output.
═══ DO ═══
1. Verify on main: payload with weatherTimeline flattens without it — paste the red.
2. Schema + golden; weather.rs (ordering tests) in mission/mod.rs; register in extensions.rs.
3. weather_timeline.rs in panels/mod.rs, undoable; TBD_WeatherRuntime.c; mod compile.
4. Perturbation: accept equal atMinutes → red; restore, touch, green.
═══ DO NOT ═══
No edits to panels/env.rs; no compile.rs/flatten.rs edits; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask ci schema-validate ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-936.4
═══ MANUAL ═══
Checklist: three keyframes apply at their offsets on server and client.
═══ RETURN ═══
Report schema per spec §5. Ready for Cursor doc sync.
```

## Claude Code prompt — T-936.5

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-936.5 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3, §4.5; docs/plans/t-936_5_plan.md; mission/extensions.rs; the marker placement gesture in canvas/gestures.rs; TBD_TaskStateMachine.c events (T-936.2).
═══ PROBLEM ═══
Missions carry no authored sound: no emitters, no music cues, no script.
═══ SHIPPED ═══
T-936.1 extensions.rs; T-936.2 task events (task_succeeded/task_failed); marker placement gesture.
═══ LANGUAGE GATE ═══
Rust, JSON Schema, Enforce Script (.c).
═══ LOCKED ═══
- radiusM > 0, known events only, unique ids — validator refuses otherwise.
- Placement reuses the marker gesture; no new gesture code in gestures.rs.
- Absent block ⇒ byte-identical output.
═══ DO ═══
1. Verify on main: payload with audio flattens without it — paste the red.
2. Schema + golden; audio.rs (validator tests) in mission/mod.rs; register in extensions.rs.
3. audio_emitters.rs in panels/mod.rs, undoable; TBD_AudioEmitter.c; mod compile.
4. Perturbation: accept radius 0 → red; restore, touch, green.
═══ DO NOT ═══
No edits to canvas/gestures.rs; no compile.rs/flatten.rs edits; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask ci schema-validate ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-936.5
═══ MANUAL ═══
Checklist: emitter audible inside radius only; cue plays on its event.
═══ RETURN ═══
Report schema per spec §5. Ready for Cursor doc sync.
```

## Claude Code prompt — T-936.6

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-936.6 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3, §4.6; docs/plans/t-936_6_plan.md; mission/extensions.rs; T-676 trigger events; Gamemode/TBD_SpawnManager usage; faction group templates in the catalog.
═══ PROBLEM ═══
All AI is placed statically; there is no wave or garrison module in schema, editor or mod.
═══ SHIPPED ═══
T-936.1 extensions.rs; T-676 triggers; T-299/T-678 are separate tickets (do not implement them).
═══ LANGUAGE GATE ═══
Rust, JSON Schema, Enforce Script (.c).
═══ LOCKED ═══
- x/z XOR zoneId; known factionKey; positive counts; hard cap on maxAlive in the validator.
- Wave respects interval + maxAlive; garrison spawns once; cleanup on mission end.
- Absent block ⇒ byte-identical output.
═══ DO ═══
1. Verify on main: payload with spawnModules flattens without it — paste the red.
2. Schema + golden; spawn_modules.rs (validator tests) in mission/mod.rs; register in extensions.rs.
3. panels/spawn_modules.rs in panels/mod.rs, undoable; TBD_DynamicSpawner.c; mod compile.
4. Perturbation: allow x/z and zoneId together → red; restore, touch, green.
═══ DO NOT ═══
No group AI behaviour (T-678); no compile.rs/flatten.rs edits; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask ci schema-validate ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-936.6
═══ MANUAL ═══
Checklist: wave respects interval and maxAlive; garrison holds; both clean up at mission end.
═══ RETURN ═══
Report schema per spec §5. Ready for Cursor doc sync.
```

## Claude Code prompt — T-936.7

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-936.7 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3, §4.7; docs/plans/t-936_7_plan.md; mission.schema.json:1160-1200 ($defs/marker after T-673); canvas/overlays.rs:1-60; canvas/mod.rs; mission/extensions.rs.
═══ PROBLEM ═══
Markers are single-point icons; phase lines, boundaries and arrows cannot be drawn or compiled.
═══ SHIPPED ═══
T-673 marker color/fill/rotation/shape; T-936.1 extensions.rs; overlay layer + picking.
═══ LANGUAGE GATE ═══
Rust (map-engine-core, Leptos), JSON Schema.
═══ LOCKED ═══
- points ≥ 2 (validator); style reuses T-673's vocabulary; no new render lane in map-engine-render.
- Vertex drag is one undo step.
- Absent block ⇒ byte-identical output.
═══ DO ═══
1. Verify on main: payload with tacticalGraphics flattens without it — paste the red.
2. Schema + golden; tactical_graphics.rs (tests) in mission/mod.rs; register in extensions.rs.
3. canvas/tactical_graphics.rs in canvas/mod.rs: draw per kind, Catmull-Rom arrows, vertex drag, selection.
4. Perturbation: accept a one-point phase line → red; restore, touch, green.
═══ DO NOT ═══
No edits to overlays.rs beyond the mount; no compile.rs/flatten.rs edits; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask ci schema-validate ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-936.7
═══ MANUAL ═══
Draw one graphic of each kind; drag a vertex; undo once.
═══ RETURN ═══
Report schema per spec §5. Ready for Cursor doc sync.
```
