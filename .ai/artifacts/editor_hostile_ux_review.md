# Mission Creator — hostile UX/UI review

**Written 2026-08-09.** Driven in real Chrome 151 (CDP/puppeteer) against `localhost:3000`,
release wasm (`trunk serve --release`, pinned by `ps`), viewport 1920×1080 unless stated, admin
dev session. Throwaway mission `UXREVIEW-A` (`1877c175-…`); the pre-existing `sds` draft was never
written to.

Judged against `.ai/artifacts/editor_chrome_direction.md`, the Eden corpus, and the 191-id census
(`eden/gap_analysis.md`). **The census is stale** — the editor factory shipped past it. Context
menu, markers, triggers, compositions, comments, connections, snap grid, widgets 1/2, cut, Ctrl+A,
scale bar and layer-create all exist now despite `missing` rows. I judged the live app.

Read §7 first if you read nothing else: it is what I did **not** cover.

---

## 1. Fix these first

1. **F-01 / F-26 — Typing into an attribute field loses focus after one character, and the rest of
   your word executes as editor shortcuts. In multi-edit the stray character is written to every
   selected slot.** Breaks the core authoring loop; corrupts N slots at a time.
2. **F-02 — Layer rename opens a field that never receives focus at all.** Renaming is impossible.
3. **F-03 — Markers are invisible as authored objects**: icon choice and caption never render.
4. **F-04 — Save Version hides the version field off-screen** while demanding you pick a version.
5. **F-05 — Dead asset catalog shows one flat sentence** with no cause and no retry.

Then the second tier, from the later passes:

- **F-30** — composition placement is a silent dead end (save + arm work, the map click does nothing)
- **F-32** — two tabs diverge with no warning; the conflict modal doesn't say which button destroys work
- **F-33** — an `enlisted` account can open the editor with full chrome
- **F-34** — the two exports disagree about the mission's name and player count
- **F-31** — `Esc` cancels everything except an in-progress zone draw

Then, from the Eden pass — and **F-25 first**, because it is a one-line change that gets harder to
fix every day the group uses the editor:

6. **F-25 — the widget number keys are off by one against Eden.** `1`/`2` do the wrong thing for
   anyone with Eden muscle memory.
7. **F-24** — no visible save safety net, where Eden auto-saves on a timer.
8. **F-22** — three asset trees where Eden has one, and no recently-placed list.
9. **F-23** — raw class-name box where Eden has a searchable picker, and no Cancel.

---

## 2. S1 — loses work, blocks a task, or lies about state

### F-01 · Attribute text fields drop focus after the first keystroke; remaining characters fire global shortcuts
**Where** Attributes ▸ Identity ▸ ROLE (double-click a unit, or outliner row). Reproduced twice.
**What happens** Click the field → `activeElement` is `INPUT`, correct. `Ctrl+A` behaves correctly
(does *not* hijack to select-all-entities — I tested that hypothesis and it was wrong). Then type
`AT Rifleman`:

| after | field value | focus | side effects |
|---|---|---|---|
| typing | `A` | `BODY` | right dock collapsed, `GRID off` → `GRID move` |

Only the first character lands. Focus is lost, and characters 2..n go to the window keydown
handler: the `R` in "Rifleman" hit the dock-collapse binding, and grid mode changed.

**Blast radius — mapped in a third pass.** I probed every reachable input by clicking it, typing
`qqq`, and checking whether the tagged DOM node survived:

| input | result |
|---|---|
| Attributes ▸ Identity ▸ **Asset id** | **node replaced after 1 char** → `q` |
| Attributes ▸ Identity ▸ **ROLE** | **node replaced after 1 char** → `Aq` |
| Attributes ▸ Identity ▸ **ROLE DESCRIPTION** | **node replaced after 1 char** → `q` |
| Attributes ▸ Identity ▸ **TAG** | **node replaced after 1 char** → `q` |
| Attributes ▸ **multi-edit** text field | **node replaced after 1 char** → see F-26 |
| Layers ▸ **Rename** | never focuses at all (F-02) |
| Attributes ▸ Transform ▸ X / Y / Z / ROTATION | **fine** — typed `135`, focus kept |
| Mission title (top strip) | **fine** |
| Left dock mission search | **fine** |
| Marker caption | **fine** |

So it is **not** universal — it is the Attributes panel's text inputs plus layer rename. Number
inputs in the same panel are unaffected, which is why the mechanism reads as a controlled-component
remount on the store round-trip rather than a global keydown problem (`HYPOTHESIS`: the DOM node is
provably destroyed — `data-`tagged element gone after keystroke one — but I did not read the
component source).

**Why it matters** Naming a slot is the most common attribute edit a milsim author makes. Today it
silently truncates to one letter *and* reconfigures the editor around them — panels vanish, grid
mode flips — with no error. The author's mental model ("I typed a name") and reality diverge
completely. This is the single most embarrassing thing in the build: it will happen within the
first two minutes of anyone from the group touching it.
**Fix** The input is almost certainly a controlled component remounting on the store round-trip
(`HYPOTHESIS` — not verified in source). Keep the DOM node stable across the update, or make it
uncontrolled with a commit on blur/Enter. Independently, harden the global keydown guard: bail
when `document.activeElement` is an input/textarea/`contenteditable`, not only when a field is
*believed* focused — the guard is currently defeated the moment focus is lost.

### F-26 · In multi-edit, the F-01 stray character is written to every selected slot
**Where** Select N slots → right-click a selected unit → `Attributes…` → Identity → tick
`Apply to all` on a field → type.
**What happens** Same remount as F-01, but the one character that lands before the node dies is
committed across the whole selection. Driven with 9 slots selected: typed `qqq` into `Asset id`,
got `q`. Two independently re-opened slots (`n2`, `n7`) both then read `assetId = "q"`, and the
app's own validator lit up with **9 × `ASSET-RESOLVES`** errors, one per slot, each citing
`/editor/slots/N/assetId` — *"placed asset prefab \"q\" does not resolve in the live catalogue …
this placement will not spawn."*
**Why it matters** This is F-01 turned into a data-corruption multiplier. One mistyped word silently
invalidates every selected slot, and multi-edit is exactly the feature an author reaches for when
they have a lot of slots — so the blast radius scales with how much work is at stake. Undo will
walk it back, but only if the author notices; the panel reports success and the map looks fine.
**And it is expensive to reverse.** One `Ctrl+Z` after the fan-out took the validator from **9
errors to 8** — the apply-to-all writes one undo step *per slot*, so unwinding a mistyped multi-edit
over N slots costs N presses. Cheap to cause, slow to undo, and nothing tells the author how many
steps back the damage goes. (Contrast: a 9-slot *delete* is a single undo step.)
**Fix** Same root cause as F-01. Until it is fixed, multi-edit text fields are the most dangerous
control in the editor. Batch the fan-out into one transaction while you are in there.
**Credit where due:** the validator caught this cleanly and its message is the best copy in the
product — it names the bad value, the consequence, the JSON pointer, and two remedies.

