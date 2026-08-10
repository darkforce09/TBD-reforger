# Editor wave 206 — adversarial verify

**Target:** merged main `3692a76e` (base `8a4fed37`; merges `efbfb07c` T-796 · `aab595fd` T-804 ·
`3692a76e` T-810). SPA :3000 (trunk DEBUG watching main), API :8080, dev-login `?role=admin`.

**Method:** a stdlib-only CDP driver written for this session (Python `socket` WebSocket + Chrome
DevTools Protocol; playwright `chromium-1228` `chrome-linux64/chrome` `--headless=new`,
`--use-angle=swiftshader --enable-unsafe-swiftshader`, fresh browser + profile per probe,
`?force=webgl&sat=preview`). 8 live probe scripts (driver smoke, A T-796 drag, B T-804 chip,
restore/reload isolation ×2, C/C2/C3 T-810 picker, D T-785/T-813 regressions, E register).
Pixel analysis via a stdlib PNG decoder + crop-diff. Missions created/saved/deleted through the
same `/api/v1` lifecycle the smokes use; **all `VERIFY206 *` DELETEd (204s), zero leaked**.
Cargo suite via `wave.sh test --slice T-810 -p website-frontend` into the private
`~/.cache/tbd-target-T-810` (deleted after). **4 pin perturbations across 4 files**, each RED with
its verbatim message, each restored byte-exact (`git diff --exit-code` clean) + touched.

**Scope purity:** the wave touched EXACTLY 5 files (`asset_catalog.rs`, `attributes.rs`,
`eden_top_strip.rs`, `mission_editor.rs`, `yrs_persist.rs`). Diff-of-diffs empty for all three
merges (merge delta byte-identical to slice delta): T-796 vs `353be3e0`, T-804 vs `003d00ca`,
T-810 vs `47897f26`.

---

## Findings

`SEVERITY | file:line | what is wrong | how you proved it`

**NIT | apps/website/frontend/src/eden_top_strip.rs:1652 (data-draft-chip) ← yrs_persist.rs:839 `note_flush_completed` | The "Draft saved just now" chip appears on BOOT of any content-bearing mission (~1 s), BEFORE any operator edit — the hydrate persists the server content to IndexedDB, a real flush completes, and the chip surfaces it. The acceptance line "no chip on a mission that has never been edited" holds only under the code's own definition ("never edited" == content-empty); an operator reopening an unmodified saved mission still sees "Draft saved just now". | Clean fresh-profile boot of a 2-slot saved mission, ZERO operator actions: chip flips `false→true` at t+1.0 s with `last_flush_ms=1786395829672`, while `__editorHistory.can_undo()==false` and `__missionPersist.edit_persist_count()==0` — a flush completed with no edit and no undo step. Honest by construction (a local draft genuinely IS written on hydrate; the T-374 content guard passes because content exists) and the two-layer tooltip explains the model, so this is legibility nuance, not a lie and not a data risk. The TRULY never-edited case is correct: a brand-new EMPTY mission shows NO chip and `last_flush_ms` stays `null` through 8 s of idle (multiple debounce ticks) — the content guard refuses the empty write, so no flush completes.**

No BLOCKER, no MAJOR.

---

## Is main safe to build the next wave on? **YES.**

