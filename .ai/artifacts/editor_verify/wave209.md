# Wave 209 adversarial verification — T-811 / T-812 / T-813 / T-814 / T-815

Verifier: Claude (Fable), 2026-08-09. Verified on MERGED MAIN at **5f47c7a2** (base 9e34aa18; merges
27d29b24 T-811 · 73a8999a T-812 · 4e7a1ef3 T-813 · ae963c36 T-814 · 3dc9520d T-815 then the two
unpoison re-lands b31a7f25 + 5f47c7a2). Tree left exactly as found — every pin perturbation was
restored and `touch`ed; `git status` shows only the pre-existing untracked `.ai/artifacts/wave209_briefs/`;
zero tracked mutations, nothing committed, no tickets filed. HEAD unchanged at 5f47c7a2 at exit.

These five tickets fix wave-200's F1..F8. I re-ran the MEASUREMENTS the gate cannot see (the gate
passed 30/30 first-party; the crate suite is 1086 green). Every live number below was produced with
**real CDP input** — per-char `Input.dispatchKeyEvent` keyDown+text/keyUp, `rawKeyDown` Escape,
`dispatchMouseEvent` press/release pairs — driven through a stdlib WebSocket client against the
playwright chromium 149 (`--headless=new`, `--use-angle=swiftshader --enable-unsafe-swiftshader`,
`Emulation.setDeviceMetricsOverride` for the geometry pass), pointed at the **live trunk serve on
:3000** (debug build, dev-login admin), surface `/missions/smoke/edit?force=webgl&sat=preview` (the
seeded 8-slot smoke doc; fresh chrome profile per probe). Zero wasm panics in every session
(`window.__panics` empty throughout; SwiftShader avoids the known `__editorCamSet` headless-vulkan
artifact). Probe scripts + raw JSON live in the session scratchpad (`probe_811/812/813a/813b/814/
814_s8/815/geo` + `*.json`). Pin perturbations were run via `bash scripts/platform/wave.sh test
--slice T-999 -p website-frontend <filter>` into a private CARGO_TARGET_DIR (deleted class — private
`$HOME/.cache/tbd-target-T-999`, not the shared root).

## FINDINGS

### F1 — T-814's O-3 wiring pin is DISARMED: a stray second `#[test]` orphaned the real test fn
`MAJOR | apps/website/frontend/src/ui.rs:1400-1434 | the pin the T-786/T-814 O-3 acceptance depends on
(ORBAT must derive z from modal_stack::z_class) never runs; a duplicate-named test runs twice in its
place | proven by name-filtered cargo test: 0 vs 2`

- Evidence: ae963c36 inserted a new `#[test]` + doc-comment + `fn
  opening_an_overlay_fires_registered_transient_closers` **between** the existing `#[test]` attribute
  (ui.rs:1400) and its owner `fn orbat_manager_overlay_derives_z_from_the_modal_stack` (ui.rs:1434).
  Result: `opening_an_overlay_fires_registered_transient_closers` carries **two** `#[test]` attributes
  (1400 + 1402) and `orbat_manager_overlay_derives_z_from_the_modal_stack` carries **none** — it is a
  plain, never-invoked function (dead code). Measured on merged main:
  `wave.sh test … orbat_manager_overlay` → `running 0 tests`;
  `wave.sh test … opening_an_overlay` → `running 2 tests` (the same name, registered twice).
- Impact: the guard that would catch a regression of the flagship O-3 fix — the assertion that
  `OrbatManagerDialog` takes its overlay z from `modal_stack::z_class(modal_id)` rather than a literal
  `z-50` — is silently disarmed, and the gate's 1086-pass counts the harmless duplicate instead. This
  is the "gate reports success on code it never examined" class. The **shipped behavior is correct**
  (live: ORBAT drops to z-40 under the Arsenal's z-50, `elementFromPoint(arsenal centre)` inside the
  arsenal — see register), so main is not broken and no data is at risk; what shipped broken is the
  regression net for T-814's own acceptance surface.