### F-30 · Saving a composition works; placing one is a silent no-op you cannot exit
**Where** Select entities → right dock ▸ Compositions ▸ `Save composition… (N selected)` → name it →
Save → click the saved row → click the map.
**What happens** Save works: the row appears (`Compositions 1`, "by Dev O…", under `Uncategorized`),
and the name field is healthy (typed `qqq`, focus kept — not an F-01 casualty). Arming works: the
panel shows the **"click the map"** hint. Then the map click does **nothing** — `OBJ9 → OBJ9`. I
tried a plain click at two separate map points and a press-drag-release; no entity appears, the
count never moves, **and no console error or `pageerror` is emitted**. The arm never clears either,
so the hint sits there telling you to do the thing that doesn't work.
**Why it matters** Compositions are the "stamp a fireteam eight times" feature — the single biggest
time-saver for the kind of mission this tool exists to build. Today the author does the setup work,
gets every affordance telling them it's armed and ready, and the payoff silently never comes. A
control that states its next step and then ignores it is worse than a disabled one.
**Fix** Wire the place handler (or disable the arm and label the row "coming soon" until it exists).
**Census note** `COMP-PLACE-001` is marked `missing`/T-078 — the *save* half has since shipped, so
that row is stale, but the place half really is absent and the UI no longer admits it.

### F-31 · `Esc` cancels every transient surface in the editor except an in-progress zone draw
**Where** Right dock ▸ Zones ▸ `Circle` → click the centre → `Esc` → click anywhere.
**What happens** The second click still completes the circle. Verified twice, the second time in
isolation: zone count `2 → 3`, and immediately after `Esc` the panel **still prompts "click the
rim"** — the draw mode is genuinely still live. By contrast an armed **marker** disarms correctly on
`Esc` (count unchanged, later map click places nothing), and `Esc` closes the File menu, the Export
dropdown, the Save dialog, ORBAT Manager, the context menu and the Attributes panel — all six tested
and all correct.
**Why it matters** `Esc` is the editor's universal escape hatch and it is honoured everywhere except
the one place where the consequence is a permanent object appearing in the mission. The author who
starts a zone by mistake has no way to back out, and the mental model "Esc gets me out" is exactly
what makes them press it and then click again.
**Fix** Cancel the pending draw on `Esc` and clear the rim prompt.

### F-02 · "Rename layer" opens a text field that never receives keyboard focus
**Where** Left dock ▸ Layers ▸ pencil icon (`title="Rename layer"`).
**What happens** Field appears pre-filled (`New Layer 1`), `focused: false`, `activeElement: BODY`.
Typing does nothing to it. With the field visibly open, pressing `g` flipped `GRID off` → `GRID
move` instead of typing "g". I attempted `Assault` via the create-flow inline field too — the layer
is still called `New Layer 1`.
**Why it matters** Layers are the organising tool for a mission with two squads and a vehicle
element. They ship un-nameable, so the panel fills with `New Layer 1/2/3` and stops being
navigation. Worse, the author's keystrokes are silently executing commands.
**Fix** Focus the input on mount (and select its contents); same keydown-guard hardening as F-01.
**Census note** `ATTR-FIELD-LYR-NAME` / `LAYER-CREATE-001` are marked `missing` "no UI reaches the
mutator". Stale: create works and a rename control exists — it just cannot be typed into.

### F-03 · Markers render as identical featureless dots; icon and caption never reach the map
**Where** Right dock ▸ Markers → pick icon → click map.
**What happens** Placed marker is a pale circle. Every icon type produces the same circle. The
caption field is labelled **"Caption shown on the map"** — I set `UXR CAP`; nothing appears at
5.58, 1.30 or 1.06 m/px. The dock row correctly reads `Attack (BLUFOR) · 4921, 5982`, so the data
is stored; only the rendering is absent.
**Why it matters** Markers *are* briefing symbology — their entire purpose is to be read off the
map. An author places "Attack", "Casevac", "Rally" and gets three identical dots, then re-opens
each to remember which is which. And the field copy states a behaviour the product does not have.
**Fix** Render the icon glyph and label on the map layer. Until then, change the placeholder — a
field that promises map text and delivers none is worse than an unlabelled field.

### F-04 · Save Version demands a semver in a field it renders off-screen
**Where** Top strip ▸ Save Version.
**What happens** Dialog copy: *"Versions are immutable — pick a new semver."* The VERSION input
measures `y = -22` — above the viewport top — at **both** 1920×1080 and 1366×768. NOTES, the size
estimate, the confirmation and the Save button are all on-screen and work; saving succeeded
(`Saved v0.1.0`, then `Saved v0.2.0`).
**Why it matters** The one decision the dialog insists on is the one thing the author cannot see or
change. They can only ship whatever was pre-filled, and cannot read what that was before clicking.
Immutable versions make a wrong number permanent.
**Re-verified three ways** (2026-08-09, second pass, clean reload): global input query gives
`VERSION` input at `y=-22` and its label at `y=-42`, `vis:false`, identical at 1920×1080 and
1366×768; the screenshot shows the dialog's content beginning at the clipped bottom edge of a field
above `NOTES`. The Save button sits at `y=146` and works at both sizes.
**Also:** the dialog opens showing a stale **"Saved v0.2.0"** from a previous save, before you have
saved anything in this session. With the version field invisible, the author cannot tell whether
that line describes the past or the present.
**Keyboard addendum (fifth pass):** the invisible field IS in the Tab order — focus lands on it two
Tabs after the dialog's ✕ — so a keyboard user can *blind-type* a version into a field they cannot
see. And the dialog has **no focus trap**: the Tab cycle runs ✕ → version → notes → Save and then
walks straight out into the left dock (`chevron_left → Layers → Locations`), with initial focus
left on the *opener* button rather than moved into the dialog.
**Correction to my own first reading:** I initially measured the dialog as clipped by 226px at
1366×768 with the Save button unreachable, and nearly filed "you cannot save on a laptop". That was
a stale DOM node. The real defect is narrower — the version field only — and that is what stands.
**Fix** Clamp the dialog into the viewport (it appears anchored upward from the button rather than
centred/constrained); clear the last-save message when the dialog reopens.

### F-32 · Two tabs on one mission diverge silently, and the prompt that catches it doesn't say what it will destroy
**Where** Open the same mission in two tabs (T-190 has a ticket for this; repro run on a throwaway).
**What happens** Neither tab warns the other exists. Tab B loaded the same `OBJ9`; I deleted
everything in B (`OBJ0`) while A still showed `OBJ9`. Both write drafts to the same per-account
IndexedDB key. On the next reload the app **did** catch it — credit where due, this is not a silent
clobber — and showed:

> **Unsaved local changes**
> Saved version v0.2.0 on the server differs from your local copy. Which version should win?
> `[Keep local copy]` `[Load server version]`

**Why it matters** Three problems with that modal. It **never says either option destroys work** —
"Load server version" discards the local document permanently and the copy doesn't warn. It offers
**no way to compare** (no object counts, no timestamps, no preview) so the author is choosing blind
between two things they cannot see. And it **misattributes the cause**: it frames a two-tab
divergence as a local-vs-server drift, so the author never learns that a second tab is what ate
their work, and will do it again.
**Fix** Detect the second tab at open (BroadcastChannel / storage lock) and say so. Failing that,
put counts and timestamps on both options and mark the destructive one.

