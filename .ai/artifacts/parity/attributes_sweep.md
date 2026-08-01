# Attributes sweep — all 93 `ATTR-FIELD-*` ids triaged against the live editor

**2026-08-01.** Produced by re-running the lane that died on the session limit. Written by one agent,
no sub-agents. Inputs: `docs/specs/Mission_Creator_Architecture/eden/attributes.md`, the three
recovered inventories in this directory, and the live source — every claim below that a recovered
inventory supplied was re-checked against the file before it was used, and the four places they were
wrong are in §4.

**Headline:** of 93 Eden attribute ids, **22 are factory-dispatchable today (a)**, **20 need
`mission.schema.json` widened (b)**, **29 need Enfusion-side support (c)**, and **22 are genuinely
`na` for this product (d)**. Parity is **7 match · 5 partial · 59 missing · 22 na**. The existing
parity table covers **3 of 93**, and disagrees with this sweep on one of the three.

---

## 1. Method

### Enumeration

```
$ cd docs/specs/Mission_Creator_Architecture/eden
$ grep -oE '\bATTR-FIELD-[A-Z0-9-]+\b' attributes.md | sort -u | wc -l
93
$ grep -oE '\bATTR-FIELD-[A-Z0-9-]+\b' attributes.md | sort -u \
    | sed -E 's/ATTR-FIELD-([A-Z]+)-.*/\1/' | sort | uniq -c
   3 CMT     3 COMP    10 GRP     3 LYR    10 MRK    31 OBJ    11 SCN    13 TRG     9 WP
```

Re-derived independently by counting the catalogue's own tables: OBJ 31 (`attributes.md:31-61`),
CMT 3 (`:75-77`), GRP 10 (`:89-98`), MRK 10 (`:112-121`), LYR 3 (`:133-135`), SCN 11 (`:149-166`),
TRG 13 (`:185-197`), WP 9 (`:209-217`), COMP 3 (`:235-237`). **93.** The `SYS` section
(`:223-225`) declares no ids — it is prose pointing at a wiki scrape, so there is nothing to triage.

### The build-class rule, stated so it is checkable

The point of the split is **factory vs workbench**, so the classes are drawn on that line:

| Class | Test applied | Program |
|---|---|---|
| **(a)** | No *mission-contract* blocker. The compiled schema already carries the key, **or** the value is editor-only state that is never compiled. Work is SPA (and where noted, website API). | factory, `executor: claude-code` |
| **(b)** | `mission.schema.json` must be widened first. A mod-side reader exists or is a small addition once the key is legal. | `executor: workbench` |
| **(c)** | Enfusion-side support must be built — no concept exists in `apps/mod/tbd-framework` at all, or a runtime system (AI, triggers, damage model) would have to exist first. Usually needs (b) as well; the deeper blocker wins. | `executor: workbench` |
| **(d)** | Out of scope: an A3-engine concept with no Enfusion analogue, a scripting handle with no scripting layer, or something TBD refused **by design with a test pinning the refusal**. | none — closed question |

**(d) ⇔ parity `na` by construction.** All 22 `d` rows are `na` and no other row is.

**(a) is wider than "the SPA can do it alone" for exactly two families**, and this is called out
rather than hidden: `COMP-*` (a composition library needs a table + API) and `SCN-PICTURE` /
`SCN-OVERVIEW-TEXT` (the API already accepts them; only the caller is missing). Both are still
factory work — no mission contract and no Enfusion change — which is the distinction the class is
drawn to capture.

### Commands run, with their counts

```
$ grep -c '"additionalProperties": false' packages/tbd-schema/schema/mission.schema.json
25                          # confirms the README; closed on $defs/slot, group, meta, environment,
                            # marker, entity, flow, winConditions, settings, zone, …
$ grep -c '"additionalProperties": false' packages/tbd-schema/schema/mission-editor-payload.schema.json
0                           # THE EDITOR PAYLOAD IS OPEN — see the finding below
```

Word-boundary sweep over `apps/mod/tbd-framework` (`grep -rwoE <word> . | wc -l`):

| word | hits | word | hits | word | hits |
|---|---|---|---|---|---|
| `stance` | **0** | `rank` | **0** | `leaderSlotId` | **0** |
| `formation` | **0** | `combatMode` | **0** | `speedMode` | **0** |
| `skill` | **0** | `health` | **0** | `simulation` | **0** |
| `behavior` | **0** | `behaviour` | 44 — all prose | `trigger` | 109 — win-condition triggers, not Eden triggers |
| `tag` | 60 | `callsign` | 34 | `marker` | 131 — the T-181 consumer |
| `fuel` | 1 | `ammo` | 12 | `damage` | 95 — the damage system, not authored initial health |

Methodology check, reproducing the trap the README records:

```
$ grep -rl  stance apps/mod/tbd-framework | wc -l    → 39 files
$ grep -rlw stance apps/mod/tbd-framework | wc -l    → 0  files
```

**Confirmed exactly as recorded.** Every one of the 39 was `instance`.

Frontend, `apps/website/frontend/src` (`grep -rnwE <w> --include=*.rs .`): `combatMode` 0,
`speedMode` 0, `formation` 1 (prose about a placement anchor, `editor_ops.rs:1324`), `behaviour` 13
(all prose in doc comments).

### The finding that resized the (a) column

`packages/tbd-schema/schema/mission-editor-payload.schema.json` carries **zero**
`additionalProperties: false` and its `editor` block is explicitly unconstrained — *"`squads`,
`slots` and `editorLayers` are intentionally unconstrained (no per-item schema) so validation stays
O(1)"*. Its root already declares `markers`, `vehicles`, `zones`, `objectives`.

So the 25 closed objects live **only** on the compiled mod contract. **Anything Eden defines as
editor-only — comments, layer visibility, layer transform-enable, composition metadata — is not
contract-blocked at all.** That moves 9 ids (CMT×3, LYR×2, COMP×3, plus `LYR-NAME` already shipped)
out of the workbench program and into the factory. Without this check they would all have been
mis-filed as (b).

### The `match`/`partial` audit against the T-216 drop ledger

