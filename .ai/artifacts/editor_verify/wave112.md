# Wave 112 adversarial verification — merged main @ 4973de19

Range 1de39175..4973de19 (T-649, T-686, T-692 + orchestrator merge fix). Full native suite
re-run on merged main: **718 passed / 0 failed** (`cargo test -p website-frontend`, run 1).
Working tree untouched; the only file created is this report. Fail-open proofs were run in a
scratch crate (outside the repo) against the checker code extracted verbatim from
`arsenal_rules.rs:999-1505`.

---

## Findings

### MAJOR | apps/website/frontend/src/arsenal_rules.rs:1299-1310, 1257, 1222-1226, 2259-2262 | T-686's "fail closed" claim is false in three forms, and the keyword-pin's schema walk is blind to two of them

**Evidence.** I extracted the checker (lines 999-1505, unmodified except the schema became a
parameter) into a scratch crate and ran four tests; all passed:

1. **`oneOf` swallows refusals** (`check_schema_one_of`, :1299-1310): a branch whose schema
   carries an unimplemented keyword (`{"maxItems": 1}` behind a `$ref`) records a refusal into
   that branch's error list — which is **discarded** when any other branch passes
   (`passing == 1 → return`). Document accepted; the unimplemented keyword never surfaced.
2. **`additionalProperties` in schema form is a silent no-op** (:1257 — only `== false` is
   read). `{"additionalProperties": {"type": "string"}}` + `{"x": 123}` → `Ok(())`. The keyword
   is whitelisted wholesale in `SUPPORTED_SCHEMA_KEYWORDS` (:1022) but only the boolean form is
   implemented — acceptance over a constraint never evaluated, with zero refusals.
3. **`items` in tuple (array) form is silently dropped** (:1222-1226 feeding
   `check_schema_node`, whose `node.as_object()` miss at :1152-1154 returns without recording
   anything). `{"items": [{"type": "string"}]}` + `[123]` → `Ok(())`.
4. **The pin test cannot catch any of the three.** `an_unimplemented_keyword_is_a_refusal_not_a_shrug`
   (:2240) walks the schema but only asserts on nodes where `is_schema` is true — i.e. nodes
   containing **at least one supported keyword** (:2259-2262). A node like `{"maxItems": 1}`
   contains none, so the walk skips it and the pin stays green. I ran the walk logic verbatim
   against all three trap schemas: zero flags. (Control: a mixed node
   `{"type": "object", "maxProperties": 1}` is correctly refused at runtime AND flagged by the
   walk — the guard works where the slice tested it.)

**Impact.** Not live: I read all 101 lines of the shipped
`packages/tbd-schema/schema/loadout-export.schema.json` — every node carries a supported
keyword, `additionalProperties` appears only as `false`, `items` only as an object, no
assertion sits beside a `$ref`. Today the importer examines everything it accepts. But the
slice's central claim — "a keyword the checker does not implement is a refusal, not a skipped
check" — is disproven as a mechanism, and the pin designed to catch the *future schema edit*
that would make it live has a blind spot exactly over the shapes that slip. A `$defs` entry
gaining a bare `{"maxItems": …}` (or an `additionalProperties` sub-schema) ships with every
test green and the importer waving documents through unexamined — this repo's signature defect,
one schema edit away.

**Disposition.** Documented, not fixed (standing instruction). The fix shape, for whoever takes
it: (a) `check_schema_one_of` must propagate *refusal-class* faults from non-passing branches
instead of discarding them; (b) treat non-boolean `additionalProperties` and non-object `items`
as refusals; (c) the pin walk's `is_schema` heuristic should treat any object reachable in a
schema position as a schema node, not keyword-sniff.

### MINOR | apps/website/frontend/src/arsenal.rs:4732 | the strengthened one-commit pin still only catches the literal loop spellings

**Evidence.** `the_import_applies_in_one_commit` asserts `persist(` count == 1 plus a blacklist
`["for ", "while ", "for_each", ".iter()"]`. A multi-commit apply written as
`doc.picks.into_iter().map(|p| persist1(p)).count()`, a bare `loop { … }` with a manual index,
or a recursive helper contains one textual `persist(` and none of the four needles — it passes.
`.into_iter()` does not contain the substring `.iter()`.
**Impact.** None today: the live `apply_import` (arsenal.rs:952-957) is verifiably three signal
writes + one `persist` → one `set_loadout` (editor_ops.rs:1731) → one `after_local_edit`
(:1745). One undo step, re-proved by read. The pin is weaker than its comment claims — it
defeats the shape it was strengthened against, not the class.
**Disposition.** Documented. (Same class as the next finding — the factory's loop-blindness in
count-based pins is now a pattern across two slices.)