### F-33 · An `enlisted` account can open the mission editor
**Where** `dev-login?role=enlisted` → navigate to `/missions/:id/edit`.
**What happens** The editor **loads**. Full chrome — File / Edit / Arrange / View / Mission /
Environment / Help, undo/redo, ORBAT Manager, the docks, the mission document. No redirect, no
"forbidden", no read-only banner. `mission_maker` also loads, as expected, with Save Version, Export
and ORBAT present.
**Why it matters** The route is documented as `mission_maker`+. An ordinary member of the group can
open any mission in the editor and start changing it.
**Backend verified in a follow-up pass — it is safe.** With an `enlisted` token,
`POST /missions/:id/versions` → **403** and `GET /missions/:id` → **404**. So this is a UI-route
gap, not an authorisation hole. That makes the failure mode *worse for the author, not the data*:
an enlisted user can open the editor, spend an hour building, and discover at Save Version that
nothing they did could ever be persisted.
**One open question I could not resolve:** the enlisted shell rendered **8 objects** (`UNA 8 ·
TOTAL 8`) despite the mission GET returning 404 — content of unclear provenance, possibly a
cross-account local draft leaking through the IndexedDB scoping. Worth one code look
(`yrs_persist.rs` key scoping); I am flagging it, not asserting it.
**Fix** Gate the route (or render read-only with a banner), and check the draft-key scoping above.

### F-34 · The two exports disagree with each other about the same mission
**Where** Export ▸ `Export JSON` vs `Export Compiled`, captured by intercepting the blob.

| field | Export JSON (8,600 B) | Export Compiled (3,379 B) |
|---|---|---|
| mission name | `"UXREVIEW-Aqqqqqq"` (live editor title) | `"UXREVIEW-A"` (**stale library-row title**) |
| players | `"maxPlayers": 0` | `"playerRange": [1,64]` ✔ |
| game mode | `"gameMode": ""` | *(not carried)* |
| version | `"version": "0.1.0"` | *(not carried)* — last save was **v0.2.0** |

**Why it matters** Two artifacts from the same menu, one mission, two different names and two
different player counts. The title split is the known `RowMirror` gap made visible — the editor
title never reaches the library row, so the compiled export (which reads the row) ships under the
old name. `maxPlayers: 0` and `gameMode: ""` mean the JSON export silently drops what the author
chose in the create dialog, and the version field reports a save that isn't the latest.
**Fix** One source of truth for title/mode/players across both exports; mirror the title on edit.
**Positive, verified in a follow-up pass:** the compiled artifact itself is structurally sound —
the full captured payload **validates clean against `packages/tbd-schema/schema/mission.schema.json`**
(ajv, 2020-12 dialect, zero errors), even after this mission had been through attribute corruption,
a two-tab divergence and a conflict-resolve. The compile pipeline's discipline is real; the
disagreement above is a metadata-source problem, not a schema one.

### F-35 · `Backspace` hides the interface but leaves the validation legend, and offers no way back
**Where** Press `Backspace` over the map.
**What happens** Chrome hides correctly (buttons 62 → 19) and a second `Backspace` restores it — the
Eden parity works. But the **validation legend stays on screen** bottom-left, so the one thing
Backspace exists for (a clean screenshot of the map) still has a debug-looking panel in the corner.
And nothing on screen says how to get back — no hint, no overlay, no `Backspace` reminder.
**Why it matters** Small, but this is the feature people use to produce the images they post to the
community, so the leftover panel is visible precisely when the tool is being shown off. The missing
way-back is a classic footgun for anyone who pressed it by accident.
**Fix** Include the legend in the hide set; show a dismissible "press Backspace to restore" toast.

### F-36 · The validation error count fails WCAG AA contrast — and it is the only failure
`8 errors · 2 warnings` renders `rgb(239,68,68)` on `rgba(31,41,55,0.7)` at 14px — **3.9:1**, under
the 4.5:1 minimum. The earlier caveat about unmeasurable `oklab()` pairs is now closed: a second
pass converting oklab→sRGB and compositing alpha over the base background measured every visible
text pair on the idle chrome and **all pass AA** — worst is 5.96:1 (the top-strip `·` separators),
with the working chrome at 7–15:1. So the Aegis palette is fundamentally sound; the one genuine
failure is the red error count, which is also the text most worth reading. Darken its plate or
brighten the red one step.

### F-05 · Dead asset catalog states the failure and nothing else
**Where** Right dock ▸ Factions / Vehicles, on a database with no current modpack.
**What happens** *"Could not load the catalog."* `GET /api/v1/registry` and `/registry/compat`
return 404 (`resolve_modpack` → "no current modpack configured"). No cause, no retry, no link. The
search-syntax hint chips (`class: · mod: · *Rifleman · /regex/`) keep rendering above the error, so
the panel looks operational. Applying the committed dev seeds fixed it and the tree populated
(NATO ▸ US_Army ▸ US Rifleman…), after which placement worked normally.
**Why it matters** The root cause here was my empty dev DB, not your code — but the *handling* is
the finding, and it generalises: any registry hiccup in front of the community produces a
five-word dead end in the panel that is the whole point of the editor. It also blocks placement
entirely, so it is a task-blocker whenever it fires.
**Fix** Distinguish "no modpack configured" from "request failed"; offer Retry; in development,
name the fix. Hide the search-syntax affordances when there is no catalog behind them.

---

## 3. S2 — makes the author think about the tool

Ranked.

**F-06 · Transform, grid and snapping modes are keyboard-only — no menu row, no toolbar, no
on-screen state.** `1`/`2` (widgets), `G` (snap grid), `[`/`]` (snap step) exist solely as key
bindings. Nothing in the chrome names them, shows which mode is active, or offers a mouse path.

**Corrected after viewing the Eden frames.** My first pass claimed "Eden puts all of it in Edit plus
a 25-button strip" and treated TBD's two-item Edit menu as the defect. That was wrong on both
counts, and frame `163508` disproves it: **Eden's menu-bar Edit contains no clipboard verbs at
all** — it is Undo, Redo (dimmed when unavailable), Select All on Screen, then Transformation
Widget ▸, Grid ▸, Vertical Mode ▸, ✓ Toggle Surface Snapping, ✓ Toggle Waypoint Snapping, Phase ▸,
Asset Type ▸. Cut/Copy/Delete live in the **right-click** Edit submenu (`161640`), which is exactly
where TBD puts them. TBD's context menu is a faithful clone of Eden's, greyed rows and all.

So the real gap is narrower and different: Eden gives every transform/grid/snap mode **three**
surfaces — a menu row carrying its own shortcut, a toolbar button, and a submenu of the six widget
modes with icons (`No Widget`, `Translation`, `Rotation`, `Area Scaling`, `Area`) — while TBD gives
them **none**. Eden's toolbar (`164000`, y 22–40) is ~25 buttons in deliberate separator groups. I read all 16
hovered tooltips off the pixels — in sweep order: `New (Ctrl+N)` · `Open (Ctrl+O)` ·
`Save (Ctrl+S)` · `Undo (Ctrl+Z)` · `Redo (Ctrl+Y)` · `No Widget (1)` · `Translation Widget (2)` ·
`Rotation Widget (3)` · `Area Scaling Widget (4)` · `Area Widget (5)` ·
`Toggle Widget Coordinate Space` · `Toggle Vertical Mode (adiaeresis)` ·
`Toggle Surface Snapping (')` · `Toggle Translation Grid (odiaeresis)` · `Toggle Rotation Grid` ·
`Toggle Area Scaling Grid`. All 16 match the batch-04/05 transcriptions exactly — no corpus errata.