`crates/map-engine-core/src/mission/flatten.rs:2584-2649` — **read in full and verified verbatim.**
It enumerates six author-facing values the compile silently drops (`leaderSlotId`, slot `tag` /
`callsign` / `rank` / `stance`, the vehicle roster) and states *"not one of its twenty-two test
selectors names a test in this file"*. It also supplies the exact contract delta per value, and the
warning that emitting them today would 500 `/compiled` rather than deliver the feature.

Cross-checked against the mod reader: `TBD_MissionSlotStruct.c:59-69` declares exactly `id · uid ·
faction · groupCallsign · role · kit · x · z · y · headingDeg · loadout`. **No `tag`, `callsign`,
`rank` or `stance` — the drop is real on both sides of the wire, not only in `flatten`.**

**Three ids demoted from what the editor surface alone would have scored:** `OBJ-CALLSIGN`,
`OBJ-RANK`, `OBJ-STANCE` are authored by live controls and are `partial`, never `match`.

---

## 2. The full table — all 93

`tbd_id` uses the `gap_analysis.md` vocabulary where a row exists, otherwise names the live surface.

### Object — `OBJ` (31)

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-OBJ-TYPE | ATTR-TAB-002 | partial | a | `assetId` authored on palette drop (`store.rs:551`) and mutable (`:1276`), but `read_attrs` never reads it (`editor_ops.rs:122-132`) so the type cannot be changed in the modal. Compiles to `kit:` — 342 of 354 characters have no `kit-aliases.json` row and take a faction default (`flatten.rs:411,479`) | T-082 |
| ATTR-FIELD-OBJ-VARNAME | — | na | d | Scripting handle. TBD has no author-facing script layer; `slot.uid` (`$defs/slot`) already gives durable identity | — |
| ATTR-FIELD-OBJ-INIT | — | na | d | SQF string eval'd at spawn. TBD compiles a declarative JSON document; author-supplied code has no evaluator and would be a new attack surface | — |
| ATTR-FIELD-OBJ-POSITION | ATTR-TAB-001 | match | a | X/Y/Z editable (`attributes.rs:255-319`) → `store.rs:1345-1355`; compiled `x`/`z`/`y`, all three read by `TBD_MissionSlotStruct.c:65-67`. Caveat: editing X or Y resets Z to 0.0 (`store.rs:1354-1358`) — deliberate, see §4 | T-049 (shipped) |
| ATTR-FIELD-OBJ-ROTATION | ATTR-TAB-001 | match | a | Number field, normalised `[0,360)` (`store.rs:1351-1353`) → compiled `headingDeg`, read `TBD_MissionSlotStruct.c:68`. Vehicles have their own `Heading°` input (`eden_chrome.rs:2088-2104`) | T-049 (shipped) |
| ATTR-FIELD-OBJ-SIZE | — | missing | c | No size/scale key on `$defs/slot` or `$defs/entity` (both closed). `entities[]` — the prop lane — has no consumer on any shipped build (`mission.schema.json:72`) | new — prop transform |
| ATTR-FIELD-OBJ-SHAPE | — | missing | b | Eden pairs `IsRectangle` with `placementRadius` to shape a random-spawn area. `$defs/shape` exists (circle\|polygon) but is zone-only; `$defs/slot` is closed | new — placement scatter |
| ATTR-FIELD-OBJ-PLACEMENT-RADIUS | — | missing | b | Same pair. A 2D top-down editor is the *natural* surface for this — draw the scatter circle. `$defs/slot` closed | new — placement scatter |
| ATTR-FIELD-OBJ-PLAYER-SP | — | na | d | Single-player playable flag. TBD is MP-only milsim | — |
| ATTR-FIELD-OBJ-PLAYABLE-MP | — | na | d | Every TBD slot is a roster seat by construction; `TBD_SpawnManager.c:963,1166` spawns each body with **AI disabled**. There is no playable/AI distinction to author | — |
| ATTR-FIELD-OBJ-ROLE-DESC | ATTR-TAB-002 | partial | a | `role` (`attributes.rs:336`, placeholder `"Rifleman"`) doubles as label and description and does reach the game (`slot.role`, read `TBD_MissionSlotStruct.c:63`). No separate free-text description | T-082 |
| ATTR-FIELD-OBJ-LOCK | — | missing | c | Vehicle lock state. `lock` word-boundary in `apps/mod` = 4, none an authored vehicle lock. T-215 shipped vehicle placement + cargo, not lock | new — vehicle states |
| ATTR-FIELD-OBJ-SKILL | ATTR-TAB-003 | na | d | AI skill, and there are no AI units to skill (`TBD_SpawnManager.c:963`). `skill` word-boundary in `apps/mod` = **0**. **`gap_analysis.md:100` scores this `missing`; that is wrong — see §4** | — |
| ATTR-FIELD-OBJ-HEALTH | — | missing | c | No authored initial health anywhere. `health` word-boundary in `apps/mod` = **0**; the 95 `damage` hits are the runtime damage system | new — entity states |
| ATTR-FIELD-OBJ-FUEL | — | missing | c | `fuel` word-boundary in `apps/mod` = 1. No key on `$defs/entity` (closed) and no reader | new — vehicle states |
| ATTR-FIELD-OBJ-AMMO | — | missing | c | Vehicle/turret ammo as an *attribute*. `slot.loadout.cargo` and `$defs/entityInventory` cover carried items, not a turret ammo count | new — vehicle states |
| ATTR-FIELD-OBJ-RANK | ORBAT Manager | **partial** | b | **Authored and dropped.** Input `orbat_manager.rs:1336-1355` → `store.rs:1301` → dropped at compile (T-216 ledger, `flatten.rs:2584-2649`). `$defs/slot` closed; `TBD_MissionSlotStruct.c` has no field | T-216 follow-on (new) |
| ATTR-FIELD-OBJ-STANCE | ATTR-TAB-001 | **partial** | c | **Authored and dropped.** `<select>` stand/crouch/prone `attributes.rs:297-312` → `store.rs:1246` → dropped. `stance` word-boundary in `apps/mod` = **0** — needs a schema key *and* an Enfusion spawn-pose call | T-216 follow-on (new) |
| ATTR-FIELD-OBJ-DYN-SIM | — | na | d | A3 dynamic simulation. `simulation` word-boundary in `apps/mod` = **0**; Enfusion has no equivalent author-facing system | — |
| ATTR-FIELD-OBJ-WAKE-DYN-SIM | — | na | d | Same system | — |
| ATTR-FIELD-OBJ-ENABLE-SIM | — | na | d | Same system | — |
| ATTR-FIELD-OBJ-SIMPLE-OBJ | — | na | d | A3 render-optimisation concept (`objectIsSimple`); no Enfusion analogue | — |
| ATTR-FIELD-OBJ-SHOW-MODEL | — | missing | c | `hideObject`. No per-entity hide; `entities[]` unconsumed. Distinct from the editor's own layer visibility (`LYR-ENABLE-VIS`) | new — entity states |
| ATTR-FIELD-OBJ-ALLOW-DAMAGE | — | missing | c | Per-entity invulnerability. No key, no reader | new — entity states |
| ATTR-FIELD-OBJ-STAMINA | — | missing | c | No per-slot stamina key or reader. `UNKNOWN:` whether Reforger exposes a per-character stamina toggle at all — a Workbench API check, not a code search | new — entity states |
| ATTR-FIELD-OBJ-REVIVE | — | na | d | **Refused by design.** `SETTINGS_UNREAD_NOTE` (`eden_chrome.rs:370`) with rationale at `:4672-4685` — *"TBD events are one life"*. `$defs/settings` is `respawn`/`spectatorPolicy`/`nightVision`, mission-global | — |
| ATTR-FIELD-OBJ-DOORS | — | na | d | Per-model door states, driven in Eden by 3D LMB/RMB gestures on the door itself. No 2D top-down representation and no Enfusion authoring path | — |
| ATTR-FIELD-OBJ-LOCAL-ONLY | — | na | d | A3 MP locality flag. Enfusion replication is engine-managed, not per-entity authored | — |
| ATTR-FIELD-OBJ-UNIT-NAME | — | missing | b | No per-slot display name. `slot.id` is **derived** each compile (`{faction}:{groupCallsign}:{role}:{index}`) and shifts under renames — it is not a name field. `$defs/slot` closed | T-082 |
| ATTR-FIELD-OBJ-FACE | — | na | d | Character face/identity — a 3D appearance value with no 2D top-down surface and no mod reader | — |
| ATTR-FIELD-OBJ-CALLSIGN | ORBAT Manager | **partial** | b | **Authored and dropped.** Input `orbat_manager.rs:1314-1333` → `store.rs:1293` → dropped (T-216). Note `PASTE_KNOWN_SLOT_KEYS` (`store.rs:2571-2582`) also omits it, so it survives paste only via the extras branch | T-216 follow-on (new) |

