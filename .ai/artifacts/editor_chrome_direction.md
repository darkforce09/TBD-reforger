# Editor chrome — design direction

**Operator direction, 2026-08-01:** *"I want it to pretty much look like the screenshots I took. The
current UI is very inconsistent and very disjointed. Something I like is the bar at the bottom."*

Two decisions taken on top of that:

1. **Eden's layout, Aegis's colours.** Match Eden's *structure* — geometry, state vocabulary,
   density — but keep the Aegis palette so the Mission Creator still belongs to the Library, Events
   and ORBAT pages it sits inside. Not a reskin.
2. **The bottom bar stays, stretched to full width.** Same content and feel, spanning the viewport
   like Eden's status bar instead of floating centred.

This supersedes the framing of group B in
[`editor_ui_ticket_drafts.md`](editor_ui_ticket_drafts.md), which treated the chrome as nine
independent measured defects. It is one layout, and the defects are symptoms.

All Eden numbers below are measured from the 75-screenshot corpus — see
[`eden_screenshots/README.md`](eden_screenshots/README.md) for provenance and the batch-level
conflicts already adjudicated.

---

## The target geometry

Eden, at 1920×1077, verified consistent across all 75 screenshots:

| Region | Eden | TBD today |
|---|---|---|
| Menu bar | `y 0–22` — 8 menus | one 48px strip holding **everything** |
| Toolbar | `y 22–40` — 25 icon buttons, **its own row** | — |
| Left panel | `x 0–240` | 256px |
| Right panel | `x 1680–1920` — **240, same as left** | ~310px, **and the 5th tab is clipped off-screen** |
| Panel top | `y 47` | 48 |
| Status bar | full width, bottom | floating centred pill, ~580px wide |
| Collapse | **24×24 chevron**, each panel's outer top corner | none |

**Four concrete moves:**

1. **Split the top strip into two rows.** Eden fits 8 menus *and* 25 tool icons into 40px. We fit 5
   menus, the title, a time scrubber, a weather select, undo/redo/history, three buttons and a gear
   into 48px — which is why it reads as crowded. Menus on row 1, an icon toolbar on row 2.
2. **Equalise the docks to one width** and fix the clipped tab. Eden is 240/240 in every screenshot.
3. **Add the collapse chevrons** — already ticketed as T-638; the viewport reflows 1440→1920px.
4. **Stretch the bottom bar to full width.** This also gives the scale bar and grid references
   (T-641) somewhere natural to live, and makes the right end a home for a primary action — Eden
   puts `PLAY SCENARIO` there on its own black surface.

---

## The state vocabulary — this is what actually fixes "disjointed"

Geometry is the visible half. The reason Eden reads as *one product* and ours reads as assembled is
that Eden uses **one state language everywhere**, and we use several.

Adopt these four rules across the whole editor chrome:

| State | Eden's treatment | Why it matters |
|---|---|---|
| **Hover** | solid amber fill | **Orange is hover, NOT toggled-on.** Easy to copy backwards — batch 08 proved it via frame `164000`, where the New button is amber purely because the cursor is over it |
| **Toggled on** | lighter plate + **1px dark top border** | Distinct from hover by construction, so the two can never be confused |
| **Disabled** | dimmed glyph — **and it still shows its tooltip** | Verified on Redo. A disabled control that explains itself is strictly better than one that goes silent |
| **Unavailable** | two deliberate strategies, chosen by verb | Clipboard verbs (Cut/Copy/Delete) **keep their slot and grey out**; scope/query verbs (Select…, Log…) are **dropped from the menu entirely** |

Plus three conventions worth taking wholesale:

- **Reserve the checkmark gutter always.** Eden only allocates it when a menu happens to have a
  checked item, so label indent jumps between menus. That is a bug to *not* copy — reserve it
  unconditionally and menus stop shifting.
- **`…` means "opens a dialog".** Consistently, everywhere.
- **Parenthetical scope qualifiers** — `(Selected)` / `(View)` — and axis glosses `(Width)`,
  `(Length)`, `(Height)`.

---

## What we keep, and what Eden gets wrong

**Keep — Aegis palette.** Desaturated `#adc6ff` primary, the Aegis type scale, our glass surfaces.
The editor should not look like a different product from the rest of the platform.

**Keep — the bottom bar's content mix.** Operator likes it. Tools and readouts stay together;
only the geometry changes.

**Do not copy — translucent panels over a live 3D scene.** Batch 05 flagged it: Eden puts small
grey text on a moving background. Our map is 2D and static under the docks, but the legibility risk
is the same and we have no reason to take it.

**Do not copy — the jumping checkmark gutter.** See above.

**Do not copy — Eden's absence of map furniture.** Eden ships no scale bar, no north arrow, no
legend, no grid coordinate labels — only a small X/Y gizmo. The operator has already chosen to
exceed Eden here (T-641), because a 2D top-down planner needs distance cues that a 3D-first editor
does not.

---

## Effect on the drafted tickets

| Ticket | Was | Becomes |
|---|---|---|
| T-632 | "right dock tab strip overflows" | **absorbed** — a symptom of the 240px equalisation |
| T-633 | native `<input type=range>` + `<select>` | **unchanged** — replace with Aegis controls |
| T-634 | "top strip has no action hierarchy" | **rescoped** — split into two rows, Eden structure |
| T-635 | debug HUD overlaps the toolbelt | **unchanged** |
| T-636 | split toolbelt into tools/telemetry | **rescoped** — full-width status bar, content unchanged |
| T-637 | "dock density — 85% empty" | **rescoped** — Eden's density *and* the 240px equalisation |
| **new** | — | **State vocabulary pass** — hover/toggled/disabled/unavailable, one language, whole chrome |

The state-vocabulary ticket is new and is the one that most directly answers *"inconsistent and
disjointed"*. It is also cheap: it changes classes, not structure, and it should land **early** so
every later wave builds against the finished vocabulary rather than retrofitting it.

---

## Open

**Where does the primary action go?** Eden's bottom-right is `PLAY SCENARIO` on its own black
surface — the single loudest thing on screen. Our equivalent is either **Save Version** or
something playtest-shaped that does not exist yet. The full-width bottom bar creates the slot;
what fills it is undecided.