**The mechanism worth copying: 13 of the 16 tooltips carry their keyboard shortcut in
parentheses.** Eden's toolbar is how an author learns Eden's keyboard — the mouse path and the key
are taught by the same control. TBD's tooltips (`Factions`, `Vehicles`, `Collapse panel`,
`Rename layer`…) name the function but never the key, and TBD's keys live only behind
`Help ▸ Keyboard Shortcuts`. Also verified from pixels on frame `164031`: **the disabled Redo
button still renders its tooltip** — the direction doc's rule, confirmed in the wild. TBD has no
second row at all.
**Fix:** add `Select All on Screen` and a widget/grid/snapping block to the Edit menu with their
keys shown; ship the row-2 icon toolbar the direction doc already calls for. Do **not** move
clipboard verbs into the menu bar — Eden doesn't, and TBD already matches it.

**F-07 · Marker copy points at the map; the affordance is in the dock.** "Select a marker to caption
it or nudge its position" — clicking the marker on the map opens nothing. The caption/X/Z/Delete
editor only appears when you click the marker's *row* in the dock list. **Fix:** on-map selection
should open the same editor.

**F-08 · The icon picker is 64 rows of alias soup.** Every row wears the same generic pin glyph;
the second column is raw schema slugs; case-duplicates are all listed as separate rows
(`Waypoint`/`waypoint`/`Waypoint2`/`waypoint2`, `Objective`/`Obj`/`Target`, `mark`/`marker`/
`point`). The display name truncates to `Ob…` while the slug gets full width. Choosing by sight is
impossible. **Fix:** one row per canonical icon, real glyph, slug to tooltip/search only.

**F-09 · Bottom chrome is four competing objects.** The status bar is genuinely full-width with
CUR/OBJ/SEL/SZ/scale — the direction doc's main ask, delivered. But the Select/Ruler/LoS pill still
floats centred *above* it, the validation legend floats bottom-left, and a `GRID off` chip floats
bottom-right. Eden has one bar plus one loud primary action. Four focal points where the doc
specifies one. **Fix:** merge the mode toggles and GRID into the bar; anchor validation to it.

**F-10 · Map symbology is dot soup.** Slots are plain circles (amber + heading arrow when selected,
pale blue otherwise); markers are near-identical circles. No role glyph, no leader mark, no
side-distinct symbol, and nothing separating "unit" from "marker" at a glance. At two squads the
map is already unreadable without the outliner. Milsim authors read maps by symbology — this is the
comparison they will make first. **Fix:** side-coloured role glyphs for slots, distinct marker
glyphs (F-03), distinct shape class between the two.

**F-11 · The validation error appears only once you author something.** An empty mission reports
"No issues". Placing one *marker* flips it to red `1 error — V1-PLAYER-SPAWN: declares a faction
but has no slots`. The empty mission was equally unspawnable; the first mark you make summons an
error about something you didn't touch. The message itself is well written. **Fix:** evaluate V1
from load, not from first content.

**F-12 · ORBAT "ADD SQUAD / GROUP" dismissed the whole modal on a real click** while creating the
squad — one action, two effects. With synthetic pointer events the modal correctly stayed open, so
this is likely activation-on-`pointerup` plus the list reflowing under the cursor
(`HYPOTHESIS`). Self-closing dialogs destroy trust in a panel you're meant to work in for minutes.

**F-13 · A 15-digit coordinate in an editable field, and no units anywhere.** The marker Position
field is editable at `4921.268596532038`.

**Corrected after viewing the Eden frames.** I claimed "Eden prints whole metres". It does not —
frame `170422`'s status bar reads `X⊥ 8762.61 m · Y↑ 12381.3 m · Z⊿ 24.5396 m · 3.40597 m/pix`, so
TBD's 3-dp status readout is **normal**, not noisy, and that half of the finding is withdrawn.
What survives: Eden's *input* fields print 3 dp (`3713.068`, `10516.866` in `163121`), never 15,
and **Eden suffixes its readouts with `m` and prefixes them with axis glyphs**, where TBD prints a
bare `4921.269`. **Fix:** round the editable field to 3 dp and add unit suffixes to the status bar.

**F-27 · Multi-edit is real, well-designed, and hidden behind the one gesture nobody uses.** The
panel itself is the best-designed surface in the editor — header `9 slots selected · multi-edit`,
copy that reads *"Fields that differ across the selection are blank and locked. Tick Apply to all
to overwrite that field on every selected slot"*, `—` placeholders on differing fields, per-field
opt-in checkboxes. But it opens from **exactly one path**: right-click a selected entity ▸
`Attributes…`. Both natural gestures destroy it first — with 9 slots selected, a **map double-click**
gives `SEL1` and single-slot attributes, and an **outliner double-click** does the same. Source
explains it: `open_attrs_modal` keeps the selection only when `sel.len() > 1 && sel.contains(&id)`
(`editor_ops.rs:1241-1243`), and both double-click paths re-select the single entity on the first
click before activate fires. **Why it matters:** T-649 shipped a good feature that most authors will
never discover, and worse, the gesture they *will* try silently discards a nine-slot selection with
no hint that multi-edit existed. **Fix:** don't collapse the selection on a click that lands inside
the current selection; or open multi-edit from double-click when the target is already selected.

**F-28 · Export Compiled downloads the file twice, and neither export is gated on validation.**
Instrumenting `URL.createObjectURL` / `a.click`: **Export JSON** fires once (7,205-byte
`application/json`, `mission-<id>.json`) — correct. **Export Compiled** fires **twice** per single
click — two `createObjectURL` calls and two anchor clicks for the same 3,213-byte blob — so the
author gets two copies of `mission-<id>.compiled.json` in their Downloads folder. Same
double-activation smell as F-12. Separately, **both exports proceeded happily while the mission
carried 9 `ASSET-RESOLVES` errors** stating those placements "will not spawn". For the raw JSON dump
that is defensible; for the **compiled** artifact — the thing that goes to the game — silently
emitting a mission the editor knows is broken is not. **Fix:** de-duplicate the compiled export
handler; make compiled export either refuse on errors or force an explicit "export anyway"
acknowledgement.

**F-29 · The Attributes panel does not follow the selection.** With attributes open on one slot,
`Ctrl+A` took the selection to `SEL9` while the panel kept showing `Rifleman · n2` and single-slot
fields. The panel and the status bar now disagree about what is being edited, and an author who
edits a field in that state reasonably expects it to hit the nine. **Fix:** re-render the panel
against the live selection, or close it when the selection changes underneath it.

**F-25 · The widget number keys are off by one against Eden — the muscle memory your group already
has will do the wrong thing.** I read all 16 toolbar tooltips off the pixels; Eden's widget row is
numbered explicitly:

| key | Eden (`164038`–`164107`) | TBD (`eden_help.rs:198-205`, driven and confirmed) |
|---|---|---|
| `1` | **No Widget** | **Translate widget** |
| `2` | **Translation Widget** | **Rotate widget** |
| `3` | **Rotation Widget** | — |
| `4` | Area Scaling Widget | — |
| `5` | Area Widget | — |