> **TBD-only, no Eden id:** slot `tag` (`MED · ENG · SL…`, `attributes.rs:337`). It is authored,
> it is in `PASTE_KNOWN_SLOT_KEYS`, and it is **dropped at compile** by the same T-216 ledger. It
> belongs in the T-216 follow-on with `callsign`/`rank`/`stance` even though this catalogue has no
> id for it. Same for a squad's `leaderSlotId`.

### Comment — `CMT` (3)

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-CMT-TITLE | — | missing | a | Editor-only annotation, **never compiled** (`attributes.md:79`) → the open editor payload carries it with no contract change. No entity, no tool, no ticket in the registry — the mission-detail "Comments" Sheet (`missions.rs:2501-2513`) is a social thread stub, not a canvas annotation | **new** |
| ATTR-FIELD-CMT-TOOLTIP | — | missing | a | Same | **new** |
| ATTR-FIELD-CMT-POSITION | — | missing | a | Same. `grep -in comment mission.schema.json` → 0 hits, as expected for editor-only state | **new** |

### Group — `GRP` (10)

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-GRP-VARNAME | — | na | d | Scripting handle; no script layer | — |
| ATTR-FIELD-GRP-INIT | — | na | d | SQF string; no evaluator | — |
| ATTR-FIELD-GRP-CALLSIGN | LEFT-ORBAT-001 | match | a | Squad callsign authored in ORBAT Manager → compiled `$defs/group.callsign`, copied verbatim into `slot.groupCallsign` and read by `TBD_MissionSlotStruct.c:62`. One of only two ORBAT identity values that survives the compile | T-180 (shipped) |
| ATTR-FIELD-GRP-PLACEMENT-RADIUS | — | missing | b | `$defs/group` is `callsign`/`type`/`roles`, closed. Natural 2D control | new — placement scatter |
| ATTR-FIELD-GRP-COMBAT-MODE | — | missing | c | `combatMode` word-boundary = **0** in both frontend and `apps/mod`. Presupposes AI groups, which do not exist | T-079 |
| ATTR-FIELD-GRP-BEHAVIOUR | — | missing | c | `behaviour`/`behavior` as a field = **0** in both trees (44 mod hits + 13 frontend hits are all doc prose). No AI subject | T-079 |
| ATTR-FIELD-GRP-FORMATION | — | missing | c | `formation` = 1 frontend hit (prose, `editor_ops.rs:1324`), **0** in `apps/mod` | T-079 |
| ATTR-FIELD-GRP-SPEED-MODE | — | missing | c | `speedMode` = **0** both trees | T-079 |
| ATTR-FIELD-GRP-DYN-SIM | — | na | d | Same A3 system as `OBJ-DYN-SIM` | — |
| ATTR-FIELD-GRP-DELETE-EMPTY | — | na | d | Garbage-collects an emptied AI group; no AI groups to collect | — |

### Marker — `MRK` (10)