Tree clean at `3692a76e` (this report is the only addition); frontend suite **1167 / 0 / 0**
(+19 over wave-205's 1148, matching the 19 new `#[test]` fns), re-green after every perturbation
restore; all 4 perturbations went verbatim-RED across 4 different files; scope purity exact for all
three commits; every registry ACCEPTANCE for all three tickets measured green live; the emergent
T-810 pair (Retry = `window.location.reload` with unsaved edits) is SAFE — edits survive via IDB
restore and a conflict modal explicitly protects them. All `VERIFY206 *` missions deleted, all
chromium dead, private target dir removed, no DB flip performed.

---

## Verified clean (measured)

### T-796 — a comment can be DRAGGED (probe A, live)
- **Place → drag 90 px:** placed via the RMB `Place Comment` row; camera scale 0.5, a 90 px drag
  moved the stored `x` by **+180.0 m** (`dx=180.0, dz=0.0`, expected 180.0) — dock/`compile_save_json`
  readback. **ONE Ctrl+Z fully restored** (`restored: true`, undo depth back to base). Single-note
  drag = exactly ONE undo step (`undo_delta: 1`).
- **Mid-drag glyph follows the cursor:** authored crop lost ink and the cursor crop gained ink
  between the at-rest and mid-drag screenshots (crop-diff 5817 authored / 5250 cursor) — the O-7
  preview parity.
- **Click still selects (T-784):** clicking a note selected it (`sel == ["note-a"]`).
- **Precedence boundaries:** note ON a unit → **the unit wins** (slot moved +120 m, note moved
  (0,0), 1 undo). Empty ground near a note → **marquee** (note moved (0,0), 0 undo, selection
  cleared to 0). Rotate-mode ring over a comment under the band → **the ring wins** (v1 selection
  kept, note moved (0,0), rotate txn minted 1 undo).
- **Three non-commit exits, each probed after:** wrong-button release (synthetic `pointerup`
  button=2), zero-delta release (out past threshold and back to the press point), and
  `pointercancel` mid-drag — **each left the stored position unmoved ((0,0)), minted 0 undo, and
  left the authored comment crop byte-identical to the at-rest baseline (crop-diff 0)**: no stale
  preview position anywhere.
- **Multi-note drag = N undo steps (the DISCLOSED class):** 2 selected notes dragged −80 m
  together minted **2 undo steps**; first Ctrl+Z restored note-b, second restored note-a — every
  moved note recoverable, and the single-note case above is exactly one.
- The persisting amber-ring glyph look is EXPECTED (glyph restyle deferred to T-808/W207), not
  probed as a finding per the brief.

### T-804 — the draft-saved chip (probe B, live)
- **Edit → chip within 6 s with fresh recency:** chip appeared **0.90 s** after a slot drag; the
  `last_flush_ms` bridge read a **39 s** age at that instant (the boot flush), then the edit's own
  flush advanced it (b3 read "5s ago"). Tooltip carries the two-layer copy (`/Save Version/`).
- **Idle → counts up:** "Draft saved 5s ago" (t+11 s) → "Draft saved 26s ago" (t+32 s), monotone.
- **Inline, not fixed:** chip `position: static, display: block`, sits in the strip band
  (top 4 / bottom 20, within 48 px) — no new fixed element.
- **Save-dialog rect smoke still green with the chip mounted:** version input in-viewport at
  **1920×1080 (top 494.5 / bottom 520.5)** AND **1366×768 (338.5 / 364.5)**, focused, closed by
  ONE Esc, chip survived the Esc — the T-789 portal contract holds.
- **No new Esc consumer:** opening Controls Hint then one Esc closed the hint and left the chip
  (and its text) intact; source: the chip block is a passive `<span>` with no `on:keydown` and no
  `modal_stack` entry (the only Escape/modal_stack lines in the diff are the comment stating it
  deliberately adds neither).
- **The T-779 ack is honest (source-verified):** `note_flush_completed()` has EXACTLY ONE call
  site — `run_save`'s success branch (yrs_persist.rs:839), after every guard's early `return` AND
  after the newly-added IO-error `return` (`+ return;` in the diff). The error branch cannot stamp.
- **Never-edited = no chip:** truly-empty mission shows no chip, `last_flush_ms` null through 8 s
  idle (the content guard refuses the empty write). (The content-bearing boot case is the NIT
  above.)

### T-804/T-810 — the emergent pair: Retry reload preserves unsaved work (isolation probes)
- **Retry = `window.location.reload`** (source: attributes.rs:1377, inside the `type-picker-retry`
  handler; the empty-state surface is pin-proven).
- **A real `window.location.reload()` with unsaved edits preserves them:** with the SPA's rotated
  refresh token kept (auth seed removed before reload so the live session survives), an unsaved
  slot edit (v0 → 6480) SURVIVED the reload (`reload_doc_v0 == 6480`), the `beforeunload` guard
  fired, and a **conflict modal appeared ("Keep local copy" / "Load server version")** protecting
  the local edit. A Retry cannot silently lose operator work. (An earlier "edit lost" reading was a
  fresh-dev-login two-page artifact — a second login is not a model of a reload; the authoritative
  same-session reload is clean.)

### T-810 — searchable TYPE picker + Revert + axis colours (probes C/C2/C3, live)
- **TYPE is a searchable picker, not freetext:** the field renders a `<button>`
  (`type-picker-trigger`), not an `<input>`; clicking opens the popover; search 'rifle' filtered to
  `["US Rifleman","US Automatic Rifleman"]`; picking US Rifleman wrote the **canonical**
  `assetId = {26A9756790131354}…Character_US_Rifleman.et`.
- **ASSET-RESOLVES clears:** a slot seeded with `assetId="bogus-unresolvable-xyz"` showed the
  validation chip at `issue_total=1` with an `ASSET-RESOLVES` finding; after picking the real
  rifle the chip dropped to **`issue_total=0` ("No issues")**.
- **Revert, single-select:** edited TYPE (→ cleared) + Role (→ "ZZ role"); Revert restored BOTH to
  pre-open (`assetId` back to the rifle canonical, role back to "Rifleman") — readback equality.
- **Revert, DIFFERING MULTI (the case a batch would flatten — PROVEN):** v1 rot 10 / v2 rot 45;
  apply-to-all set both to 90 in **ONE undo step**; Revert restored **v1→10 and v2→45, distinct**
  (`distinct_preserved: true`) — the per-slot correctness rationale holds; a homogeneous batch
  could not express it.
- **Axis colour chips:** X/Y/Z/Rotation carry **4 distinct computed colours**
  (`bg-red-500` oklch(.637…) / `bg-emerald-500` .696 / `bg-sky-500` .685 / `bg-amber-500` .769),
  all `aria-hidden`.
- **Advanced freetext:** works on single-select (visible; a hand-typed id committed to `assetId`).
  On a DIFFERING-TYPE multi (v1 rifle / v2 empty), the trigger reads "Multiple values", the picker
  carries the "Apply Type to all" box, and the advanced freetext is **hidden** (the documented
  trade). Apply-to-all via the picker wrote BOTH slots to Medic in **ONE undo step** (T-788 batch).
- **Esc ladder (picker → field → modal, one per press):** Esc#1 closed the popover only (modal
  stayed); Esc#2 (freetext focused) left the modal open and blurred the field; Esc#3 closed the
  modal.