An author who uses Eden daily presses `1` to *drop* the gizmo and instead arms translate; presses
`2` for translate and gets rotate; presses `3` for rotate and gets nothing. Every one of those is a
silent wrong-mode, and the whole point of taking Eden's number keys was that the mapping came with
them. The census noted "Eden's own `1`–`5` direct widget keys are all free in TBD, which dissolves
the clash" — TBD then took the keys and dropped the mapping.
**Fix:** shift to Eden's numbering (`1` No Widget, `2` Translate, `3` Rotate) and reserve `4`/`5`.
This is a one-line change now and an unfixable habit later — it is the cheapest item in this report
and the most likely to be noticed on day one.

**F-22 · The Asset Browser is split three ways where Eden has one tree, and there is no "recently
placed".** Eden's F1 Objects tree (`165952`) is **one tree per faction covering everything** —
under `NATO` sit Anti-Air, APCs, Artillery, Boats, Cars, Drones, Helicopters, Men, Men (Combat
Patrol), Men (Special Forces), Planes, Submersibles, Tanks, Turrets. TBD splits the same material
across separate **Factions**, **Vehicles** and **Objects** surfaces, so before you can find a thing
you must already know which category TBD filed it under — a question about the tool, not the
mission. Eden also carries an **`Assets | History`** tab pair; History is the recently-placed list,

which is what you want when laying down eight of the same fireteam. TBD has Favourites (manual,
opt-in) but nothing automatic. **Fix:** one tree per faction spanning all placeable kinds, with the
existing side chips as the filter; add a recently-placed list. (Census `RIGHT-MODE-001` marks this
`partial` — seeing both side by side, the split is the weaker design, not merely a different one.)

**F-23 · Attributes: a raw class-name text box where Eden has a searchable picker, and no way to
cancel.** TBD's Identity tab asks for `Asset id — empty uses the faction default` as free text.
Eden's `Object: Type` (`163121`) is a **searchable tree** — Cars / Drones / Helicopters / Men ▸ with
a magnifier — so the author browses instead of recalling a class string. Eden's dialog also commits
on **OK** and has an explicit **CANCEL**; TBD's panel applies live and offers only ✕, so there is no
back-out path — which matters much more given F-01 can corrupt a field mid-edit. Eden further
**colour-codes the axes** (Position/Rotation X=red, Y=green, Z=blue); TBD's Transform tab is
monochrome. **Fix:** the axis colours are near-free and worth taking; the type picker and a
Cancel/revert are the substantive items.

**F-24 · Eden auto-saves on a configurable interval; TBD never tells the author their work is
safe.** Eden's Preferences (`163916`) opens with **`Saving ▸ Auto-save: 15 min`** as a first-class
setting. TBD has no server autosave by design — but its only safety net is an invisible IndexedDB
draft on a ~5 s debounce, with no "last saved" time, no autosave indicator, and no setting. The
author's evidence that anything is persisted is a single `•` dirty dot. Combined with F-04 (the
version field they cannot see), the whole save story is the weakest area of the editor.
**Fix:** surface the local draft — "draft saved 12s ago" next to the dirty dot — and let the
author see, at a glance, that closing the tab will not cost them the session.

**F-21 · Vehicles cannot be authored at all in a documented dev setup, and both routes fail
silently.** The committed seed `registry_dev.sql` contains **zero vehicle rows** (8 characters, 13
gear items — confirmed by `GROUP BY category, kind`), so the Vehicles palette reads *"No placeable
vehicles."* even after a correct `make seed`. The second route, **ORBAT ▸ squad ▸ Add Vehicle**
(`title="Add Vehicle"`, clicked with a real trusted click at its true coordinates), is a **silent
no-op**: `Vehicles: 0` is unchanged, no picker opens, no toast, no error. **Why it matters:** a new
contributor — or you, on a fresh machine — cannot demo or test the vehicle half of the editor, and
the failure presents as "this button is broken" rather than "there is no catalogue". **Fix:** ship
a few vehicle rows in the dev seed; make `Add Vehicle` say why it can't when the catalogue is
empty (same class as F-05).

---

## 4. S3 — cosmetic, still worth a sweep

- **F-14** `View ▸ "Map layers — render host (T-159.28)"` — a ticket ID in a user-facing label, on a
  row that does nothing. **Verified by clicking it**: opacity 0.3, `cursor: default`, click is a
  no-op and the menu stays open. Ship it or drop it. (Same class as the t178 audit's "ticket
  jargon" note — **known since July**. The ORBAT footer jargon it flagged *is* fixed.)
- **F-15** One feature, two names, two menus: `View ▸ Controls Hint` and
  `Help ▸ Keyboard Shortcuts (Controls Hint)`. **Verified**: both open the identical
  "Controls — keyboard shortcuts" surface.
- **F-37** Disabled rows go silent — the direction doc's own rule, broken. The doc: *"disabled =
  dimmed glyph — and it still shows its tooltip"*, and Eden honours it (frame `164031`, disabled
  Redo still tooltips). TBD: the disabled `Play from Here` context row has **no tooltip anywhere in
  its chain**; the disabled `Select` container's title just repeats the word "Select". Meanwhile the
  **History button does it right** — disabled with `title="Version history (soon)"`, which is
  exactly the pattern; it was wrongly listed as dead chrome in my earlier pass and is hereby
  un-filed. Give every disabled row a "why" tooltip; History is the template.
- **F-38** The status-bar **OPEN button is confirmed inert by clicking it**: enabled styling,
  `title="Open"`, and a click produces no dialog, no navigation, no DOM change, no network request.
  It occupies Eden's PLAY SCENARIO slot — the loudest position on the screen — and does nothing.
  Hide it until it has a verb (upgraded from HYPOTHESIS; this was the plan's oldest unverified
  claim).
- **F-16** Implementation parentheticals in labels: `Mission Settings…`, `Briefing & Thumbnail
  (Mission Settings)…`, `Time & Weather (Mission Settings)…`.
- **F-17** `"1 slots · server cap 128 players"` — plural bug in the ORBAT header.
- **F-18** Left dock truncates `Default La…` at 240px while the eye/lock/edit/delete icon cluster
  keeps full width.
- **F-19** *(peripheral, outside the editor)* `/missions` features a **draft** mission as
  **"LIVE OPERATION — Command has flagged this operation as the priority deployment."** A draft
  called `sds` presented to the community as the priority op. Gate the hero on published status.
- **F-20** On a brand-new empty mission the console logs `[yrs-persist] T-374: refused to persist …
  no authored content (or that do not replay at all)` eight times in a few seconds. Harmless, but
  it reads like corruption to anyone who opens devtools.

---

## 5. Divergences from Eden I judged **fine** — examined, not missed

- **2D-only, no 3D viewport.** Correct for a browser planner; census `na` rows agree.
- **No F1–F6 key bindings on the palette tabs** (deliberate, T-180.5) — the icon tab strip with
  tooltips is a reasonable browser substitute.
- **Right dock has 7 tabs vs Eden's 6 modes** (adds Favourites, Zones; splits Factions/Vehicles).
  Defensible: TBD's slot/roster model genuinely differs from Eden's entity model.
