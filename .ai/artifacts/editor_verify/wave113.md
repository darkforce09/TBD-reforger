# Wave 113 adversarial verification — merged `main` @ 62a76146

Range b57477cc → 62a76146 (T-082 ca6acd30, T-669 7c452868, T-694 62a76146). Seven files, all
accounted to the three slices; no stray artifact/ticket/migration in the range. Working tree left
clean; the two pin-firing experiments below were working-tree edits reverted with `git restore`
(confirmed clean before and after).

Runs: `cargo test -p map-engine-core --all-features` → 599 passed / 0 failed;
`cargo test -p website-frontend` (native) → 739 passed / 0 failed.

---

## Findings

### F-1 MAJOR | crates/map-engine-core/src/doc/store.rs:2175-2178 | ACTION-PASTE-ORIG-001 does not paste at the original position — it pastes 20 m off, and three authority docs state otherwise

- **Evidence.** `paste_slots`'s no-anchor arm is unconditional:
  `(dx, dy) = match (anchor_x, anchor_y) { (Some(ax), Some(ay)) => …, _ => (PASTE_NUDGE, PASTE_NUDGE) }`
  with `PASTE_NUDGE = 20.0` (store.rs:4225). `Ctrl/Cmd+Shift+V` → `paste_at_cursor(None, None)` →
  this arm, so every slot lands at `(x+20, y+20)` clamped. The T-669 agent's contradiction of its
  briefing is **correct**: `.ai/tickets/registry.json:16681` ("`paste_at_cursor(None, None)` is
  LITERALLY paste-at-original"), `docs/specs/.../gap_analysis.md:265` and
  `.ai/artifacts/parity/interactions_sweep.md:233` are all wrong on this point of fact. The nudge is
  byte-parity with the JS oracle (`ydoc.pasteSlots`), so it is not the arm's choice.
- **Impact.** The shipped feature is "paste near original (+20 m x/y)". The arm comment discloses
  it honestly, but the help row copy (`eden_help.rs:108` — "Paste at the source position") repeats
  the overstatement to the operator, and the row/gap docs claim a behavior the code has never had.
- **Disposition.** Not agent dishonesty and not silently fixable (parity constraint). Operator
  decision: either re-scope the row to "paste without moving to cursor (oracle 20 m nudge)" and
  soften the help copy, or defer an exact-coordinate variant as its own ticket. The three documents
  above need the factual correction either way. **The row as literally named is not closed.**

### F-2 MINOR | crates/map-engine-core/src/doc/store.rs:596-599 + apps/website/frontend/src/attributes.rs:236-240 | hiding a slot closes its open Attributes modal, indistinguishable from undo-away (cross-slice, claim 15 confirmed)

- **Evidence.** `materialize()` drops layer-hidden (T-665) and `editorHidden` (T-701) slots before
  any column is pushed; `read_attrs` positions on `soa.ids` → `None`; the modal's `None` arm calls
  `close_attributes()` — the same path as "slot was undone away".
- **Impact.** Hide (a view flag) while the modal is open silently discards modal state (tab,
  multi-edit latches, focused draft). No data loss: text-field commits land per keystroke and
  number-field drafts are the only thing droppable. Confusing, not destructive.
- **Disposition.** Document as a known conflation; a fix would be `read_attrs` consulting the raw
  rows (which T-082 now parses anyway) instead of the SoA for existence. Not this wave's regression
  — T-082 found and reported it, did not cause it.

### F-3 NIT | apps/website/frontend/src/editor_ops.rs:1855-1880 (`attrs_update_slot`) | single-target commit fires the history tail unconditionally

- **Evidence.** `did` is `true` whenever ctx+doc exist — an all-`None` call, or a call against a
  nonexistent slot id, still runs `after_local_edit()` (dirty + persist arm) with zero doc change.
  The multi variant got the widened no-op guard; the single variant did not (`attrs_update_position`
  has the same pre-existing shape for a nonexistent id).
- **Impact.** Unreachable from the modal today (every `commit_slot` caller passes exactly one
  `Some`), so no false "saved" is currently producible. A future caller could produce one.
- **Disposition.** Defer; a one-line guard mirroring the multi variant.

### F-4 NIT | .ai/tickets/registry.json:16681 | T-669's "touches exactly one file" claim is stale

- **Evidence.** The slice touched `mission_editor.rs` **and** `eden_help.rs` (2 rows + the count
  sentence) — forced by T-692's set-equality pins and the derived-count pin, so the second file was
  mandatory, but the registry summary still claims one file.
- **Impact.** Bookkeeping only. **Disposition.** Doc correction alongside F-1's.

### F-5 NIT | apps/website/frontend/src/eden_settings.rs (`ShapeMirror::load` / `set_game_mode`) | no single-flight between the open-GET and an in-flight PATCH

- **Evidence.** `load()` fires one GET per open edge (no storm — verified the `Effect` tracks only
  `open`); a reopen while a `set_game_mode` PATCH is in flight can land a pre-PATCH row after the
  optimistic set, showing the old mode until the next open. PATCH success never writes `shape`, so
  nothing reconverges until reopen.
- **Impact.** Transient stale `<select>` in a race the operator must work to hit; re-selecting is
  harmless. Failure paths are honest (failure → `shape=None` → `SHAPE_UNAVAILABLE_NOTE`, select not
  rendered — verified; no stale or invented value survives a failed GET).
- **Disposition.** Accept; note for whoever gives this dialog a sequencer later.

---

**Is `main` safe to build the next wave on — YES.** (F-1 is a claims/docs problem, not a code-safety
problem; nothing in the range breaks the build, risks data, or was gate-approved unexamined.)

---

## Verified-clean register — re-proved, not taken on trust

1. **`update_slot_object` semantics (T-082 claim 1).** `None` = leave-alone on BOTH keys,
   `Some("")` = remove-key, both keys in ONE `begin()` transaction, early-return (no txn) on
   double-`None`, no-op on absent id — read at store.rs:2016-2035 and re-ran
   `update_slot_object_sets_clears_and_leaves_none_fields_alone` green. The justification for
   leaving owns holds: `update_slot_role_character` (store.rs:1937-1963) writes `role`
   unconditionally and clears BOTH `tag` and `assetId` on `None`/empty — routing a type edit
   through it really would stamp the snapshot role and wipe `tag`. No existing mutator writes
   `description`. HARD GATE correctly invoked.
2. **One predicate, not two walks (claim 2).** `slot_layer_is_locked` (store.rs:2718),
   `update_slot_position` (store.rs:2074) and `move_entities_in_txn` (store.rs:4579) all call the
   single free fn `slot_is_transform_locked` (store.rs:4661). No restatement exists. The
   agrees-with-refusal store test re-ran green.
3. **F-7 no longer reproduces (claim 4) — confirmed independently.** Display half: `number_field`
   (attributes.rs:407-479) shows `draft` only while `focused`; unfocused it mirrors `seed()` = the
   doc value, so a refused write snaps back on blur — that is T-649's `focused`/`seed()` split, as
   T-082 reported. State half: refused writes skip the tail (`attrs_update_position` pre-checks
   `slot_layer_is_locked` and returns false, editor_ops.rs:1247; `_multi` fires only on `moved`),
   so no doc_ver bump / dirty / persist arm. Affordance half: `all_locked → Gate::refused()` →
   field disabled and the opt-in checkbox inert (`disabled=gate.shut`), straddled selections stay
   live with the count note — the "partial claimed as total" inversion is guarded
   (`all_locked = n > 0 && locked_n == n`, pinned). A refused locked-slot edit cannot even be typed
   any more. **F-7 is closed; the constraint was honoured, not dodged.**
4. **Paste-arm partition (claim 7), re-derived.** The two `KeyV` guards differ only in shift
   polarity; conjunction is unsatisfiable, union is exactly `modk && !alt`. Exactly two `KeyV`
   arms exist file-wide; no other frontend module binds `KeyV` or `KeyX` (grepped). The pin's
   truth-table + literal-guard-string double-check is sound.
5. **Cut cannot become a silent Delete (claim 8).** `copy_selection` (editor_ops.rs:481-517)
   returns false on: no ops ctx, empty selection, no doc, `slots_json` parse failure, empty clip —
   and the arm is `copy_selection() && delete_selection()`, so the delete is unreachable on any of
   them; `delete_selection` independently refuses an empty selection. Clipboard snapshot precedes
   the remove, so a cut is always paste-recoverable. `description`/`assetId` survive cut→paste
   (clipboard carries raw rows; `PASTE_KNOWN` excludes `description` → `extras` path).
6. **T-692 blindness (claim 9) — proven empirically, not by argument.** Deleted the
   `Ctrl/Cmd + Shift + V` row from `SHORTCUTS`: **every t692 pin stayed green** (sets compare
   `KeyV` = `KeyV`), and only T-669's compensating chord pin
   `both_new_chords_are_documented_in_the_help_table` went red, naming the row. Restored; tree
   clean. The compensating pin is real and placed in the right file.
7. **T-694 scope literalism (claim 10).** `dto.rs` diff = doc comments only, no serde field, name,
   or order change (R-api goldens untouched and green); no migration, no backend file in the range;
   `game_mode` editable via PATCH; player count derived from `slot_count()` at render, re-derived
   per `doc_tick` (eden_settings.rs:362) — so a Ctrl+X cut updates it live (claim 16); no
   reconciliation — disagreement is displayed (`PLAYER_COUNT_DISAGREE_NOTE`), never resolved.
   `GAME_MODES` matches `handlers/missions.rs::valid_game_mode` string-for-string; the 403 message
   matches the handler's author gate.
8. **The T-694 pin fires (claim 11) — proven empirically.** Inserted `placed.min(9999)` into
   `render_shape_section`: `shape_section_invents_no_player_limit` failed naming `.min(`.
   Reverted; tree clean. Scope read from source: `clamp`/`128`/`.min(`/`.max(`/`min_players`
   banned in the fn body only; `min_players` additionally file-wide. Not a nuisance pin.
9. **`is_row_id` duplication (claim 12).** Byte-identical to `eden_top_strip::is_mission_row_id`
   (eden_top_strip.rs:281-287) today. Drift risk stands but is documented on both sides.
10. **mission_editor.rs:8013 sibling surface (claim 14).** The fan-out pin's needles
    (`for id in ids {`, `core.update_slot(id,`, `core.update_slot_position(id,`) survive T-082's
    `slot_half` / `continue`-on-locked reshape; the whole t649 module and the
    `Gate::maybe(is_multi && differs, latch)` pin ran green in the baseline 739.
11. **Free-text `assetId` is safe (claim 5).** An unknown id cannot make a mission unrenderable or
    unloadable: the SoA carries no asset column (renderer indifferent); `flatten` resolves unknown
    ids to the faction-default kit **with** a `KitSubstitutionReport` (not silently, not fatally);
    the validate rule only fires when a catalogue is supplied; `description` is structurally absent
    from the compiled document (`SlotIn` omits it). Free text is a vocabulary superset, as argued;
    T-146 remains the real picker.
12. **Help count (claim 16).** "eighteen" is machine-derived from `SHORTCUTS`' distinct codes,
    which t692's two-way set equality pins to the real arms; manual recount of the arm census also
    gives 18. The Ctrl+X-in-a-text-field case is covered by the top-of-closure
    `in_editable_field()` guard (mission_editor.rs:2035); cutting a slot whose modal is open closes
    the modal through the same path as Delete (consistent; see F-2 for the hide variant).
13. **Attack that found a near-miss but no defect:** `cargo test -p map-engine-core` **without
    features compiles zero of this wave's store tests** (`doc` is feature-gated,
    map-engine-core/Cargo.toml:22 — 139 tests vs 600). Checked the gate: Makefile:183 runs
    `--all-features`, and CI's workspace `cargo test` unifies features, so the gate genuinely
    examined the new code — this is not the reports-success-unexamined BLOCKER, but anyone
    hand-running the crate's tests without `--all-features` gets a vacuous pass.

Falsification attempts that found nothing: forcing the two paste arms to overlap (unsatisfiable);
making `update_slot_object` clobber `role`/`tag` (leaves both, test green); getting a refused
transform write to dirty the mission through either modal path (both gated); getting the multi-edit
latch to re-enable a refused field (`shut ||` short-circuits, pinned); finding a third keydown
binding X or V (none); finding a serde-visible change in dto.rs (comments only); making the shape
dialog show a stale/invented mode after a failed GET (clears to `None`); finding an eighteenth-code
miscount (derived, tied, recounted).
