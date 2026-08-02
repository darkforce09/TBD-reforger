# Wave 120 — adversarial verification (schema-contract wave)

**VERDICT: 0 BLOCKER / 4 MAJOR / 5 MINOR / 4 NOTE — merge stands; fix M-findings before the reader waves dispatch.**

Range `5432cca1..HEAD` = T-701 (slice `a16e9188`, merge `27b8cbc7`) + T-706 (slice `843f3438`, merge `0e05e727`) + fixup `9228a458`. Verifier: Fable 5, read-only except this file. All perturbations below were transient (sed/probe-file → run → `git checkout`/`rm`); working tree after: only the pre-existing uncommitted `.ai/artifacts/editor_factory_run.md`.

## Executed evidence (question F/E baseline)

| Check | Result |
|---|---|
| `cargo test -p map-engine-core --features mission,doc` | **464 passed** (claim: 464 store) + 1 pre-existing ignored; all-features run: 597 |
| `cargo test -p website-frontend` (native) | **691 passed** (claim: 691); incl. `eden_env::keys_nothing_reads_are_not_authored` green |
| `cargo test -p xtask` (unread module) | 5/5 gate unit tests green (6th match on filter is unrelated `…unreadable`) |
| `cargo run -p xtask -- schema validate` | rc=0; all 7 goldens PASS incl. new `schema-1_3-wire-fields.json`; 6 negative goldens still correctly rejected; "T-706 unread 1.3 wire fields: PASS 45 field(s)" — gate IS wired into `validate_all` |
| `cargo test -p website-api contract::` | 16/16 — `validate.rs` `include_str!`s the LIVE `mission.schema.json`, so the API accepts 1.3 on rebuild; **no codegen step involved** (`contract/generated/` has no mission.rs; typify set = editor-payload/registry×2/faction-library/loadout). 1.2 propagated the same way + a mod-struct edit — which is exactly the half missing now (see M-1) |
| store.rs `#[test]` count | 112 → 120 (+8 = the eight claimed T-701 tests, all read) |
| `additionalProperties: false` | 25 → 35 = +10, one per new object def; sole open object = `editorTrigger.effects[].params`, documented open-by-design — the "preserved everywhere" claim holds with that one stated exception |
| 1.1/1.2 goldens | untouched by the range diff (only the new golden added) ✓ additive claim |
| `editorHidden` | declared in NEITHER mission.schema.json nor mission-editor-payload.schema.json ✓ structurally editor-block-only |

## Fired proofs (all restored)

1. **Gate fires on covered name (real tree):** probe `.c` with a `waypoints` member → `all_1_3_fields` RED with the exact remediation text ("'waypoints' now has 1 … baseline 0 … T-677"). 
2. **Gate hole (M-2):** probe `.c` declaring `vehicleClasses`, `alpha`, `shape`, `color`, `variants` members → gate stays **GREEN**. Live readers of five 1.3 fields, invisible.
3. **Ledger declared-half:** schema `leaderSlotId` key renamed away → ledger test RED: "no object in [/$defs/group, /$defs/slot] declares … Revert this row to `Blocked`". 
4. **Ledger wire-half at root scope:** vehicles row `wire_key` retargeted to `zones` → RED: "\"\" in the compiled document NOW carries a \"zones\" key … move to `Reaches`" — proves `scope: ""` genuinely scans the compiled document root (`wire.pointer("")` + `any_object_has_key` checks the root map itself; plus the direct `wire.get("vehicles").is_none()` assert below the loop). 
5. **T-701 enforcement is the one materialize site:** removing `|| read_bool(… "editorHidden")` from `materialize()` → 4 of 5 `editor_hidden*` tests RED (survivor = the wire test, correctly independent). The wire test additionally self-fires in-tree (perturb: smuggle `editorHidden` onto every compiled slot → still stripped by `SlotIn`/`ModSlot`).

---

## MAJOR