Every row `missing`: the **producer does not exist**. Tab button `eden_chrome.rs:3046`, no match arm,
body is one sentence at `:3337` (`"Marker placement lands in T-069."`), pinned by a test at
`:4499-4502` that says the stub *"must be untouched"*. `PaletteKind` (`:1833`) has no marker variant.
The two doc mutators `set_faction_briefing_marker` / `remove_faction_briefing_marker`
(`store.rs:2030`, `:2070`) have **zero product callers** — re-verified word-boundary, repo-wide:
all 15 + 9 hits are inside `store.rs` itself.

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-MRK-TYPE | RIGHT-STUB-002 | missing | a | `$defs/marker.icon` already exists — a **closed 64-alias enum** (verified: enum length 64). Schema is ready; the authoring UI is not | T-069 / T-213 |
| ATTR-FIELD-MRK-VARNAME | — | na | d | Scripting handle | — |
| ATTR-FIELD-MRK-TEXT | RIGHT-STUB-002 | missing | a | `$defs/marker.label` exists and is `required` | T-069 / T-213 |
| ATTR-FIELD-MRK-POSITION | RIGHT-STUB-002 | missing | a | `$defs/marker.x`/`.z` exist and are `required`. 2D map is the ideal surface | T-069 / T-213 |
| ATTR-FIELD-MRK-SIZE | — | missing | b | `$defs/marker` is exactly `{x,z,icon,label}` and closed. Eden's Area marker (metres in world space) has no representation | new — marker style |
| ATTR-FIELD-MRK-ROTATION | — | missing | b | Same closed def | new — marker style |
| ATTR-FIELD-MRK-SHAPE | — | missing | b | Icon-vs-Area is the whole second half of Eden's marker model; absent | new — marker style |
| ATTR-FIELD-MRK-BRUSH | — | missing | b | Area fill pattern; presupposes Area markers | new — marker style |
| ATTR-FIELD-MRK-COLOR | — | missing | b | Closed def. Note colour is partly implied — `TBD_MarkerService.BuildForPlayer` (`TBD_MarkerData.c:82`) is **side-scoped**, sending only the caller's side's rows | new — marker style |
| ATTR-FIELD-MRK-ALPHA | — | missing | b | Closed def | new — marker style |

### Layer — `LYR` (3)

Eden layers and TBD "Editor Layers" are different concepts (`gap_analysis.md:96` scores
`LAYER-CREATE-001` as `tbd_only`), but all three Eden fields have a direct TBD meaning.

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-LYR-NAME | LEFT-LAYER-005 | match | a | `rename_editor_layer` (`store.rs:1886`), with create/reparent/remove/refile alongside (`:1872`, `:1895`, `:1527`, `:1915`) | T-037 (shipped) |
| ATTR-FIELD-LYR-ENABLE-XFORM | — | missing | a | No per-layer transform lock. Editor-only state — the editor payload is open, so no contract change | **new** |
| ATTR-FIELD-LYR-ENABLE-VIS | — | missing | a | **No layer visibility toggle exists** — `editor_ops.rs` has one layer fn (`set_active_layer:1025`) and `store.rs` has five, none about visibility. Not to be confused with the 12 **world-layer** checkboxes (`world_layer_prefs.rs:61-76`), which are localStorage basemap prefs on a different object | **new** |

### Scenario — `SCN` (11)

Read `editor_inventory_mission_settings.md` first: most absences here are **enforced**, not
overlooked. Every environment write goes through `author_env` (`eden_chrome.rs:271`), which refuses
any key outside `CARRIED_ENV_KEYS` (5, pinned by a test asserting `len() == 5` at `:4648-4652`) or
`AUTHORED_FLOW_KEYS` (4, pinned `:4700-4711`).

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-SCN-TITLE | TOP-TITLE-001 | match | a | Top-strip input (`eden_chrome.rs:1047-1063`) → `meta.title` → compiled (`mission/compile.rs:251`). Caveat: `RowMirror` PATCHes only `time_of_day` + `weather` (`:501-508`), so the editor title never reaches the library row | T-049 (shipped) |
| ATTR-FIELD-SCN-AUTHOR | — | na | d | Server-assigned from the authenticated account. `$defs/meta.author` **exists** and `ModMeta.author` (`flatten.rs:263`) is emitted from `mission.author` (`:2177`) — the value reaches the game. A hand-typed author field would be a spoofing surface, not a gap | — |
| ATTR-FIELD-SCN-PICTURE | — | missing | a | DTO and API already exist (`dto.rs:850`, `handlers/missions.rs:658`); **no editor control and no frontend caller.** No contract change needed | **new** |
| ATTR-FIELD-SCN-OVERVIEW-TEXT | — | missing | a | **The clearest hole in the whole sweep.** `PATCH /missions/:id` accepts `briefing` (`handlers/missions.rs:657`), `meta.briefing` reaches the doc via hydrate (`store.rs:1658-1663`) — and **nothing in the SPA can edit it**, on either the create dialog or the editor | **new** |
| ATTR-FIELD-SCN-DLC | — | na | d | Reforger declares workshop dependencies in the mod manifest, not per mission | — |
| ATTR-FIELD-SCN-REQUIRE-DLC | — | na | d | Same | — |
| ATTR-FIELD-SCN-TIME | TOP-SETTINGS-001 | match | a | `<input type="time">` (`eden_chrome.rs:3850-3868`) → carried key `time` → compiled `environment.dateTime` → `ModEnvironment.date_time` (`flatten.rs:270`). Two surfaces (dialog + top strip), both mirror to the row | shipped |
| ATTR-FIELD-SCN-WEATHER | TOP-SETTINGS-001 | match | a | `<select>` of 4 presets (`:3877-3895`) → `weatherPreset` → `ModEnvironment.weather_preset` (`:272`) | shipped |
| ATTR-FIELD-SCN-FOG | — | missing | c | Not independent — only the `dense_fog` preset. Refused: `keys_nothing_reads_are_not_authored` (`eden_chrome.rs:4622-4629`) asserts `fog` is not carried. Blocked on a **reader**, not on the schema | new — env readers (workbench) |
| ATTR-FIELD-SCN-WIND | — | missing | c | The sharpest case of *"a schema field is not a reader"* (`:4619-4621`): `$defs/environment.windDirDeg` **is declared**, and `ModEnvironment` (`flatten.rs:268-275`) emits only `dateTime` + `weatherPreset` — the field is not even serialised | new — env readers (workbench) |
| ATTR-FIELD-SCN-VIEW-DIST | ENV-SETTINGS-002 | missing | c | **Was built, then removed by T-193** (`b30f5490`). Refused at `:4624`. Residual dead DTO fields `dto.rs:991-992` still parsed at `editor_ops.rs:205-212`. **`gap_analysis.md:107` still scores this `partial` — stale, see §4** | new — env readers (workbench) + a cleanup |