- Disposition: re-ticket — delete the duplicate `#[test]` at ui.rs:1402 so
  `orbat_manager_overlay_derives_z_from_the_modal_stack` is armed again. (The production
  `z_class(modal_id)` call at orbat_manager.rs:344 is present and correct; only the pin is dead.)

### F2 — T-811's layer-rename pin is HOLLOW: raw `include_str` self-matches its own literals
`MAJOR | apps/website/frontend/src/eden_tree.rs:1542-1565 | the pin claims to verify the layer-rename
NodeRef/on_load/testid wiring but examines only its own assertion strings; it survives deletion of the
production widget | proven by perturbation: removed production node_ref=rename_ref → pin still GREEN`

- Evidence: `layer_rename_uses_noderef_onload_and_decoupled_draft` (added this wave by T-811, 1d90cb3f)
  reads `TREE = include_str!("eden_tree.rs")` — the **raw whole file, test module included** (eden_tree.rs:1460).
  All five asserts are `TREE.contains("<literal>")` where that literal is spelled verbatim in the test
  body (`"NodeRef::<leptos::html::Input>::new()"`, `"node_ref=rename_ref"`, `".on_load("`/`".focus()"`/
  `".select()"`, `"renaming:"`/`"rename_draft:"`, `"data-testid=\"layer-rename-input\""`). Each needle
  therefore satisfies itself. Perturbation (restored + touched): I deleted `node_ref=rename_ref` from
  the **production** layer-rename input (eden_tree.rs:883) and ran the pin — it reported `test result:
  ok. 1 passed`. A real pin fails on "the NodeRef must be attached via node_ref=rename_ref".
- Impact: this is the exact T-759 hollow-pin class that T-815's ORBAT pins were re-landed to fix
  (b31a7f25) — shipped fresh in T-811, in the file whose wave-200 defect (F1) was "a green pin on the
  wrong widget." The **shipped feature works** (live: both flows focus on mount, type whole, GRID
  intact — see register), so this is a false safety net, not a broken feature. Note in passing: the
  production input at eden_tree.rs:887 uses `prop:value=move || rename_draft.get()` — the exact
  reactive pattern T-812/T-815 explicitly *ban* — yet it works here because the tree list does not
  re-render off `rename_draft` (live: the tagged node survives the whole word), so it is not a defect,
  only a reason the missing ban is more than cosmetic.
- Disposition: re-ticket — rewrite the pin to read the scrubbed production half
  (`class_r_scrub::live_source(include_str!("eden_tree.rs"))`, which cuts from the first `#[cfg(test)]`
  onward), exactly as its sibling T-812 pin already does in eden_dock_left.rs:1720. Consider adding the
  `prop:value=move || rename_draft.get()` ban for parity with T-812/T-815.

## Safe-line

**Yes — main is safe to build the next wave on.** No BLOCKER: every shipped fix (F1..F8) is
live-verified working, both destructive wave-200 cases (F2 truncation, F3 multi-edit wipe) are gone,
the two T-815 unpoison pins are proven non-hollow, geometry/O-3/O-5 have not regressed, and no wasm
panic occurred in any session. F1 and F2 above are disarmed/hollow **regression guards** — they need
re-tickets before anyone leans on those specific pins, but they do not endanger the tree or operator
work, because the code the pins fail to protect is itself correct today.

## Verified-clean register — claims RE-PROVED, with measured numbers

