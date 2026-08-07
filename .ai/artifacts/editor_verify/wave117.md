# Editor-factory wave 117 — adversarial verification

Range `e0c87cfe..81258bb5` (T-084 `5d655b7f`, T-671 `e0f9cd31`/merge `b03892e3`, T-672 `84a33a84`/`81258bb5`).
Method: verbatim extraction of the T-084 regex+glob engine (`asset_catalog.rs:594-1015`) into a native
harness run on a **1 MiB thread** (the conventional wasm32 stack), plus ranged reads of every claimed
mechanism and its pins across the four heavily-pinned files. Main was left untouched; nothing committed.

---

## Findings

### MAJOR | asset_catalog.rs:693-991 (RX_BUDGET / RxCtx) | the 200k step budget bounds STEPS, not STACK DEPTH — deep patterns abort the wasm runtime before the budget ever engages

**Evidence.** I compiled the engine verbatim and ran it on a 1 MiB thread (the standard Trunk/wasm-ld
stack). Every one of these *aborts the process with a stack overflow*, it does not return "no match":

| input | overflow threshold (1 MiB) |
|---|---|
| `(((…)))` balanced nesting | between **2500 and 3000** parens |
| `^^^…` (Start-node CPS chain) | between **20000 and 25000** carets |
| `.?.?…` | between **3000 and 4000** repeats (~7000-char pattern) |
| `(x+x+)+y` vs `x…` | between **2500 and 3000** haystack chars |