### Trigger — `TRG` (13)

All absent. `mission.schema.json` has 2 `trigger` hits, both prose inside `description` strings
(`:429`, `:488`); none of the 30 `$defs` is a trigger. Frontend `trigger` hits are re-read ticks,
file-download triggers and an auth test string. **Nearest analogue and it is not a trigger:**
`$defs/zone` = `{id, type, label, faction, shape, rules}` + `$defs/zoneRules` (16 keys), shipped as a
draw tool at T-582. `INFERRED:` zones give a typed area with declarative rules but no
activation/condition/timer/effects model.

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-TRG-VARNAME | RIGHT-STUB-003 | na | d | Scripting handle | — |
| ATTR-FIELD-TRG-TEXT | RIGHT-STUB-003 | missing | b | `$defs/zone.label` is the shape of the answer; no trigger def exists to hang it on | T-079 |
| ATTR-FIELD-TRG-POSITION | RIGHT-STUB-003 | missing | b | Geometry is the solved half — `$defs/shape` (circle\|polygon) + the T-582 draw tool | T-079 |
| ATTR-FIELD-TRG-ROTATION | RIGHT-STUB-003 | missing | b | Same | T-079 |
| ATTR-FIELD-TRG-SIZE | RIGHT-STUB-003 | missing | b | Same | T-079 |
| ATTR-FIELD-TRG-SHAPE | RIGHT-STUB-003 | missing | b | `$defs/shape` has no rectangle; Eden's `IsRectangle` has no home | T-079 |
| ATTR-FIELD-TRG-TYPE | RIGHT-STUB-003 | missing | c | Trigger type (None/Guarded/Switch/…) presupposes a runtime trigger system Enfusion-side | T-079 |
| ATTR-FIELD-TRG-ACTIVATION | RIGHT-STUB-003 | missing | c | Activating side/party. No runtime | T-079 |
| ATTR-FIELD-TRG-ACTIVATION-TYPE | RIGHT-STUB-003 | missing | c | Present/Not Present/Detected By. No runtime | T-079 |
| ATTR-FIELD-TRG-CONDITION | RIGHT-STUB-003 | missing | c | Eden's is an SQF expression. TBD's answer must be a **structured condition model**, not an eval'd string — an evaluator is out of scope for the same reason `OBJ-INIT` is `na` | T-079 |
| ATTR-FIELD-TRG-ON-ACTIVATION | RIGHT-STUB-003 | missing | c | Same: a structured effect list, not code. `$defs/winConditions` (`mode`, `endOn`) is the only effect vocabulary today and it is mission-global | T-079 |
| ATTR-FIELD-TRG-REPEATABLE | RIGHT-STUB-003 | missing | c | No runtime | T-079 |
| ATTR-FIELD-TRG-TIMER | RIGHT-STUB-003 | missing | c | No runtime | T-079 |

### Waypoint — `WP` (9)

All absent, and more deeply than the id list suggests. `grep -rin waypoint` across
`apps/website/frontend/src`, `crates/` and `packages/tbd-schema/schema/` returns **exactly 2 hits** —
`mission.schema.json:608`/`:609`, the strings `"waypoint"`/`"waypoint2"` inside the marker **icon
alias enum**. Glyph names, not entities. `waypoint` word-boundary in `apps/mod` = 1, the same icon file.

**The scoping fact for T-079:** a waypoint orders an AI group, and `TBD_SpawnManager.c:963,1166`
spawns every slot body with **AI disabled**. Waypoints today would have no subject.

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-WP-TYPE | — | missing | c | MOVE/SAD/GUARD/… — each is an AI behaviour to implement, not a field to store | T-079 |
| ATTR-FIELD-WP-DESCRIPTION | — | missing | b | A string on a waypoint entity; no waypoint `$def` exists | T-079 |
| ATTR-FIELD-WP-ORDER | — | missing | b | Sequence index; storage-only once the entity exists | T-079 |
| ATTR-FIELD-WP-POSITION | — | missing | b | 2D map is the ideal surface; needs the entity | T-079 |
| ATTR-FIELD-WP-COMBAT-MODE | — | missing | c | Per-waypoint AI override; `combatMode` = 0 hits anywhere | T-079 |
| ATTR-FIELD-WP-BEHAVIOUR | — | missing | c | 0 hits as a field | T-079 |
| ATTR-FIELD-WP-FORMATION | — | missing | c | 0 hits in `apps/mod` | T-079 |
| ATTR-FIELD-WP-SPEED | — | missing | c | 0 hits as a field | T-079 |
| ATTR-FIELD-WP-CONDITION | — | missing | c | Completion condition — same structured-model argument as `TRG-CONDITION`. Eden also allows completion via connected triggers (`CONN-WP-ACT-001`), so this is coupled to the trigger slice | T-079 |

### Composition metadata — `COMP` (3)

Not entity attributes — save-dialog metadata. Zero hits repo-wide for `save.*[Cc]omposition`,
`save_group`, `SaveComposition`, `user_composition`, `custom_composition`. The only frontend
`composition` occurrences are `asset_catalog.rs:345-844`, a **path-string classifier**:
`derive_object_alias` (`:352`) prefixes `comp:` vs `prop:` by whether the Arma resource path contains
`"Composition"`. That **consumes Bohemia's shipped composition prefabs; it does not author one.**

