# Wave 200 adversarial verification — T-785 / T-786 / T-787 (+ a2i fixup bef0a071)

Verifier: Claude (Fable), 2026-08-09. Verified on MERGED MAIN at **bef0a071** (base b6d53d77;
merges 83a9a73a / cc93241e / 42203df1). Tree left exactly as found — `git status` clean at exit,
zero repo mutations, nothing committed, no tickets filed.

## Harness — real CDP against the shipped trunk config

Every measurement below was produced with **real CDP input** (per-char `Input.dispatchKeyEvent`
keyDown+text/keyUp, `Input.dispatchMouseEvent` press/release pairs — the review §2 F-01 probe
shape and the smokes.rs idioms) driven through a stdlib WebSocket client against the playwright
chromium 149 (`--headless=new`, SwiftShader, the cdp.rs gate flag set), pointed at the **live
trunk serve on :3000** (debug build, dev-login admin) — NOT a throwaway static server; this closes
the T-787 slice's own environment gap. Surface: `/missions/smoke/edit?force=webgl&sat=preview`
(the seeded 8-slot smoke doc; fresh chrome profile per probe ⇒ fresh localStorage; no operator
mission touched). Probe scripts + raw JSON live in the session scratchpad (`probe_785*/786*/o3*/
bm787/esc` + `*.json`). Zero wasm panics in any session (`window.__panics` empty throughout;
the known `__editorCamSet` headless-vulkan artifact never applies — SwiftShader was used).

The wave gate's own cargo/editor suite is evidenced as having genuinely run (bef0a071 exists
because the a2i smoke went red on merged main and was fixed); I did not re-run the cargo suites —
this pass re-ran the *measurements* the slices could not (T-785 shipped with no browser at all).

## FINDINGS

### F1 — the F-02 layer rename is NOT fixed; T-785's fix landed on a different widget
`MAJOR | apps/website/frontend/src/eden_tree.rs:462,840 | the ticket's named "layer rename"
acceptance surface is untouched and still fully broken | live CDP, both flows, plus diff-stat`

