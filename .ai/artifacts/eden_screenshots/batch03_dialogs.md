# Batch 03 — Scenario Attributes modal dialogs (General / Environment / Multiplayer / Performance), the Attributes / Tools / Settings menus, and Editor Preferences

## Overview

This batch is a guided tour of Arma 3 Eden's **modal attribute dialogs** and the **menu-bar dropdowns that launch them**. The operator walks the `Attributes` menu top-to-bottom (General → Environment → Multiplayer → Performance), opening each dialog, scrolling it to the bottom, and closing it; then opens the `Tools` menu, then the `Settings` menu, and finally opens `Settings → Preferences...`. Nothing is edited — no value is changed anywhere in the batch. It is a *reference capture of the attribute surface*, which is exactly the content a rebuild needs.

The scenario in the background never changes: Altis (or an Altis-like terrain), camera on a hillside looking north-west over a bay, one BLUFOR entity placed (`Alpha 1-1` → `Asst. Missile Specialist (AA)`), asset browser on the Groups tab showing NATO infantry groups.

**Geometry note.** Screenshots `163629`, `163642` and `163658` are **1897×1077**; the rest are **1920×1077**. The modal dialog sits at exactly the same absolute coordinates (x 680–1239) in both sets, so the narrow ones are simply the same 1920-wide frame with the rightmost 23 px cropped off (you lose the right edge of the asset panel and part of the FPS counter). **All coordinates in this document are given in 1920×1077 space.**

### Chrome layout common to every screenshot in the batch

| Region | Bounds (x, y) | Notes |
|---|---|---|
| Menu bar | 0–1920, 0–21 | `Scenario  Edit  View  Attributes  Tools  Settings  Play  Help` |
| Toolbar (icon row) | 0–1920, 22–43 | icons ~16 px, grouped by 1-px vertical separators |
| FPS readout | ~1885–1918, 1–13 | green monospace, e.g. `70 FPS` (reads `89` truncated in the 1897-wide shots) |
| Left panel (Entities) | 0–239, ~44–1030 | translucent dark overlay, 3D view shows through |
| 3D viewport | 240–1679, 44–1055 | |
| Right panel (Assets) | 1680–1919, ~44–1055 | translucent dark overlay |
| Left-panel footer toolbar | 0–239, ~1035–1055 | trash + 4 layer buttons |
| Status bar | 0–1920, ~1057–1077 | coordinate/view-distance readouts + version + PLAY SCENARIO |
| PLAY SCENARIO button | ~1690–1920, ~1030–1077 | large black button, `PLAY SCENARIO` / `IN SINGLEPLAYER` + ▶ |

Menu-bar button spans, measured from the hover highlight: `Attributes` 155–226, `Tools` 227–275, `Settings` 275–338. Extrapolating: `Scenario` ~2–70, `Edit` ~70–108, `View` ~108–155, `Play` ~338–382, `Help` ~382–430.

Toolbar icon x-positions (centres, all at y≈32), left to right:

| x | Icon | Function (label not shown; read from icon + Eden knowledge) |
|---|---|---|
| 10 | blank page | New scenario |
| 26 | open folder | Open scenario |
| 42 | floppy disk | Save scenario |
| 57 | Steam logo | Publish to Steam Workshop *(inferred)* |
| — | separator | |
| 106 | curved left arrow | Undo |
| 126 | curved right arrow | Redo |
| — | separator | |
| 169 | arrow cursor **(has a raised/boxed frame — this is the active tool)** | Select mode |
| 190 | 4-way arrows | Move / translate mode |
| 208 | circular arrow | Rotate mode |
| 227 | diagonal arrow in a box | Scale mode |
| 248 | dotted box with centre dot | Bounding-box / pivot toggle *(inferred; icon only)* |
| — | separator | |
| 288 | banded sphere / globe | *(inferred, low confidence — a viewport toggle)* |
| 307 | framed curve, like a chart in a frame | *(inferred, low confidence — a viewport toggle)* |
| 327 | plus sign with a horizontal bar through it | *(inferred, low confidence — vertical/align toggle)* |
| — | separator | |
| 368 + 383 | 3×3 grid of squares **+ ▾** | Grid / position snapping, dropdown for step size |
| 402 + 415 | triangle with an angle marked **+ ▾** | Angle snapping, dropdown for step |
| 433 + 445 | vertical ruler **+ ▾** | Vertical / elevation snapping, dropdown for step |
| — | separator | |
| 477 | sun behind a cloud | Weather preview toggle *(inferred — same icon appears next to `Attributes → Environment...`)* |
| 497 | four curved vertical bars | *(inferred, low confidence)* |
| 520 | sun / light bulb with rays | Lighting-time preview toggle *(inferred)* |
| 540 | binoculars | View-distance preview toggle *(inferred)* |
| 560–670 | **combo box reading `Scenario`** with ▾ at 650–670 | Selects the environment source used by the viewport preview *(inferred)* |

Sampled UI colours (useful for a rebuild): dialog title bar `rgb(196,130,21)` orange; dialog body `rgb(52,52,52)`; text-input / combo background `rgb(13,13,13)`; other field background `rgb(26,26,26)`; dialog footer strip `rgb(37,37,37)`; OK/CANCEL button fill pure black; menu bar `rgb(27,27,27)`; toolbar `rgb(13,13,13)`; dropdown menu panel `rgb(26,26,26)`; status bar `rgb(51,51,51)`; disabled slider track `rgb(43,43,43)`.

### The modal dialog shell (identical for all four Attributes dialogs and for Preferences)

* **Outer rect:** x 680–1239 (560 wide). y 199–878 (679 tall) for General / Multiplayer / Performance / Preferences; y 257–821 (564 tall) for Environment. Horizontally centred; vertically it is *not* centred the same way for all dialogs — Environment is shorter and sits lower.
* **Title bar:** y 199–218 (20 px), full dialog width, solid orange, **black** left-aligned text at x≈686. Text is always `Edit: <category>` — `Edit: General`, `Edit: Environment`, `Edit: Multiplayer`, `Edit: Performance`, `Edit: Preferences`.
* **Scrolling content area:** y ~222–848, x 686–1233.
* **Vertical scrollbar:** x 1233–1239, with a small ▲ at the top (y≈222) and ▼ at the bottom (y≈846). **It is only drawn when the content overflows** — Performance and Preferences have no scrollbar at all.
* **Footer strip:** y ~850–878, `rgb(37,37,37)`. **OK** at x 1032–1132, **CANCEL** at x 1135–1237, both y 854–873 (≈100×19), black fill, white uppercase text, right-aligned. No Apply, no Help, no third button.
* **Column grid inside the content area:**
  * collapse triangle at x 690–698
  * group-header label text starts x 708
  * field labels are **right-aligned**, ending at x 866
  * controls start at x 872 and full-width controls end at x 1200
  * narrow numeric controls (`Limit`, `Required Keys Limit`, `Min Players`) end at x ~980
  * group separator hairlines run x 688–1225
  * row pitch ≈ 27 px, control height ≈ 17 px