| eden_id | tbd_id | parity | build_class | gap_notes | maps_to_ticket |
|---|---|---|---|---|---|
| ATTR-FIELD-COMP-TITLE | — | missing | a | Composition metadata is a user artifact, never part of the compiled mission → no contract blocker. Needs a table + API + SPA, all factory-side | T-078 |
| ATTR-FIELD-COMP-AUTHOR | — | missing | a | Same. Should be server-assigned like `SCN-AUTHOR`, not typed | T-078 |
| ATTR-FIELD-COMP-CATEGORY | — | missing | a | Same | T-078 |

---

## 3. Summary counts

### By parity

| parity | count | share |
|---|---|---|
| **match** | **7** | 7.5% |
| **partial** | **5** | 5.4% |
| **missing** | **59** | 63.4% |
| **na** | **22** | 23.7% |
| **total** | **93** | |

`match` (7): `OBJ-POSITION`, `OBJ-ROTATION`, `GRP-CALLSIGN`, `LYR-NAME`, `SCN-TITLE`, `SCN-TIME`,
`SCN-WEATHER`.
`partial` (5): `OBJ-TYPE`, `OBJ-ROLE-DESC`, and the three T-216 casualties `OBJ-RANK`,
`OBJ-CALLSIGN`, `OBJ-STANCE`.

**No id was upgraded to `match` on editor evidence alone.** Three that a modal-only survey would
have scored `match` are `partial` because of the drop ledger.

### By build class — the headline

| class | count | share | program |
|---|---|---|---|
| **(a)** SPA-buildable today | **22** | **23.7%** | factory |
| **(b)** schema-blocked | **20** | 21.5% | `executor: workbench` |
| **(c)** mod-blocked | **29** | 31.2% | `executor: workbench` |
| **(d)** `na` | **22** | 23.7% | closed |
| | 93 | | |

**Read this as: 22 factory · 49 workbench · 22 closed.** Just under a quarter of Eden's attribute
surface is dispatchable to the factory today, and of that 22, **seven are already shipped** — so the
genuinely new factory work is **15 ids**, concentrated in four small slices (comments, markers-core,
layer flags, mission presentation).

### Cross-tab

| | (a) | (b) | (c) | (d) | total |
|---|---|---|---|---|---|
| match | 7 | 0 | 0 | 0 | **7** |
| partial | 2 | 2 | 1 | 0 | **5** |
| missing | 13 | 18 | 28 | 0 | **59** |
| na | 0 | 0 | 0 | 22 | **22** |
| **total** | **22** | **20** | **29** | **22** | **93** |

### By family

| family | n | (a) | (b) | (c) | (d) |
|---|---|---|---|---|---|
| OBJ | 31 | 4 | 5 | 9 | 13 |
| TRG | 13 | 0 | 5 | 7 | 1 |
| SCN | 11 | 5 | 0 | 3 | 3 |
| GRP | 10 | 1 | 1 | 4 | 4 |
| MRK | 10 | 3 | 6 | 0 | 1 |
| WP | 9 | 0 | 3 | 6 | 0 |
| CMT | 3 | 3 | 0 | 0 | 0 |
| LYR | 3 | 3 | 0 | 0 | 0 |
| COMP | 3 | 3 | 0 | 0 | 0 |

The shape: **OBJ and TRG carry two thirds of the workbench load.** CMT/LYR/COMP are pure factory
because Eden itself defines them as editor-only. MRK splits cleanly — the four schema-carried fields
are factory, the six style fields are one schema widening.

---

## 4. Corrections

Four claims checked and found wrong, plus two stale rows in the parity table itself.

### 4.1 The Z-zeroing framing is wrong — `editor_inventory_attributes_modal.md:36-37`

> *"**A real behaviour worth a ticket:** editing X or Y **resets Z to 0.0**"*

The **behaviour** is real and the line number is right. The **framing is not.** `store.rs:1354-1358`
reads:

```rust
} else if x.is_some() || y.is_some() {
    pz = 0.0; // terrain-follow; DEM z is sampled on the JS side
}
```

It carries an explicit rationale: moving in plan view drops the slot back onto the terrain, and the
DEM sample re-derives Z. That is a deliberate terrain-follow, not a silent defect. Calling it "a real
behaviour worth a ticket" would have opened a ticket to fix intended behaviour. It may still be a UX
wart (a hand-typed Z is discarded by a later X edit) — but that is a different, smaller ticket, and
it must be argued against the terrain-follow intent rather than as a bug report.

### 4.2 `gap_analysis.md:100` scores `ATTR-FIELD-OBJ-SKILL` as `missing` — it is `na`

The row reads `ATTR-FIELD-OBJ-SKILL | ATTR-TAB-003 | missing | — | States stub`. AI skill has no
subject: `TBD_SpawnManager.c:963,1166` spawns every slot body with **AI disabled**, and `skill`
word-boundary in `apps/mod/tbd-framework` returns **0**. Scoring it `missing` implies buildable work
and has it feeding the States-tab ticket. It is a closed question unless TBD adds AI units, which is
a product decision far larger than an attribute.

### 4.3 `gap_analysis.md:107` — `ENV-SETTINGS-002` is stale (confirms the README)

Scored `partial` with `gap_notes: "Thermals + view dist in dialog"`. Neither is in the dialog. T-193
removed both, and `eden_chrome.rs:4622-4629` now actively asserts they are *not* authorable. The
recovered inventory and the README both flagged this; **independently confirmed here** by reading the
test. The dead DTO fields `dto.rs:991-992` are still parsed at `editor_ops.rs:205-212`.

### 4.4 `T-213`'s own summary cites a dead line number

The registry entry for T-213 says *"`eden_chrome.rs:1528` is a stub reading 'Marker placement lands
in T-069'"*. The stub is at **`eden_chrome.rs:3337`**; line 1528 is unrelated. The rest of T-213's
summary is accurate (doc markers root, `compile.rs` emits `markersById`, flatten never reads it,
mod side shipped and side-scoped, 64 cap — all confirmed). Worth fixing when the ticket is promoted,
since the line is the first thing an implementer will open.

### 4.5 The `T-069` spec premise is dead — confirmed

`t069_markers_on_map.md:14` says *"`MapMarker` exists in `state/schema.ts`"*. Re-verified: `MapMarker`
returns 0 hits repo-wide and no `schema.ts` exists (React deleted at T-159.29.3). **The spec needs
rewriting, not promoting.** This is the one recovered claim most likely to waste an implementer's
first hour, and it holds.