### M-1 — Schema asserts a mod allowlist that does not exist; a 1.3 document is REJECTED by the shipped mod build
`mission.schema.json` `schemaVersion` description (new in T-706): "*TBD_MissionLoader allowlists 1.1/1.2/1.3, so a 1.1/1.2 document stays valid unchanged*". **False.** `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c` has `SCHEMA_1_0/1_1/1_2` only; `CheckSchemaVersion` AddErrors any other version ("this build understands 1.0, 1.1 and 1.2") — a blocking error, so a `"1.3"` document never leaves LOADING. The wave touched zero `.c` files (correctly — out of owns), so the claim could not have been true. No live break today (`flatten.rs:2121` emits "1.1"/"1.2"), but the first reader slice that bumps the emitted version takes down every mission server-side unless it also edits the validator — and no schema description names that step (each names only its reader ticket). Also stale-adjacent: `TBD_MissionLoader.c:272/277` still say "1.0/1.1/1.2" (mod-side rot, reader tickets' to fix). **Disposition:** reword the description to future tense naming the validator bump, and pin the bump into T-674/T-675's owns (they're first to emit).

### M-2 — The mandated unread gate does not cover "EACH new 1.3 wire field": 5 names missing, live-fire proven
Re-derivation: all 45 rows are genuinely new 1.3 names (no padding), pins match the live tree (suite green), and wrapper-transitivity legitimately covers interior keys (a JsonLoadContext reader must bind the parent member: `activation`/`effects`/`seats`/`waypoints`/`gadgets` rows cover their interiors — including `map`/`radio`). But five new names have NO row and NO covering new wrapper:
- **`vehicleClasses`** — T-689's ONLY field, nested in `TBD_MissionZoneRulesStruct` which the mod ALREADY binds (`TBD_MissionLoader.c:96`), so a T-689 reader adds one member and no covered identifier moves. Baseline would be a clean 0. The T-685 fields in the SAME struct all got rows — the omission is inconsistent, not principled.
- **marker `shape` / `color` / `alpha`** — T-673: `rotationDeg`/`brush`/`area` got rows; these three didn't. `TBD_MissionMarkerStruct` already exists (`@contract #/$defs/marker`), so a reader landing exactly the minimal Eden style trio is invisible. (`alpha` = clean 0; `shape` ≈ 40, `color` ≈ 3 pre-existing identifiers → need measured pins.)
- **`variants`** — the only new TOP-LEVEL array without a row (objectives/vehicles/editorTriggers/missionParams all have one). ≈3 pre-existing identifiers.
Fired: probe readers for all five → gate green (proof 2); `waypoints` control → red (proof 1). Also: the header comment discusses `map`/`radio` collisions as if pinned rows exist (none do — they are only wrapper-covered), and the handoff claim says **5** pinned collision names while the table pins **six** (objectives 13, seats 8, area 13, gadgets 6, callsign 16, tag 42). **Disposition:** add the five rows (two clean-0, three measured pins); optionally reword the map/radio comment.

### M-4 — New identity joins hedge between `uid` and `id`; leaderSlotId denormalized onto every slot
`slot.leaderSlotId` "*keyed on a slot `uid`/`id`*" and `vehicle.seats[].slotId` "*keys on a `slots[].uid`/`id`*" refuse to pick between two DIFFERENT wire fields — and the schema's own `id` description says id is DERIVED and "*shifts under role renames/reorders/deletes; key durable references on **uid** instead*". Two .c readers WILL resolve differently; the id-reader silently breaks crew/leader joins after any ORBAT edit. This is the exact readers-diverge defect class this wave existed to prevent, on the two most load-bearing new references. Compounding: T-706 put `leaderSlotId` on `$defs/slot` as "which slot leads this slot's squad" — N copies of a per-SQUAD fact (the editor authors it at `/editor/squads/*/leaderSlotId`; the T-216 delta table at flatten.rs proposed "`$defs/group` (or a `$defs/slot` boolean)"), with cross-row agreement unenforceable by any schema, while `$defs/group` (the wire orbat group, `orbatFaction.groups[]`) was available. **Disposition:** pin "uid (fall back id for pre-B1 docs)" in both descriptions; move (or dual-declare) leaderSlotId to `$defs/group` before T-674 dispatch — the ledger row's `owners: GROUP_OR_SLOT` already anticipates either home.

### M-5 — Icon-vs-Area marker model is internally inconsistent: `shape` vocabulary ∉ `area` geometry
`marker.shape` ∈ {icon, **rectangle**, **ellipse**, **polyline**} but `marker.area` is `$defs/shape` = oneOf {**circle**, **polygon**}. A rectangle/ellipse extent is inexpressible in the geometry def (and polyline is an OPEN path — polygon is closed). Precedence when they disagree is undefined; the new golden itself ships `shape:"ellipse"` with `area.circle`. Each reader invents its own resolution (render area as-is? derive ellipse from circle? bounding-box a polygon into a rect?). **Disposition:** align the vocabularies before T-673 — either narrow `shape` to {icon, circle, polygon(, polyline)} or extend `$defs/shape` with rect/ellipse(/polyline) arms; state precedence in one sentence.

## MINOR

### m-3 — Gate blind spot for string-keyed reads exists and is not stated as a limit
`strip_enfusion_comments_and_strings` deletes string-literal bodies, so a reader addressing a key via `ctx.ReadValue("combatMode", …)` (or a runtime-built key string) is invisible by construction. Today the mod exclusively uses `ReadValue("", struct)` whole-document member binding (checked: every ReadValue call site), so the model matches practice, and the member-binding assumption IS documented — but the evasion path is not named. One sentence in the header comment turns an unknown gap into a stated limit.

### m-6 — Zone volume bounds: reference frame never defined
`zoneRules.minHeight`/`maxHeight`: "metres **relative to the objective**" — relative to *what* is unsaid: terrain at each tested entity (AGL)? ASL at the zone centre? A polygon zone has no defined anchor at all. On a slope these diverge by tens of metres — precisely the basements-in/aircraft-out behaviour the values exist for. `slot.y` shows the house style ("metres ASL"); WOG's own semantics are flagged INFERRED in the registry, so TBD's contract must define its own frame explicitly. Every .c reader will otherwise pick one.

### m-7 — `$defs/objective` ships an invented spine and omits the mandated one
`callsign`/`rank`(private…colonel)/`stance`(stand/crouch/prone) on a task record have no source — T-212's registry text (WOG's 15 WMT parameters) contains no such fields; they read as slot-identity copy-through. Meanwhile T-212's two decided imports are absent: FNF v4's per-side FRAMING ("ONE ENTITY, TWO FRAMINGS" — separate attacker/defender text; the def has one `label`) and optional `_Lock`/`_AutoLose`. `type:"defend"` alongside `side` also re-admits the two-rows-per-objective shape v4 is cited to forbid. T-212 is a design ticket; this wire shape prejudges it in the wrong direction.

### m-8 — Engine-enum attributions unverifiable and Arma-3-flavoured
"matching Enfusion's `EAICombatType` ladder" (blue/green/white/yellow/red), "`EAIWaypointCompletionType`/behaviour ladder" (a completion-type enum is not a behaviour ladder), "`ECharacterRank` ladder", "`SCR_AIWaypoint` families" — none of these identifiers appears anywhere in the mod tree, and the vocabularies are A3's setCombatMode/setBehaviour/formation sets. Each reader ticket inherits an unvetted mapping problem. Also `get_in`/`get_out` waypoints carry no vehicle target (proximity-only inference). Soften "matching" to "modelled on" now; verify against the real Reforger enums when each reader lands.

### m-9 — Ledger misses `editorTriggers` under the fixup's own framing
The fixup's rule — "T-706 opened contracts whose emits land later" — added the vehicles[] row because the payload authors a roster. The editor ALSO authors triggers today (T-079 shipped; store.rs root `triggers` map / `triggersById`), T-706 opened root `editorTriggers`, flatten emits none (0 occurrences) — and there is no DeclaredPendingEmit row (emit ticket T-676). objectives/variants/missionParams have no mutators, so their rowlessness is correct. Either add the row or state the charter as "the T-216 six only".

## NOTE

- **n-10** Fixup comment rot in flatten.rs (~:3006): "The `Blocked` row above scopes its key search to `/slots`" — that row is now `DeclaredPendingEmit`; the "43 callsign mentions" figure was already stale pre-wave (67 matching lines at both 5432cca1 and HEAD).
- **n-11** `vehicleClasses: []` (valid; no minItems) is semantically undefined — absent=everything is stated, empty-list is not ("applies to nothing" vs fallback). One clause or `minItems: 1`.
- **n-12** Collision annotations: identities verified essentially correct (callsign=LobbyData/BriefingData/MissionLoader group-callsign; objectives=TBD_ObjectivesComponent; seats=lobby/briefing UI; gadgets=TBD_RadioTuner), but `tag`=42 is dominated by UI list-row `int tag` numbering (LobbyScreen/ListBox/AdminScreen), not "loadout/spectator" as annotated, and `area`=13 includes `TBD_PlayAreaComponent` hits (play-area vocabulary — adjacent to the T-689 lane, expect a legit re-pin) beyond "loadout-area". Conclusions hold; annotations imprecise.
- **n-13** Bookkeeping in flight: registry.json still holds T-701/T-706 `queued`; `.ai/artifacts/editor_factory_run.md` carries an UNCOMMITTED wave-A dispatch note (T-674/T-675 flatten-owns serialization). Commit/sync before dispatch — the ticket bookkeeping is load-bearing.

## Verified-clean register (claims re-proven, not taken)

- **T-701:** only-when-true omit idiom (false REMOVES the key; never-hidden row byte-identical); single enforcement site fired (proof 5); effective = layer OR entity — four corners tested, and the "entity-hidden slot on a hidden layer, layer then re-shown → still hidden" case is entailed by filter purity (materialize is stateless over current flags; the (visible-layer, hidden-entity) corner is asserted directly); hydrate round-trip; wire structural absence with in-test perturb/fire + `editorHidden` absent from both schemas; hide/batch = one txn/one undo step; `clear_all_editor_hidden` touches ONLY slot `editorHidden` keys — layer flags untouched (code + `show_all_clears_every_flag_in_one_txn`); `slot_hidden_rows` accessor shipped; T-715 inheritance stated in code; 5 × `#[allow(dead_code)]` each annotated with its residue owner (T-733 family / H-key / menu), matching the stated precedent — no other allow() growth in the range.
- **Ledger fixup:** both halves fired red with correct remediation text (proofs 3/4); root-scope scan covers the compiled root; anti-vacuity witnesses present (fixture authors every flipped value; vehicles row keys on `/vehicles/0/id`); `resourceName` row correctly still `Blocked` (nothing declares it — 0 occurrences in the schema); no inverse hole — flatten emits NONE of the newly declared names (`vehicles` asserted directly; `objectives`/`editorTriggers` at 0 occurrences in flatten.rs; ModEnvironment serialises only dateTime/weatherPreset).
- **T-706 mechanics:** 45 rows all-new (no padded non-new fields); pins equal live-tree counts; gate wired into `validate_all` (observed in output) so every slice/wave gate runs it; 5 unit tests incl. the fire-once and stripper non-vacuity tests; golden exercises all five new top-level arrays + slot identity/gadgets/placement + marker style/area + zone volume rules + entity states; schemaVersion enum additive with the 1.3 allOf slots arm mirroring 1.2's; eden_env constraint green at HEAD; payload-schema (`compositionsById`/`triggersById`) deferral is stated in the commit and out of owns — honest.
- **Suites at HEAD:** 464 (mission,doc) / 597 (all-features) map-engine-core; 691 frontend; xtask green; `xtask schema validate` rc=0; api contract 16/16.

---

## Re-verification of 6382c69c

**VERDICT: 8 of 9 dispositions verified clean; m-3 silently dropped; 2 further description-level residues. Nothing reopens the wire shape — all three fixes are prose/comment edits.** Re-verifier: Fable 5, adversarial sampling per the disposition table (the commit message), not a full re-run. All perturbations transient; working tree after = pre-existing `editor_factory_run.md` edit + this file only.

### Executed evidence

| Check | Result |
|---|---|
| Diff scope | exactly `mission.schema.json` + `schema_gates.rs` + `flatten.rs` + the 1.3 golden (4 files, 286+/76−); 1.1/1.2 goldens untouched |
| `xtask schema validate` | rc=0; **"PASS 53 field(s) still unread"** (45 + the 5 mandated + `framing`/`autoLose`/`vehicleUid` for the commit's own new keys); reshaped 1.3 golden PASS |
| map-engine-core | 464 passed + 1 ignored (mission,doc); 597 (all-features) |
| xtask / frontend / api | 45 / 691 / contract 16-16 |
| M-2 probe re-fired | probe `.c` with an `alpha` member → gate RED, exact remediation ("'alpha' now has 1 … baseline 0 … T-673"); removed |
| M-4 declared-half re-fired | `$defs/group` `leaderSlotId` key renamed in scratch → ledger RED at flatten.rs:3010 with owners **`["/$defs/group"]` only** + "Revert this row to `Blocked`"; restored |
| M-4 statics | ONE `"leaderSlotId"` key in the whole schema (line 312, `$defs/group`); slot def clean; golden moved it onto the group, value = `slots[0].uid`; `seats[].slotId` values are uids; both descriptions say `slots[].uid` plainly, no `/id` hedge (no pre-B1 fallback needed — no pre-1.3 doc can carry these keys); ledger `owners` const tightened to GROUP |
| M-2 pin re-derived | independent strip+wholeword count of `shape`: **9+9+8+6 = 32** (BriefingData/ZoneRegistry/MissionValidator/Loader), per-file exactly as annotated; occurrences are `ref TBD_MissionShapeStruct shape` + `zone.shape.circle` accesses — zone-geometry as claimed. Non-zero rows are the SEVEN named and only those seven |
| M-5 walk | `$defs/markerArea` oneOf circle/polygon/rectangle/ellipse; rect+ellipse → `$defs/rectExtent` (x, z, halfWidth>0, halfHeight>0, rotationDeg 0–360); precedence ("area present ⇒ area marker, shape ignored") stated on BOTH `marker.shape` and `marker.area`; golden now `shape:"ellipse"` + `area.ellipse` — coherent |
| m-7 statics | `callsign`/`rank`/`stance` gone from `$defs/objective` (golden reshaped); `framing`/`lock`/`autoLose` present; two-rows-per-objective explicitly forbidden with v4's four defects, matching the T-212 registry summary near-verbatim; `_Lock` citation ("1 in 136 of 166") matches wog.md:399; the INFERRED caveat is carried faithfully (names+values as hard evidence, readings marked not-fact) — sound, not overreach |
| m-6 / m-8 / m-9 / n-10 / n-11 | AGL-per-entity wording with axis + evaluation point + polygon resolution ✓; all four engine attributions stripped (survivors are negations: "no `EAICombatType`-style enum is asserted") + `vehicleUid` added with gate row and golden usage ✓; `editorTriggers` DeclaredPendingEmit row with fixture witness (`editor.triggersById.trg1`) ✓; stale 43→67 prose swept in both flatten sites ✓; `[]` = applies-to-nothing clause present, golden authors the aircraft exemption ✓ |
| M-1 wording half | false allowlist claim gone; validator reality stated with the real path; bump requirement future-tense ✓ (but see R-2) |

### NEW FINDINGS (all minor, all prose-level)

**R-1 — m-3 silently dropped; the commit claims 9/9.** The title says "the 9 pre-reader ambiguities (M-1..m-9)" but the body itemizes eight — m-3 is absent — and the mandated one-sentence stated-limit does not exist at HEAD: the rewritten gate header (schema_gates.rs:1900–1946) explains the stripper as false-positive prevention and the member-binding model, but never names the evasion path (a reader addressing a key via `ctx.ReadValue("combatMode", …)` or a runtime-built string is invisible BY CONSTRUCTION because string bodies are stripped). Grep for `ReadValue`/string-keyed across the four changed files: nothing. One sentence closes it; flagged mainly because it is a silent deferral against an explicit 9-count (no-silent-deferrals lane).

**R-2 — M-1 residue: "the bump is pinned into their owns" is not true anywhere.** The new schemaVersion description asserts the SCHEMA_1_3 validator bump "is pinned into their [T-674/T-675's] owns". Registry at HEAD: both tickets carry `"notes": "… no owns, no wave row"`, and neither summary mentions `TBD_MissionValidator`/SCHEMA_1_3 at all; there is no wave-plan row either. The normative sentence beside it ("MUST LAND WITH THE FIRST READER SLICE THAT EMITS 1.3") is the correct future-tense truth M-1 demanded; the indicative "is pinned" asserts bookkeeping that has not happened — a milder recurrence of the M-1 defect class (schema describing another artifact's state inaccurately). Fix: reword to "must be pinned", or land the registry note (Cursor lane, rides the n-13 sync) before the reader waves dispatch — a T-674 slice agent reading only its ticket today would not learn the bump exists.

**R-3 — m-7 residue: `autoLose`'s observed-values citation misquotes the corpus.** Schema: "observed -1, i.e. disabled, in all 156 instances that carried it … absent/disabled is the universal observed case". wog.md:400: `WMT_Task_Point_AutoLose` = `-1` ×156, **`1` ×10**, under a table headed "Every instance carries all 15 parameters" — so all 166 carried it and TEN observed a LIVE value (side 1). Two errors: "156 … carried it" (166 did) and "universal" (dominant, not universal). Root cause visible: the T-212 registry summary truncated the distribution to "`_AutoLose` (-1x156)" and the fixer paraphrased the truncation into a stronger claim — ironically inside the wave that fixed n-12's imprecise annotations and under the registry's own DO-NOT-LAUNDER instruction, whose "observed VALUES are hard evidence" half is what got garbled. Design unaffected (ten live uses if anything strengthen carrying the field; the factionKey re-typing with absent=disabled still reads -1 correctly); description-only fix. `lock`'s citation is accurate.
