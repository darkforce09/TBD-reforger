# Wave 116 adversarial verification — T-069 / T-690 / T-696

Range: 619e5a14 → 013bb78f. Suites re-run on merged main:
`map-engine-core --all-features` **616 pass / 0 fail** (T-747 trap avoided), `website-frontend`
**817 pass / 0 fail**, `website-api` unit **267 pass / 0 fail**. The one API failure
(`aar_replay_url_backfill`) is environmental — T-542 hard-requires `TEST_DATABASE_URL`/`TBD_GATE_DB`,
which my shell does not have; it is a pre-wave-116 gate fixture, not a regression. Tree left clean;
this file is the only write.

---

## Findings

### 1. MAJOR | apps/website/frontend/src/eden_dock_left.rs:1107–1130 (`the_index_and_the_fly_to_reuse_the_shipped_paths`), :1083–1101 (`the_dock_has_two_tabs_and_defaults_to_layers`) | T-696's headline source pins are hollow — they match their own source

**Evidence.** `SRC = include_str!("eden_dock_left.rs")` — the whole file, test module included.
Every POSITIVE needle in these two tests (`"parse_locations_json"`, `"__editorCamSet"`,
`"camera_snapshot"`, `"/map-assets/{terrain}/locations.json"`,
`"let tab = RwSignal::new(LeftTab::Layers)"`, all eight `data-testid` strings) appears verbatim
inside the assertion that searches for it, so `SRC.contains(...)` is satisfied by the test's own
line. Delete `fetch_named_places`, rename the camera hook, or drop a testid from production and
every one of these stays green. The file demonstrates the author KNEW the idiom — the one negative
needle is split (`format!("{}{}", "set_view", "(")` "so the needle itself is not a hit") — and the
positives were left unsplit anyway. Contrast T-069, which split every literal in
`marker_writes_go_to_the_briefing_not_the_root_map`, and T-690, which scoped every scan to
`live_code`/pre-`#[cfg(test)]` source.
**Impact.** The registry precedent for exactly this class is T-559/T-561 (comment/string decoys keep
a Class-R pin green — filed as DIRTY MAJOR). The pinned FACTS are currently true — I verified each
independently below — but the pins prove nothing and will not catch the regression they advertise
(most importantly: "fly-to must ride the installed closure" and "no second camera mover").
**Disposition.** Not fixed, not filed (standing instruction). Needs a follow-up ticket: split the
needles or scope `SRC` to the pre-test half, the `bookmarks_and_fly_to_are_not_document_edits`
pattern three tests down in the same file.

### 2. MAJOR (scope gap, judged) | T-069 | placed markers draw NOTHING on the map — and unlike wave 114's identical gap, no follow-up ticket exists

**Evidence.** T-069's merge touches only `eden_dock_right.rs`, `editor_ops.rs`, `doc/store.rs`
(git stat — see finding-free item 12 below for the T-672 side of this). Nothing binds markers to the
engine: `mission_history.rs:364 after_doc_change` rebinds slots (`slots_bind_soa`), squad links and
vehicles only; `grep -ri marker` over map-engine-render's engine surface shows no marker lane and no
frontend feeds one. An author drops a marker, the dock lists it, the compile carries it — the canvas
shows nothing.
**Impact.** RIGHT-MODE-006 is titled "markers on the map". The agent's REFUSAL to add a lane is
correctly reasoned — the only rebind tail after undo/redo/restore is `after_doc_change`, which is
outside T-069's owns, so a lane fed from owned call sites alone would go stale exactly the way the
ticket-splitting doctrine forbids. The four ATTR-FIELD-MRK-* rows are honestly closed (they are
attribute-panel rows and they work). RIGHT-MODE-006 is at best half-closed. Wave 114 shipped T-651
comments with this same no-glyph property and it was FILED as T-748; markers have no T-748
equivalent, so the gap currently lives nowhere but this report.
**Disposition.** Operator call: either a marker-glyph ticket (naturally adjacent to T-672's
map-engine-render work next wave) or an explicit registry note that RIGHT-MODE-006 excludes the
glyph.

### 3. MINOR | apps/website/frontend/src/validation_panel.rs:524 | compile findings are never cleared on mission switch — a stale build report follows the operator across missions