### 4.6 Everything else in the three inventories held up

Spot-checked and **correct**: `TABS` (`attributes.rs:16`), `states_tab` taking no arguments
(`:351`), `SlotAttrs`' nine fields with no `assetId`/`callsign`/`rank` (`editor_ops.rs:122-132`,
`:631-663`), text fields committing `on:input` (`:246`, with the source itself calling one undo step
per keystroke "the oracle behavior" at `:230-231`), `CARRIED_ENV_KEYS` = 5 pinned by test, the
4-key flow set, the `:4624` refusal list, the marker stub at `:3337`, `set_faction_briefing_marker`
with zero product callers, `$defs/marker` = `{x,z,icon,label}` with a 64-length icon enum, the
`stance` 39-vs-0 grep trap, and the 25 `additionalProperties: false`.

One nuance worth recording rather than correcting: the modal's text fields commit per keystroke, but
the ORBAT Manager's `callsign`/`rank` inputs commit on `on:change` (`orbat_manager.rs:1325`,
`:1347`) while only *tracking* on `on:input`. The two identity surfaces have **different commit
semantics**, which any "one undo step per keystroke" ticket must not flatten.

---

## 5. Proposed ticket groupings

**Existing tickets to promote/scope — do not duplicate these.**

| # | Slice | ids | class | Ticket | Action |
|---|---|---|---|---|---|
| G1 | **Marker authoring — the four schema-carried fields** | `MRK-TYPE`, `MRK-TEXT`, `MRK-POSITION` (3) | a | **T-213** (`idea`) — promote; **T-069** (`deferred`) | Scope to `{x,z,icon,label}` only. **Rewrite the T-069 spec first** (§4.5) and fix T-213's line cite (§4.4). Two tickets for one job — collapse: T-069 owns the map tool, T-213 is the same work under another name |
| G2 | **Marker style — Area markers** | `MRK-SIZE`, `-ROTATION`, `-SHAPE`, `-BRUSH`, `-COLOR`, `-ALPHA` (6) | b | **new**, blocked by G1 | One `$defs/marker` widening. Do **not** fold into G1 — it would convert a factory ticket into a workbench one |
| G3 | **Triggers** | 13 `TRG-*` | 5×b, 7×c, 1×d | **T-079** (`idea`) | T-079 currently bundles triggers + waypoints + systems. **Split it.** Geometry rides the shipped T-582 zone draw tool; the activation/effects model is a new Enfusion runtime |
| G4 | **Waypoints** | 9 `WP-*` | 3×b, 6×c | **T-079** (split out) | **Gate this on AI units existing.** Every slot spawns AI-disabled — waypoints have no subject. This is the single biggest scoping correction available to T-079 |
| G5 | **Custom compositions** | 3 `COMP-*` | a | **T-078** (`deferred`) | Table + API + SPA. No mission-contract blocker — `comp:` already exists in `$defs/alias` and `asset_catalog.rs` already classifies Bohemia's |
| G6 | **Attributes-modal field parity** | `OBJ-TYPE`, `OBJ-ROLE-DESC`, `OBJ-UNIT-NAME` (3) | 2×a, 1×b | **T-082** (`deferred`) | T-082 "Full attribute fields" is the natural home, but it is a one-line ticket against a 31-id family. Scope it to the (a) work and let the rest route to G7–G9, or it will silently absorb 20 workbench ids |

**New tickets — nothing in the registry covers these.**

| # | Title | ids | class | Notes |
|---|---|---|---|---|
| N1 | **Mission presentation: briefing text + thumbnail in the SPA** | `SCN-OVERVIEW-TEXT`, `SCN-PICTURE` (2) | a | **Highest value-per-unit-effort in the sweep.** The API already accepts both; there is no caller. Blocks FNF-style derived briefings |
| N2 | **Editor comments (canvas annotations)** | 3 `CMT-*` | a | **No ticket anywhere** — was T-651 in the draft set only. Editor-only, never compiled, editor payload is open. Small and self-contained |
| N3 | **Editor layer flags: visibility + transform lock** | `LYR-ENABLE-VIS`, `LYR-ENABLE-XFORM` (2) | a | Editor-only. Note there is *no* layer visibility toggle today; the 12 world-layer checkboxes are a different object |
| N4 | **T-216 follow-on: carry tag/callsign/rank/stance/leaderSlotId to the wire** | `OBJ-CALLSIGN`, `OBJ-RANK`, `OBJ-STANCE` + TBD-only `tag`, `leaderSlotId` (3 ids + 2) | 2×b, 1×c | The contract delta is **already written out** at `flatten.rs:2620-2632`. `stance` additionally needs an Enfusion spawn-pose call (0 word-boundary hits in `apps/mod`). T-216 is `shipped` — it shipped the *ledger and its tripwire*, not the fix. Related: **T-242** (`idea`) |
| N5 | **Placement scatter: radius + area shape** | `OBJ-PLACEMENT-RADIUS`, `OBJ-SHAPE`, `GRP-PLACEMENT-RADIUS` (3) | b | A 2D top-down editor is the *best* surface for this — draw the scatter circle. Cheapest coherent (b) slice: one key on `$defs/slot` and one on `$defs/group` |
| N6 | **Vehicle states: lock, fuel, ammo** | `OBJ-LOCK`, `OBJ-FUEL`, `OBJ-AMMO` (3) | c | Natural successor to shipped T-215. Adjacent to **T-076** (crew UI) / **T-077** (empty vehicle), both `idea` — but those are *placement* tickets, not attribute tickets; keep separate |
| N7 | **Entity states: health, damage-allowed, hide, size, stamina** | `OBJ-HEALTH`, `-ALLOW-DAMAGE`, `-SHOW-MODEL`, `-SIZE`, `-STAMINA` (5) | c | **Gated on `entities[]` acquiring a consumer** — `mission.schema.json:72` says nothing on any shipped build reads it. `OBJ-STAMINA` carries an `UNKNOWN:` needing a Workbench API check |
| N8 | **Environment readers: fog, wind, view distance** | `SCN-FOG`, `-WIND`, `-VIEW-DIST` (3) | c | **Read the `author_env` rationale before opening this.** These are refused *by test* because nothing reads them; the mod reader comes first, the control second. `windDirDeg` is in the schema and not even serialised by `ModEnvironment` |
| N9 | **Cleanup: dead `view_distance` / `thermals` DTO fields** | — | a | No Eden id. `dto.rs:991-992` have had no writer since T-193 but are still parsed at `editor_ops.rs:205-212` |