* **Modal scrim:** while a dialog is open the **entire rest of the screen** is covered by a light, low-contrast **diagonal-hatch overlay** — it lightens and desaturates the menu bar, toolbar, both side panels and the 3D view. Everything remains legible but obviously inert. This is a distinctive and very copyable "the app is blocked" affordance.

### Control vocabulary observed

* **Text field** — dark box, left-aligned value, no border decoration.
* **Multiline text box** — drawn as a *group box* with the caption `Text` or `Init` cut into the top-left of its border.
* **Combo box** — dark box spanning the full control column with a ▾ button at the right end.
* **Checkbox** — small square, ~13×13, at x 872. Two arrangements are used: **label-right-aligned-then-checkbox** (the default everywhere) and **checkbox-then-label** inside list-style controls (`Rulesets`).
* **Slider** — `[◀] [track] [▶] [value box]`. Track fill is light grey from the left up to the value; remainder dark. Value box is at the far right (x ~1160–1200) with a white outline when enabled and a grey outline when disabled.
* **Time field** — `HH : MM : SS`, three segments in one box; hovering a segment produces a tooltip naming it (`Hours` seen).
* **Special: time-of-day slider** — the `Time` row in Environment uses a **day/night gradient track** (dark at both ends, bright in the middle) with a **sun glyph as the thumb**, and a thin secondary progress bar underneath the track.
* **Toolbox / icon-radio** — `Independents Allegiance` is a row of icon buttons inside a lighter recessed panel; the selected one gets a lit background.
* **Axis-tagged numeric field** — `Direction Start` shows a small blue `Z` chip immediately left of the number box.
* **Checkbox list** — `Rulesets`: a bordered list box with checkbox rows inside, label vertically centred against it.
* **Group header** — small solid triangle (pointing down-left = expanded) + grey caption + hairline to the right edge.
* **Sub-group header** — indented, no triangle, short vertical tick to its left, sitting on a slightly lighter band. Not collapsible.
* **Disabled state** — label text, control chrome and value all drop to mid-grey; the ◀/▶ arrows lose contrast. Disabling is driven by a sibling control (see `Manual Override` and `Respawn`).

---

## Screenshot_20260801_163629.png

**What it shows:** the `Edit: General` scenario-attributes dialog, freshly opened and scrolled to the top.

Dialog x 680–1239, y 199–878. Title bar `Edit: General`. Scrollbar thumb at y 236–687 (near the top of a 227–848 track).

Content, top to bottom (label → control, control state):

| y (approx) | Element | Value / state |
|---|---|---|
| 235 | ▾ group header **Presentation** | expanded |
| 255 | `Title` → text field (872–1200) | **empty** |
| 283 | `Author` → text field | **`Darkforce`** |
| 311 | ▾ group header **Overview** | expanded |
| 335 | `Picture` → text field | empty |
| 353–457 | group box captioned `Text` (698–1202) → multiline text area | empty |
| 483 | `DLC` → combo box | **`None`**, ▾ at x≈1185 |
| 510 | `Require DLC` → checkbox at (872–885) | **checked** |
| 535 | ▾ group header **Overview (Locked)** | expanded |
| 558 | `Picture` → text field | empty |
| 578–678 | group box `Text` → multiline | empty |
| 705 | ▾ group header **Loading Screen** | expanded |
| 728 | `Picture` → text field | empty |
| 748–845 | group box `Text` → multiline | empty |
| 854–873 | `OK` / `CANCEL` | |

Meaning of the fields (inferred from Eden knowledge, labels read verbatim): `Title`/`Author` are the scenario's name and author in the mission list; the three `Picture` + `Text` blocks are the mission-select overview shown when the mission is unlocked, when it is locked, and on the loading screen respectively; `DLC` marks a required DLC and `Require DLC` enforces ownership.

Background: menu bar, toolbar and both panels are visible under the diagonal-hatch modal scrim. Left panel shows `Entities`/`Locations` tabs with the tree `BLUFOR ▸ Alpha 1-1 ▸ Asst. Missile Specialist (AA)` (that leaf is drawn in **red**), and greyed `OPFOR / Independent / Civilian / Empty / Ambient life / Triggers / Systems / Markers / Comments`. Right panel shows the `Assets` tab, `F2` (Groups) category active, BLUFOR side filter selected.

---

## Screenshot_20260801_163642.png

**What it shows:** the same `Edit: General` dialog, now **scrolled to the bottom**. Nothing else changed on screen.

Scrollbar thumb y 541–839 — hard against the bottom of the track. (Note: the thumb is *shorter* here, 299 px, than in the previous shot, 452 px. The content height is therefore being recomputed — either a group was expanded between the two shots or Eden re-measures the content as you scroll. Flagged as an observation; I could not determine which.)

| y (approx) | Element | Value / state |
|---|---|---|
| 222 | ▾ group header **States** | expanded |
| 245 | `Show Briefing` → checkbox | **checked** |
| 272 | `Show Debriefing` → checkbox | **checked** |
| 299 | `Enable Saving` → checkbox | **checked** |
| 326 | `Show Map` → checkbox | **checked** |
| 353 | `Show Compass` → checkbox | **checked** |
| 380 | `Show Watch` → checkbox | **checked** |
| 407 | `Show GPS` → checkbox | **checked** |
| 434 | `Show HUD` → checkbox | **checked** |
| 461 | `Show UAV Feed` → checkbox | **checked** |
| 488 | `Advanced Flight Model` → checkbox | **unchecked** |
| 512 | `Debug Console` → combo box | **`Available only in editor`** |
| 538 | ▾ group header **Unlock** | expanded |
| 560 | `Unlocked Keys` → text field | empty |
| 587 | `Required Keys` → text field | empty |
| 614 | `Required Keys Limit` → narrow numeric field (872–980) | **`0`** |
| 638 | ▾ group header **Init** | expanded |
| 658–765 | group box `Init` → multiline code editor | empty |
| 782 | ▾ group header **Misc** | expanded |
| 800 | `Independents Allegiance` → icon toolbox in a recessed panel (872–1200) | two options: **blue rectangle with a white handshake at x 982–1027 — SELECTED (lit blue background)**, and a **dark red diamond with a crossed device at x 1057–1095 — not selected**. Blue = BLUFOR/West, red = OPFOR/East *(inferred from Arma side markers)*. |
| 826 | `Binarize the Scenario File` → checkbox | **checked** |
| 854–873 | `OK` / `CANCEL` | |