- **Semver "Save Version" instead of Eden's file save.** Better fit for a shared library.
- **Map furniture Eden lacks** (grid labels, scale bar, contours, spot heights). Correct call — a
  top-down planner needs distance cues a 3D-first editor doesn't.
- **Context menu keeps Eden's disabled rows** (`Play from Here`, `Find in Config Viewer…`, `Save
  Custom Composition…`) greyed rather than dropped. Matches your state-vocabulary doc, and reads as
  "coming" rather than "missing". Fine — provided the list doesn't grow.
- **Transform ▸ formations** (Column/Wedge/Vee/Echelon/Line/File/Diamond) is a genuine addition
  over what the census describes. Worked as an accordion; I did not execute one (§7).

---

## 6. Corpus errata and Eden observations

**Coverage: all 75 frames personally viewed.** 16 at crop-level detail (`161727`, `163121`,
`163448`, `163508`, `163553`, `163608`, `163720`, `163901`, `163916`, `164000`, `165926`, `165945`,
`165952`, `170028`, `170354`, `170422`) covering every distinct UI surface, and the remaining **59
viewed as ten 6-up montage grids** in a final pass. The grids confirmed, from pixels, exactly what
the corpus's own analysis claimed about them: the batch-01 frames differ only by which context
submenu is open; the batch-02/03 frames are the same dialogs at different scroll positions; the
batch-04/05 frames are a frozen scene differing only by one tooltip; batch 06's frames differ only
by F-tab; batch 07's map region never changes; batch 08's are the panels-hidden zoom set. **No new
errata surfaced from the 59** — grid-level viewing would catch structural surprises but not
text-level ones, which is the right residual risk given the 16 deep reads covered every unique
surface.

**Corpus claims I independently confirmed:** amber `#C38114` is **hover**, not toggled-on — seen
three times (`164000` New button, `163508` Transformation Widget row, `163608` General… row);
`…` reliably means "opens a dialog"; spot heights are `△166` / `·27` / `·220` drawn horizontally,
never rotated, and contour lines carry **no** labels; contours are uniform 1 px with no index
weighting; the panel-collapse chevron is `«` / `»` in the panel's outer top corner. **No batch
document contradicted what I saw** — the README's corrections (04/05 misnamed, F1=Objects,
F2=Compositions) all held.

**Eden details worth having that the batch docs under-weight**, each now feeding a finding above:
Eden's menu-bar Edit carries **no clipboard verbs** (F-06); the status bar prints **unit suffixes
and axis glyphs** at 2–5 dp (F-13); attributes are **one scrolling dialog of labelled sections with
OK/CANCEL**, not tabs, with **axis-coloured** X/Y/Z (F-23); the asset browser is **one tree per
faction** plus an **`Assets | History`** pair (F-22); Preferences ships **auto-save** (F-24).
Two more, too small to file: Eden **dims empty categories** in the entity tree (OPFOR, Independent,
Civilian, Triggers, Systems, Markers, Comments all greyed when empty) — a cheap way to show shape
at a glance; and dependent controls grey out but stay visible (Environment ▸ Rain, disabled until
`Manual Override` is ticked) rather than disappearing.

**One layout fact that reframes the bottom bar:** Eden's bottom-right is `PLAY SCENARIO ▶` with an
`IN SINGLEPLAYER` subtitle on its own black surface — the loudest element on screen. TBD occupies
that exact slot with the **inert `OPEN` button**. The direction doc calls this slot open; it is
currently filled by a control that does nothing.

**Census corrections earned in-browser** (recommend folding into `gap_analysis.md`):

| id | census says | live |
|---|---|---|
| `LAYER-CREATE-001` | `missing` — no create control | **exists**, works |
| `ATTR-FIELD-LYR-NAME` | `missing` — mutator unreachable | control exists, **unusable** (F-02) |
| `RIGHT-MODE-006` (markers) | `missing` — stub | **shipped**, placement + caption + delete |
| `RIGHT-MODE-002` (compositions) | `missing` | tab shipped with save/place copy |
| `RIGHT-MODE-003` (triggers) | `missing` — no trigger entity | tab shipped with activation + draw |
| `PLACE-COMMENT-001`, `CONN-*` | blocked on "no context menu" | context menu **exists**; rows live |
| `ACTION-CUT-001` | `missing` | `Ctrl+X` works |
| `SEL-ALL-001` | `missing` | `Ctrl+A` works, view-scoped |
| `ACTION-PASTE-ORIG-001` | corrected by T-743 | **holds** — lands exactly on source |
| `KEY-HIDE-UI-001` collision | "`Backspace` deletes — dangerous" | `Backspace` = hide UI; `Delete` deletes. Resolved (not re-tested, §7) |
| `XFORM-DEL-001` | `INFERRED`, never verified — "Delete with a vehicle selected removes nothing" | **CONFIRMED in source, and slightly worse** — see below |

**`XFORM-DEL-001` — the census's open lead, now settled (source-verified, browser-unverified).**
`delete_selection` (`apps/website/frontend/src/editor_ops.rs:485-552`) partitions the selection into
comments and "ids", removes the comments, cascades connection edges, then calls
`core.remove_slots(ids)` — **slots only, no vehicle branch**. It then clears the *entire* selection
and returns `true` unconditionally, so `after_local_edit()` fires. A selected vehicle therefore
survives the delete while the selection disappears and the document is marked dirty — the census
predicted the first half; the "reports success and dirties the doc" part is new. I could not
confirm this in the browser because no vehicle can be created (F-21), so this stays **source-only**.

**Also settled: multi-select delete is one undo step, not several.** The code comments at
`editor_ops.rs:513-527` warn that the T-672 edge cascade and the comment loop each open their own
transaction, so a multi-delete costs "extra Ctrl+Z presses". Driven: `Ctrl+A` over 9 slots →
`Delete` → `OBJ0`, and a **single** `Ctrl+Z` restored all 9 (`OBJ9`). The warning is real only when
connections or comments are in the selection; for plain slots the common case is clean.

**Verified working, so you don't re-audit them:** click-pick and marquee; `Ctrl`-additive select;
drag-move (100 px → 105.7 m, exact); undo/redo across move, delete, cut and paste — one step per
operation, coordinates exact, selection preserved; redo correctly invalidated by a new op after
undo; a 9-slot multi-delete undone by one `Ctrl+Z`; paste-at-cursor lands at cursor and
paste-at-source lands exactly on source; scale bar tracks zoom; `Esc` closes ORBAT and the context
menu; dirty dot appears and clears on save; reload restored all 9 objects from IndexedDB with no
conflict prompt.

**Responsive geometry — swept and clean.** Measured at **2560×1440, 1920×1080, 1600×900, 1536×864,
1366×768 and 1280×720** (the last two also stand in for 125%/150% browser zoom, which reflows as a
smaller CSS viewport):

| | result at every size |
|---|---|
| Left dock | **240 px @ x=0** |
| Right dock | **240 px**, flush to the right edge |
| Top strip | full width, **48 px** |
| Status bar | full width, **36 px**, pinned bottom |
| Horizontal document overflow | **0 px** |
| Interactive controls clipped outside the viewport | **0** |