- Evidence: T-785's slice (6051a2ec) changed only `attributes.rs`, `eden_dock_left.rs`,
  `mission_history.rs` (`git show --stat`). The actual Layers rename — pencil
  `aria-label="Rename layer"` at eden_tree.rs:462, input at :836-867 — still relies on bare
  `autofocus=true` (:840) on a reactively-inserted node, the exact mechanism T-785's own new
  comment in eden_dock_left.rs:330-338 documents as impossible ("a node created by a later
  reactive update is skipped").
  Live, creation flow (New layer → armed inline rename): input mounts prefilled "New Layer 1",
  `focused_on_mount:false`, activeElement=BUTTON; real `g` keydown flipped `GRID  off` →
  `GRID  move off · rot off`; typed "Assault Sqd" never reached the input.
  Live, pencil flow (the review's F-02 repro): `focused_on_mount:false`, activeElement=BODY,
  `g` flipped GRID again, value stayed "New Layer 1" after 11 typed chars.
- Impact: the acceptance clause "layer rename: value === typed, activeElement === input, GRID
  readout unchanged" fails on all three counts; F-02 ships un-fixed while the registry and the
  commit message say it is fixed. The slice's source-pins (eden_dock_left.rs:1937-1961) pin the
  *bookmark* rename and pass — a green pin on the wrong widget, which is precisely the risk of a
  slice that shipped with no live browser (its worktree had no chromium).
- Disposition: needs a re-ticket against eden_tree.rs; the fix is the one already written next
  door (NodeRef + on_load focus+select).

### F2 — the bookmark rename T-785 DID fix now destroys the typed name (last-char-only)
`MAJOR | apps/website/frontend/src/eden_dock_left.rs:310-360 | typing a name commits only its
final character | live CDP: typed "Ridge OP Two", committed row is named "o"`

- Evidence: the rename input is controlled through the `renaming` signal, and the whole bookmark
  list closure reads that signal — every keystroke re-renders the list, rebuilding the input node;
  the new `on_load` then re-focuses AND `select()`s the full text on every remount, so each next
  keystroke replaces the entire draft. Live: `ren_focused_on_mount:true` (the fix "works"),
  typed "Ridge OP Two" per-char → input value "o", committed row label "o". Two independent reads
  (live value + post-commit row).
- Impact: the one rename field the wave changed went from "unfocused, keystrokes leak as chords"
  (pre-wave F-02 family) to "focused, silently truncates any typed name to one character" — a new
  data-destroying behavior on the fixed field. The acceptance analog (value === typed) fails.
- Disposition: re-ticket; the remount-per-keystroke is the same root cause class T-785 fixed in
  attributes.rs (draft must not round-trip through the signal the list renders from), so the
  on_load band-aid masks it instead of fixing it.

### F3 — multi-edit differing field: focus + blur with ZERO typing wipes the field on every selected slot
`MAJOR | apps/website/frontend/src/attributes.rs:795-801 | the differing-field exemption defeats
the no-op skip and stamps the empty draft across the selection | live CDP with digest/undo depth`

- Evidence: `text_commit` skips the write only when `!gate.differs()`; a differing field's draft
  seeds from `text_display()` = `""`. Live: 2 slots with differing roles selected, `Multiple
  values` shown, apply-to-all ticked, click into ROLE, click the modal header (blur), **no key
  ever pressed** → undo depth +2 (one step per slot, the F-26 cost), `slots_digest` role fields
  emptied for exactly the selected slots (`s1|…|||sq|`, `s5|…|||sq|`); 2× Ctrl+Z restored the
  digest byte-identical.
- Impact: destroys operator-authored values via a pure no-op interaction (tick the box, click in,
  change your mind, click out), silently, at N-undo-presses recovery cost. Directly violates the
  wave's own T-775-family acceptance ("focus/blur without typing ⇒ NO document write, NO undo
  step") in the multi-edit case; single-selection no-op skip holds (measured, see register).
- Disposition: re-ticket. The comment's intent ("typing the settled value back is a deliberate
  stamp") needs a "did the operator actually edit the draft" latch, not a blanket differs-exemption.

### F4 — O-5 exclusivity and "one Esc, one layer" fail for every dialog not opened from the strip
`MAJOR | apps/website/frontend/src/eden_top_strip.rs:758-767,787 | close_transients only guards
the strip's own three open paths, and the any_open() Esc guard is defeated by listener order |
live CDP at single-keydown granularity`

- Evidence: hint open → **canvas dblclick** opens Attributes → hint still up under the dialog
  (measured true; same for the export dropdown under Attributes). Then ONE `rawKeyDown` of
  Escape closed BOTH the Attributes dialog AND the hint (and, in the export variant, dialog AND
  export) — the exact wave139-F3 pile-up the `any_open()` guard at :787 exists to prevent. The
  guard fails because the dialog's own window keydown listener runs first in the same event,
  closes the dialog, and the strip then observes `any_open() === false` in that same keydown.
  Acceptance pair itself passes: hint → Save Version button → hint absent, "Versions are
  immutable" present (measured; F4 is the adjacent pairs, not the acceptance pair).
- Impact: the shipped comment "a dialog and a reference card can no longer be up at once"
  (:758-760) is not delivered for the attributes/context-menu dialog family, and Esc violates
  "one layer per press" on those pairs. Guard correctness currently depends on window-listener
  insertion order, which reshuffles on strip remounts — fragile by construction.
- Disposition: re-ticket: either wire transient-closing into the non-strip dialog opens, or make
  the strip's Esc guard consume-aware (e.g. check a "dialog handled this event" mark, not live
  any_open()).

### F5 — Esc over arsenal-on-ORBAT closes the HIDDEN dialog first
`MINOR | apps/website/frontend/src/ui.rs:466-474 vs :566-571 | mount-order Esc + open-order z
diverge on the flagship acceptance pair | live CDP ladder`

- Evidence: ORBAT (z-40, underneath) + Arsenal (z-50, on top): Esc1 → ORBAT closed, Arsenal
  stayed (`after_esc1: attr:true, orbat:false`); Esc2 → Arsenal closed. One layer per press held.
- Impact: matches the deliberately-retained T-333 mount-order rule, but now that paint order is
  open-order (T-786), the first Esc is visually a no-op that silently discards the ORBAT context
  behind the arsenal's scrim. Spec-tension the band accepted; flagging because it manifests on the
  exact O-3 acceptance pair.

### F6 — Escape inside a focused attribute field abandons the draft AND closes the modal in one press
`MINOR | apps/website/frontend/src/attributes.rs:834-845 + :230-236 | field-level and modal-level
Escape both consume the same keydown | live CDP`

- Evidence: typed 11 chars into Asset id, pressed Escape: draft abandoned with no write and no
  undo step (correct) — but `modal_still_open:false` in the same press. The field's keydown does
  not stop propagation and the modal's window listener closes on the same event.
- Impact: an operator backing out of a half-typed value loses the whole modal. Family-consistent
  with number_field pre-wave; T-785 extended the shape to text fields.

### F7 — the bookmark ADD ("name this view") input in the fixed file still has the F-02 defect
`MINOR | apps/website/frontend/src/eden_dock_left.rs:275-304 | bare autofocus on a reactive
insert, no NodeRef/on_load | live CDP`

- Evidence: click "Bookmark this view" → naming input mounts (`dock-left-bookmark-name`),
  `focused_on_mount:false`, activeElement stays BUTTON; real `g` flipped GRID with the naming box
  open; the keystroke never reached the input.
- Impact: the defect class T-785 existed to kill survives one screen away from its fix, in the
  file the fix touched.

### F8 — observations (no action required to build on main)
- `NIT | orbat_manager.rs (squad rename) | the ORBAT squad-rename inline input opens unfocused;
  typing before clicking into it runs as chords | live CDP` — pre-existing family member, not a
  T-785 acceptance surface; once clicked, typing works and focus holds ("qq q" landed).
- `NIT | live API DB | two leftover probe missions "T787 WT Probe" / "T787 Probe" from the T-787
  slice's checks are still in the mission library | GET /api/v1/missions` — delete when convenient
  (same class as the review's UXREVIEW-A note).

## Safe-line

**Yes — main is safe to build the next wave on** (no BLOCKER: nothing corrupts outside the flows
named above, both destructive cases are undo-recoverable, T-786/T-787 acceptance surfaces and the
T-785 core typing fix all hold) — but F1/F2/F3/F4 are shipped-claim failures that need re-tickets
before anything builds ON those specific surfaces.

## Verified-clean register — claims RE-PROVED, with the falsification attempt per category

**T-785 acceptance, single selection (re-measured, real per-char CDP keys):**
- ROLE / ROLE DESCRIPTION / TAG / Asset id ("Type"): 11-char strings with spaces
  ("AT Rifleman", "AT Riflem n", "MED ENG SLx") — every char landed, `activeElement` === the
  input, `data-`tagged node survived the whole word, dock rect fingerprint and GRID chip
  byte-identical before/after, and NO mid-word commit (undo depth and slots_digest unchanged
  until the seam). Caveat recorded: a pre-filled field inserts at the caret (typed into seeded
  "Rifleman" → "RiflemanAT Rifleman"), so "value === typed" holds for the typed span, not as a
  full-field replace.
- Commit seams: Enter → exactly +1 undo step, digest changed, value round-trips through close/
  reopen; blur-by-click → +1 step (Role Description round-tripped "AT Riflem n"; note
  slots_digest excludes description — depth+roundtrip prove that path).
- Escape abandons: no write, no undo step (see F6 for the modal side effect).
- No-op skip (single selection): focus+Enter and focus+blur → depth 4→4, digest unchanged.
  (Falsification: the same attempt on a differing multi-edit field is F3.)
- ROTATION regression: real digits "135" → value "135", focus kept, Enter → one step, digest
  moved. number_field did not regress.
- Multi-edit locked-state trap: differing ROLE shows placeholder "Multiple values", `disabled`
  until the apply-to-all checkbox is ticked; after ticking, typed "Multi Rle X" landed whole
  (no per-keystroke fan-out — depth/digest unchanged mid-word) and committed to every selected
  slot. (The no-op variant of the same flow is F3; the per-slot undo cost is pre-existing F-26.)
- Healthy fields that must stay healthy: mission title, left-dock mission search, ORBAT inspector
  text input — "qq q" typed with real keys, focus kept, values intact. NOT separately driven:
  marker caption and composition-name (no marker/composition in the smoke seed; flows not
  exercised) — both are outside T-785's diff, and every attributes-modal text field shares the
  one text_field widget verified above.
- Claim 7 (guard): while focused, `g` landed in the field and GRID never moved; after a forced
  `blur()`, the live guard correctly reports not-editable and `g` chords (GRID flipped) — that is
  the guard's designed limit, not a leak. Counterexample construction attempted: no
  operator-reachable path bumps `doc_tick` mid-type (undo/redo chords are guarded while focused;
  every mouse path blurs-and-commits first); the only external write found is the gate-only
  `__missionDoc.seed_slots` debug bridge, which does unmount the field when invoked — flagged as
  the standing design boundary, unreachable from the UI.

**T-786 (re-measured):**
- O-3 acceptance: ORBAT alone z-50 → slot → OPEN ARSENAL: ORBAT dropped to **z-40**, Arsenal
  z-50, `elementFromPoint(arsenal centre)` inside the arsenal surface = true. Falsification that
  failed: DOM order puts the attributes panel BEFORE the ORBAT panel, so a z tie would paint
  ORBAT on top — the pass is genuinely stack-driven, not DOM-order luck; the reconcile's
  untracked reads did not go stale on this path (ORBAT re-rendered within the interaction).
- O-5 acceptance pair: hint ("Controls — keyboard shortcuts") → Save Version → hint absent,
  "Versions are immutable" present; Esc then closed Save alone. (Adjacent pairs: F4.)
- Sufficiency of ORBAT-only stack registration (claim 3), attacked via reachability: Save-open →
  ORBAT button hit-test = the z-50 scrim; Settings-open → canvas slot point = dialog subtree
  (dblclick cannot reach a slot); ORBAT-open → dock "Manage factions" hit-test = the z-50 scrim.
  No keyboard path opens any dialog (source sweep: the only global chords are undo/redo +
  editor keys; every `*_open.set(true)` call site is a strip/menu/dock button). So every literal
  z-50-vs-z-50 tie ordering is unrealizable by mouse or keyboard — the sufficiency claim holds.
- Claim 4 (close+reopen collapsed into one frame): no JS bridge writes those signals; every UI
  open is a discrete click with renders between; Esc-close → reopen cycles re-stamped correctly
  (fresh z-50 each time). Not falsifiable live; the batch-stamp registry-order fragility noted in
  ui.rs:522-559 remains a design smell with no reachable trigger found.
- Esc one-layer-per-press among DIALOGS: held everywhere measured (two presses for two stacked
  dialogs, one for one). The order defect is F5; the transient pile-up is F4.

**T-787 acceptance (re-measured on the shipped trunk config, three viewports):**
- 1920x1080: bar top 1044, h 36; docks (0,48,240,996) and (1680,48,240,996) — dock.bottom
  **== 1044 == bar.y exactly** (the equality claim holds, not just <=).
- 1366x768: bar 732; both dock bottoms == 732. 2560x1440: bar 1404; both == 1404.
- `elementFromPoint(120, barY+10)` and `(width-120, barY+10)` resolve inside the status-bar
  subtree at ALL three viewports (the O-1 click-theft is gone).
- No dead strip: `elementFromPoint` at barY−2 under a dock = the dock ASIDE (dock touches bar);
  the 36-vs-96 (TOOLBELT_BAND_PX) argument is settled by measurement — a 96px inset would have
  left the 60px strip the slice predicted; 36 leaves zero.
- Dock inner scroll: right-dock scroller scrollGap 0 after scroll-to-bottom, last row bottom
  1036/724/1396 — visible above the bar at every viewport.
- Chevrons: collapse/expand cycled live; collapsed rails are 24×24 buttons at y48 (nowhere near
  the bar); the point under a collapsed dock resolves into the status bar.
- Source tie: STATUSBAR_H_PX = 36.0 == painted h-9 (eden_toolbelt.rs:334, pinned :992-1006).

**a2i fixup bef0a071 (claim 6):**
- Fails closed, proven live in the app the smoke drives: the pre-fixup shape (focus + set value +
  `input` event, NO blur) produces NO digest change and NO undo step — a hollow blur-commit path
  therefore leaves `a2i_digestChanged` false, and it is one of the 9 all-must-be-true checks
  (smokes.rs:1511 `checks_pass(&checks, 9)`); after `blur()` the digest changed and depth rose
  exactly one ("Marksman" present in the digest).
- No other smoke encodes the per-keystroke contract: the only three synthetic `input` dispatches
  are :1430 (number_field X — input+blur, correct seam), :1477 (a2i — fixed), and :1877 (faction
  name — a local-`editing`-signal widget committed via the Save-button POST and asserted on the
  POST, not the digest; source-verified unaffected by T-785).

**Cross-cutting:** no wasm panics in ~10 headless sessions; repo tree untouched (no perturbation
was ever applied to a repo file — all probes were external); probe artifacts confined to the
session scratchpad and the throwaway chrome profiles (deleted).

## What I attacked and FAILED to break (nobody needs to re-audit these)
1. The four Identity text fields' focus retention under real per-char CDP typing — ~60 characters
   across 6 fields, zero focus losses, zero node replacements, zero chord leaks while focused.
2. The single-selection no-op skip and Escape-abandon (no write, no undo step, both blur paths).
3. ROTATION / number_field digit entry (the named regression trap).
4. Mission title, mission search, ORBAT inspector inputs (healthy-field traps).
5. The O-3 acceptance measurement itself, including the DOM-order falsification (the stack, not
   luck, delivers the arsenal on top) and the elementFromPoint hit.
6. The O-5 acceptance pair (hint + Save Version) and export-vs-Save via the strip.
7. Every reachability route to a z-50/z-50 tie (Save→ORBAT, Settings→Arsenal, ORBAT→Faction
   Manager, keyboard-driven opens — all scrim-blocked or nonexistent).
8. The T-787 geometry at all three viewports including equality, hit-tests, scroll reach,
   chevrons, collapsed rails, and the STATUSBAR_H_PX == painted-height tie.
9. The a2i smoke's fail-closed property and the claimed isolation of the :1877 faction input.
10. The reconcile close+reopen single-frame gap (claim 4) — no reachable trigger found from the
    real UI; stamps stayed correct through every close/reopen cycle driven.