`Show *` toggle whether the corresponding UI element is available to the player; `Enable Saving` toggles mid-mission saves; `Debug Console` gates script console access; `Unlocked Keys` / `Required Keys` / `Required Keys Limit` drive campaign progression gating; `Init` is scenario init code; `Binarize the Scenario File` controls whether `mission.sqm` is saved binarised.

**Changed from 163629:** scroll position only.

---

## Screenshot_20260801_163658.png

**What it shows:** the dialog is closed; the **`Attributes` dropdown menu is open** with `Environment...` hovered. The 3D viewport is fully visible again (no scrim), so the terrain, sea and hillside read normally.

* `Attributes` menu-bar button, x **155–226**, y 0–21, drawn with an **orange highlight** while its menu is open.
* Dropdown panel: x **155–347** (192 wide, left-aligned to its button), y 23–~128, background `rgb(26,26,26)`, 1-px border. Menu items are 23 px tall with a left icon gutter at x≈160–180 and right-aligned shortcut text.

| Row (y) | Label | Icon | Shortcut | State |
|---|---|---|---|---|
| 27–49 | `General...` | none | — | normal |
| 50–72 | `Environment...` | sun-behind-cloud (same glyph as the toolbar icon at x 477) | **`Ctrl+I`** | **hovered — full-width orange fill, black text** |
| 73–95 | `Multiplayer...` | none | — | normal |
| 96–118 | `Performance...` | none | — | normal |

Only four items; no separators. The `...` suffix marks "opens a dialog". Note that the icon gutter is present but only populated for the one command that also has a toolbar button — a nice, cheap consistency cue.

**Changed from 163642:** `Edit: General` dismissed; `Attributes` menu opened; scrim gone; viewport restored.

---

## Screenshot_20260801_163720.png

**What it shows:** the `Edit: Environment` dialog (opened from the item hovered in the previous shot), scrolled to the top, with a **tooltip visible**.

Dialog x 680–1239, **y 257–821** (this one is shorter, 564 px, and sits lower than the other four). Title bar `Edit: Environment` at y 257–276. Scrollbar thumb y 294–684.

| y (approx) | Element | Value / state |
|---|---|---|
| 292 | ▾ group header **Date** | expanded |
| 313 | `Date` → **three combo boxes in one row**: year `2035` ▾ (872–990), month `June` ▾ (995–1105), day `24` with a greyed secondary label `Sun` and ▾ (1110–1200) | 24 June 2035, a Sunday |
| 341 | `Time` → `[◀] [day/night gradient track with a sun glyph thumb + thin progress bar beneath] [▶]` then time box | **`12 : 00 : 00`** |
| 368 | ▾ group header **Weather Forecast** | expanded |
| 393 | `Time of Changes` → `[◀][slider][▶]` + time box | **`00 : 30 : 00`** |
| 420 | ▾ group header **Overcast** | expanded |
| 443 | `Overcast Start` → slider + value box | **`30%`** (track filled ~30% from the left) |
| 470 | `Overcast Forecast` → slider + value box | **`30%`** |
| 496 | ▾ group header **Fog** | expanded |
| 520 | `Fog Start` → slider + value | **`0%`** |
| 547 | `Decay` → slider + value | **`1%`** |
| 574 | `Base` → slider + value | **`0 m`** |
| 601 | `Fog Forecast` → slider + value | **`0%`** |
| 628 | `Decay` → slider + value | **`1%`** |
| 655 | `Base` → slider + value | **`0 m`** |
| 678 | ▾ group header **Rain** | expanded |
| 700 | `Manual Override` → checkbox | **unchecked** |
| 727 | `Rain Start` → slider + value | **`0%`, DISABLED** (label, arrows and value box all greyed) |
| 754 | `Rain Forecast` → slider + value | **`0%`, DISABLED** |
| 778 | ▾ group header **Lightnings** | partially visible at the bottom edge |
| 797–816 | `OK` / `CANCEL` | |

**Tooltip:** a dark box with light text reading **`Forecasted rain strength.`**, at roughly x 875–1030, y 770–793 — i.e. anchored just below/left of the `Rain Forecast` row the cursor is over. Tooltips overlay dialog content and are unbordered dark rectangles.

**Key interaction to copy:** `Manual Override` is a **gate**. When unchecked, every slider in its group is disabled but still fully visible, retaining its greyed value. Each weather channel (Rain, Lightnings, Waves, Wind) has its own independent `Manual Override`.

**Changed from 163658:** menu dismissed, dialog opened, scrim back on.

---

## Screenshot_20260801_163735.png

**What it shows:** `Edit: Environment` **scrolled to the bottom** (~367 px further down). Scrollbar thumb y 498–781.

| y (approx) | Element | Value / state |
|---|---|---|
| ~278 | tail of the previous `Base` slider | clipped at the top of the viewport |
| 311 | ▾ group header **Rain** | expanded |
| 333 | `Manual Override` → checkbox | **unchecked** |
| 360 | `Rain Start` → slider | **`0%`, disabled** |
| 387 | `Rain Forecast` → slider | **`0%`, disabled** |
| 411 | ▾ group header **Lightnings** | expanded |
| 434 | `Manual Override` → checkbox | **unchecked** |
| 461 | `Lightnings Start` → slider | **`0%`, disabled** |
| 488 | `Lightnings Forecast` → slider | **`10%`, disabled** (track shows a small filled stub) |
| 512 | ▾ group header **Waves** | expanded |
| 535 | `Manual Override` → checkbox | **unchecked** |
| 562 | `Waves Start` → slider | **`10%`, disabled** |
| 589 | `Waves Forecast` → slider | **`10%`, disabled** |
| 613 | ▾ group header **Wind** | expanded |
| 636 | `Manual Override` → checkbox | **unchecked**; **label rendered in bright white = hovered** |
| 663 | `Wind Start` → slider | **`10%`, disabled** |
| 690 | `Wind Forecast` → slider | **`10%`, disabled** |
| 717 | `Gusts Start` → slider | **`0%`, disabled** |
| 744 | `Gusts Forecast` → slider | **`0%`, disabled** |
| 771 | `Direction Start` → **blue `Z` chip (872–890) + numeric field (892–1000)** | **`0`, disabled** (chip is blue with grey glyph when disabled) |
| 798 | `Direction Forecast` → blue `Z` chip + numeric field | **`0`, disabled** |
| 797–816 | `OK` / `CANCEL` | |

Every one of the four Manual Override groups is off, so the whole lower half of this dialog is greyed. `Direction *` are the only non-slider weather inputs — a compass bearing on the Z axis.

**Changed from 163720:** scroll only; the tooltip is gone and the cursor has moved to the `Wind → Manual Override` label.

---

## Screenshot_20260801_163758.png