The cause is structural, not a tuning miss: `RxCtx::node`/`repeat`/`seq`/`alt` recurse through boxed
`RxCont` continuations, one native frame per matcher step, and `step()` only decrements the 200k budget
— it never bounds recursion depth. 200k frames overflow any realistic stack (1 MiB / ~frame-size ≈ a few
thousand) long before the budget hits zero. So the field-note claim at :694-697 ("exceeding it returns
'no match' instead of hanging the tab") and brief point 1 ("there is NO input that escapes the budget")
are **false for deep input** — the failure is a wasm trap, which kills the Leptos runtime and any unsaved
editor placements.

**Impact — bounded, and I want to be precise about it.** None of these are reachable from *catalogue
data*: the matcher's depth for a fixed short haystack (resource names are ~100-150 chars) stays small, so
a pathological pattern like `(x+x+)+y` is safe against every real row (it needed ~2700 haystack chars to
blow). Every reachable vector requires the **operator to type or paste a multi-thousand-character regex**
into the search box — a self-inflicted local DoS, recoverable by reload, no server/data-at-rest exposure,
no silent wrong answer. That is why this is MAJOR (a stated safety property is demonstrably false and the
failure mode aborts the runtime and loses unsaved work), **not** BLOCKER (not spontaneously reachable; a
2500-char pasted pattern is the trigger, and the catalogue cannot supply one).

**Disposition.** Left as-is per standing instruction. A real fix is a depth cap (or an explicit worklist
instead of native recursion), not a bigger budget — the budget is the wrong axis. Nobody needs to
re-audit the *glob* path for this; it is iterative and proven below.

### MINOR | asset_catalog.rs:629 vs 639 (GlobPattern) | case-folding asymmetry for non-ASCII literals

**Evidence.** `parse` lowercases pattern literals with `to_ascii_lowercase()` (:629) while `matches`
lowercases the haystack with full `to_lowercase()` (:639). So `CAFÉ*` does **not** match `café_x`
(probe: got=false, reference=true). The regex path is unaffected — it compares through `eq_ci` (full
`to_lowercase`), and `/café/` vs `CAFÉ` correctly matches.

**Impact.** Cosmetic in this product: Reforger classnames/labels are ASCII. It is also a *refusal*, not
a false positive, so it can never surface a wrong row — only miss one that a non-ASCII uppercase glob
literal should have caught.

**Disposition.** Left as-is. Note, not a gate concern.

### MINOR | eden_settings.rs:399-409 (mirror_briefing_into_document) | clearing a briefing leaves the EXPORT envelope stale for the rest of the session (disclosed gap, point 15)

**Evidence.** `mirror_briefing_into_document` returns early on `briefing.trim().is_empty()` (:400), and
`map-engine-core` exposes no other writer of `meta.briefing` (only `apply_row_meta`, which also treats
blank as "not supplied", store.rs:3060-3064). So after an operator *clears* a briefing: the DB column is
PATCHed to `""` (every DB-backed surface — library card, dossier, approval queue — updates correctly),
but the live doc's `meta.briefing` keeps the old text, and `compile_export` reads `meta.briefing`. A same-
session Export therefore ships the deleted briefing.

**Impact.** Narrow and honestly disclosed by the author (the field note spells out exactly this). Confined
to the compiled `.mission` envelope, same-session, until the doc is rehydrated. The guard it comes from
(blank = "not supplied") is itself defensible — it is what stops boot hydrate wiping a good briefing.

**Disposition.** Left as-is. Rated MINOR; it would edge to MAJOR only if "clear a briefing then export
without reload" is a real operator flow, since the export ships un-authored content — worth a follow-up
ticket to give `apply_row_meta` an explicit clear, but not a wave gate.

### NIT | mission.schema.json:286 | `formation` enum description is now stale

The `$defs/group.formation` description still says "the sole frontend hit for `formation` is prose in a
comment … no mod reader", but T-672 added `force_to_formation` + the Transform submenu, which now consume
these tokens. The enum *values* are correct and agree everywhere (see register); only the prose lags.
Doc-owned by Cursor per CLAUDE.md; not code.

---

## Is `main` safe to build the next wave on? — **YES.**

No BLOCKER. No pre-existing pin was weakened or deleted anywhere in the range. The one MAJOR is a
non-spontaneous, operator-must-paste-a-huge-pattern local self-DoS that never touches catalogue data,
server state, or data at rest, and recovers on reload — it does not put `main` or authored work at risk
under any normal use. The two MINORs and the NIT are cosmetic / disclosed / doc-lag.

---

## Verified-clean register — re-proved, not taken on trust

**T-084 regex engine**
- **Correctness (37 hand-built + doc cases, 0 wrong answers).** Anchors `^`/`$`, alternation with
  backtracking across a group boundary (`(ab|a)b` on both `ab` and `abb`), classes incl. `[]a]` POSIX,
  `[a-]` trailing dash, `[A-F]` uppercase range vs lowercased hay, `\d\w\s` inside and outside classes,
  greedy `*+?`, dot over multibyte, `café`/`CAFÉ` CI. All correct. `\n` → literal `n` is intended
  ("anything else escapes to itself", :826) — not a bug.
- **`{n,m}` is a LITERAL, both forms.** `/^{26A9/` and `/^\{26A9/` both match the GUID head; `/a{2,3}/`
  matches the literal `a{2,3}` and does **not** repeat (`aaa` → no match); `class:/us_[a-z]{2}\.et$/`
  correctly misses. Pinned in-repo at asset_catalog.rs:1638-1642.
- **Dangling-quantifier refusal.** `/*foo/`, `/+x/`, `/?x/` → `Invalid` (not a silent literal `*`).
  Unbalanced `(`/`[` and trailing `)` → `Invalid`. `//` → `Pending`. Lone `/` → literal, not a regex.
- **Glob correctness — 2,000,000 randomized cases vs a memoized reference matcher, 0 mismatches.**
- **Glob is non-recursive / cannot blow the stack (brief point 4).** `*` at both ends, 2000 stars, and
  `a*`×2000 worst-case all terminate; slowest was 234 ms (bounded O(p·h)), zero stack growth. Confirmed
  by construction (single-star backtrack, no `matches` recursion).
- **Multibyte does not panic.** `strip_prefix_ci` is `is_char_boundary`-guarded (:1025); Rx and Glob work
  over `Vec<char>` (char indices, no byte-boundary slice); `regex_body` slices only on the ASCII `/`.
  No index/unwrap/boundary panic found (the wave-105 class of BLOCKER).

**T-084 the rest**
- Classname-tail decision (point 5): `class:Character_US_Rifleman` and partial `class:character_us_ri`
  HIT the real GUID-headed id; T-646's `class:{26A9…}Prefabs` GUID-prefix still hits (OR, not replace);
  **`class:Rifleman` still matches nothing** — asserted at :1388 and again at :1480. Survives.
- SearchQuery reshape (point 6): diff of `#[cfg(test)]` shows **no test function removed** and only one
  assert line changed (the forward-contract test, which was *strengthened* from one row to a per-row
  loop). Every T-646 case (b_soldier, CI operator, empty operand, `classy`/`first class:` = label) is
  re-expressed through `{field, pattern}` intact.
- RIGHT-SEARCH-005 deferral override (point 7): **correct.** The `deferred` marker is agent-authored
  (the same sweep row 494 says "promote"), which the HARD GATE explicitly does not honor; all four ids
  are T-084's declared scope. Not 290 unasked lines.
- `mod:` = depth-0 (point 8): the shipped registry fixture has **zero `addon` fields** (grep), so
  `RegistryItem.addon` is `None` everywhere — depth-0 folder is the only coherent reading. Test at :1496.

**T-672 connection graph**
- Never-compiles (point 9): `connections` is not a key of `mission::flatten::EditorPayload`, so serde
  drops it — the proven T-651 absence mechanism. The leak probe (store.rs:13144-13173) is
  **non-tautological**: it routes the same `triggerOwner`/`connections` tokens through `entities[]`
  (which *is* emitted) and asserts they DO appear in the mod bytes, giving step 3's absence assertion
  real detection power. Persist round-trip and delete-clears-the-wire both pinned.
- CHECK rules (point 10): CONN-KIND / SELF / DANGLING / DUPLICATE all fire off distinct branches; a
  clean graph yields an empty `out` (silent). The `sync` normalisation (endpoints sorted) makes a
  reversed re-draw a real DUPLICATE.
- Cycle detector (point 10): iterative 3-colour DFS over directed kinds only, `sync` excluded. Traced by
  hand — the re-push of `(node, edge_idx+1)` before descending keeps grey = active-stack-path exactly, so
  cross/forward edges (diamonds) do **not** produce a false CONN-CYCLE, and a real back edge into a grey
  ancestor is always caught regardless of root sort order or entering mid-cycle. Self-loops excluded
  (already CONN-SELF). Could neither make it miss a real cycle nor invent a false one.
- Cascade on delete (point 11): `delete_selection` calls `remove_connections_touching` for every selected
  id **before** `remove_slots` (editor_ops.rs:491-494); it drops every edge with the id at either end, so
  no dangling edge can survive. The extra undo steps it discloses add granularity — they do not lose work.
- T-664 pins (point 12): `unblocked_by()` returns `None` for `Connect` and `Transform` (context_menu.rs
  :291); `Connections...` (`ShowConnections`) is on **both** takes (:502 EmptyGround, :522 OnEntity);
  Connect/Transform are enabled `open_parent`s.
- Gesture test (point 13): `the_two_act_connect_gesture_shows_one_face_at_a_time` is literally an event
  sequence — ACT 1 (unarmed source → three kinds) then ACT 2 (armed target → Complete/Cancel), asserting
  one face at a time — not a source pin.

**T-671 mission presentation**
- Three bugs reproduced on base `e0c87cfe` and fixed:
  (a) base `create_mission_dialog.rs` had **0** `briefing` references though `POST /missions` binds
  `input.briefing` into the INSERT (missions.rs:626); now the body carries it, with a `live_source` pin.
  (b) `compile_export` reads `meta.briefing` (store.rs:3061), whose only writer was boot hydrate; an
  in-session PATCH updated the DB column but not the doc — now `mirror_briefing_into_document` writes it
  on PATCH success.
  (c) base Escape handler was `open.set(false)` with **no blur** (verified in `git show e0c87cfe`); now
  `blur_focused_control()` runs first (:649) so the `change` commit fires before close.
- No per-keystroke PATCH (point 16): `set_presentation` commits on `change` (blur/Enter), early-returns on
  a no-op edit, and reverts to `previous` on refusal (:603). Same shape as the T-694 game-mode select.
- doc_handle (point 17): `mission_history::doc_handle()` is `pub` — a legitimate path; the mirror only
  runs after a successful server PATCH, only for non-blank briefing, and `apply_row_meta("","",None,None,
  Some(b))` writes nothing but `meta.briefing`. No unexpected dirtying.

**Cross-slice**
- `draw_order.rs` UNTOUCHED this wave (point 19, confirmed by `git diff --stat`); `LaneRole` keeps
  `MissionZones` directly above `SquadLinks` (:63/:65) with ordering intact, leaving room for T-760's
  one-line insert. T-672 correctly added no render lane (connections have no map glyph this slice).
- Formation vocabulary (point 20): schema `$defs/group.formation.enum` (9 tokens) == `formation_offsets`
  tokens (store.rs) == `FormationKind::ALL` menu tokens — the menu test reads the schema at compile time
  and asserts equality (context_menu.rs:1613). All three agree. T-084 touched no schema.

**Falsification attempts that found nothing:** hang the regex via nested quantifiers / alternation
blowup (terminates in <1 ms, budget works for *bounded-depth* input); glob mismatch over 2M cases;
glob stack overflow; multibyte panic anywhere in parse/match/strip_prefix; a wrong (not just missing)
regex answer; a false or missed cycle; a dangling edge surviving delete; a weakened or deleted pre-existing
pin in any of the four pinned files.