**Evidence.** `COMPILE_FINDINGS` is a thread_local written only by `publish_compile_findings`, whose
only production caller is `export_compiled_now`. Nothing on editor mount, doc hydrate, or route
change resets it. `/missions/:id/edit` is a leptos_router client-side route (app_routes.rs:90), so
editor→library→other-editor navigation reuses the same wasm instance.
**Impact.** Export mission A (findings fire), navigate to mission B: B's panel shows A's compile
rows — messages naming A's squads/slots, `subject_id`s that resolve to nothing in B. Not data loss,
not a false ticket claim (T-690 never promised cross-mission behaviour), but the panel lies until
the next export.
**Disposition.** Follow-up: clear (or key by mission id) on editor mount. One line in the hydrate
path plus a pin.

### 4. MINOR | apps/website/frontend/src/eden_dock_left.rs:821–844 (`fly_to`) | production feature rides the T-166 smoke hook, and every failure mode is silent

**Evidence.** `fly_to` resolves `window.__editorCamSet` via `Reflect::get` with let-else `return` at
every step. The hook is installed by `mission_editor.rs:4572 register_editor_cam`, called
unconditionally at engine init (line 2780, same block as `register_render_ctx`) — so the
"not yet installed" window coincides with "no engine", where `live_camera()` already returns `None`
and the click is a no-op anyway. Body verified: exactly `set_view` → `on_camera_changed` →
`flush_viewport` — the same sequence as the shipped mover, zero duplicated camera math, and no path
into the document or history.
**Impact.** Behaviourally identical TODAY. But the hook's own doc says it exists so `smoke_fullmap`
can probe (T-166); someone "cleaning up test hooks" renames it and fly-to dies with no error, no
toast, no failing test — the only pin tying this file to the hook name is the hollow one in
finding 1. The agent's in-owns argument is sound (`world_assets` exposes a camera reader and no
writer; the writer seam would mean editing files outside owns).
**Disposition.** Accept for this wave; the promoted seam (`world_assets` camera writer or a
`named_locations()`-style accessor) plus a non-hollow pin is the follow-up. The agent says it filed
this in its slice report — the seam ticket should actually exist before T-672-era churn touches the
editor init path.

### 5. MINOR | T-069 slice narrative | the "T-695 pin was passing on a DECOY" claim does not reproduce — correct the registry summary, in both directions

**Evidence.** At base 619e5a14, `"Marker placement lands in T-069."` occurs in
`eden_dock_right.rs` exactly TWICE: line 1376 — the LIVE stub in the rendered view — and line 3354,
the T-695 pin itself with its literal correctly split (`format!("Marker placement {} T-069.",
"lands in")`), which its own source therefore cannot satisfy. The T-215 pin's `stub()` helper
(line 2606) builds the same sentence, also split. No comment anywhere in the base file quotes the
stub sentence. Both pins were passing on the REAL stub, not a decoy.
**Impact.** A registry row that records "found a decoy pin" for a decoy that never existed corrupts
exactly the audit trail findings 1 fed off (T-559/T-561). Meanwhile the SAME summary needs the
correction the agent DID earn: the ticket's `markersById` premise is dead (verified below, item 1).
**Disposition.** When the registry row for wave 116 is written: record the premise correction,
strike the decoy claim.

### 6. NIT | apps/website/api/src/handlers/missions.rs:2358 | the /compiled diagnostics headers are pinned by source scan only — no response-level test

`t690_compiled_route_surfaces_the_structured_diagnostics` proves lift-before-serialize, both header
constants named, `warn!` present, no refusal minted — all against source text. No test builds an
actual axum response and asserts `x-compile-diagnostics-count: 0` on a clean mission. I verified the
"always present, `0` included" claim by reading: the insert is unconditional and
`HeaderValue::from_str` over `usize::to_string` cannot fail. Correct today; unpinned behaviourally.

### 7. NIT | apps/website/frontend/src/eden_dock_right.rs:2809 (Attributes lookup), editor_ops.rs `marker_rows` | marker selection is by id alone; a hydrated foreign payload can carry the same marker id under two factions

`mint_marker_id` guarantees uniqueness across factions for markers minted HERE, but briefing markers
hydrate with whatever ids the stored payload carries; two factions with `mk-1` would make
`find(|r| r.id == id)` edit the first faction's marker while the operator has the second selected.
Unreachable through this editor's own authoring; reachable through an imported/merged payload.