**What it shows:** `Edit: Multiplayer`, scrolled to the top. Dialog back at x 680–1239, y 199–878. Scrollbar thumb y 236–705.

| y (approx) | Element | Value / state |
|---|---|---|
| 235 | ▾ group header **Type** | expanded |
| 256 | `Game Type` → combo | **`Undefined Game Mode`** |
| 283 | `Min Players` → narrow numeric field | **`0`** |
| 310 | `Max Players` → narrow numeric field | **`0`** |
| 334 | ▾ group header **Lobby** | expanded |
| 355 | `Summary` → text field (full width) | empty |
| 382 | `Enable AI` → checkbox | **checked** |
| 409 | `Auto Assign Slots` → checkbox | **unchecked** |
| 433 | ▾ group header **Respawn** | expanded |
| 458 | `Respawn` → combo | **`Disabled`** |
| 480 | *(list, unlabelled at this scroll)* `Mission fail when everyone is dead` | **unchecked** |
| 495 | `Singleplayer death screen` | **checked** |
| ~510 | `Rulesets` → checkbox-list box (872–1200, ~110 tall) | contains the two rows above |
| 585 | `Respawn Delay` → slider + time box | **`00 : 00 : 00`, DISABLED** |
| 612 | `Vehicle Respawn Delay` → slider + time box | **`00 : 00 : 00`, enabled — label in bright white and the value box highlighted (hovered)** |
| 639 | `Show Scoreboard` → checkbox | **checked but DISABLED (greyed)** |
| 666 | `Allow Manual Respawn` → checkbox | **checked but DISABLED** |
| 693 | `Enable Team Switch` → checkbox | **checked but DISABLED** |
| 720 | `Allow AI Score` → checkbox | **unchecked, enabled** |
| 744 | ▾ group header **Tasks** | expanded |
| 765 | `Shared Objectives` → combo | **`Disabled`** |
| 790 | ▾ group header **Revive** | expanded |
| 812 | `Revive Mode` → combo | **`Disabled`** |
| 840 | `Required Trait` → combo | **`None`** |
| 854–873 | `OK` / `CANCEL` | |

**Key interaction to copy:** `Respawn = Disabled` cascades — `Respawn Delay`, `Show Scoreboard`, `Allow Manual Respawn` and `Enable Team Switch` are all greyed out, but `Vehicle Respawn Delay` and `Allow AI Score` remain live because they do not depend on player respawn. The disabled checkboxes keep their tick, greyed — the state is preserved, not cleared.

Note also the **two checkbox layouts in one dialog**: everywhere else it is `right-aligned label … checkbox`, but inside the `Rulesets` list the checkbox is on the left with the label to its right.

**Changed from 163735:** Environment closed, Multiplayer opened.

---

## Screenshot_20260801_163811.png

**What it shows:** `Edit: Multiplayer` scrolled down ~139 px (thumb y 332–801, same 469-px thumb length as the previous shot, so the content height is stable here). A `Hours` tooltip is showing.

| y (approx) | Element | Value / state |
|---|---|---|
| 228 | `Summary` → text field | empty |
| 255 | `Enable AI` → checkbox | checked |
| 282 | `Auto Assign Slots` → checkbox | unchecked |
| 306 | ▾ group header **Respawn** | expanded |
| 328 | `Respawn` → combo | `Disabled` |
| 350 | `Rulesets` list, row 1: ☐ `Mission fail when everyone is dead` | unchecked |
| 366 | `Rulesets` list, row 2: ☑ `Singleplayer death screen` | checked |
| ~390 | `Rulesets` label, vertically centred against the list box | |
| 458 | `Respawn Delay` → slider + `00 : 00 : 00` | disabled |
| 485 | `Vehicle Respawn Delay` → slider + `00 : 00 : 00` | enabled/highlighted |
| 512 | `Show Scoreboard` ☑ | disabled |
| 530 | `Allow Manual Respawn` ☑ | disabled |
| 557 | `Enable Team Switch` ☑ | disabled |
| 584 | `Allow AI Score` ☐ | enabled |
| 608 | ▾ group header **Tasks** | expanded |
| 630 | `Shared Objectives` → combo | `Disabled` |
| 655 | ▾ group header **Revive** | expanded |
| 678 | `Revive Mode` → combo | **`Disabled`** |
| 705 | `Required Trait` → combo | **`None`** |
| 732 | `Required Items` → combo | **`None`** |
| 762 | `Revive Duration` → slider + time box | **`00 : 00 : 06`** |
| 789 | `Medic Speed Multiplier` → slider + value box | value box **obscured by the tooltip** |
| 816 | `Force Respawn Duration` → slider + time box | **`00 : 00 : 03`** |
| 854–873 | `OK` / `CANCEL` | |

**Tooltip:** reads **`Hours`**, drawn at approximately x 1132–1200, y 779–800, i.e. directly over the value box the cursor is on. This confirms the `HH : MM : SS` boxes are **three independently-hoverable, independently-editable segments**, each with its own tooltip.

**Changed from 163758:** scroll position + cursor moved onto a time-field segment.

---

## Screenshot_20260801_163830.png

**What it shows:** `Edit: Performance`, entirely visible — **no scrollbar is drawn**, confirming Eden only shows the scrollbar when content overflows. Dialog x 680–1239, y 199–878.

| y (approx) | Element | Value / state |
|---|---|---|
| 235 | ▾ group header **Garbage Collection** | expanded |
| 258 | `Minimum distance` → text field (full width, 872–1200) | **`0`** |
| 288 | sub-header **Character Corpses** (indented, no triangle, lighter band, vertical tick at left) | |
| 318 | `Mode` → combo | **`None`** |
| 345 | `Limit` → narrow numeric field | **`15`** |
| 372 | `Min Delay` → slider + time box | **`00 : 00 : 10`** |
| 399 | `Max Delay` → slider + time box | **`01 : 00 : 00`** (track filled ~full width) |
| 428 | sub-header **Vehicle Wrecks** | |
| 458 | `Mode` → combo | **`None`** |
| 485 | `Limit` → narrow numeric field | **`15`** |
| 512 | `Min Delay` → slider + time box | **`00 : 00 : 05`** |
| 539 | `Max Delay` → slider + time box | **`01 : 00 : 00`** |
| 566 | ▾ group header **Dynamic Simulation** | expanded |
| 588 | `Enable Dynamic Simulation` → checkbox | **checked** |
| 618 | sub-header **Activation Distance Settings** | |
| 648 | `Characters` → slider + value box | **`500m`** |
| 675 | `Manned Vehicles` → slider + value box | **`350m`** |
| 702 | `Props` → slider + value box | **`50m`** (track fill nearly empty) |
| 729 | `Empty Vehicles` → slider + value box | **`250m`** |
| 758 | sub-header **Activation Distance Modifiers** | |
| 788 | `Is Moving` → slider + value box | **`2x`** |
| 815 | `Limit by View Distance` → checkbox | **checked** |
| 854–873 | `OK` / `CANCEL` | |