- **Empty-catalog surface:** pin `the_empty_catalog_shows_cause_and_retry_not_a_dead_list` green
  and `catalog_leaf_count` pure-fn proven; not forced live (would require an `is_current` DB flip
  and only re-confirm rendering the pin already covers — the Retry safety is proven above).

### T-785 / T-813 — regressions under the new TYPE surface (probe D, live, REAL keystrokes)
- **ROLE focus retention across a WHOLE word:** typing "Sniper" char-by-char kept focus on the
  **same input node every keystroke** (`sameNode: true` ×6) — the snapshot Effect did NOT add a
  doc-tick re-render loop; the node survives the word. Committed once. (The accumulated value string
  garbles under CDP double-injection — a harness artifact; focus retention is the regression test
  and it held.)
- **No-op focus+blur writes nothing:** single-select Role focus+blur → unchanged, **0 undo**;
  differing-multi (Alpha/Bravo) Role focus+blur with zero typing → both unchanged, **0 undo**
  (the T-813 wipe class stays fixed).
- **ROTATION digit entry:** focus held on the same node through "135"; committed to 315.0.

### Register re-checks (probe E + suite pins)
- **Key-map parity:** press 2 → Translate plate on + hint "Translate"; 3 → Rotate + "Rotate";
  1 → No widget + "No Widget" — plate and hint agree on all three (wave-205 spot).