### 8. NIT | crates/map-engine-core/src/doc/store.rs (T-069 test prose) | "EditorPayload … declares no root key whatsoever" overstates

`EditorPayload` does declare root keys (`zones`, `entities`, `vehicles`, …). The load-bearing half —
it declares no `markers` root key, so a root-map marker is compile-invisible — is true and is what
the test actually proves. Prose only.

---

## Is `main` safe to build the next wave on — **yes.**

Nothing shipped is broken, no data is at risk, and every gate-visible suite is green when run the
way the gate runs it (Makefile line 183 does use `--all-features`, so the T-747 blind spot does not
apply to the wave gate). The two MAJORs are a hollow pin (the pinned facts are independently true
today) and a tracked-nowhere scope gap — both need tickets, neither needs a revert.

---

## Verified-clean register — re-proved, not taken on trust

**T-069**
1. **The briefing-vs-root call (the ticket contradiction) — the agent is RIGHT.** Independently:
   `mission.schema.json` top level = 19 properties, no `markers`, `additionalProperties: false`;
   the ONLY `markers` key in the whole schema is `$defs/briefing.properties.markers` (walked the
   JSON myself); `flatten.rs:907 EditorPayload` declares no root `markers`; and
   `flatten.rs:1654 derive_briefings` pushes `briefing.markers` into the compiled document. The
   store test compiles a root-map marker AND a briefing marker in one document and shows only the
   briefing one arriving. The ticket's `markersById` premise is dead; registry summary needs the
   correction (finding 5).
2. **Addressability + stable order.** `briefing_marker_rows_json` emits `(factionId, id)` per row —
   the exact pair both T-345 mutators take; single STABLE sort by `factionId` over rows pushed in
   per-faction array order, so array order survives inside each group; the move test proves an
   upsert does not reorder; byte-identical across 8 repeated reads. All green under `--all-features`.
3. **Icon vocabulary.** Enum length is exactly 64 (counted in the schema, not the test); the list is
   parsed from the embedded schema at runtime; NO second copy exists (`objective_marker` grep over
   crates+apps hits only the schema, a doc comment, and a test); every write path gates —
   `begin_place_marker` refuses at arm, `set_marker_icon` refuses at write, and the only three
   `set_/remove_faction_briefing_marker` call sites in the frontend are those verbs plus the
   label/position upsert that carries the existing icon through unchanged.
4. **Pin inversions preserved, not weakened.** T-215's inversion still proves tab-distinctness
   (stub gone + `begin_place_marker(armed.clone())` present, literal split); T-695's still proves
   the INDEX claim (`2 => markers_panel(` split, tab 6 untouched) and adds the comment-decoy
   negative. Attacked for weakening; found none. (The DECOY story about the old pin is a separate
   matter — finding 5.)
5. **Arm lifecycle.** `has_pending()` is variant-agnostic (`is_some`), so `mission_editor.rs`
   needed no change — confirmed at the 3090/3329 gates; `cancel_pending` drops every non-Zone
   variant including `Marker`; `place_at_keep` snapshots and re-arms only on success (no stranded
   arm, no spin — the T-723 shape); a marker place runs `after_local_edit`, so undo/dirty are
   correct.
6. **map-engine-render absent from the T-069 diff** — merge stat is exactly three files; the T-672
   precondition holds.

**T-690**
7. **The out-of-owns `validation_panel.rs` edit is additive and safe.** New thread_locals + a
   publish fn + a two-line `evaluate_now` extend; the view, registry, debounce and every existing
   test untouched; `a_clean_payload_produces_an_empty_panel` present at line 1164 and green;
   `PANEL_SINK` cleared `on_cleanup`. The justification (the panel must stay the single claimant, so
   the feed lives in the panel module) survives attack — the alternative was a second render surface
   in the command layer, which its own Class-R pin now forbids. Process breach for the operator to
   log; not a code defect.
8. **The exhaustiveness claim is TRUE — hardest attack, held.** `validate.rs:1041`
   `ORBAT-SQUAD-HAS-LEADER` (Warning) fires for every non-empty squad whose `leaderSlotId` is
   absent, blank, or not one of its own `slotIds`; `COMPILE-DROP-SQUAD-LEADER` fires for every
   wire-reaching squad where `leaderSlotId` IS authored. Every compiled non-empty squad trips at
   least one (an invalid-but-authored leader trips both), so as registry rules the panel could never
   go green — the fnf_tooling.md 1.3 defect, verbatim. The chosen design does not reintroduce it:
   the registry is untouched, publish replaces the whole list per compile-act, and a clean compile
   clears (pinned). The one residue is finding 3 (cross-mission staleness), which is a lifecycle
   bug, not a green-ness bug.