This dialog is the clearest demonstration of the **two-level heading hierarchy**: collapsible top-level groups with a triangle and a full-width hairline, and non-collapsible indented sub-headers on a lighter band.

**Changed from 163811:** Multiplayer closed, Performance opened.

---

## Screenshot_20260801_163901.png

**What it shows:** no dialog. The **`Tools` dropdown menu is open** (menu-bar button `Tools` highlighted orange, x 227–275), nothing hovered inside the menu. Full un-dimmed editor visible.

Dropdown panel x ~227–415 (~188 wide), y 23–~172, left-aligned to the `Tools` button. Icon gutter at x≈235–255.

| Row (y) | Label | Icon | Shortcut | State |
|---|---|---|---|---|
| 27–49 | `Debug Console...` | none | **`section`** — the literal word, i.e. the `§` key, which Arma names "section" | normal |
| 50–72 | `Functions Viewer...` | italic `fx` | — | normal |
| 73–95 | `Config Viewer...` | `{ }` | — | normal |
| 96–118 | `Animations Viewer...` | none | — | normal |
| 119–141 | `Camera...` | none | — | normal |
| 142–164 | `Field Manual...` | none | — | normal |

**Rest of the screen (fully readable here, so recorded once for the whole batch):**

*Left panel (Entities), x 0–239:*
* `«` collapse button at x ~2–20, y ~52–68.
* Tabs: **`Entities`** (selected, lighter fill, x ~24–140) and `Locations` (x ~144–239), y ~50–68.
* Search row y ~72–88: text field x 4–188 (empty), magnifier button x 190–208, `−` button x 210–224 (collapse-all), stacked-pages-with-`+` button x 226–239.
* Tree, row pitch ≈ 15.5 px, each row prefixed by an expander triangle and a **checkbox** (visibility toggle):
  * y 112 `▾ ☑ 📁 BLUFOR` — expanded, checked, folder icon
  * y 127 `▾ ☑ ▭ Alpha 1-1` — blue rectangle side icon
  * y 142 `● Asst. Missile Specialist (AA)` — **drawn in red**; the selected entity *(red colouring meaning not determined from the image)*
  * y 157 `☑ OPFOR` — greyed (no entities on that side)
  * y 172 `☑ Independent` — greyed
  * y 188 `☑ Civilian` — greyed
  * y 204 `☑ Empty` — greyed
  * y 222 `☑ Ambient life` — greyed
  * y 238 `☑ Triggers` — greyed
  * y 254 `☑ Systems` — greyed
  * y 269 `☑ Markers` — greyed
  * y 285 `☑ Comments` — greyed
* Footer toolbar y ~1035–1055: **trash/delete** icon at x ~6–25 (far left); then right-aligned, **folder + `+`** (x ~140–158), **folder + prohibition sign** (x ~166–184), **folder + padlock** (x ~192–210), **folder + eye** (x ~218–236). These are the layer create / disable / lock / hide actions *(inferred from the icons)*.

*Right panel (Assets), x 1680–1919:*
* Tabs y ~50–68: **`Assets`** (selected) and `History`, plus a `»` expand button at the far right (x ~1905–1918).
* Category row y ~70–104 — six buttons, each with its **F-key label above the icon**:
  * `F1` single figure — Units
  * **`F2` three figures — Groups (ACTIVE: label and icon are bright white; the others are dim)**
  * `F3` flag — Triggers
  * `F4` footprints — Waypoints
  * `F5` stacked cubes — Systems / modules
  * `F6` circle with an X — Markers
* Side filter row y ~118–144 — six colour plates: **blue rectangle (BLUFOR) — SELECTED, brighter with a highlight border**, dark red diamond (OPFOR), dark green square (Independent), dark purple square (Civilian), olive rounded blob, grey three-linked-circles. The last two are dimmed and their meaning could not be read from the icon.
* Search row y ~150–170: `▾` dropdown button x 1685–1700, text field 1703–1858 (empty), magnifier 1860–1878, `−` 1882–1896, stacked-pages-`+` 1898–1918.
* Asset tree from y 178, row pitch ≈ 15.7 px:
  `▸ CTRG` (187) · `▸ FIA` (206) · `▸ Gendarmerie` (222) · `▾ NATO` (237) · `  ▸ Armor` (252) · `  ▾ Infantry` (267) → `Air-defense Team` (283), `Anti-armor Team` (298), `Assault Squad` (314), `Fire Team` (330), `Fire Team (Light)` (346), `Recon Patrol` (361), `Recon Sentry` (377), `Recon Squad` (393), `Recon Team` (409), `Rifle Squad` (425), `Sentry` (441), `Sniper Team` (457), `Weapons Squad` (473) · `  ▸ Mechanized Infantry` (488) · `  ▸ Motorized Infantry` (504) · `  ▸ Special Forces` (520) · `  ▸ Support Infantry` (536) · `▸ NATO (Pacific)` (551) · `▸ NATO (Woodland)` (567). Leaf rows carry a small blue group symbol — a rectangle with an `X` for line infantry, a rectangle with a single diagonal for recon.

*Viewport:* a **three-axis translate gizmo** is visible at around x 250–300, y 950–1010 — blue `Z` arrow up, green `Y` arrow to the upper-right, red `X` arrow to the lower-right, each axis labelled with its letter in the matching colour. The selected unit sits mostly off the left edge of the viewport.

*Status bar, y ~1057–1077, left-aligned readouts in individual dark chips:*
* `X ↔` **`-4366.44 m`** (x ~4–90)
* `Y ↑` **`17518.9 m`** (x ~96–190)
* `Z` with a ground/terrain glyph **`-185.97 m`** (x ~196–290)
* eye glyph **`10902.4 m`** — view distance (x ~296–420)
* centre of the bar is empty
* right side: **`2.20.153973`** version string (x ~1570–1640), then a horizontal-double-arrow icon and a monitor icon (x ~1648–1682)
* **`PLAY SCENARIO`** / `IN SINGLEPLAYER` with a large ▶, x ~1690–1920, occupying the full status-bar height and a bit above it.

**Changed from 163830:** `Edit: Performance` dismissed; scrim gone; `Tools` menu opened.

---

## Screenshot_20260801_163909.png

**What it shows:** the **`Settings` dropdown menu** open with `Preferences...` hovered. Everything else is identical to 163901.

`Settings` menu-bar button highlighted orange at x **275–338**. Dropdown panel x **275–450** (~176 wide), y 23–~167.

