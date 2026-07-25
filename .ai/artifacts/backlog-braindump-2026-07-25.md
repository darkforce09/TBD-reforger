# Backlog brain-dump — 2026-07-25

Raw capture of the owner's verbal backlog dump, structured but not yet filed as tickets.
Nothing here is analysis; it is what was in his head and nowhere else. Four derivation
agents are separately deriving the gaps he *couldn't* name — see the companion sections
marked `[DERIVED]` once those land.

Status key: `NEW` = no ticket known · `STALE?` = a ticket may exist but may not match intent.

---

## Mission Creator — authoring

| # | Item | Status |
|---|---|---|
| 1 | Item data is inadequate — needs improving | NEW |
| 2 | Vehicle data does not exist — needs sourcing | NEW |
| 3 | Vehicle inventory/cargo model | NEW |
| 4 | Easy UI to add inventory items to a vehicle | NEW |
| 5 | Arsenal is far short of target — needs a proper container/inventory system | STALE? |
| 6 | Full mission export: not just per-slot loadouts, but slot locations, object/entity locations, everything | NEW |
| 7 | Markers — cannot be placed at all (`eden_chrome.rs:1528` defers to T-069) | STALE? |
| 8 | Objectives — cannot be authored at all; mod has a runner with nothing to feed it | NEW |
| 9 | Custom play area / play zone — no authoring, and no decision on JSON representation | NEW |
| 10 | ORBAT / faction / side system — "70% there", not fully connected | STALE? |

## Mission Creator — settings & UX

| # | Item | Status |
|---|---|---|
| 11 | No mission SETTINGS tab. Wants OFCRA/WOG-style: duration, view distance, thermals on/off, weather | NEW |
| 12 | Those settings must actually take effect in-game, not just serialize | NEW |
| 13 | General editor UX is poor — no clear path to mission-level controls | NEW |

## Mission Creator — collaboration

| # | Item | Status |
|---|---|---|
| 14 | Git-diff-style versioning for missions | NEW |
| 15 | Realtime multiplayer / collaborative editing | NEW |
| 16 | Commenting + mission review workflow: reviewer leaves feedback, author sees and acts on it | NEW |

## Website platform

| # | Item | Status |
|---|---|---|
| 17 | Server manager — not wired up. Start/stop servers from the site | NEW |
| 18 | Mod manager — push the right modpack to a server | NEW |
| 19 | Proper Discord integration | STALE? |
| 20 | Discord bot | NEW |
| 21 | Event calendar exists but is not properly integrated | STALE? |
| 22 | "Live site system" — intent ambiguous; candidates are live server status, live event state, or live stats | NEEDS CLARIFICATION |

## Mod

| # | Item | Status |
|---|---|---|
| 23 | Command / ARC menu — needs connecting up | NEW |
| 24 | Mission triggers | NEW |

## End-to-end

| # | Item | Status |
|---|---|---|
| 25 | **Never tested:** place a unit in Mission Creator → export JSON → load in the mod. The single highest-information test available | NEW |

## Meta

| # | Item | Status |
|---|---|---|
| 26 | Registry staleness — some shipped tickets no longer reflect intent; design changed after the fact | IN PROGRESS |
| 27 | "I don't know what I don't know" — the unknown-unknowns gap | IN PROGRESS (derivation agents) |

---

## Owner's own framing, verbatim in substance

- The blocker is not writing code; it is having to hold the correct end-state of the
  mission creator, the website, Discord and the mod in one head simultaneously.
- He is aware items 1–25 are incomplete as a list, and that is the part that hurts most.