9. **Never debug-gated.** Independent grep: the only `cfg` in production flatten.rs is
   `#[cfg(feature = "doc")]` at 3667/3700, nowhere near the four emission sites (2302/2397/2545/
   2574); plus the shipped behavioural + source-scan test pair.
10. **No severity on correct input.** Clean FIXTURE → empty list (with anti-vacuity seed);
    blank/whitespace/null → not authored; an unreferenced squad reports nothing.
11. **One compile body.** `flatten_mod_document_json` → `with_substitutions` → `_full`;
    `with_diagnostics` → `_full`; byte-identity of the diagnostics path vs the plain path pinned by
    test; the API's `flatten_to_mod_document_with_catalog` (mission_compile.rs:131) is a
    refusal-only prescan that delegates to the same `flatten_to_mod_document` body — diagnostics
    populated on the `/compiled` path too, lifted before `validated_compiled_body` consumes the doc.
12. **Headers carry ids, never messages.** Rules header is built exclusively from
    `Finding.rule_id: &'static str` — the six ASCII constants — deduped in fire order; messages
    reach only the `warn!` line; the body stays schema-valid with findings present and carries no
    `diagnostics` key (behavioural test). Header-injection attack found no route in. (Behavioural
    header-presence test missing — finding 6.)
13. **`FLOW_DEFAULT_*` untouched and unread** — zero hits in the T-690 diff; constants intact at
    flatten.rs:1763–1769 for T-753.

**T-696**
14. Camera hook: same mover verified instruction-for-instruction (item 4 above) — the RISK is
    finding 4, the "behaviourally identical" claim itself held.
15. **Same URL, same parser, nothing hardcoded.** `labels.rs:64` fetches `{base}/locations.json`
    with `base = "/map-assets/{terrain}"` (world_assets/mod.rs:249) through
    `parse_locations_json` — byte-identical source and parser to `fetch_named_places`; no location
    datum appears in the dock source.
16. **House storage pattern matched.** `tbd-mc-editor-bookmarks` + `BOOKMARKS_VERSION` + defaults on
    parse failure + cap + one migrate chokepoint = the T-695 favourites shape exactly
    (eden_dock_right.rs:443–451); migrate drops unnamed/duplicate/non-finite rows (tested);
    garbage → empty, v0 → stamped forward. Fragmentation claim confirmed: three `tbd-mc-editor-*`
    keys, three boot reads, three migrate fns (prefs / favourites / bookmarks).
17. **Not a document edit.** The properly-scoped pin (production half only) bans `mission_history::`
    / `after_local_edit` / `add_slot` / `remove_slots`, and the actual camera path
    (`set_view` → `on_camera_changed` → `flush_viewport`) touches neither the doc nor the dirty
    flag. A bookmark restore cannot dirty or enter undo.

**Cross-slice**
18. Markers trip no drop diagnostic (the walk scans squads, slot identity keys, and the vehicle
    roster — never briefings); `comments_never_reach_the_mod_document` (store.rs:11709) still
    present and green beside T-069's markers-must-compile pin — comments do not compile, markers do,
    in the same suite run.
19. Confirmed (item 6 above).
20. No other interaction found: the three slices share no signal, no storage key, no engine lane;
    T-690's byte-identity pin protects T-069's compiled markers from the diagnostics walk by
    construction.

**Attacked and failed to break** (nobody needs to re-audit these): the briefing-vs-root schema
argument; marker row addressability/ordering/move-stability; the closed icon vocabulary and its
single source; the two pin inversions' index claims; the marker arm lifecycle incl. Ctrl multi-place
and release-over-chrome; the ORBAT/DROP exhaustiveness proof and its non-reintroduction; debug-gating
(behavioural + independent source scan); clean-input silence; single-compile-body byte identity
across all four entry points; header injection via findings; schema validity of a findings-bearing
/compiled body; `FLOW_DEFAULT_*` isolation; bookmark storage migration/integrity floor and the
not-a-document-edit property; comment/marker compile asymmetry; the wave gate's feature coverage
(Makefile runs map-engine-core with `--all-features`).