### MINOR | apps/website/frontend/src/mission_editor.rs:8007-8011 | T-649's one-tail pin cannot tell "one tail after the loop" from "one tail inside it"

**Evidence.** `multi_edit_commits_fan_out_to_every_selected_id` asserts
`src.matches("after_local_edit()").count() == 1` over each `_multi` fn. An implementation that
moved the single `after_local_edit()` call *inside* `for id in ids { … }` still counts 1 and
passes, while firing N persist/rebind tails.
**Impact.** None today: both `attrs_update_position_multi` (editor_ops.rs:1210-1217) and
`attrs_update_slot_multi` fire the tail outside the loop — verified by read. Claim 10's
one-tail behaviour is true; its pin does not actually enforce it.
**Disposition.** Documented.

### MINOR | apps/website/frontend/src/arsenal.rs:1031 | import refusals drop the row key — two identical compat messages are indistinguishable

**Evidence.** The refusal render maps `refusals.into_iter().map(|e| e.message)` — `RowError.key`
is discarded. `validate_loadout`'s messages (arsenal_rules.rs:386, :394) name the *dependency*
("Requires a Primary weapon pick", "Not compatible with the selected Primary weapon") but not
the failing row. A document with both a stranded optic and a stranded magazine renders two
near-identical lines with no way to tell which row each refers to.
**Impact.** The claim-4 asymmetry is reachable (export refuses on capacity **only** —
arsenal.rs:743-750 — so a stranded-optic loadout downloads fine and is refused on re-import
whenever the compat feed is `Ready`), and when it fires, the author is told what is missing but
not *which slot to fix*. Weakens, does not void, the "refusal tells the author what to do"
claim. The doc-level (schema) faults are fine — they carry the JSON path in the message.
**Disposition.** Documented.

### MINOR | apps/website/frontend/src/eden_help.rs:303-342 | the coverage census only sees the two `match ev.code().as_str()` blocks — and the editor already binds Escape in three places outside them

