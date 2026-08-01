# Adversarial coverage verification — editor UI planning corpus

**Written 2026-08-01.** Lens: **coverage and completeness** — what was never asked, never looked
at, or quietly dropped. Claims-accuracy is the sibling verifier's job; nothing here is fixed, only
documented. Everything below was checked against the corpus in `.ai/artifacts/`, the authority
docs in `docs/specs/Mission_Creator_Architecture/`, `.ai/tickets/registry.json`, the mission
schema, and the live editor source.

**Verdict in one line:** the corpus is excellent on the things it looked at — contours, panels,
chrome, frameworks — and it never states the boundary of what it did not look at, which leaves
ask #1 (Eden parity) roughly half-covered while reading as fully covered.

---

## Executive summary — the five most serious gaps

**GAP-1 (blocks ask #1). Ten of the 31 `missing` parity rows have no ticket, no draft, and no
disposition anywhere in the corpus.** `editor_ui_ticket_drafts.md` §F opens "32 rows read
`missing`… Grouped richest-first", which reads as full coverage. The seven F tickets
(T-645…T-651) actually close **16** rows — and T-645, the "richest", closes zero (it is a 3den
Enhanced borrow, by its own admission "Not in the gap table"). The entire
**trigger / waypoint / systems-module / crew / vehicle-attribute** surface is silently absent:
`RIGHT-MODE-003/004/005`, `PLACE-005`, `CONN-TRG-OWNER-001`, `CONN-WP-ACT-001`,
`CONN-WP-ATTACH-001`, `CREW-SEAT-001`, `ATTR-FIELD-OBJ-SKILL`, `ATTR-FIELD-OBJ-FUEL`. The words
"waypoint" and "trigger" do not appear in the program plan or the drafts at all. These rows share
a root cause no document names: each presupposes an entity class (triggers, AI waypoints,
crewable vehicles, logic modules) that `mission.schema.json` does not have — top-level props are
`slots/zones/entities/…`; no trigger, no waypoint, no AI concept. The synthesis even contains the
evidence to close some of them deliberately (§A.9: 2/4 frameworks abandoned triggers; zones won,
and TBD's typed zones are "better than any of the four originals") — but nobody wrote the
sentence "therefore RIGHT-MODE-003 / CONN-TRG-OWNER-001 / PLACE-005 are won't-build; zones
supersede them". A gap closed by accident is still open.

**GAP-2 (blocks ask #1 as stated). The parity table is a sample, not a census, and no corpus
document says so.** The handoff and plan call it "87 rows"; the table actually holds **~61 data
rows** (and **31** `missing`, not 32 — `grep -c '| missing |'` = 31). Against its own inputs:
`interactions.md` defines 77+ interaction IDs of which **41 have no parity row at all**
(cut/paste-at-original, quick waypoints, formation, crew unboard, snap/level actions, widget
rotate/area-scale, the whole status-bar family, RIGHT-SEARCH-003/004/005…), and `attributes.md`
enumerates **95** `ATTR-FIELD-*` ids of which the gap table carries **3**. The table's own
summary line admits it ("Attributes … Missing: 10+") without enumerating. "Add all the things
that should be there, against the parity table" therefore under-scopes the operator's ask by
roughly an order of magnitude on attributes — and nobody would find out by reading the corpus.

**GAP-3 (blocks ask #3 as designed). Line of Sight is designed against bare terrain, and the
obstruction question was never asked.** T-643/T-644 sample "the loaded DEM (6400² uint16)". The
DEM is bare-earth. The same engine already holds **4,131 building OBBs and 501,861 trees plus a
canopy-density texture** (T-090.3.2/T-151 lanes). A milsim LoS/viewshed that reports "clear"
through a forest block or a village is not a rough first cut — it is the tool confidently wrong,
i.e. exactly the "UI claiming knowledge it does not have" defect the handoff warns about. The
corpus flags eye-height and the viewshed colour language as open decisions; canopy/structure
occlusion, max range, and viewshed compute cost (full-raster over 6400² in wasm/WebGPU) appear
nowhere. The research proved no framework has prior art — it did not then do the design work the
absence of prior art makes necessary.

**GAP-4 (blocks filing). Two unreconciled ticket sets, an un-updated gap table, and a stale
registry.** The synthesis Part D was written *after* the drafts and proposes: group I validation
(T-655…T-660 — six tickets covering the corpus's #1-ranked build), T-654 conditional inclusion,
re-ranking T-651, splitting T-641, and two `gap_analysis.md` corrections (`LAYER-CREATE-001`
tbd_only→partial; a warning note on `CONN-SYNC-001`). None of that is applied to the drafts file,
nothing is filed, and `gap_analysis.md` is untouched. Verified: the registry's highest id is
**T-626** while T-627/T-628 exist in shipped code (`mission_editor.rs:185,213`) — so "next free
id T-631" rests on a registry that is already four ids stale (synthesis D.6 says this; nobody
acted). Anyone implementing today must first decide which of two documents is the program.

**GAP-5 (program scoping). The dimensions with documented operator demand were never in the
research schema.** `backlog-braindump-2026-07-25.md` items **14–16** (git-diff-style mission
versioning, realtime collaborative editing, commenting + reviewer workflow) and **25** ("never
tested: place unit → export JSON → load in mod — the single highest-information test available")
are operator-named wants. The 14-section framework schema has no section for collaboration,
review, versioning-as-UX, or the author's test loop, so all five analyses and the synthesis are
structurally silent on them — reasonable for Arma 3 file-based frameworks, but TBD is a CRDT
(`yrs`) multi-user web app where these are native questions. T-651's map annotations are not the
review-comment workflow of item 16. Nothing in the 23 drafts + 7 proposed additions touches any
of the four items.

---

## Parity table walk — every `missing` row

Count discrepancy first: the corpus says **32** everywhere (handoff, plan, drafts §F); the table
greps **31**. Either a row was lost in an edit or the number was never true; no document
reconciles it. `RIGHT-SEARCH-001` also carries status `working`, which is not in the table's own
legend (match/partial/missing/deferred/na/tbd_only).

Legend: **DRAFT** = closed by a T-631…T-653 draft · **PRIOR** = only a pre-existing registry
ticket outside this program · **FLAG** = discussed in synthesis, no ticket · **NONE** = no
ticket, no draft, no mention, no disposition anywhere · **STALE** = already shipped, table not
updated.

| # | eden_id | Gap | Coverage | Where / what is missing |
|---|---------|-----|----------|--------------------------|
| 1 | RIGHT-MODE-002 | Compositions save mode | **DRAFT** | T-650 (absorbs T-078 `deferred`); synthesis B6 orders "check T-180 overlap first" — unresolved |
| 2 | RIGHT-MODE-003 | Triggers palette (F3) | **NONE** | Synthesis §A.9 has the evidence to close it as won't-build (zones won); nobody disposed of the row |
| 3 | RIGHT-MODE-004 | Waypoints palette (F4) | **NONE** | "Waypoint" appears zero times in plan/drafts/synthesis; no AI/waypoint concept in schema; whether TBD missions have AI at all is undecided anywhere |
| 4 | RIGHT-MODE-005 | Systems/modules (F5) | **NONE** | Synthesis B5 (objectives as placed entities) is the moral equivalent of Eden modules; the connection to this row is never made |
| 5 | RIGHT-MODE-006 | Markers palette | **PRIOR** | T-069 `deferred`; drafts never mention markers; synthesis B4 (briefing) *depends* on T-069 and says so — no ticket revives it |
| 6 | RIGHT-SUBMODE-001 | Side submode chips | **DRAFT** | T-646 — but the row's mapped ticket T-074 is `cancelled`; synthesis D.4#5 flags the silent revival; unresolved |
| 7 | RIGHT-SEARCH-002 | `class:` prefix search | **DRAFT** | T-646 — pre-existing T-084 `deferred` covers the same row and is absent from the drafts' absorption list (dup risk) |
| 8 | RIGHT-CREW-001 | Place vehicle ± crew | **DRAFT** | T-646 — presupposes placeable vehicles (T-070 `idea`); dependency unstated |
| 9 | PLACE-003 | Dbl-click empty → picker | **DRAFT** | T-647 |
| 10 | PLACE-004 | Ctrl multi-place | **DRAFT** | T-647 (absorbs T-072 `queued`) |
| 11 | PLACE-005 | Area draw (trigger/marker) | **NONE** | Zone circle/polygon draw shipped since (editor_ops.rs `ZoneDraft`) partially supersedes the trigger half; marker half needs T-069; no document says either |
| 12 | PLACE-COMMENT-001 | Editor annotations | **DRAFT** | T-651; synthesis D.2 re-ranks it up and adds template-seeding — unapplied |
| 13 | PLACE-CREW-001 | Alt = empty vehicle | **DRAFT** | T-647 (absorbs T-077 `idea`); same unstated T-070 dependency |
| 14 | XFORM-SHIFT-001 | Shift rotate | **DRAFT** | T-648 (absorbs T-073 `queued`) |
| 15 | WIDGET-CYCLE-001 | Space cycles widget | **DRAFT** | T-648 — Space-vs-flyTo collision named, undecided (acceptable in-ticket) |
| 16 | WIDGET-TRANS-001 | Axis widget | **DRAFT** | T-648 |
| 17 | TOOLBAR-GRID-MOVE-001 | Snap grids | **DRAFT** | T-648 |
| 18 | COMP-SAVE-001 | Save composition | **DRAFT** | T-650 |
| 19 | COMP-PLACE-001 | Place composition | **DRAFT** | T-650 |
| 20 | CONN-GROUP-001 | ORBAT squad authoring | **STALE** | T-071 `shipped`, T-180 COMPLETE per CLAUDE.md; row still reads `missing`; no corpus doc corrects it |
| 21 | CONN-SYNC-001 | Entity sync | **FLAG** | Synthesis B.5#1/D.4 attach the v4 warning (inspector + validation before edges) but no ticket and no build/defer decision; still "a bare missing row" by its own description |
| 22 | CONN-TRG-OWNER-001 | Trigger owner link | **NONE** | Trigger cluster — see #2 |
| 23 | CONN-WP-ACT-001 | Waypoint activation link | **NONE** | Waypoint cluster — see #3 |
| 24 | CONN-WP-ATTACH-001 | WP attach/detach | **NONE** | Waypoint cluster — see #3 |
| 25 | CREW-PANEL-001 | Hover crew list | **PRIOR** | T-076 `idea` only; drafts silent |
| 26 | CREW-BOARD-001 | Drag unit into vehicle | **PRIOR** | T-076 `idea` only; drafts silent |
| 27 | CREW-SEAT-001 | Change seat (RMB) | **NONE** | Not even mapped to T-076 in the table |
| 28 | SEL-ALL-001 | Ctrl+A select all | **DRAFT** | T-649 (copies Eden's "on screen" scope) |
| 29 | ATTR-FIELD-OBJ-SKILL | AI skill attribute | **NONE** | Presupposes AI units — a decision no document has made |
| 30 | ATTR-FIELD-OBJ-FUEL | Vehicle fuel attribute | **NONE** | Depends on T-070 `idea`; unmentioned |
| 31 | ATTR-MULTI-CHK-001 | Multi-edit checkboxes | **DRAFT** | T-649 |

**Tally: 16 DRAFT · 10 NONE · 3 PRIOR · 1 FLAG · 1 STALE.** The ten NONE rows cluster into four
entity classes the document model lacks; a one-page disposition note (build / defer / superseded
by zones / no-AI decision) would close most of the exposure cheaply. Beyond the 31: the ~41
un-rowed interaction IDs and ~92 un-rowed attribute fields (GAP-2) have no disposition either.

---

## The four asks — readiness

**Ask 1 — Eden parity: NOT buildable to completion from this corpus.** The 16 drafted rows are
buildable (T-646–T-651 are well-specified, sources traceable to screenshots/wiki/3den
catalogue). What stops "add all the things that should be there": the 10 NONE rows, the crew/
marker rows parked on `idea`/`deferred` tickets the drafts never absorb, the un-enumerated
attribute surface (95 fields, 3 rows), and the 41 un-rowed interaction IDs. Still unknown and
undecided: does TBD have AI (waypoints/skill)? vehicles-with-crew (T-070 is `idea`)? triggers
(or are zones the final answer)? Eden modules (or is B5 the answer)? None of these is expensive
to *decide*; all are currently silent drops.

**Ask 2 — UI cleanup: buildable.** Finding 5's nine concrete problems + the Eden convention list
+ B1–B6/C1 are a real design brief. Weakest: T-637 (dock density) states the problem and no
target layout — it is a design task wearing a ticket number, acceptable if treated as such.
Sequencing gap: T-636 moves tools out of the toolbelt while T-642/T-643 wire tools *into* it;
no stated ordering. No mockups exist, but the design skills are named in the handoff.

**Ask 3 — Line of Sight: MVP buildable, design NOT resolved.** T-643 (ray) is genuinely
implementable: input model, DEM source, and two named decisions (eye height, profile rendering).
But "actually work" is undermined by GAP-3 — occlusion by buildings/canopy is never mentioned in
any document, and bare-earth LoS on a forested island gives confident wrong answers. T-644
(viewshed) is honest that the colour language must be prototyped first, but the compute
feasibility (full-island raster, wasm/WebGPU budget) is unassessed. The corpus proved the design
is unconstrained; it did not then constrain it. **Still an open design question wearing two
ticket numbers — deliberately and visibly for the viewshed, silently for occlusion.**

**Ask 4 — Ruler: buildable.** The operator settled the three shape decisions (polyline,
persistent, bearing); T-642 enumerates the residual ones (label placement, slope/Δelev, dismissal,
save/reload survival) as in-ticket decisions, which is the right size. One unflagged document
question: "persists until dismissed" + "survive save/reload?" implies a measurement-annotation
entity that `mission.schema.json` has no home for — if the answer is yes, this touches the
schema, and the corpus treats it as pure UI.

**Contours / height legibility: buildable — the best-covered item in the corpus.** Measured
doubling ladder, screen-space band, tint arithmetic, spot-height density, acceptance criteria at
4+ zooms, and reconciliation against the existing §N3 LOD contract. Nothing material missing.

**Panel show/hide: buildable.** Mechanism measured across all 75 screenshots (chevron bbox, stub
behavior, reflow, keys); the one real risk (wgpu canvas resize + camera hold) is called out in
the draft. Minor unknowns: TBD docks are 256/310 px vs Eden's 240, and whether `E`/`R`/
`Backspace` collide with existing editor bindings was not checked.

---

## Dimensions never asked about

The 14-section schema was workflow-and-config shaped. What a mission *editor* needs that no
section covered, ranked by TBD relevance:

1. **Multi-author editing / permissions / review.** Operator-demanded (braindump #14–16). TBD is
   CRDT-based; concurrent-editor semantics, locking, review comments, share/permission model —
   zero coverage in 6,300 analysis lines + synthesis. The frameworks could never teach this
   (single-file, single-author), which is exactly why the schema needed a TBD-native section.
2. **Testing a mission before shipping.** Braindump #25. Eden has Play-in-preview (batch03/04
   captured the Play menu); the parity table has no PLAY row; the synthesis's validation group is
   static rules, not the author's editor→mod→game round-trip. The closest artifact (OFCRA's
   lobby-selected run modes → "training mode later", §A.10) was ranked low for genre reasons.
3. **Versioning & migration UX.** Half-covered: A.1 makes the migration-harness argument
   (build before the first schema break) and B.5#5 repeats it — but as a warning, not a ticket;
   it appears in neither drafted nor proposed ticket sets. Mission *diffing* (braindump #14) is
   uncovered entirely.
4. **Localisation.** WOG is Russian, OFCRA French — the corpus itself is bilingual evidence that
   milsim communities are non-English; DTAS's i18n-key params are mentioned in passing. Whether
   TBD briefings/mission text need i18n: never asked.
5. **Accessibility.** Never mentioned in any document, despite `accessibility-review` being one
   of the four skills the handoff names for this work.
6. **Performance at scale** — half-covered: TBD's 367k-entity validation is cited, and §A.4 notes
   the real constraint is *legibility* at 137 slots/32 groups. Viewshed/contour compute budgets
   are not assessed (see GAP-3).
7. **JIP as an authoring concern** — grazed (v4's sync-vs-JIP defect; the Mission Settings JIP
   toggle) but never treated: what should a maker be able to author about JIP behavior?
8. **The mod/schema boundary** — the best-handled of these: D.5 splits editor-UI from
   mission-content precisely because the latter needs `executor: workbench`. Covered.

---

## Asymmetric evidence

Where a best-of-breed choice rests on unequal depth:

- **Loadouts (§A.5): the two winners are the thinnest sections.** OFCRA §5 is ~369 lines;
  WOG §5 is ~53 and FNF v4 §5 ~40. The verdict takes "OFCRA's inheritance + WOG's round-trip" —
  the WOG round-trip (U2, ranked #6) rests on one of the corpus's shortest sections, from the
  framework analysed without source docs. The *loser* (v4 baked kits) has the deepest
  adverse evidence; the *winner's* mechanics are comparatively under-documented.
- **Objectives (§A.7): the winning spine has unread semantics.** WOG's `WMT_Task_Point` wins on
  166 observed instances — but `wmt_main`, the module's implementation, is absent from the corpus
  entirely; semantics are `INFERRED:` from usage. FNF v4's objective system, the runner-up, was
  read from source. The synthesis says this honestly ("take the parameter set, decide the
  semantics") — but B5 is ranked "Very high" on evidence that is half inference, and U1 would
  write inferred semantics into `zoneRules`.
- **fnf_v3.md: zero `INFERRED:` markers over 1,198 lines** (vs 17/8/5/3 elsewhere), flagged by
  the README itself as "unusual rigour or unlabelled inference — indistinguishable from
  outside", with ~2× token spend through sub-agents whose labelling was not enforced. Every
  v3-sourced claim (the derived-briefing model, the 28 comments, the two-axis kit matrix) carries
  this asymmetry. Both places v3 conflicts with v4, v4 was correctly preferred.
- **House rules (§12):** OFCRA ~223 lines vs WOG/v4 ~42-44 — expected (README predicted it), and
  the synthesis correctly excludes the deep material as runtime scope. No bad choice traced.
- **fnf_tooling.md follows a different schema entirely** (Analyzer + DTAS), so FNF gets three
  documents and ~3,000 lines to OFCRA's one/1,949 and WOG's one/1,084. The validation ticket
  group (T-655…T-660) is therefore built almost wholly on FNF-side evidence, with WOG's two
  unreachable checkers as corroboration — acceptable, but it means validation *rule content* is
  one-community-shaped.
- **README's own provenance note is stale on OFCRA**: it says `ofcra_omtk.md` "skips from §4 to
  §6"; the file has a §5 at line 432 — the largest §5 in the corpus. Trivial in itself; it means
  the provenance layer was not re-checked after the analyses landed.

---

## Unestablished assumptions

1. **Arma 3 authoring models transfer to Reforger.** Asserted, never examined. The README scopes
   it correctly ("what does this model teach", "never port") — but no document asks what
   *Reforger* changes: different engine entity model, different modding surface (Enfusion
   GameMode/ScenarioFramework vs SQF modules), no `description.ext`, different respawn/JIP
   machinery. Every "the mod must read it" note (U1, U6, B5) assumes the Enfusion side can
   consume these concepts; no Enfusion-side feasibility check exists in the corpus.
2. **The browser editor can carry the proposed compute.** Viewshed raster, screen-space contour
   relabelling per zoom, spot-height culling, live validation over the full doc — all proposed;
   only contours have any perf grounding (the existing LOD ladder). The corpus's own boot-failure
   finding (T-631: swiftshader `createBuffer` panic) is evidence that GPU headroom is not
   uniform across user machines; no budget is stated for any new render lane.
3. **The mission document model can carry what is proposed.** Rulers-that-persist (T-642),
   comments (T-651), conditional-inclusion predicates (T-654), validation findings, review
   comments — several imply new entity kinds or editor-state persistence. Only comments get a
   scoping statement ("never compile"). Where editor-only entities *live* (doc vs local store vs
   new schema section) is undecided everywhere.
4. **"87 rows / 32 missing" is the size of the parity job.** Both numbers are wrong or
   unverifiable (61-ish rows; 31 missing; 95-field attribute surface unrowed). See GAP-2.
5. **The screenshots cover what their filenames claim.** Batches 04/05 are *not* asset-browser
   walkthroughs (the README's own correction — both agents pixel-diffed an unchanging right
   panel). The Eden asset-browser interaction model in the corpus rests on batch06 tooltips +
   the wiki scrape, not on observed use. The drafts' search-grammar and submode claims trace to
   wiki text, which is fine — but "75 screenshots analysed" overstates what was *seen*.

---

## Dropped threads

Ranked by how hard they bite at implementation time.

1. **Synthesis Part D → drafts: never merged, nothing filed.** T-654…T-660 exist only inside the
   synthesis; the drafts file still says 23 tickets; `gap_analysis.md` still lacks the two
   corrections (LAYER-CREATE-001 status, CONN-SYNC-001 warning note). Two documents currently
   disagree on what the program is.
2. **Registry stale by four ids** — T-627–T-630 in shipped code, absent from
   `.ai/tickets/registry.json` (verified: highest id T-626; `mission_editor.rs:185,213` tag
   T-627/T-628). Synthesis D.6 said "backfill before filing"; not done.
3. **The 32-vs-31 count.** Propagated through handoff → plan → drafts; never reconciled against
   the table. Whoever files group F will discover their checklist doesn't sum.
4. **T-074 is `cancelled` and T-646 revives its row without saying so** (synthesis D.4#5 raised
   it; no resolution). Also **T-084** (`deferred`, classname search = RIGHT-SEARCH-002) is
   missing from the drafts' absorb list, which names only T-072/073/075/077/078 — duplicate
   ticket risk on filing.
5. **Markers (T-069) is a load-bearing dependency left `deferred`.** B4 briefing links, T-641's
   sibling furniture, PLACE-005's marker half, and RIGHT-MODE-006 all route through it; no
   corpus document schedules or revives it.
6. **The program plan's open decision #3 (scale bar / north arrow / grid labels)** was answered
   for scale bar + grid labels (folded into T-641) — the **north arrow** silently vanished:
   neither adopted nor rejected in the drafts.
7. **`settings.respawn: "tickets"` dead enum** — synthesis says "remove or annotate" (twice:
   §A.8, C.4); no ticket carries it.
8. **INFERRED-status laundering risk on U1/B5**: WOG capture semantics are marked inferred in
   §A.7 and stay marked in C.2/C.3 (good) — but the proposed `zoneRules` field list in U1 is
   concrete enough that a ticket writer could copy it as spec. One more hop and the label is
   gone. (D.2's fabricated-quote incident shows exactly this failure mode already happened once
   in this program.)
9. **The eden-feds JSONL** (`.ai/artifacts/eden-feds-draft.jsonl`, 159 rows of wiki-derived
   feature entries) is referenced by nothing in the current corpus — an orphaned earlier attempt
   at the same coverage problem; if it contains rows the gap table lacks, nobody will ever know.
10. **Braindump items 1–6, 14–16, 24–25** (item data, vehicle data, full export, versioning,
    collab, review, mod triggers, E2E test) were captured 2026-07-25 with "STALE?/NEW" statuses
    and a promise of `[DERIVED]` companion sections — the editor-UI program neither absorbs nor
    dispositions them, and no derived sections appear in the corpus.

---

## Bottom line

- **Ruler, contours, panel show/hide, chrome cleanup:** buildable from this corpus today.
- **LoS:** ray MVP buildable; "actually work" needs the occlusion decision nobody has raised.
- **Eden parity:** buildable for the 16 drafted rows; **not** completable as asked — 10 rows
  plus two whole authority-doc surfaces (41 interaction IDs, 92 attribute fields) have no
  disposition, and the corpus never tells its reader that boundary exists.
- **Frameworks ask:** fully delivered (five analyses + synthesis), with the asymmetries noted
  above; its output is not yet reconciled into the ticket program.
- Cheapest high-value fix the corpus could receive: a one-page **disposition table** for every
  un-covered parity row and un-rowed surface (build / defer / superseded-by-zones / no-AI
  decision), plus the drafts↔synthesis merge and the registry backfill, before anything is filed.