- **One-hint-home:** exactly **1** Controls Hint entry (Help menu).
- **Comment click-select (T-784):** clicking a comment selects it.
- **Save-dialog portal / rects at both viewports:** measured green (T-804 §above).
- **Suite-pinned green** (in the 1167): composition stamp, armed pointerup placement, zone-draw
  Esc, keep-multi, ring boundary, export latch, validation chip, plates-subscribe-to-generation,
  merged tree / failure view / recently-placed. (The Markers-tab selector missed in probe E — a
  probe-selector artifact; the pointer-commit path was exercised end-to-end throughout probe A.)

### Hollow-pin sweep (4 perturbations, 4 files — each RED then restored byte-exact)
1. **attributes.rs** `axis_chip_class` "Y" `bg-emerald-500`→`bg-red-500` → RED
   `axis_chip_class_is_three_distinct_axis_colours_plus_rotation` ("got bg-red-500/bg-red-500/bg-sky-500", left 2 right 3).
2. **asset_catalog.rs** `catalog_leaf_count` `usize::from(payload.is_some())`→`1` → RED
   `catalog_leaf_count_counts_leaves_not_folders` (left 10 right 5).
3. **eden_top_strip.rs** `RECENCY_JUST_NOW_MS` `5_000.0`→`3_000.0` → RED
   `draft_recency_reads_up_the_ladder` ("4s ago" vs "just now").
4. **mission_editor.rs** `dragged_comment_points` filter `d == &p.id`→`d != &p.id` → RED
   `dragged_points_are_the_document_notes_filtered_by_id` ("note-a" vs "note-b").
Each restored via `git show HEAD:` / `git checkout`, `git diff --exit-code` clean, then touched.

**Suite:** website-frontend **1167 / 0 / 0** (18.23 s, private dir `~/.cache/tbd-target-T-810`,
deleted after). 19 new `#[test]` fns (asset_catalog 3 · attributes 8 · eden_top_strip 1 ·
mission_editor 7) — the +19 over wave-205's 1148.

## Attacked and FAILED to break
Scope purity of all three merges (diff-of-diffs empty ×3) · the comment drag delta (90 px → +180 m,
readback) · single-note one-undo · mid-drag glyph-follows-cursor (crop-diff) · click-still-selects
· note-on-unit precedence (unit wins) · empty-ground-near-note marquee · rotate-ring-over-comment
precedence (ring wins) · all three non-commit exits (stored unmoved, 0 undo, authored crop
byte-identical) · multi-note N-undo disclosure + full recovery · the draft chip's edit→appear
timing, count-up, inline placement, tooltip, and Save-dialog rect at both viewports · no-new-Esc-
consumer · the T-779 single-call-site flush ack · never-edited-empty = no chip · the emergent-pair
reload (edits survive + conflict modal) · TYPE picker search/pick/canonical-id · ASSET-RESOLVES
clear · single-select Revert · **differing-multi Revert restoring distinct per-slot values** ·
4 distinct axis colours · advanced freetext single vs hidden-on-differing-multi · apply-to-all
one-undo batch · the 3-press Esc ladder · ROLE focus retention across a whole word · no-op-blur
writes-nothing (single + differing multi) · ROTATION digit entry · key-map 1/2/3 plate+hint parity
· one-hint-home · all 4 pin perturbations (verbatim RED, byte-exact restore) · the full 1167 suite.

## Environment left as found
HEAD `3692a76e`, `git status` clean (this report the only addition; all 4 perturbed files
`git diff`-identical to HEAD and touched). All `VERIFY206 *` missions DELETEd (204s; final API
list shows zero). Chromium all dead (`ps` shows 0 `chrome-linux64/chrome`); all `profile-206-*`
dirs removed; playwright cache intact. Private target dir `~/.cache/tbd-target-T-810` deleted after
the final suite run. No DB rows touched beyond the deleted probe missions; no `is_current` flip
performed. No packages installed.