**Evidence.** `all_bound()` reads exactly `mission_editor.rs` + `mission_history.rs` and, in
each, only the first `match ev.code().as_str()` arm list. But the editor has at least three
more window-level keydown listeners, all matching via `ev.key()`:
- mission_editor.rs:1200-1201 — asset-picker Escape;
- attributes.rs:167-171 — Attributes-modal Escape;
- eden_top_strip.rs:680-696 — menus / Save dialog / **Controls Hint** Escape (T-692's own close path).

None is visible to the census. The claimed property — "a coverage test that cannot silently
miss an unanticipated key" — holds only for arms added to those two matches; a future slice
adding a listener anywhere else (or matching `ev.key()`) ships undocumented with both pins
green. The arm-extraction itself I could not break: multi-line or-patterns, `if`-guards, and
body literals are all handled (the `no '('` honesty floor at :375-378 is real), and the
`_ =>` truncation is safe today because no arm body nests a match.
**Impact.** Live, small: the help table's own Escape row (:172-176) documents only
"Dismiss the ruler / line-of-sight / viewshed measurement" — it omits that Esc also closes the
Controls Hint (which the hint's close button *advertises*, :227), the menus, the Save dialog,
the Attributes modal, and the asset picker. The one key bound outside the census is the one key
whose help entry is incomplete today.
**Disposition.** Documented.

### MINOR | apps/website/frontend/src/eden_help.rs:5 | the shortcut count is 17 on merged main, and T-692's own prose still says sixteen

**Evidence.** Hand-derived from the arm lists: mission_editor.rs:2042-2185 binds 15 codes
(Escape, KeyC, KeyV, KeyA, KeyD, Space, Delete, Backspace, KeyE, KeyR, KeyG, BracketLeft,
BracketRight, Digit1, Digit2); mission_history.rs:493-503 binds 2 (KeyZ, KeyY). Total **17**.
`SHORTCUTS` documents exactly those 17 — the two-direction pins are genuinely satisfied. But
T-692's claim of "16" was true only pre-merge; the 4973de19 merge fix added the KeyA *row*
without updating the module doc's "sixteen `KeyboardEvent.code` values" (:5). The spec's "10"
is stale twice over.
**Impact.** Prose drift only; the checkable artifact (the table) is correct.
**Disposition.** Documented — the orchestrator's own fix repeated the drift class in miniature.

### NIT | line cites drifted across the wave — claim 11 confirmed, plus three more instances

- `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md:244` cites
  `editor_ops.rs:583-585` for the suppress-on-multi guard. The guard is **gone** (T-649
  inverted it; the opener now lives at editor_ops.rs:994-1018), so the cite is not just
  drifted — the row's statement "any multi-selection suppresses it" is now false.
- `arsenal.rs:947` (T-686, this wave) cites `editor_ops.rs:1611` for `set_loadout`'s tail; it
  is at :1745.
- `eden_top_strip.rs:713/:1425/:2064` cite `editor_ops.rs:1055` for `refresh_docks`; it is at
  :2076. `:714/:1423` cite `mission_history.rs:452` for `refresh_signals`; it is at :438.
- `editor_ops.rs:1236` (T-648's doc comment) still says the Attributes modal "suppresses on a
  multi-selection" — inverted by T-649 in this same wave.

Yes: other briefs are citing drifted lines; anything derived from gap_analysis ATTR-OPEN-001 is
now describing inverted behaviour.

### NIT | apps/website/frontend/src/eden_help.rs:166 | redo chord under-documents Cmd+Y

mission_history's guard is `(ctrl || meta) && !alt` (:490), so **Cmd+Y** redoes on Mac; the row
says "Ctrl + Y or Ctrl/Cmd + Shift + Z".

### NIT | apps/website/frontend/src/editor_ops.rs:1826-1842 | mixed slot+vehicle selections make the multi-edit wording slightly overclaim

`attrs_multi_ids` filters the selection to slot ids (`soa.ids`), so with 2 slots + 3 vehicles
selected the header says "2 entities selected" (5 are) and the banner's "overwrite that field
on every selected entity" (attributes.rs:295-297) will not touch the vehicles. Ctrl+A selects
vehicles too (`view_ids_with_vehicles`), so this combination is one keystroke away. Defensible
(vehicles have no Attributes rows) but the wording says "entity" where it means "slot".

### Cross-slice observations (item 13) — checked, nothing new broken

- **Ctrl+A with the Attributes/Arsenal modal open:** fires (only `in_editable_field()` guards
  it), replaces the selection under the modal; the modal re-reads `attrs_multi_ids` per render
  (attributes.rs:194-198) and flips to multi-edit live, banner included; opt-in latches reset
  only on `attrs_open` change, so they start unticked. Coherent, disclosed. If the modal's slot
  is off-screen, Ctrl+A drops it from the selection and the modal stays single-edit on it —
  odd-looking but honest.
- **Help overlay swallowing keys:** it binds none; backdrop is `pointer-events-none`; window
  keydowns keep firing with the card open. Verified by read of `ControlsHint`.
- **Escape ordering:** there is none. One Esc press fans out to four independent window
  listeners and simultaneously (a) steps the measurement dismissal, (b) closes menus + Save
  dialog + Controls Hint, (c) closes the Attributes modal, (d) closes the asset picker. This is
  the known, explicitly-deferred T-726 pile-up (mission_editor.rs:2015, eden_top_strip.rs:689
  both cite it); T-692 deliberately rode an existing listener rather than adding a fifth. This
  wave made the pile denser, not different. Not re-litigated per standing instruction.

### Claim 8 assessment (T-649 "source pins were forced")

Overstated. The constraint is real *as the frontend is gated*: `select_tool` is a
`#[cfg(target_arch = "wasm32")]` module (per select_tool.rs:798 / main.rs), so `website-frontend`'s
native tests cannot call `select_all_in_view`. But the claim that `OrthoCamera` and `SlotSoa`
"are not constructible from a native cargo test" is wrong as stated: both live in
`map-engine-core`, which compiles natively and has native tests
(`crates/map-engine-core/tests/camera_props.rs`), and `view_ids_with_vehicles`
(select_tool.rs:378-389) is pure over `(&OrthoCamera, &SlotSoa, &[(String,f64,f64)])` — a
behavioural test was writable in map-engine-core, or by widening the module cfg (arguably
outside the slice's `owns`). And the pins constrain shape, not dataflow: they require
`cam.size_px()`, `cam.unproject_xy(0.0, 0.0)` and the call
`marquee_ids_with_vehicles(cam, soa, vehicle_points,` to appear, but not the argument order
after `vehicle_points` — an implementation passing `(w, h, tl[0], tl[1])` (world/px corners
swapped) satisfies every pin. I therefore read the live implementation instead of trusting the
pins: select_tool.rs:383-388 and editor_ops.rs:1052-1101 are correct (top-left unprojected as
the world corner, `size_px` as the pixel corner, frozen camera, selection-only refresh, no
history step, engine tint fed slots-only).

---

## Is `main` safe to build the next wave on — **yes.**

No blocker: every live code path attacked behaves as shipped-and-claimed; the MAJOR is a latent
mechanism defect plus a blind pin, armed only by a future schema edit.

---

## Verified-clean register — re-proved, not taken on trust

- **Schema pin byte-equality (claim 2):** `include_str!` (arsenal_rules.rs:996-997) and the
  test's `fs::read` (arsenal.rs:4421-4436) resolve to the *same file*, so equality holds by
  construction; the test additionally pins the `$id` and would catch a repointed include path.
  It cannot catch a plain edit to the shipped file (rebuild refreshes both sides) — the real
  guarantee is `include_str!` itself, which is sound.
- **Shipped schema is fully within the implemented subset:** every node of all 101 lines
  carries ≥1 supported keyword, `additionalProperties` only as `false`, `items` only as a
  single object schema, `$ref` never beside an assertion, both `oneOf` branches
  const-discriminated on `loadoutVersion`. Today's importer examines everything it accepts.
- **Import is one undo step (claim 3):** live path read end-to-end — `apply_import` = three
  signal writes + one `persist` → one `editor_ops::set_loadout` → one `after_local_edit`
  (editor_ops.rs:1731-1747). Nothing is applied on `Err` (picks are only built after the schema
  gate; refusal arm writes signals only).
- **The anchored-pattern matcher fails closed:** attacked with unanchored, alternation, `.`,
  escapes, unterminated/empty classes, inverted `{3,2}`, unparseable quantifiers, >512-char
  input, >8 terms — all `None` (refuse) by construction (:1385-1481); backtracking is bounded;
  the shipped wear pattern `^[a-zA-Z][a-zA-Z0-9_]{0,63}$` evaluates correctly incl. the 64/65
  boundary (in-repo test :2157 re-read, logic re-derived).
- **`$ref` handling:** sibling-assertion refusal (:1128-1143), remote/unresolvable-pointer
  refusal (:1145-1150) — both real. (A cyclic `$ref` would recurse to stack overflow: crash,
  not accept; shipped schema is acyclic.)
- **T-504 asymmetry (claim 4):** unworn-container check genuinely excluded from the import
  gate (arsenal.rs:778-784) and only there; capacity/compat/attachments genuinely refuse.
- **KeyA merge fix (claim 12):** arm is `"KeyA" if modk && !ev.alt_key() && !ev.shift_key()`
  (mission_editor.rs:2095) — chord "Ctrl/Cmd + A" is exact under the file's own convention,
  action matches the call, row files under Selection beside KeyC/KeyV, and mission_history's
  keydown cannot claim KeyA (guard requires modk, arms are KeyZ/KeyY only; census pin re-read).
- **Chrome gating by mount site (claim 6):** chain verified by read, not by the pin —
  mission_editor.rs:3724-3726 `(!chrome_hidden.get()).then(` → `eden_chrome::TopCommandStrip`
  (re-export, eden_chrome.rs:35) → `ControlsHint` mounted inside the strip
  (eden_top_strip.rs:1016). Backspace unmounts it; `HINT_SHOWN` latch restores it on re-show.
  The eden_help pin's `strip.contains("ControlsHint")` alone would be satisfiable by the
  `MenuAction::ControlsHint` enum variant, but eden_top_strip.rs:2284 pins the literal mount
  expression `ControlsHint open=hint_open`, closing that gap — the pin pair is not tautological.
- **Shortcut census (claim 7):** independently re-derived — 17 codes across the two keydowns on
  merged main, table documents exactly those 17, both difference-pins re-checked by hand.
- **T-649 guard inversion + selection preservation (claim 9):** both entry points route through
  `open_attrs_modal`; `sel.len() > 1 && sel.contains(&id)` preserves the multi-set; the Arsenal
  honesty banner (attributes.rs:337-346) renders unconditionally on `is_multi` in the Arsenal
  tab arm, names the one slot it edits, and is backed by a pin on the live view fn — the
  context-menu rows now open something on multi-selection, so the T-716 class is closed here.
- **One tail / N undo steps (claim 10):** `after_local_edit` outside the loop in both `_multi`
  fns (read); `capture_timeout_millis = 0` confirmed at
  crates/map-engine-core/src/doc/store.rs:242, so N core transactions = N undo steps exactly as
  disclosed.
- **Select-all scoping:** viewport-rect through the marquee primitive, empty on non-finite
  unproject, never a `soa.ids` dump, selection-only refresh with no history step
  (select_tool.rs:378-389, editor_ops.rs:1052-1101 — read, since the pins alone don't prove it).
- **Full native suite on merged main: 718/0**, first run, no flakes.

Falsification attempts that produced nothing (so nobody needs to re-audit them): partial
application of a refused import; a schema/regex input the matcher passes instead of refusing;
double-claimed keys across the two keydowns; a phantom or missing help row on merged main;
chrome-gate bypass for the hint; multi-edit stamping an un-opted field (the `None`-column
discipline and per-field latches held under read); checkbox latches surviving the wrong event
(they reset on `attrs_open`, not `doc_tick` — verified in attributes.rs:179-182).