The direction doc's 240/240 equalisation and full-width status bar are met, and the shell does not
break down to 1280×720. **Every clipping defect I found is dialog-level (F-04), not layout-level** —
worth knowing before anyone re-opens the chrome geometry tickets.

**`prefers-color-scheme: light` — no defect.** Emulated light and dark: body stays
`rgb(13,19,34)` with `rgb(221,226,247)` text in both. The editor is deliberately dark-only and does
not half-apply a light theme. (One cosmetic inconsistency: the Save Version button's fill is
`oklab(… / 0.90)` under dark and solid `rgb(173,198,255)` under light — visually near-identical,
not worth a ticket.)

---

## 7. What I did NOT check

Read this as scope, not as clearance — an area listed here has **no** finding either way.

**The reference corpus — now largely closed.** Second pass: 16 frames examined at crop level,
covering every distinct surface (see §6 for the list and why the remaining 59 are near-duplicates).
This directly overturned two of my own findings — F-06 was substantially wrong about where Eden
puts clipboard verbs, and F-13 was wrong that Eden prints whole metres — and produced four
findings I could not have made from the written corpus (F-22, F-23, F-24, F-25).

**Third pass: all 16 toolbar tooltips now read off the pixels** (frames `164000`–`164147`), closing
the residual gap. Every one matches the batch-04/05 transcription — no errata — but reading them
surfaced **F-25**, the off-by-one widget key mapping, which no amount of trusting the corpus would
have produced: the batch docs record Eden's tooltips correctly and TBD's bindings correctly, and
nobody had put the two tables side by side.

**Flows I never drove:** Ruler; LoS/viewshed; comments (menu row seen, never placed); connections
(never started one); compositions save/place; triggers draw; the Arsenal/loadout tab; Export JSON
and Export Compiled Mission; Mission Settings and Environment dialogs; Locations tab; ORBAT
templates, `Load Predefined ORBAT…`, drag-refile, and leader assignment; multi-edit attributes
across a multi-selection (I only opened single-slot attributes).

**Vehicles — closed, but not the way I wanted.** Both creation routes are dead (F-21), so no
vehicle was ever placed and "Place with crew" is still untested. `XFORM-DEL-001` is therefore
settled **in source only** (§6) — I could not put a vehicle on the map to confirm it in the
browser, and I am not claiming I did.

**Stress passes — now run** (§6): 2560×1440 / 1600×900 / 1536×864 / 1366×768 / 1280×720 and
`prefers-color-scheme: light`, all clean. Note the zoom check is an *equivalence* — I emulated
125%/150% as smaller CSS viewports rather than driving Chrome's zoom UI, which is faithful for
reflow but would not catch a zoom-specific rendering bug.

**Closed in passes four and five.** The F-01 blast radius is mapped input-by-input (§2); multi-edit
is driven end-to-end (F-26, F-27, F-29); both export payloads are captured and diffed (F-28, F-34);
and the previously-unrun list is now done: `Esc` matrix (6 surfaces, all correct), zone/trigger/
marker draw modes (F-31), `Backspace` hide-UI (F-35), two-tab clobber (F-32), comments, connections,
compositions (F-30), triggers, Ruler, LoS, Locations tab, Mission Settings, Arsenal, ORBAT
slot-inspector fields, role views (F-33), contrast (F-36).

**Verified working in this sweep, so you don't re-audit them:** `Esc` closes the File menu, Export
dropdown, Save dialog, ORBAT Manager, context menu and Attributes panel — all six. Ruler measures
(`1.17 km`). LoS returns a verdict (`Clear`). Comments place and open an editor. Connections show
the full `Sync to / Group to / Set Trigger Owner` submenu, give in-progress feedback, and cancel on
`Esc`. Trigger draw works (centre + rim → count 0→1). Marker arm disarms on `Esc`. Composition
*save* works, including its name field. Arsenal renders its PRIMARY slots and Export control.
Mission Settings opens with title, time, **briefing textarea and thumbnail URL** — which makes the
census's "clearest hole in the sweep" (`ATTR-FIELD-SCN-OVERVIEW-TEXT`, "nothing in the SPA can edit
it") **stale**. ORBAT slot-inspector text fields are healthy.

**Closed in the sixth pass:** enlisted API writes (403 — F-33 updated), the Save dialog Tab-walk
(no focus trap — F-04 addendum), the full oklab contrast audit (F-36 — one failure total), the
compiled export validated against `mission.schema.json` (clean — F-34 addendum), all 59 remaining
Eden frames (grid pass, §6), the OPEN button (inert — F-38), the Map-layers row (no-op — F-14),
Controls Hint duplication (same surface — F-15), disabled-row tooltips (missing — F-37, with the
History button as the counter-example done right).

**The full residue — everything still unchecked, with why:**
- **Keyboard-only authoring end-to-end** — I verified Esc, Tab behaviour in the Save dialog, and
  that every shortcut fires, but never attempted to build a mission with no mouse. (There is no
  keyboard placement path, so the honest expectation is "impossible"; unproven.)
- **Focus traps in the other modals** (ORBAT, Mission Settings, Attributes) — only the Save dialog
  was Tab-walked.
- **The enlisted 8-object render** (F-33) — provenance unresolved; needs a code look, not a drive.
- **Screen-reader/ARIA semantics** — never audited; the pointerup-activation pattern (F-03) makes
  AT behaviour a real question.
- **Live multiplayer, real Discord auth, staging, and the Enfusion runtime side** — out of scope
  throughout; the T-216 ledger owns the compile-drop story.
- **Perf under load** — release build confirmed, but no stress scene was ever built; the 9-slot
  mission never taxed it.
- **Text-level reads of the 59 grid-passed frames** — structural pass only, accepted risk (§6).

**Two findings withdrawn on Eden evidence.** F-06's original framing ("Eden puts clipboard verbs in
the Edit menu") and F-13's original claim ("Eden prints whole metres") were both wrong, and both
were assertions about Eden I had made *without looking at Eden*. They are corrected in place rather
than deleted so the error is visible. Anything else in this report that compares TBD to Eden
without citing a frame number should be treated as the same class of claim until checked.

**Method note — two bad measurements, caught.** Twice I read geometry off the wrong DOM node and
nearly filed a defect that does not exist: once on the Save dialog (§2 F-04), and once on the ORBAT
Manager, which I measured at `x=-121, y=-240` with 14 controls "unreachable" including the ✕ — a
screenshot showed the modal perfectly centred with every control visible, and a direct per-button
query confirmed it (`close` at 1457,148). **There is no ORBAT clipping bug; it was never filed.**
Both misreads came from walking up parent nodes to find a container. Everything geometric in this
report that survived is backed by a direct element query *and* a screenshot.

**Deliberately out of scope:** the Enfusion/compile side (the T-216 drop ledger owns it);
multiplayer; real Discord auth; `mission_maker`/`enlisted` role views (I ran as admin throughout,
so anything role-gated was invisible to me); screen-reader accessibility and contrast ratios;
performance under load.

