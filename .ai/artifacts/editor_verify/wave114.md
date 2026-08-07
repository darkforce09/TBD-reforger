# Editor factory — wave 114 adversarial verification

Verifier: Claude (Fable 5), post-merge on `main` @ d037a47b (base 7028ecb3).
Slices: T-633 (17fe692e / merge fbb930be), T-651 (ef98b687 / 7a16eca0), T-695 (0fa3b530 / d037a47b).
Independent test replays on merged HEAD (both through `cargo xtask ai run`, distrobox-host-exec):
`cargo test -p map-engine-core --all-features` → **606 passed / 0 failed / 1 ignored** (the T-747
vacuous-pass trap avoided; the doc-module suite genuinely ran). `cargo test -p website-frontend`
→ **768 passed / 0 failed**. Merge hygiene: all three merges leave every file byte-identical to the
slice tips (`git diff <merge> <tip>` empty) — no evil-merge edits.

---

## Findings

### MAJOR | docs/specs/Mission_Creator_Architecture/eden/interactions.md:214 vs shipped T-651 | PLACE-COMMENT-001 closes with two spec-row clauses undelivered, one of them silently

**Evidence.** The spec row's Procedure step 3 is "Draggable, copy/paste, layerable, **composable**"
and its Postcondition is "**Comment icon at position**"; Edge case: "Saved in custom compositions as
documentation." Shipped: a comment is absent from the render SoA and **draws nothing on the map** —
by the slice's own words (mission_editor.rs `CommentEditorOverlay` doc: "with a comment absent from
the render SoA there is nothing on the map to grab"). Compositions capture is selection-based
(`editor_ops.rs:728` `capture_selection_entities` over the selection) and comments sit in no
selection lane (outliner.rs:99-101), so a comment can **never** enter a composition — the
"composable" clause is structurally impossible in this delivery and is mentioned nowhere in the
commit message, the store.rs field note, or the disclosed partial-delivery list (which covers only
drag→outliner-drag+X/Z-fields and copy→duplicate-verb).
**Impact.** The two acceptance checkboxes ("RMB → Place Comment", "Title/tooltip editable") ARE met,
and the registry summary's scope ("appear in the outliner, support drag/copy/layers") is met under
the disclosed narrowings. But the feature id being closed carries a postcondition (map icon) that is
not delivered — a placed comment is invisible on the canvas, findable only in the Outliner — and the
compositions clause is a **silent** drop, which is what the HARD GATE bars. No data is at risk and
nothing lies about what the code does; this is a scope-accounting defect, not a code defect.
**Disposition.** Documented, not fixed (standing instruction). The close note for T-651 should name
"no map glyph" and "not composable" explicitly; the drag/copy narrowing is already honestly
disclosed and I judge it acceptable (the mechanisms delivered are the ones a future map glyph would
call — `move_comment` is the same mutator).

### MINOR | apps/website/frontend/src/eden_top_strip.rs:1166, :2374, :2389-2390 | T-633's settle-only rationale is false and a test comment overclaims

**Evidence.** The commit and in-code comments justify `on:change`-only by saying the HH:MM readout
gives live feedback during the drag. It does not: the readout reads the `env` memo, which recomputes
on `doc_tick` only, and the doc mutates only on settle — the readout is frozen mid-drag and jumps on
release (same as pre-T-633, so no regression). Separately, the test comment at :2374 claims the pin
"proves the STRIP did not grow [an `on:input`] around it"; the test at :2376 asserts no such
absence (pre-existing unrelated `on:input` handlers live at :1355/:1367).
**Impact.** Behavior is correct (commit only on settle, RowMirror debounce intact — verified below);
the written justification and one test comment misstate the mechanism, which will mislead the next
editor of this file. **Disposition.** Documented, not fixed.

### MINOR | apps/website/frontend/src/mission_editor.rs:1791-1794 + eden_dock_right.rs:873-881 | Favourites tab dead-ends in "Resolving…" forever on registry fetch failure

**Evidence.** On registry fetch failure only `catalog`/`vehicle_catalog` are set to `Failed`;
`registry_items` stays `None`, and the favourites panel's `None` arm renders "Resolving {n}
favourite(s) against the catalogue…" with no failure arm and no retry. **Impact.** Honest (nothing is
marked stale on a failure — the important half of claim 12 holds) but the panel has an unrecoverable
loading state on a failed fetch. The failure-path omission on `registry_items` predates T-695; the
perpetual spinner surface is new. **Disposition.** Documented, not fixed.

### NIT | apps/website/frontend/src/ui.rs Select | Disabled select's chevron does not dim

Sibling span, `disabled:` variants fire on the element only, no `peer-disabled` — a disabled Select
dims to 30% while its chevron stays full-opacity. No current caller passes `disabled`. Cosmetic.

### NIT | apps/website/frontend/src/eden_dock_right.rs:745-751 | T-215 pin distinctness is prose-enforced

`arm_favourite_place` avoids the pin's needle only because it MOVES the payload
(`begin_place_vehicle(payload)`, no `.clone()`). Nothing asserts it stays clone-free; a future
`&PlacePayload` refactor could let the needle be satisfied from the favourites path while the
palette regressed. Sound today; not perturbation-proof.