| Row (y) | Label | Shortcut | State |
|---|---|---|---|
| 27–49 | `Preferences...` | **`Ctrl+K`** | **hovered — full-width orange fill, black text** |
| ~52–58 | *(horizontal separator hairline, inset from both edges)* | | |
| 62–84 | `Video Options...` | — | normal |
| 85–107 | `Audio Options...` | — | normal |
| 108–130 | `Game Options...` | — | normal |
| 131–153 | `Controls...` | — | normal |

The separator cleanly divides *editor* preferences from *game* options — worth copying. No icons in this menu.

**Changed from 163901:** `Tools` menu closed, `Settings` menu opened, cursor on `Preferences...`.

---

## Screenshot_20260801_163916.png

**What it shows:** the `Edit: Preferences` dialog (from `Settings → Preferences...`). Same shell as the attribute dialogs — x 680–1239, y 199–878, orange `Edit: Preferences` title bar, `OK`/`CANCEL` footer. **No scrollbar** — content fits.

| y (approx) | Element | Value / state |
|---|---|---|
| 235 | ▾ group header **Saving** | expanded |
| 258 | `Auto-save` → combo (full width) | **`15 min`** |
| 285 | `Binarize New Scenario Files` → checkbox | **checked** |
| 313 | ▾ group header **Camera** | expanded |
| 335 | `Default Speed` → slider + value box | **`1x`** (track ~one-third filled) |
| 362 | `Fast Speed` → slider + value box | **`1x`** |
| 389 | `Mouse wheel Sensitivity` → slider + value box | **`1x`** |
| 408 | `Copy Terrain` → checkbox | **unchecked** |
| 431 | `Adaptive Speed` → checkbox | **checked** |
| 458 | `Start in Map` → checkbox | **unchecked** |
| 481 | `Start on Random Position` → checkbox | **checked** |
| 512 | ▾ group header **Misc** | expanded |
| 531 | `Automatic Grouping` → checkbox | **checked** |
| 556 | `Recompile Functions` → checkbox | **unchecked** |
| 580 | `Environmental Sounds` → checkbox | **unchecked** |
| 604 | `Automatic Composition Layering` → checkbox | **checked** |
| 854–873 | `OK` / `CANCEL` | |

Everything below y ≈ 620 is empty dialog body — the footer stays pinned to the bottom rather than the dialog shrinking to fit. So the dialog height is **fixed** (679 px), not content-driven.

Note the label casing quirk read verbatim: `Mouse wheel Sensitivity` (lower-case "wheel", capital "Sensitivity").

**Changed from 163909:** menu dismissed, dialog opened, scrim back on.

---

## Consolidated findings

### A. Menus

| Where | Label | Type | Value or options | What it does | Notes |
|---|---|---|---|---|---|
| Menu bar x 155–226 | `Attributes` | menu button | — | opens the Attributes dropdown | orange highlight while open |
| Attributes menu | `General...` | menu item | — | opens `Edit: General` | no icon, no shortcut |
| Attributes menu | `Environment...` | menu item | shortcut **`Ctrl+I`** | opens `Edit: Environment` | sun-behind-cloud icon; shown hovered in 163658 |
| Attributes menu | `Multiplayer...` | menu item | — | opens `Edit: Multiplayer` | |
| Attributes menu | `Performance...` | menu item | — | opens `Edit: Performance` | |
| Menu bar x 227–275 | `Tools` | menu button | — | opens the Tools dropdown | |
| Tools menu | `Debug Console...` | menu item | shortcut **`section`** (the `§` key) | opens the script debug console | Arma spells the key name out |
| Tools menu | `Functions Viewer...` | menu item | — | browse scripted functions | italic `fx` icon |
| Tools menu | `Config Viewer...` | menu item | — | browse the config tree | `{ }` icon |
| Tools menu | `Animations Viewer...` | menu item | — | browse animations | |
| Tools menu | `Camera...` | menu item | — | camera tool *(inferred)* | |
| Tools menu | `Field Manual...` | menu item | — | in-game field manual | |
| Menu bar x 275–338 | `Settings` | menu button | — | opens the Settings dropdown | |
| Settings menu | `Preferences...` | menu item | shortcut **`Ctrl+K`** | opens `Edit: Preferences` | separator below it |
| Settings menu | `Video Options...` | menu item | — | game video settings | |
| Settings menu | `Audio Options...` | menu item | — | game audio settings | |
| Settings menu | `Game Options...` | menu item | — | game gameplay settings | |
| Settings menu | `Controls...` | menu item | — | key bindings | |
| Menu bar | `Scenario`, `Edit`, `View`, `Play`, `Help` | menu buttons | — | not opened in this batch | |

### B. `Edit: General` (scenario attributes)

| Where | Label | Type | Value or options | What it does | Notes |
|---|---|---|---|---|---|
| Presentation | `Title` | text field | *(empty)* | scenario display name | |
| Presentation | `Author` | text field | **`Darkforce`** | scenario author | only non-default value in the batch |
| Overview | `Picture` | text field | *(empty)* | path to overview image | |
| Overview | `Text` | multiline box | *(empty)* | overview description | drawn as a captioned group box |
| Overview | `DLC` | combo | **`None`** | required/advertised DLC | |
| Overview | `Require DLC` | checkbox | **checked** | enforce DLC ownership | |
| Overview (Locked) | `Picture` | text field | *(empty)* | image shown when locked | |
| Overview (Locked) | `Text` | multiline box | *(empty)* | text shown when locked | |
| Loading Screen | `Picture` | text field | *(empty)* | loading-screen image | |
| Loading Screen | `Text` | multiline box | *(empty)* | loading-screen text | |
| States | `Show Briefing` | checkbox | **checked** | briefing screen available | |
| States | `Show Debriefing` | checkbox | **checked** | debriefing screen available | |
| States | `Enable Saving` | checkbox | **checked** | mid-mission saves | |
| States | `Show Map` | checkbox | **checked** | map available to player | |
| States | `Show Compass` | checkbox | **checked** | compass available | |
| States | `Show Watch` | checkbox | **checked** | watch available | |
| States | `Show GPS` | checkbox | **checked** | GPS available | |
| States | `Show HUD` | checkbox | **checked** | HUD available | |
| States | `Show UAV Feed` | checkbox | **checked** | UAV feed available | |
| States | `Advanced Flight Model` | checkbox | **unchecked** | forces AFM | |
| States | `Debug Console` | combo | **`Available only in editor`** | who can open the console | other options not shown |
| Unlock | `Unlocked Keys` | text field | *(empty)* | campaign keys this scenario grants | |
| Unlock | `Required Keys` | text field | *(empty)* | keys needed to play | |
| Unlock | `Required Keys Limit` | numeric field (narrow) | **`0`** | how many of them are required | |
| Init | `Init` | multiline code box | *(empty)* | scenario init script | |
| Misc | `Independents Allegiance` | icon toolbox (2 options) | blue handshake **selected**, red diamond not selected | which side Independent is friendly to | icons only, no text labels |
| Misc | `Binarize the Scenario File` | checkbox | **checked** | save `mission.sqm` binarised | |
| footer | `OK` / `CANCEL` | buttons | — | commit / discard | 100×19, bottom-right |