**Environment caveats on my findings.** Release build, so timing observations would be valid — but
I made none. My DB started with no seeds; F-05's trigger was that, and I applied the five
`make seed` files partway through (with your go-ahead, run over TCP with a `pg` client because
neither `make` nor a container runtime is on this host), which changed catalog behaviour
mid-review. The pre-existing `sds` mission was opened read-only and never saved to. Two throwaway
versions (`0.1.0`, `0.2.0`) exist on `UXREVIEW-A`, plus a stray `New Layer 1` and one marker;
delete the mission when convenient.

**One methodological note.** `F-12` and `F-01`'s mechanism are marked `HYPOTHESIS` because they
rest on comparing real CDP input against synthetic events, not on reading the handler. F-01's
*behaviour* is observed and reproduced; only the "controlled component remounts" explanation is a
guess.

---

## 8. Operator pass — Sam's hands-on findings, verified 2026-08-09

Sam drove the editor himself and reported ~a dozen defects. Ten were new — and they cluster
precisely where my instrumented pass was blind: **transition frames and hand-feel**. My driver read
state after it settled; a human eye catches what happens on the way there. Verification status on
each:

**O-1 · Docks overlap the status bar — VERIFIED, and it was sitting unflagged in my own data.**
Both docks render `y48 → y1080`; the status bar starts at `y1044`. **36px overlap**, and
`elementFromPoint` at the bar's left end (120,1055) and right end (1800,1055) returns *dock*
elements — the docks eat clicks aimed at the readouts and the right-end controls. My responsive
sweep recorded dock height 1032 at every viewport and I never subtracted. Operator eye 1,
instruments 0. **Fix:** end the docks at the bar's top edge.

**O-2 · Grid reference labels are wrong — VERIFIED with numbers.** Statically they roughly register
(labels within ~1 label-anchor of CUR-derived truth). But after a pan that moved the camera 240 m,
the label set half-updated: positions held while the world moved, and the top edge showed **"090"
at x1593 and "100" at x1663 — 70 px apart at 4 m/px**, where kilometre lines must be 250 px apart.
Two adjacent labels that cannot both be true. A milsim group reads grids aloud; wrong grid labels
are worse than none. **Fix:** derive labels from the live camera transform each frame, not from a
cached set.

**O-3 · Arsenal opens behind ORBAT — VERIFIED with the mechanism.** ORBAT ▸ slot ▸ `OPEN ARSENAL`
opens the Attributes/Arsenal surface at `z-index: 50` — **the same z as the ORBAT modal** — and
ORBAT wins the paint order: hit-testing the arsenal's centre returns ORBAT's element. The author
clicks a button and sees nothing happen (it happened, underneath). **Fix:** a real modal stack —
last-opened wins; the `modal_stack` registry exists, the z assignment doesn't use it.

**O-4 · Menus and modals visibly slide into position — VERIFIED, it's the entrance animation.** A
MutationObserver over spawn: the right-click context menu first paints at (685,500) and settles at
(800,600) — **215 px of travel**; Mission Settings first paints at (458,−352) — off-screen above —
and settles at (704,81) — **679 px of travel**. The `animate-dialog-in` keyframes translate the
surface over ~350 ms. At popover scale that reads as "spawns in the wrong place and drifts in".
**Fix:** entrance animation ≤8 px of travel and ~120 ms, or opacity/scale only. (`animate-menu-in`
moves 12 px — that one is nearly right.)

**O-5 · Help surface doesn't close when another modal opens — VERIFIED.** Controls Hint and the
Save Version dialog stack simultaneously. No surface exclusivity. **Fix:** opening a dialog closes
open popovers/help surfaces.

**O-6 · Comments render as an amber ring — the selection colour — VERIFIED.** So every comment
permanently looks selected (Sam's read exactly), and there is no comment glyph at all — Eden's
Comments layer uses a distinct symbol. **And comments cannot be dragged — VERIFIED:** a 90 px drag
across a placed comment left its position unchanged. Move exists only via delete/re-place.
**Fix:** speech-bubble glyph in a neutral colour; wire comments into the drag-move path.

**O-7 · Sync/squad tether lines only update on release — operator-observed, mechanism confirmed in
design.** The engine drags sprites via GPU preview and rebuilds lines on the one-txn commit
(`select_tool.rs` design per the census) — so the line necessarily points at the stale position
until mouseup. I could not reproduce it cleanly (my drag kept marquee-ing), but the architecture
makes Sam's observation the expected behaviour. **Fix:** include tether endpoints in the preview
pass.

**O-8 · No cursor affordance over selectable entities — VERIFIED.** `canvas` cursor is `auto` over
a unit, over empty ground, everywhere. Nothing tells the hand "this is grabbable". **Caveat:**
hover hit-testing was deliberately removed for perf at T-057, so this is not a one-line CSS fix —
it needs cheap hover picking (e.g. spatial-hash lookup, not full pick).

**O-9 · Placement targets an invisible "drop target" layer — operator-reported, corroborated.** The
outliner's layer rows set a drop target on click, indicated only by a `title` tooltip ("Click:
drop target + select units"), and the active target has no persistent visual state — so which layer
receives the next placement is a guess. **Fix:** visible persistent highlight on the drop-target
row + a "placing into: X" chip near the palette.

**O-10 · The `GRID off` chip — operator-questioned, and he's right that it's mislabelled.** It
shows the *snap*-grid state, but it sits in the map corner next to the map's *grid references*
(O-2), so "GRID" reads as "the coordinate grid is off". Naming collision between two different
grids. **Fix:** label it `SNAP off/move/rot` and move it into the (future) toolbar's snapping
group, per F-06.

**Operator decisions recorded** (these close questions the direction doc left open):
1. The floating Select/Ruler/LoS pill **goes** — tools live Arma-style in the fixed chrome (closes
   F-09's question in the direction the doc hinted).
2. **ORBAT Manager moves into the menu row** (with File…Help), not a row-2 button.
3. **Validation moves to a top-bar error-count chip** that drops down on click (replaces the
   floating bottom-left panel; keeps the validator's copy, which he likes).

**Operator likes, matching the data:** the WEST/EAST/IND/TOTAL counts chip (stayed truthful through
every operation all session), the Locations tab, and the validator's message quality.

**One correction to the rant:** the Environment menu's thinness is not a forgotten UI — fog, wind
and view-distance are census-blocked on the Enfusion *reader* side (`windDirDeg` is even in the
schema; nothing consumes it), and the editor refuses to author values the game ignores. That
restraint is correct; the ticket is mod-side (N8), not editor-side.

## 9. Existing tickets this lands on

Cite rather than re-file: **T-142** (shell layout polish) and **T-158** (editor shell UX
consolidation) own F-06/F-09; **T-157** (visual overhaul) overlaps F-10; **T-146** (asset browser
data wiring) is adjacent to F-05 and **F-21**; **T-190** owns the two-tab case I did not run;
**T-704** (command palette) would soften F-06. F-01 and F-02 fit none of them — they are new, and
they are the two that should be fixed before anyone else opens this editor.

One scoping note for whoever picks up the chrome tickets: **T-142/T-157/T-158 should not be scoped
as responsive/layout work.** The sweep in §6 shows the shell is already correct from 1280×720 to
2560×1440. What is actually wrong is menu completeness (F-06), bottom-bar consolidation (F-09),
map symbology (F-10) and dialog positioning (F-04) — four specific things, not a layout pass.