**T-811 layer rename (live, both flows — real per-char CDP):**
- Creation (New layer → armed inline rename): `focused_on_mount=true`, prefilled "New Layer 1",
  select-all on mount `true`, typed "Assault Sqd" → value === "Assault Sqd", tagged node survived the
  whole word (no remount trap — did NOT import T-812's trap), caret collapsed at end, activeElement ===
  `INPUT#layer-rename-input`, GRID chip byte-identical, `g` landed in field ("Assault Sqdg") with GRID
  unchanged, undo depth unchanged mid-edit, Enter committed the row.
- Pencil (`aria-label="Rename layer"`): `focused_on_mount=true`, prefill "Assault Sqd", typed
  "Bravo Lne X" landed whole, node survived, activeElement === input, GRID same, **Escape abandoned**
  (input gone, row still "Assault Sqd", no "Bravo Lne X" row). GRID identical to session start.

**T-812 bookmark ADD + rename (live, Places tab):**
- ADD (`dock-left-bookmark-add` → `dock-left-bookmark-name`): `focused_on_mount=true`, `g` landed in
  the field with GRID unchanged (F7 fixed), typed "Ridge OP One" whole, node survived, Enter committed;
  Escape on a fresh ADD abandoned ("Zed" never committed).
- Rename (`dock-left-bookmark-rename`): `focused_on_mount=true`, select-all on open, typed
  **"Ridge OP Two" → committed row "Ridge OP Two"** (F2 fixed; was "o"), old name gone, node survived
  the word, caret collapsed, activeElement === input; Escape on a re-open abandoned ("Nope Nope" never
  committed, "Ridge OP Two" kept). GRID identical throughout.
- Pin honesty: `bookmark_rename_and_add_decouple_draft_and_focus_on_mount` reads `live_src()` =
  `class_r_scrub::live_source` (cuts the test module — its own comment cites the T-759 defense), asserts
  BOTH `node_ref=rename_ref` and `node_ref=add_ref` and both `value=…get_untracked()` — binds the two
  named widgets in production, not a decoy. (Contrast F2 above: the T-811 sibling did NOT do this.)

**T-813 multi-edit + field Escape (live, digest + undo depth):**
- F3 — differing ROLE (`s1`="RiflemanAT Rifleman" vs `s5`), "2 slots selected · multi-edit",
  placeholder "Multiple values", disabled until "Apply Role to all selected" ticked; then focus ROLE +
  blur on the modal header with **zero keys pressed** → **undo depth Δ0, slots_digest UNCHANGED** (was
  +2 and role-wiped). Undo restored digest byte-identical.
- Typed "Multi Rle X" → stamped **both** selected slots, mid-word depth Δ0 (no per-keystroke fan-out),
  commit +2 steps (the F-26 per-slot cost, preserved by design).
- The edited-latch does NOT eat a deliberate re-type: after undo, re-typing one member's settled value
  ("RiflemanAT Rifleman") → stamped **all** selected slots, +2 steps.
- Single-selection no-op skip holds: focus+Enter Δ(0, digest same); focus+blur Δ(0, digest same).
- F6 — field Escape consumes: text ROLE, typed "Zz Qq", Escape → draft reverted, **modal STILL OPEN**,
  no write, field blurred; second Escape closed the modal. number_field X: Escape → **modal STILL
  OPEN**, digest stable; second Escape closed. Differing NUMBER (X) focus+blur zero-typing → Δ(0,
  digest same).
- ROTATION regression: "135" → value "0135", focus kept every char, Enter +1 step / digest moved.
- a2i fail-closed (bef0a071 property, live): focus+set value+`input` with **no blur** → Δ(0, digest
  same); `blur()` → +1 step, digest changed; Ctrl+Z restored.
- Four Identity fields focus retention ("qq q" per field, Type/Role/Role Description/Tag): focus kept
  and typed text landed on all four; modal survived four field-level Escapes.

**T-814 composed Esc ladder + exclusivity (live, single-keydown granularity):**
- **The composed ladder** (focused ROLE field with "Ab" draft, over Arsenal z-50, over ORBAT z-40),
  one Escape per press: Esc1 → field draft abandoned + blurred, **both dialogs still open** (active
  BODY); Esc2 → **Arsenal (visually top) closed, ORBAT stays**; Esc3 → ORBAT closed; Esc4 → no-op.
  Exactly one layer per press, in order field-draft → visually-top dialog → next dialog. **No pile-up
  on any press.**
- Rapid double-Esc (no sleeps) over field+Arsenal+ORBAT: after two → field + Arsenal peeled, **ORBAT
  survives**; third → ORBAT. One-per-press holds under rapid repeat (wave-139 property).
- F4 exclusivity: hint open + canvas-dblclick Attributes → **hint gone at 80ms and 600ms** (Attributes
  up); Esc1 closed Attributes alone, hint stayed absent. Export dropdown + dblclick Attributes → export
  items dropped to 0 (transient closer fired). One Escape per surface.
- F5 / O-3 order: ORBAT alone z-50; after OPEN ARSENAL → **ORBAT z-40, Attributes z-50**,
  `elementFromPoint(arsenal centre)` inside the arsenal (`true`) — z-stacking unchanged by the Esc-order
  fix. Esc1 closes the Arsenal the operator sees, not the hidden ORBAT.
- O-5 pair: hint → Save Version → **hint absent, "Versions are immutable" present**; Esc closed Save
  alone (attrs/hint/orbat all false).
- Context menu keeps its own dismissal: RMB → "Attributes..." menu; one Escape closed it and nothing
  else opened. Context-menu-over-hint: opening the menu fired the transient closer (hint already gone
  **before** any Escape — disambiguated with a 400ms wait past the 16ms pump), so the single Escape
  closed only the menu (one-per-press, not a pile-up). Strip View menu: one Escape closed it.
- Transient-over-dialog (hint is `pointer-events-none fixed inset-0 z-50`, a passive reference card):
  hint opened over ORBAT survived; Esc1 closed ORBAT, Esc2 closed the hint — the transient is peeled
  last, matching the specified order; one layer per press throughout.

**T-815 ORBAT squad rename + the two unpoison pins:**
- Live (created a squad via ADD SQUAD/GROUP → "Squad 1"): rename `focused_on_open=true`, select-all on
  open, typed **"Talon Actual" landed whole**, node survived, activeElement === `INPUT#orbat-squad-rename`,
  GRID unchanged, `g` landed in field with GRID same; Enter committed + **ORBAT stayed open** + name
  present; Escape on re-open abandoned ("ZZZ" gone, "Talon Actual" kept, ORBAT stayed open).
- **Source pin proven NON-HOLLOW** (perturb → RED → restore + touch): deleted production
  `node_ref=rename_ref` (orbat_manager.rs:1057) → `orbat_squad_rename_focuses_via_noderef_on_load`
  FAILED at :1926 "the NodeRef must be attached via node_ref=rename_ref". Restored, no residual diff.
- **Class-R ban pin proven NON-HOLLOW** (perturb → RED → restore + touch): injected the banned
  `prop:value=move || rename_draft.get()` alongside the kept `value=…get_untracked()` in production
  stitch_row → the ban FAILED at :1942 "reactive prop:value on squad rename clears on_load select-all —
  banned" (fired in isolation, positive asserts still green — proves the concat needle + `only_body`
  scope catch the real production defect, not a self-match). Restored, no residual diff.