**T-074 (`cancelled`) — flagged explicitly, as instructed.** **No id in this sweep revives it.**
T-074 is faction submode / catalog filter — a *palette* concern, not an attribute. Its cancellation
note says it was absorbed by T-180.5. Nothing here needs it reopened. `EDEN_SIDE_CHIPS`
(`eden_chrome.rs:2871`) already sets placement side in `DockRight`.

**Duplication check.** T-069 and T-213 are the same work (G1). T-079 is three programs in one ticket
(G3, G4, systems). T-082 will silently absorb 20 workbench ids unless scoped (G6). Those are the
three duplication/scope hazards in the existing set; every other id above routes to exactly one
ticket, new or existing.

---

## 6. Program split

### Factory-safe — `executor: claude-code` (22 ids, of which 15 are new work)

Already shipped (7, no action): `OBJ-POSITION`, `OBJ-ROTATION`, `GRP-CALLSIGN`, `LYR-NAME`,
`SCN-TITLE`, `SCN-TIME`, `SCN-WEATHER`.

Dispatchable now (15):

| Slice | ids | Ticket |
|---|---|---|
| Marker authoring core | `MRK-TYPE`, `MRK-TEXT`, `MRK-POSITION` | T-069/T-213 (G1) — **spec rewrite first** |
| Mission presentation | `SCN-OVERVIEW-TEXT`, `SCN-PICTURE` | N1 |
| Editor comments | `CMT-TITLE`, `CMT-TOOLTIP`, `CMT-POSITION` | N2 |
| Layer flags | `LYR-ENABLE-VIS`, `LYR-ENABLE-XFORM` | N3 |
| Composition metadata | `COMP-TITLE`, `COMP-AUTHOR`, `COMP-CATEGORY` | T-078 (G5) |
| Attributes modal fields | `OBJ-TYPE`, `OBJ-ROLE-DESC` | T-082 (G6) |

**Ordering note for wave packing:** the marker slice, the comment slice and the layer-flag slice each
touch `eden_chrome.rs` (5119 lines) — the hottest file in the editor. They are three separate
slices but **one collision domain**; do not pack them in the same wave. N1 (presentation) touches
`missions.rs` / `create_mission_dialog.rs` and is collision-free against all three.

### `executor: workbench` — a second program (49 ids)

| Sub-program | ids | class | Gate |
|---|---|---|---|
| T-216 follow-on (schema widening) | 5 (3 Eden + `tag` + `leaderSlotId`) | b/c | Contract delta pre-written at `flatten.rs:2620-2632`. Widening `/compiled` without care 500s every mission |
| Marker style / Area markers | 6 | b | After factory G1 |
| Placement scatter | 3 | b | Independent; cheapest (b) slice |
| Triggers | 12 | b/c | New Enfusion runtime. Geometry can ride T-582 |
| Waypoints | 9 | b/c | **Blocked on AI units existing** |
| Group AI state (`GRP-COMBAT-MODE`, `-BEHAVIOUR`, `-FORMATION`, `-SPEED-MODE`) | 4 | c | **Blocked on AI units existing** — same gate as waypoints, and they should ship together or not at all |
| Vehicle states | 3 | c | After T-215 |
| Entity states | 5 | c | Blocked on `entities[]` getting a consumer |
| Environment readers | 3 | c | Mod reader first, control second — `author_env` enforces this ordering by test |
| `OBJ-UNIT-NAME` | 1 | b | Rides the T-216 follow-on schema slice |
| `TRG-TEXT` etc. counted above | — | — | — |

### Closed — 22 `na` ids, no ticket

Scripting handles with no script layer (6: `OBJ-VARNAME`, `OBJ-INIT`, `GRP-VARNAME`, `GRP-INIT`,
`MRK-VARNAME`, `TRG-VARNAME`). A3-engine concepts with no Enfusion analogue (7:
`OBJ-DYN-SIM`, `-WAKE-DYN-SIM`, `-ENABLE-SIM`, `-SIMPLE-OBJ`, `-LOCAL-ONLY`, `-DOORS`, `GRP-DYN-SIM`).
No-AI-subject (4: `OBJ-SKILL`, `OBJ-PLAYABLE-MP`, `OBJ-PLAYER-SP`, `GRP-DELETE-EMPTY`). Refused by
design with a test (1: `OBJ-REVIVE`). Product-model mismatch (4: `OBJ-FACE`, `SCN-AUTHOR`,
`SCN-DLC`, `SCN-REQUIRE-DLC`).

**These should be written into `gap_analysis.md` as `na` with their reasons when that table is
next revised** — an unexplained `missing` invites a ticket, and 22 of them would invite 22.

---

## 7. What this sweep did not settle

- **`OBJ-STAMINA`** — marked `UNKNOWN` on whether Reforger exposes a per-character stamina toggle.
  A Workbench API question, not answerable by code search in this repo.
- **`$defs/zoneRules`' 16 keys** were not triaged against Eden — they have no `ATTR-FIELD-*` id, so
  they are outside this sweep's scope, but they are the closest thing TBD has to trigger semantics
  and belong in whatever scopes G3.
- **The `SYS` family** declares no ids in `attributes.md:223-225`. T-079's "systems" third is
  therefore un-enumerated — it needs its own catalogue pass before it can be sized.
- **`interactions.md`'s 83 ids** and the `owns`/wave packing derivation remain **NOT WRITTEN**
  (see `README.md:29-30`). The collision note in §6 is a partial input to the second, not a
  substitute for it.