### C. `Edit: Environment`

| Where | Label | Type | Value or options | What it does | Notes |
|---|---|---|---|---|---|
| Date | `Date` | 3 combos in one row | **`2035`**, **`June`**, **`24` (`Sun`)** | scenario date | day combo shows the weekday as a greyed secondary label |
| Date | `Time` | day/night gradient slider + time box | **`12 : 00 : 00`** | time of day | sun glyph as thumb; secondary progress bar under the track |
| Weather Forecast | `Time of Changes` | slider + time box | **`00 : 30 : 00`** | how long weather takes to reach the forecast | |
| Overcast | `Overcast Start` | slider + value | **`30%`** | cloud cover at mission start | |
| Overcast | `Overcast Forecast` | slider + value | **`30%`** | cloud cover target | |
| Fog | `Fog Start` | slider + value | **`0%`** | fog at start | |
| Fog | `Decay` (start) | slider + value | **`1%`** | fog decay with altitude | |
| Fog | `Base` (start) | slider + value | **`0 m`** | fog base altitude | |
| Fog | `Fog Forecast` | slider + value | **`0%`** | fog target | |
| Fog | `Decay` (forecast) | slider + value | **`1%`** | | duplicate label within the group |
| Fog | `Base` (forecast) | slider + value | **`0 m`** | | duplicate label within the group |
| Rain | `Manual Override` | checkbox | **unchecked** | enable the two sliders below | gate control |
| Rain | `Rain Start` / `Rain Forecast` | sliders + value | **`0%` / `0%`, disabled** | rain intensity | tooltip on Forecast: `Forecasted rain strength.` |
| Lightnings | `Manual Override` | checkbox | **unchecked** | gate | |
| Lightnings | `Lightnings Start` / `Lightnings Forecast` | sliders + value | **`0%` / `10%`, disabled** | lightning frequency | |
| Waves | `Manual Override` | checkbox | **unchecked** | gate | |
| Waves | `Waves Start` / `Waves Forecast` | sliders + value | **`10%` / `10%`, disabled** | sea state | |
| Wind | `Manual Override` | checkbox | **unchecked** (hovered) | gate | |
| Wind | `Wind Start` / `Wind Forecast` | sliders + value | **`10%` / `10%`, disabled** | wind strength | |
| Wind | `Gusts Start` / `Gusts Forecast` | sliders + value | **`0%` / `0%`, disabled** | gustiness | |
| Wind | `Direction Start` / `Direction Forecast` | blue `Z` chip + numeric field | **`0` / `0`, disabled** | wind bearing | only axis-tagged fields in the batch |

### D. `Edit: Multiplayer`

| Where | Label | Type | Value or options | What it does | Notes |
|---|---|---|---|---|---|
| Type | `Game Type` | combo | **`Undefined Game Mode`** | MP game-type tag for the server browser | |
| Type | `Min Players` | numeric (narrow) | **`0`** | minimum slots | |
| Type | `Max Players` | numeric (narrow) | **`0`** | maximum slots | |
| Lobby | `Summary` | text field | *(empty)* | lobby description | |
| Lobby | `Enable AI` | checkbox | **checked** | AI fills empty slots | |
| Lobby | `Auto Assign Slots` | checkbox | **unchecked** | auto-assign players to roles | |
| Respawn | `Respawn` | combo | **`Disabled`** | respawn mode | gates four controls below |
| Respawn | `Rulesets` | checkbox list | ☐ `Mission fail when everyone is dead`, ☑ `Singleplayer death screen` | respawn rule flags | checkbox-left layout, unlike the rest of the dialog |
| Respawn | `Respawn Delay` | slider + time | **`00 : 00 : 00`, disabled** | delay before respawn | greyed because Respawn = Disabled |
| Respawn | `Vehicle Respawn Delay` | slider + time | **`00 : 00 : 00`, enabled** | delay before vehicle respawn | stays live |
| Respawn | `Show Scoreboard` | checkbox | **checked, disabled** | | keeps its tick while greyed |
| Respawn | `Allow Manual Respawn` | checkbox | **checked, disabled** | | |
| Respawn | `Enable Team Switch` | checkbox | **checked, disabled** | | |
| Respawn | `Allow AI Score` | checkbox | **unchecked, enabled** | AI kills count for score | |
| Tasks | `Shared Objectives` | combo | **`Disabled`** | task sharing scope | |
| Revive | `Revive Mode` | combo | **`Disabled`** | incapacitation/revive system | |
| Revive | `Required Trait` | combo | **`None`** | trait needed to revive | |
| Revive | `Required Items` | combo | **`None`** | item needed to revive | |
| Revive | `Revive Duration` | slider + time | **`00 : 00 : 06`** | time to revive | |
| Revive | `Medic Speed Multiplier` | slider + value | *(hidden by tooltip)* | medics revive faster | tooltip `Hours` shown over the time segment |
| Revive | `Force Respawn Duration` | slider + time | **`00 : 00 : 03`** | hold-to-give-up duration | |

### E. `Edit: Performance`

| Where | Label | Type | Value or options | What it does | Notes |
|---|---|---|---|---|---|
| Garbage Collection | `Minimum distance` | text field (full width) | **`0`** | don't clean up within this distance of a player | |
| Garbage Collection → Character Corpses | `Mode` | combo | **`None`** | corpse-removal policy | |
| " | `Limit` | numeric (narrow) | **`15`** | max corpses kept | |
| " | `Min Delay` | slider + time | **`00 : 00 : 10`** | earliest removal | |
| " | `Max Delay` | slider + time | **`01 : 00 : 00`** | latest removal | |
| Garbage Collection → Vehicle Wrecks | `Mode` | combo | **`None`** | wreck-removal policy | |
| " | `Limit` | numeric (narrow) | **`15`** | max wrecks kept | |
| " | `Min Delay` | slider + time | **`00 : 00 : 05`** | | |
| " | `Max Delay` | slider + time | **`01 : 00 : 00`** | | |
| Dynamic Simulation | `Enable Dynamic Simulation` | checkbox | **checked** | master switch | |
| Dyn Sim → Activation Distance Settings | `Characters` | slider + value | **`500m`** | activation radius | |
| " | `Manned Vehicles` | slider + value | **`350m`** | | |
| " | `Props` | slider + value | **`50m`** | | |
| " | `Empty Vehicles` | slider + value | **`250m`** | | |
| Dyn Sim → Activation Distance Modifiers | `Is Moving` | slider + value | **`2x`** | multiplier while the entity moves | only `x` unit in the batch |
| " | `Limit by View Distance` | checkbox | **checked** | clamp activation to view distance | |

