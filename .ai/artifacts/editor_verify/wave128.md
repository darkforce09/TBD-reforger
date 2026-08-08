# Wave 128 — adversarial verification (merged main @ 0d98a6e2)

Verifier: Fable 5, 2026-08-08. Scope: T-764 (04f2087c), T-735 (65244f80), T-774 (0d98a6e2) over
base 4d343b0d. Only the three claimed files changed in the wave (checked via
`git diff --stat 4d343b0d..0d98a6e2`: asset_catalog.rs, arsenal_rules.rs, eden_help.rs — nothing
else). Main left untouched: `git status --porcelain` empty at exit; this file is the one write.

**Execution environment (matters for trust):** this container ships no C toolchain and its glibc
(Debian 2.36) cannot run host-linked build scripts. All builds went through a host-gcc shim
(`/run/host` toolchain under the host loader, plus a local stub that unversions Rust std's weak
`pidfd_spawnp`/`pidfd_getpid` refs). Every green below was EXECUTED here, none inferred.

**Harness-lie discipline (T-742):** everything ran in a private
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-verify128` (worktree attacks in
`…-verify128-wt`), built from scratch. Reconciliation: `cargo test -p website-frontend -- --list`
= **919 tests**; run = **919 passed / 0 failed** — totals AGREE, and all six new tests are present
by name in the listed binary (2× asset_catalog T-764, 4× arsenal_rules T-735; T-774 adds none, as
claimed). `cargo test -p map-engine-core --all-features` on MAIN: **625 passed** (+5/+5/+3 in the
smaller binaries), `dem::peaks::tests::everon_peaks_max_above_350` **ok** — the LFS failure is
worktree-only, as the run-log note says. **Both private target dirs were deleted after this report
was written** (`tbd-target-verify128`, `tbd-target-verify128-wt`); nothing was ever written to the
shared dir.

---

## FINDINGS

### 1. MAJOR | arsenal_rules.rs:1411 (doc walk) + :1210 (audit) | a chained `$ref` — and a `$ref` cycle — fail OPEN: the document is ACCEPTED with the target's assertions silently dropped

**Evidence.** `schema_deref` resolves exactly ONE pointer hop. `check_schema_node` derefs once at
:1411 and never re-derefs; if the target is itself `{"$ref": …}`, the only keyword on the node is
`$ref` (supported, so no unknown-keyword refusal), no assertion fires, and the walk returns having
checked NOTHING. `audit_schema_support` (:1210) meanwhile follows chains recursively and audits the
final target clean — so the document-independent audit passes a schema the document walk cannot
evaluate, which is precisely the gap the audit exists to close. Proven executable: in a throwaway
worktree at HEAD (main untouched, worktree removed after) I appended five adversarial tests;
`verify128_chained_ref_must_not_fail_open` and
`verify128_ref_cycle_terminates_and_does_not_fail_open` both **FAILED** —
`{"modpackId": 123}` validated Ok against a schema routing `modpackId → $defs/hop → $defs/strict
{"type":"string","minLength":3}`, and likewise through a `$defs/a ⇄ $defs/b` cycle. The single-hop
control (`verify128_single_hop_control`) passes, so the edit shape is sound and the defect is the
chain, not my harness.

**Impact.** Not exploitable through the SHIPPED schema — I verified every `$ref` in
`loadout-export.schema.json` targets a concrete subschema (single hop). But T-735's shipped claim
is "the schema says something it cannot evaluate ⇒ refusal, for every document"; a one-line `$defs`
refactor (aliasing a def to another def — an entirely normal schema edit) reopens silent
acceptance, and every pin stays green. This is the ticket's own definition of the defect it claims
to have designed out.

**Disposition — fix shape.** Minimal, in the ticket's own idiom: in `audit_schema_support`'s `$ref`
arm (arsenal_rules.rs:1210-1213), after resolving `target`, refuse when
`target.get("$ref").is_some()` — "`$ref` whose target is itself a `$ref` is a form this importer
does not implement". Document-independent, fires for every document, two lines. (Alternative:
loop the deref in `check_schema_node` with a visited set and refuse on cycle — more code, implements
rather than refuses; either closes it. If the loop is chosen, :1210 must still refuse cycles.)
Regression tests: the two verify128 tests above, verbatim.

### 2. MINOR | asset_catalog.rs:730 | RX_MAX_PATTERN's safety derivation is wrong by 2×: "512 chars caps parser nesting at ~256 levels" is false — 512 UNBALANCED `(` recurse 512 parser levels

**Evidence.** The ~256 figure assumes every nesting level costs `(` + `)`. `RxParser::atom`
recurses into `alt` on `(` alone (asset_catalog.rs:847-849); the missing `)` is only discovered on
the way back out. So the worst input inside the length cap is `"(".repeat(512)` — 512 levels, not
256. Rig measurement (engine lines 699-1102 extracted verbatim, native debug = the most
frame-hungry build, 1 MiB thread, one subprocess per probe): raw `RxParser` on `(`×N returns
cleanly at N=850 and **aborts at N=900** — so the claimed "~3x under the abort floor" is actually
**~1.7x** for the real worst case. The shipped path itself is safe: `Rx::parse("("×512)` returned
cleanly (None) on a 1 MiB thread in both debug and release.

**Impact.** No live defect — measured margin exists. But this comment is the load-bearing safety
argument for the constant: anyone raising RX_MAX_PATTERN "safely under the documented 3x" (to,
say, 768) walks straight into the debug-build abort window (768 > my 850-floor only barely; a
frame-fatter build aborts). A wrong number in exactly the place future maintainers will trust it.

**Disposition — fix shape.** Rewrite asset_catalog.rs:729-731 to state: worst case is 512 levels
(unbalanced `(`), measured native-debug abort floor between 850 and 900 levels on 1 MiB, margin
~1.7x; do not raise RX_MAX_PATTERN without re-measuring. And add the unbalanced vector to
`deep_regex_input_refuses_instead_of_trapping_the_wasm_stack` (its current vectors are balanced or
over-length; nothing exercises unbalanced-`(` at exactly the cap):
`assert!(Rx::parse(&"(".repeat(512)).is_none());` inside the existing `on_a_wasm_sized_stack` body.

### 3. NIT | arsenal_rules.rs:1322 | comment claims "`false`/`true` are the two boolean schemas, handled by the walk" — true for `additionalProperties`, false for `items`

**Evidence.** The form check at :1249-1251 refuses non-object `items` outright (message text at
:1258), so a boolean `items` never reaches any walk; only boolean `additionalProperties` is
"handled by the walk" (:1560-1575). Behaviour is fail-closed either way; the comment misstates
which mechanism catches which keyword, in the module whose whole subject is exactly that
distinction.

**Disposition.** Reword the comment at :1322-1323: booleans are handled by the walk for
`additionalProperties` and refused by the form check for `items`.

---

## The T-774-reported flake (asset_catalog honest_catalogue test) — SAFE TO KEEP; the observed failure was a foreign binary, not load

- The failing assertion ("subject must exceed the real worst case") is `hay.chars().count() > 95`
  over a **fixed 103-char literal** (rig-asserted == 103). With the shipped source this assertion
  is unfalsifiable; a run in which it fired was executing a binary built from OTHER source — the
  exact T-742 shared-target shape, reported under exactly the conditions (parallel worktree load on
  the shared dir) T-742 produces.
- Stack overflow is deterministic in stack USE, not in scheduling; load cannot change frame sizes.
  Measured: the deepest anything in this test reaches is ~256 depth units (nest-255 shape) ≈
  ~135 KB of the 1 MiB thread. The capped tests peak at 400 units; worst shape I could build
  (250 nested groups + `x+x+` over 3000 chars) used **249,632 bytes** at the cap — ~4x headroom in
  the fattest-frame build.
- Empirical: **40/40 green** runs of both T-764 tests under six concurrent full-suite processes
  (provenance-verified private-dir binary), on top of the reporter's 6/6 isolated and the
  orchestrator's 3/3 idle.
- Abort risk: to abort, a pattern must reach ~1 MiB of matcher frames; the depth cap turns every
  such input away at ~250 KB. I could not construct a capped input that aborts (shapes tried:
  `^`×512, `a`+`?`×511, `.?`×256, `(x+x+)+y`/3000, `((x+)+x+)+y`/3000, `(.*|.*)*y`/3000,
  250-deep group nest + posessive tail; all returned cleanly, all `depth_capped=true` where
  expected). Keep the test; no change needed. The only standing requirement is the one already in
  force: never run the suite on the shared target dir.

## T-764 — remaining claims

- **The bound (claim 1): holds, with more margin than documented.** At the cap, per-depth-unit
  stack cost measured 370-624 bytes/unit across seven shapes (native debug); worst total 249 KB of
  1 MiB. The doc's ~1.3 KB/frame is conservative relative to this toolchain. No capped pattern
  aborts. Answers stay correct where a cap trips mid-search (`a?…` still matched=true).
- **Parser bounded (claim 2): yes empirically, no as-documented** — finding 2. `Rx::parse` of every
  ≤512-char pattern I could build returns; the derivation underpinning the constant is wrong 2×.
- **No answer-narrowing (claim 4):** suite green includes the pre-existing correctness cases; rig
  spot-checks on the realistic 103-char catalogue subject: 8 patterns (incl. `.*.*.*.*rifleman.*`,
  anchored GUID idiom, alternations, classes) all answer correctly with `depth_capped=false`. The
  2,000,000-case glob figure in the brief is the wave-117 verifier's rig, not a suite test; I did
  not re-run it (globs are untouched by this diff — the depth cap lives in Rx, `GlobPattern` has no
  recursion into it).

## T-735 — remaining claims

- **Tuple-form `items` refusal (claim 6): right call, correctly wired.** All three `items` in the
  shipped schema are single-subschema objects (verified by walking the JSON). The refusal fires at
  the keyword (empty-array documents included — test pins it) and reaches the operator through
  `try_import` → `RowError` rows (arsenal.rs:765-773), same channel as every other import fault.
- **Scope of the 618-line diff (claim 7):** the growth is the audit pass + refusal/fault split +
  tests; I found no behavioural regression in it (919 green includes every pre-existing arsenal
  test). The two-pass split's "audit failure returns without a document verdict" is fail-closed by
  construction and unreachable with the shipped (clean) schema; the comment at :1131-1136 argues
  the UX honestly. The REAL gap in the audit is finding 1, not the split.
- Attacks that did NOT break it: unknown keyword in unreferenced `$defs` (refused), unknown keyword
  under an `additionalProperties` subschema (refused — my verify128 test), boolean/array in
  `properties` value position (refused), `patternProperties` bad regex on document-unreached keys
  (refused via audit), `{"$ref": 5}` (refused via `as_str()?`), `$ref` with siblings (refused),
  `not`/`if`/`then`/`else`/`allOf`/`anyOf`/`prefixItems` (all outside SUPPORTED_SCHEMA_KEYWORDS ⇒
  structural unknown-keyword refusal at any schema position the audit visits — and the audit
  visits every position reachable through the five descent channels; the only positions it cannot
  see are behind the `$ref`-chain hole of finding 1).

## T-774 — the numbers, re-derived independently

Method: my own extractor (Python: comment-stripper, brace-balancer, arm-head regex — written blind
to `keymap_census`'s slicing rules), plus manual reads of every non-obvious listener body.

- **13 listeners** ✓ — per file: mission_editor 4 (three `window_event_listener` at :1200/:1319/
  :1554 + the `Closure` keydown at :2537 wired to `window` at :2741), mission_history 1,
  attributes 1, eden_top_strip 1, context_menu 1, eden_settings 3, faction_manager 1,
  orbat_manager 1.
- **34 bindings** ✓ (17+3+1+1+4+3+1+1+1+1+1: editor keydown 17 incl. two guarded `KeyV` arms at
  :2603/:2625 — real, verified by eye; history 3 incl. `KeyZ`×2 at :494/:497). Pre-widening total
  **32** ✓ — so T-703's "39" was wrong and the wave-119 verifier's 32 was right, exactly as the
  ticket says. **34 is correct.**
- **12 listeners claim Escape** ✓ (all but mission_history). **21 distinct codes** ✓. **8 modules**
  ✓. "Eleven more" beyond the two `ev.code()` matches ✓ (2+11=13).
- **No 14th listener.** Swept the entire frontend for every idiom: both census heads, bare
  `add_event_listener_with_callback("keydown")` (only mission_editor:2741 and mission_history:507,
  both censused), gloo `EventListener::new`, `window_event_listener_untyped`, `keypress`/element
  `on:keydown` (none window-level). Remaining window keydowns: ui.rs:486/:561 (Dialog/Sheet —
  scope-excluded, and genuinely `modal_stack::is_topmost_open`-gated, verified), layout.rs:94/:145
  (nav — excluded), and missions.rs:215 — which is inside `MissionLibraryPage` (fn at :132), a
  different route; `mission_editor.rs` references nothing from `missions::` (grep). The
  faction/orbat mounts are real and live (mission_editor.rs:4439/:4442 — the corrected cites), both
  Escape-only and `open.get_untracked()`-gated, matching the module doc word for word.
- The widened prose pin (header + census doc block, whitespace-flattened) passed in my
  reconciled binary; each pinned number above is independently TRUE, not merely pinned.

## Cross-cutting

- **z-rule (wave 127):** zero occurrences of `update_slot_position` / `move_entities_and_vehicles`
  in any of the three wave files. Clean.
- **Line cites:** mission_editor mounts verified at :4439/:4442 as the fix says; all other cites I
  relied on were re-derived by grep, not trusted.
- **everon_peaks on MAIN: passes** (see header). The worktree-only LFS note stands.

---

## Is `main` safe to build the next wave on?

**Yes** — with finding 1 queued for the no-deferral fix pass: it is a latent fail-open behind a
schema edit that has not happened, not a live misbehaviour; nothing in the wave gate lied, and both
suites reconcile list-vs-run in a private dir.

## VERIFIED-CLEAN REGISTER (re-proved, not trusted)

1. Suite integrity: 919 `--list` == 919 run, fresh private-dir build, all six new tests present by
   name; map-engine-core 625+13 green on main incl. the LFS-sensitive peaks test.
2. RX_MAX_DEPTH=400 is a safe matcher bound on 1 MiB: seven adversarial shapes at the cap, worst
   249 KB, zero aborts, correct answers, `depth_capped` latching exactly where claimed.
3. `Rx::parse` of the worst ≤512-char input (unbalanced `(`×512) returns cleanly on 1 MiB in debug
   and release — the shipped parser gate holds (even though its documented derivation doesn't;
   finding 2).
4. The honest-catalogue test is deterministic and load-immune: 40/40 under 6-way suite load; the
   flaked assertion is unfalsifiable from shipped source (103-char literal vs >95) ⇒ the one
   observed failure was a foreign shared-dir binary (T-742), not this test.
5. The depth cap does not narrow honest answers: 8 realistic patterns over the real worst-case
   subject, correct and uncapped, debug + release.
6. T-735 fail-closed holds against: unreferenced `$defs` traps, losing-oneOf-branch refusals
   (reachable and unreachable), schema-form `additionalProperties` (asserted, satisfied, `true`,
   `false`), tuple/boolean/array `items`, boolean/array `properties` values, invalid
   `patternProperties` regexes, `$ref` siblings, non-string `$ref` — every one refuses or faults.
   The one exception it does NOT hold against is finding 1 ($ref chains/cycles), proven by two
   failing adversarial tests at HEAD.
7. Shipped loadout schema: every `items` single-form, every `$ref` single-hop, no keyword outside
   SUPPORTED_SCHEMA_KEYWORDS (walked the JSON myself); refusals surface as import RowErrors.
8. T-774 census: 13/34/32/12/21/8 all re-derived by an independent extractor + manual arm reads;
   no 14th listener under any registration idiom frontend-wide; ui/layout exclusions verified
   gated as the scope note claims.
9. Wave hygiene: only the three claimed files changed base→HEAD; z-rule clean; main working tree
   byte-identical at exit (worktree used for attacks, removed; `git worktree list` clean).

**Falsification attempts in categories with no finding:** capped-pattern abort hunt (7 shapes, all
survived); 14th-listener hunt (4 idioms × whole src, none); census miscount hunt (independent
parser agreed at per-listener granularity); honest-pattern narrowing hunt (8 vectors, none capped);
audit-bypass hunt beyond $ref chains (9 schema shapes, all refused); flake reproduction under load
(0/40); main-tree LFS peaks failure (passes).

*Private target dirs `/home/Samuel/.cache/tbd-target-verify128` and `…-verify128-wt` deleted after
this report; shim and rig artifacts live only in the session scratchpad.*

## Focused re-verification of the fix pass

Merged main, HEAD = c60ef031 (fixes d82dca4b + c60ef031 over my audit at 0d98a6e2). Private
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-reverify128`, built from scratch, deleted after
this report. **Environment note:** the host currently has NO native C toolchain and no `git-lfs`
on PATH (the post-checkout hook errors; checkouts themselves complete). All builds and test runs
here went through `org.freedesktop.Sdk//25.08` (flatpak) for the link step only — rustc is the
pinned 1.95.0 host toolchain, so codegen (and therefore stack-frame measurements) is unchanged.

**Suite reconciliation (both totals, both crates):** `website-frontend` `--list` = **921**, run =
**921 passed / 0 failed** — matches the expected 921 (919 at my audit + the 2 chain/cycle pins).
`map-engine-core --all-features` `--list` = **639**, run = **638 passed + 1 ignored** (625+5+5+3),
including `dem::peaks::tests::everon_peaks_max_above_350` — **ok on MAIN**. All four load-bearing
tests present by name in `--list` (`a_ref_chain…`, `a_ref_cycle…`, `deep_regex_input…`,
`honest_catalogue…`).

### d82dca4b — the $ref chain/cycle refusal

Attacked with 7 temporary adversarial tests spliced into the tests mod (reverted; tree
byte-identical at exit). All 7 green. What was attacked:

1. **Fail-closed everywhere:** my two original audit cases (chained `modpackId: 123`, `$defs`
   cycle) now REFUSE, document-independently, naming the pointer. A chain buried one level deeper
   — a clean one-hop `$ref` whose TARGET's own subschema holds the chain — is refused in **all
   five** subschema positions (`properties`, `items`, `patternProperties`,
   `additionalProperties`, `oneOf`), and in an UNREFERENCED `$defs` entry (structural descent).
   The check sits before the `visited` dedup (arsenal_rules.rs:1226 vs :1232), so no visit order
   suppresses it. A chain whose middle hop carries siblings still refuses. `$ref` to `#`, `#/`,
   an array index (`#/oneOf/0`), a URL-encoded pointer, an empty string, and a remote URL all
   land in the existing "cannot resolve" refusal — fail-closed, legible. A boolean `$ref` target
   refuses as "boolean where a subschema belongs".
2. **Over-refusal hunt — nothing legitimate refused:** the pinned `$defs/rec` array recursion
   validates Ok; **mutual** recursion through subschemas (a→items→b→items→a, one hop at every
   position) validates Ok; an unreferenced one-hop `$defs` alias Ok; the same one-hop ref used
   from two positions (visited-dedup path) Ok; `$ref` with only `title`/`description` siblings
   Ok; the shipped schema end-to-end Ok.
3. **Legibility:** the refusal names the dropped pointer and the path, in the T-735 idiom
   ("refuses rather than skipping…"), and reaches the caller through the same
   `cap_schema_messages` surface as every other refusal. Not a silent Err.
4. Walk-side note, checked and fine: `check_schema_node` itself would still pass a chain (post-
   deref, `$ref` is in SUPPORTED_SCHEMA_KEYWORDS and nothing else fires), but every production
   path enters through `validate_against_schema`, which runs the audit first and early-returns;
   the only direct `check_schema_node` callers are tests. Defense rests on the audit by design —
   consistent with the commit's claim, not a hole.

**One finding, out of the two commits but inside the attack surface I was told to sweep:**

MINOR | apps/website/frontend/src/arsenal_rules.rs:1371-1373 (`schema_deref`) | RFC 6901 escaped
tokens are neither unescaped nor refused, and when a LITERAL key containing `~1`/`~0` exists the
pointer resolves to the wrong node — accepting a document the schema, read per spec, rejects. |
Proved executable: shipped schema + `$defs` keys `"a/b"` = `{"minLength": 9999}` (the spec target
of `#/$defs/a~1b`) and literal `"a~1b"` = `{}`; the valid v1 document came back **Ok(())** where
the spec-resolved target faults `modpackId: "mp"`. Guard and walk mis-resolve identically (the
audit audits the node the walk enforces, so nothing the walk reads is unaudited), it predates
d82dca4b (T-735's `schema_deref` is unchanged by the fix), and it needs a literal `~`-bearing key
— the shipped schema has none, and the no-literal-key variants all refuse cleanly (verified).
Fix shape, in the module's own doctrine (refuse what you don't implement): in `schema_deref`,
refuse any segment containing `~` — `for seg in …split('/') { if seg.contains('~') { return
None; } cur = cur.get(seg)?; }` — which routes into the existing "cannot resolve" refusal; or
implement RFC unescaping (`~1`→`/` then `~0`→`~`, in that order). One-line fail-closed version
preferred; add a pin with the literal-key schema above.

### c60ef031 — the RX_MAX_PATTERN derivation

5. **Whose measurement is right: the fix agent's.** Independent re-measurement, same methodology
   (`(`×N through `RxParser::alt`, fresh 1 MiB thread per N, abort = floor), step 5 then step 1:
   **native debug clean at 793, aborts at 794** — inside the agent's 790/795 bracket. Native
   release: **clean at 2700, aborts at 2750** — inside their 2500/3000 bracket. My audit's
   850/900 was the wrong number; the correction ran the conservative way, as the commit says.
   The true floor is NOT lower than their 790: 512 is 1.55x under the exact floor (793), ~1.3 KB
   per debug frame, ~65% of 1 MiB at 512 levels — every figure in the new doc table checks out
   (release margin re-derives to 5.27x from 2700; their 4.9x from 2500 is the conservative end).
6. **Keeping 512 is defensible.** The 65%-of-stack case exists only in native debug — a test-only
   configuration — and is itself pinned: `(`×512 at the cap runs in CI on exactly that build, so
   margin erosion aborts loudly rather than shipping. The shipped config (release proxy) spends
   ~19-21%. And the floor under the cap is real: **any cap under 401 kills the shipped 200-level
   nesting vector** (`(`×200 + `a` + `)`×200 = 401 chars, `honest_catalogue…` requires it to
   ANSWER) **and the `^`×512 caret depth vector** — the "two shipped vectors" claim is true as
   stated (caps in 401-511 still kill the caret vector). 512 is the smallest power-of-two-ish
   value clearing both.
7. **The `assert_eq!(RX_MAX_PATTERN, 512)` pin is sound.** Its message ("re-measure the abort
   floor before changing this") is the procedure a correct future change must follow; the pin
   forces that reader to the doc table rather than letting a raise ride in silently while `(`×512
   stops being the worst case. That is the mechanism working, not a regression trap.
8. **The NIT comment fix is now TRUE**, verified against the code it describes: `items`' form
   check (arsenal_rules.rs:1276-1280) admits only objects — booleans and the tuple form are
   REFUSED, never "handled by the walk" — while `additionalProperties` (:1281-1286) passes
   `true`/`false` and the walk implements both meanings (:1598-1602).

### Hygiene

- Neither fix diff contains `update_slot_position` or `move_entities_and_vehicles` (grep over
  both commits: no hits) — wave-127 z-rule clean.
- No `tmp_*` test survived into either commit or the tree (`tmp_measure…` absent from both files
  and both diffs).
- Working tree at exit: byte-identical to merged main (only this artifact untracked). Temporary
  test splices reverted via checkout; `RV128-TEMP` marker count 0 in both files.

### Verdicts

**Are the wave-128 fixes complete and correct — YES.** d82dca4b refuses every chain/cycle shape I
could construct, over-refuses nothing I could construct, and reports legibly; c60ef031's numbers
re-derive exactly and its arithmetic correction is honest in the conservative direction.

**Is main safe to close this wave and build wave 129 on — YES.** 921==921 and 639-listed/638+1
green in a private dir, peaks test passes on main, both fixes hold under attack. The one MINOR
(escaped-pointer mis-resolution) is latent behind a schema shape the shipped file does not
contain and a fix commit did not touch; under the no-deferral regime it takes the one-line
fail-closed fix above before close.

### VERIFIED-CLEAN REGISTER (this pass)

1. Suite integrity: website-frontend 921 `--list` == 921 run; map-engine-core 639 `--list` ==
   638 passed + 1 ignored, `everon_peaks_max_above_350` ok on MAIN; fresh private-dir build.
2. $ref chain refusal: fail-closed at every constructed depth and position (5 subschema
   positions, unreferenced $defs, sibling-carrying middle hops, boolean targets, 8 pointer edge
   forms), refusal precedes `visited`, message names the pointer.
3. $ref over-refusal: none — pinned recursion, mutual recursion, unreferenced aliases,
   dedup'd double use, annotated refs, and the shipped schema all validate.
4. My two audit cases refuse; the single-hop control asserts both far-end keywords and passes a
   good document.
5. Parser floor: 793/794 debug, 2700/2750 release, measured here — the fix agent's 790/795 and
   2500/3000 verified, my 850/900 retracted; every figure in the new doc table re-derives.
6. Cap floor: 401 is the real functional minimum (two shipped vectors die below it); 512 clean
   in every configuration measured; the cap pin and its re-measure instruction verified present.
7. Hygiene: z-rule clean, no scratch tests, tree byte-identical at exit.

**Failed to break (attacked, survived):** chain-through-every-subschema-position (×6),
visited-order suppression, sibling-laundered chains, boolean/root/array/URL-encoded/remote/empty
pointer forms, over-refusal via mutual recursion and aliases (×5), the `(`×512/`^`×512/
`(x+x+)+y`-over-3000 stack vectors in both build profiles, and both suite totals against their
`--list` counts.

*Private target dir `/home/Samuel/.cache/tbd-target-reverify128` (including the scratch test
bodies staged inside it) deleted after this report.*