### NIT | crates/map-engine-core/src/doc/store.rs:11639-11641 | Undo test asserts row restore, not filing restore

`comments_file_into_layers_and_each_gesture_is_one_undo_step` undoes the delete and asserts
`comment_count == 1` but does not assert the L2 `entityIds` filing came back. The delete is one txn
over both maps and both are in undo scope (store.rs:321 `expand_scope`), so it almost certainly does
— but the property is not pinned.

### NIT | crates/map-engine-core/src/mission/compile.rs:37-38 | "Keep in lockstep" header is now misleading

The header on `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS` says keep in lockstep with store.rs's twin, but
the two lists deliberately diverge on `zones`/`compositions`/`triggers` (pre-existing) and now
`comments` (T-651, correctly). The divergence is documented on the store side only
(store.rs:5058-5065); compile.rs's own prose still implies identity.

---

## Is `main` safe to build the next wave on — **yes.**

The one MAJOR is ticket-accounting (what "closed" means for PLACE-COMMENT-001), not broken code,
lost data, or a lying gate. Every load-bearing runtime claim held under attack.

---

## Verified-clean register — claims re-proved, not taken on trust

**T-651 — the never-compiles rule (claim 1).** Re-proved from four directions. (a) The exclusion IS
the claimed absence: `mission/flatten.rs:652-685` `EditorPayload` derives plain `Deserialize` with
`#[serde(default)]`, **no `comments` field, no `#[serde(flatten)]` catch-all, no
`deny_unknown_fields`** — serde drops the key before the mod document exists; nothing incidental is
doing the work. (b) The `payloadExtras` route: `small_maps_json` (store.rs:549-594) emits
`commentsById` + a `payloadExtras.comments` projection (and **removes** it when the last comment is
deleted, so the wire goes clean); `compile_payload` (compile.rs:268-281) promotes extras keys NOT in
its known list — `comments` is deliberately absent from that list, so the promotion is what makes
Save persist a comment, and the promoted `comments[]` then dies at the flatten boundary. Both the
Save wire and the Export envelope carry it; the mod document (the only thing the game consumes, via
`/compiled` and the wasm Export's `flatten_mod_document_json` — same function, flatten.rs:1562-1563)
cannot. (c) The layer-filing back door is closed: a filed comment id lands in
`editorLayers[].entityIds`, which reaches the EDITOR payload — but flatten's `EditorGraph`
(flatten.rs:698-702) declares only `factions/squads/slots`; `editorLayers` never reaches the mod
document at all, so not even the id leaks. (d) The test
(`comments_never_reach_the_mod_document`, store.rs:11455) is genuinely non-tautological: step 4
re-routes the same token through `entities[]` and asserts the mod bytes turn dirty, so step 3's
absence assertion is armed. Ran green in my independent `--all-features` replay.
**T-651 — the two-list divergence (claim 2).** Judged CORRECT and load-bearing: adding `comments` to
compile.rs's list would make the T-219 promotion skip it → every comment dropped on Save; adding it
with an author would compile an annotation. Store's list must have it (store.rs:5065) so hydrate
loads rather than double-parks. No unknown-key diagnostic exists to fire (validate.rs has none;
the backend save schema's root has **no `additionalProperties: false`** — checked the JSON — so the
new top-level `comments[]` validates on Save).
**T-651 — eden_tree.rs out-of-owns edit (claim 3).** `match row.kind` (eden_tree.rs:502) is
exhaustive with no catch-all (the `_ =>` hits at :635/:845 are other scopes); `NodeKind::Comment`
cannot compile without the arm — a compile-forced edit. The arm routes dblclick to
`open_comment_editor` and pointerdown to `begin_layer_comment_drag`; **no** `select_slot` /
`open_attributes` — no T-716 live-but-inert row.
**T-651 — has_content (claim 4).** `self.comments.len(&txn) > 0` is in the disjunction
(store.rs:3368); `blob_has_content` replays into a throwaway core and asks the same predicate, so
the IDB write guard and `classify_local` both count a comment-only doc as work. No new
spurious-conflict vector: the fresh doc has always been content-positive (8-slot fixture seed,
yrs_persist.rs:96-97), so seeded comments change nothing there.
**T-651 — undo / delete / duplicate (claim 5).** `remove_comment` (store.rs:1770-1774) removes the
row AND calls `remove_id_from_all_layers` in the SAME txn; that helper is a byte-faithful extraction
of `move_slot_to_layer`'s detach half (diff-verified), so one implementation serves both.
`duplicate_comment` is one `begin()` txn. Pinned by
`comments_file_into_layers_and_each_gesture_is_one_undo_step` (dangling-id assertion + Ctrl+Z
restore), ran green.
**T-651 — BigInt position (claim 6).** `comment_xz` (store.rs:5305-5316) reads `Any::Number` AND
`Any::BigInt`; `duplicate_comment_copies_fields_and_offsets_even_after_a_hydrate` hydrates
integer-valued 6400 and asserts the copy offsets from 6400, not 0. Ran green.
**T-651 — place gesture (claim 8).** The contextmenu handler unprojects against the same frozen
camera as the pick and rides the point on `MenuTarget` via `.at_world` (mission_editor.rs); zero
`LeftGesture`/`Pending` state added; T-651 never touched select_tool.rs (6-file diffstat). Nothing
can strand.
**T-651 — seed ordering (claims 16).** Seed runs at fresh-doc mint (mission_editor.rs:1876-1893)
under INIT origin (not an undo step — pinned by test, undo_depth 0). The IDB restore swaps in a
different core; the server-hydrate adopt path calls `hydrate`, whose clear loop wipes **all sixteen
roots including `self.comments`** (store.rs:2729-2748) before reloading `comments[]` — so a restored
or downloaded mission gets exactly its saved comments, never seeds on top. Seed declines on any
non-empty comments map (belt and braces). Verified both directions.
**T-633 (claims 9-11).** Layering: ui.rs:8 imports two `pub(crate) const &str` class recipes from
eden_layout.rs:265/:284; nothing runtime crosses; eden_layout has zero `crate::ui` references (no
cycle); 34 modules including non-editor pages consume ui.rs with no behavioral drag (one wasm crate,
consts inline). Scrubber: Slider's only handler is `on:change` (ui.rs:187), no `on:input`, no
internal signal; strip wiring goes `on_change` → `author_env` → `row_mirror.set_time` — no store
write per input event, RowMirror debounce intact. CSS: the pseudo-element rules are unbroken string
literals (ui.rs:134-141), `style/aegis.css` has `@source "../src/**/*.rs"` and defines every token
used (`--color-primary: #adc6ff` etc.); the freshly-built `dist/aegis-*.css` (mtime one minute after
HEAD's commit) contains the `webkit-slider-{runnable-track,thumb}` and `moz-range-*` rules — the
slider is really styled in the release build, re-verified, not taken from the slice's out-of-band
run.
**T-695 (claims 12-14).** `resolve_favourites` (eden_dock_right.rs:660-683) is a single
`.map().collect()` — no filter/retain/truncate anywhere; count-preservation pinned by
`stale_favourite_is_kept_and_marked_not_dropped`. Registry `None` (still loading) renders a
"Resolving…" arm and never calls resolve — nothing marked stale on a slow load. Saves persist the
raw collection, never a resolved subset; no prune path exists. T-215 pin: the needle
`editor_ops::begin_place_vehicle` + `(payload.clone())` (test `vehicles_tab_places_instead_of_
promising`, :2603-2633) matches exactly ONE site — the palette-leaf arm at :209; `arm_favourite_
place` moves the payload and cannot satisfy it. Storage: v0 blob migrates via `#[serde(default)]` +
version stamp (pinned test); garbage → `unwrap_or_default()` with **no save on the load path** (one
bad read cannot overwrite the blob); cap truncates oldest; dedupe drops only empty-id and exact-dup
`asset_id`s; identity is `resource_name` end-to-end (`find_catalog_item` matches
`i.resource_name == asset_id`, never a registry uuid) — re-ingest cannot orphan the collection.
Saves fire only from the two user-action handlers; no effect-loop writes.
**Cross-slice (claims 15, 17).** Commit footprints are fully disjoint (T-695 touched only
asset_catalog.rs + eden_dock_right.rs; it put the star verb on palette-leaf hover buttons, not in
T-651's context_menu.rs). Dock tab: `Favourites` takes new index 6, indices 0-5 untouched, tab state
not persisted — no stale-index risk. localStorage census: `tbd-mc-editor-favourites` is unique
(anti-collision asserts at eden_dock_right.rs:3104-3105); T-651 writes no web-storage key at all.
Hidden/locked inheritance: comments deliberately pin all four flags false (outliner.rs:167-189,
reasoned in-code); delete-under-lock is PARITY with slots (`remove_slot` has no lock gate either —
T-665's lock is transform-only); comments sit in no selection lane so Del-key cannot reach them.
Merged files byte-identical to slice tips — no conflict-resolution damage.
**Gate integrity.** The 30/30 wave gate was not vacuous where it matters most: my independent
`--all-features` replay compiled and ran the full 606-test doc suite, including every T-651 test,
and the 768-test frontend suite on the merged tree. No gate reported success on unexamined code.

**Falsification attempts that found nothing** (no re-audit needed): payloadExtras passthrough leak
into the mod document; `commentsById`-crafted-payload leak; layer-filing id leak via
`editor.editorLayers`; schema root rejection of `comments[]` on Save; duplicate-notes via
adopt-into-seeded-core; spurious conflict-gate from seeded comments; undo scope excluding comments;
dangling folder ids after delete; BigInt zeroing; gesture-machine stranding; ui↔eden_layout cycle;
per-input-event store writes defeating RowMirror; unstyled slider in release CSS; favourite pruning
on any path; stale-marking on registry-loading; favourites data loss on garbage/v0 blobs; uuid
identity orphaning; T-215 needle satisfiable from the favourites path; cross-slice tab/menu/
localStorage collisions; evil-merge damage.