### F. `Edit: Preferences` (editor settings, not scenario data)

| Where | Label | Type | Value or options | What it does | Notes |
|---|---|---|---|---|---|
| Saving | `Auto-save` | combo | **`15 min`** | autosave interval | |
| Saving | `Binarize New Scenario Files` | checkbox | **checked** | default binarise setting for new scenarios | |
| Camera | `Default Speed` | slider + value | **`1x`** | camera fly speed | |
| Camera | `Fast Speed` | slider + value | **`1x`** | shift-boost speed | |
| Camera | `Mouse wheel Sensitivity` | slider + value | **`1x`** | wheel-driven speed change | label casing as written |
| Camera | `Copy Terrain` | checkbox | **unchecked** | camera follows terrain height *(inferred)* | |
| Camera | `Adaptive Speed` | checkbox | **checked** | scale camera speed with altitude *(inferred)* | |
| Camera | `Start in Map` | checkbox | **unchecked** | open the editor in 2D map mode | |
| Camera | `Start on Random Position` | checkbox | **checked** | randomise the initial camera position | |
| Misc | `Automatic Grouping` | checkbox | **checked** | auto-group units placed together | |
| Misc | `Recompile Functions` | checkbox | **unchecked** | recompile scripted functions on preview | |
| Misc | `Environmental Sounds` | checkbox | **unchecked** | ambient audio while editing | |
| Misc | `Automatic Composition Layering` | checkbox | **checked** | put placed compositions on their own layer | |

### G. Persistent editor chrome (recorded from 163901/163909)

| Where | Label | Type | Value or options | What it does | Notes |
|---|---|---|---|---|---|
| Left panel, x 2–20 y 52–68 | `«` | button | — | collapse the left panel | |
| Left panel tabs | `Entities` / `Locations` | tabs | `Entities` **selected** | switch panel content | |
| Left panel | *(search)* | text field + magnifier + `−` + pages-`+` | empty | filter the entity tree; collapse-all; *(3rd button unread)* | |
| Left panel tree | `BLUFOR` → `Alpha 1-1` → `Asst. Missile Specialist (AA)` | tree with per-row checkbox | BLUFOR/Alpha expanded, leaf **in red** | scenario entity hierarchy; checkbox = visibility | red colouring meaning not determined |
| Left panel tree | `OPFOR`, `Independent`, `Civilian`, `Empty`, `Ambient life`, `Triggers`, `Systems`, `Markers`, `Comments` | tree rows | all checked, all **greyed (empty)** | side/category buckets | |
| Left panel footer | trash · folder+ · folder⊘ · folder🔒 · folder👁 | buttons | — | delete; new layer; disable; lock; hide *(inferred from icons)* | trash far left, other four right-aligned |
| Right panel tabs | `Assets` / `History` / `»` | tabs + button | `Assets` **selected** | asset browser vs. undo history; `»` expands the panel | |
| Right panel | `F1`…`F6` | icon radio row with key hints | **`F2` active** | Units / Groups / Triggers / Waypoints / Systems / Markers *(inferred from icons + key order)* | F-key label printed above each icon |
| Right panel | side filter | icon radio row (6) | **blue/BLUFOR selected** | filter assets by side | last two plates unread |
| Right panel | *(search)* | ▾ + text field + magnifier + `−` + pages-`+` | empty | filter asset tree | ▾ is on the **left** here, unlike the left panel |
| Right panel tree | `CTRG`, `FIA`, `Gendarmerie`, `NATO`(expanded), `NATO (Pacific)`, `NATO (Woodland)` | tree | `NATO → Infantry` expanded, 13 group entries | asset catalogue by faction → category → asset | |
| Status bar | `X ↔` | readout | **`-4366.44 m`** | cursor/camera world X | |
| Status bar | `Y ↑` | readout | **`17518.9 m`** | world Y | |
| Status bar | `Z` (terrain glyph) | readout | **`-185.97 m`** | world Z / height | negative → below sea level under the cursor |
| Status bar | eye glyph | readout | **`10902.4 m`** | current view distance | |
| Status bar right | version | label | **`2.20.153973`** | game build | |
| Status bar right | `PLAY SCENARIO` / `IN SINGLEPLAYER` | large button + ▶ | — | launch the preview | occupies the full bar height |
| Top right | FPS | readout | **`70 FPS`** (`89` in the 1897-wide shots) | performance counter | green monospace |
| Viewport | translate gizmo | 3-axis widget | red `X`, green `Y`, blue `Z`, each labelled | move the selection | at ~(250–300, 950–1010) |

### H. Interaction design worth copying

1. **Modal scrim is a diagonal hatch, not a dim.** It lightens rather than darkens the blocked UI, so the dialog (dark on light) is the highest-contrast thing on screen. Instantly readable as "blocked".
2. **Dependency gating never hides controls.** Disabled rows keep their position, their label and their value (including a checked tick) and just drop to grey. Two mechanisms are shown: an explicit per-group `Manual Override` checkbox (Environment) and a mode combo whose value implies the dependents (`Respawn = Disabled` in Multiplayer).
3. **One consistent dialog shell.** Fixed 560×679 body, orange `Edit: <name>` title bar, right-aligned label column ending at a fixed x, controls starting at a fixed x, `OK`/`CANCEL` pinned bottom-right. Five different dialogs, zero layout variance. The shell does not resize to content — Preferences leaves 230 px of empty body.
4. **Scrollbar appears only when needed** and the content is scrolled with the group headers scrolling too (no sticky headers).
5. **Two heading levels** — collapsible groups (triangle + hairline) and non-collapsible sub-groups (indent + tick + lighter band) — gives Performance/Environment their structure without any tab strip. There are **no tabs inside any dialog in this batch**; categorisation lives entirely in the menu that opened it.
6. **Slider = arrows + track + editable value box.** Every numeric range in the whole batch uses this one composite, and the value box carries the unit (`30%`, `0 m`, `500m`, `2x`, `00 : 00 : 06`). Copy this and you cover Environment, Multiplayer and Performance in one widget.
7. **Time fields are three hoverable segments** with their own tooltips (`Hours`), so keyboard/mouse editing works per unit.
8. **The time-of-day slider is skinned to its domain** — day/night gradient track, sun thumb. Small touch, big legibility win.
9. **Menus carry the same icon as the toolbar button for the same command**, in a dedicated left gutter, and right-align their shortcut. Only `Environment... Ctrl+I`, `Preferences... Ctrl+K` and `Debug Console... section` have shortcuts among the menus opened here.
10. **Both side panels are translucent** over the 3D view, which keeps spatial context while the browser is open.