- NIT-2 confirmed: `GET /api/v1/missions` lists no name containing "T787"; both ids
  85b61b6a-33f6-417f-ad50-fce2ce514a3e and a121023d-7dd0-4ccc-9d10-41be3fd32ab1 return **404** (their
  absence is correct, not data loss).

**T-787 geometry (re-check, no wave-209 ticket touches it; T-814's ui.rs must not have moved it):**
- 1920×1080 (forced via device-metrics override): status bar top **1044**, h 36, w 1920; both docks
  (0,48,240) and (1680,48,240) with **bottom == 1044 == bar.y exactly** on both. Bar-end hit-tests
  `elementFromPoint(120, 1054)` and `(1800, 1054)` both resolve **inside the status-bar subtree** — the
  O-1 click-theft is gone. Matches wave-200 measurements exactly.

**Cross-cutting:** no wasm panics in ~9 headless sessions; repo tree restored byte-identical after
every perturbation (three pins perturbed: eden_tree hollow pin, orbat source pin, orbat ban pin — all
restored + touched, zero tracked diff at exit); private test target dir used for all cargo runs. Six
pre-existing chromium processes (port 9229, profile `/tmp/t812-chrome-Kd67`, foreign flag set) were NOT
spawned by this session and were left untouched; every probe chromium I launched (ports 9401–9411,
scratchpad profiles) is confirmed dead.

## What I attacked and FAILED to break (nobody needs to re-audit these)

1. **The composed Esc ladder** — every combination of focused field / Arsenal / ORBAT / hint / export /
   context menu, including rapid back-to-back Escapes: one layer per press, correct order, no pile-up
   on any press, in every combination driven.
2. **T-813 F3** — the differing-field zero-typing wipe: could not reproduce a write, an undo step, or a
   digest change from any focus+blur / focus+Enter with no keystroke, on ROLE (text) or X (number).
3. **The edited-latch over-correction trap** — re-typing a slot's already-settled value still stamps
   the whole selection; the latch does not swallow a deliberate no-change re-type.
4. **T-813 F6** — text and number field Escape both keep the modal open on the first press and close it
   on the second; the field-level consume composes with T-814's guard cleanly.
5. **The two T-815 unpoison pins** — both go RED on deletion/injection of their production subject and
   GREEN on the shipped code; the include_str self-match and Class-R ban are genuinely closed.
6. **T-811 / T-812 / T-815 live rename widgets** — focus-on-mount, whole-word typing, node survival (no
   remount trap), GRID immunity, Enter-commit, Escape-abandon: all four rename surfaces (layer, bookmark
   rename, bookmark ADD, ORBAT squad) hold.
7. **O-3 z-stacking and hit-test** — ORBAT z-40 under Arsenal z-50, arsenal-centre elementFromPoint
   inside the arsenal; T-814's Esc-order change did not disturb the z answer.
8. **O-5 pair and the four Identity fields' focus retention** — re-proved green.
9. **a2i smoke fail-closed** — input-without-blur writes nothing; the blur/Enter seam is the only commit.
10. **T-787 dock/bar geometry** — dock.bottom == bar.y equality and bar-end hit-tests, unregressed.

## Fixup re-verification (27343f82, focused)

Scope: commit 27343f82 only ("T-814/T-811: re-arm the O-3 wiring pin, un-hollow the layer-rename pin"), fixing F1/F2 from the pass above. Tree was clean at 27343f82 before and after this pass.

### 1. Scope purity — PASS
`git diff --stat 5f47c7a2..27343f82` touches exactly two files: `apps/website/frontend/src/eden_tree.rs` (+10/−7) and `apps/website/frontend/src/ui.rs` (+6/−6). Every hunk sits inside a `#[cfg(test)] mod tests` block:
- ui.rs: `#[cfg(test)]/mod tests` opens at 973/974 (both commits); hunks at old lines 1387–1408 and 1426–1441. Production half (lines 1..972) extracted from both commits and `cmp`-compared: **byte-identical**.
- eden_tree.rs: `#[cfg(test)]/mod tests` opens at 1260/1261 (both commits); hunk at old lines 1535–1570. Production half (lines 1..1259) `cmp`-compared: **byte-identical**.
- The production layer-rename input perturbed-and-restored in the prior pass is intact: `node_ref=rename_ref` present at eden_tree.rs:883 and covered by the byte-identical production comparison.
- The diff content: ui.rs moves the O-3 doc comment + `#[test]` from above `opening_an_overlay_fires_registered_transient_closers` (which had two attributes) down to `orbat_manager_overlay_derives_z_from_the_modal_stack` (which had none); eden_tree.rs rewires the layer pin's five asserts from raw `TREE` to `class_r_scrub::live_source(TREE)` plus a comment. No production tokens in any hunk.

### 2. F1 — re-armed O-3 wiring pin: RESOLVED
Attribute audit at HEAD: `#[test]` at ui.rs:1396 → `fn opening_an_overlay_fires_registered_transient_closers` (1397); `#[test]` at ui.rs:1433 → `fn orbat_manager_overlay_derives_z_from_the_modal_stack` (1434). Exactly one each (was 2 and 0).

Name-filtered runs (`wave.sh test --slice T-814 -p website-frontend -- <name>`), both now report **`running 1 test`** (was 0 and 2):
- `orbat_manager_overlay_derives_z_from_the_modal_stack`: ok. 1 passed; 0 failed; 1085 filtered out.
- `opening_an_overlay_fires_registered_transient_closers`: ok. 1 passed; 0 failed; 1085 filtered out.

Perturbation: replaced `let z = crate::ui::modal_stack::z_class(modal_id);` (orbat_manager.rs:344) with `let z = "z-50";` → pin went RED (exit 101), verbatim:

```
test ui::tests::orbat_manager_overlay_derives_z_from_the_modal_stack ... FAILED
thread 'ui::tests::orbat_manager_overlay_derives_z_from_the_modal_stack' (729030) panicked at apps/website/frontend/src/ui.rs:1438:9:
OrbatManagerDialog must take its overlay z from modal_stack::z_class (T-786 O-3). Body was: ...
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1085 filtered out; finished in 0.10s
```

Restored via `git checkout -- apps/website/frontend/src/orbat_manager.rs` (restore verified: `git diff` empty, `z_class(modal_id)` back at :344; the repo's post-checkout LFS hook complained about a missing git-lfs binary — cosmetic, no LFS file involved), `touch`ed, re-run: **ok. 1 passed; 0 failed** — green.

### 3. F2 — un-hollowed layer-rename pin: RESOLVED
The pin now reads a scrubbed haystack: `let tree = crate::arsenal::class_r_scrub::live_source(TREE);` and all five asserts consume `tree`, so the test module's own assertion strings can no longer satisfy the needles.

Perturbation: deleted the production line `node_ref=rename_ref` (eden_tree.rs:883, inside the layer-rename `<input`) → pin went RED (exit 101), verbatim:

```
test eden_tree::tests::source_pins::layer_rename_uses_noderef_onload_and_decoupled_draft ... FAILED
thread 'eden_tree::tests::source_pins::layer_rename_uses_noderef_onload_and_decoupled_draft' (733295) panicked at apps/website/frontend/src/eden_tree.rs:1549:13:
the NodeRef must be attached via node_ref=rename_ref
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1085 filtered out; finished in 0.11s
```

(Before the fix this deletion stayed green — the hollow self-match. It now trips exactly the assert the fix rewired.)

Restored via `git checkout -- apps/website/frontend/src/eden_tree.rs` after confirming `git status` showed that file as the only modification (restore verified: `git diff` empty, `node_ref=rename_ref` back at :883, occurrence count 3 matching HEAD), `touch`ed, re-run: **ok. 1 passed; 0 failed** — green.

### 4. Foreign-binary discipline — PASS
`--list` on the website-frontend suite: **1086 tests, 0 benchmarks**. Full unfiltered run: **ok. 1086 passed; 0 failed; 0 ignored; 0 filtered out** (23.97s). List total == run total == expected 1086; every filtered run above also reconciles (1 passed + 1085 filtered = 1086).

### Verdict
F1 and F2 are both RESOLVED at 27343f82 by a test-only commit (production byte-identical to 5f47c7a2); both pins are live (red under targeted production perturbation, green restored); wave 209 verdict upgrades to **safe — no open findings**. Tree left clean at 27343f82; private target dir `~/.cache/tbd-target-T-814` deleted.
