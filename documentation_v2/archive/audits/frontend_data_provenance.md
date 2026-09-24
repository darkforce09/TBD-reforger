# Frontend data provenance — which render sites are API-fed, which are still mock

Derived 2026-07-26 by an exhaustive sweep of every `view!` render site in
`apps/website/frontend/src/`. Recorded because it cost ~150k tokens to derive and at least five
open tickets need it. **Line numbers are as-of that date and drift; the per-file classification
does not.**

Two things this exists to answer without re-deriving:
  1. Is a given column actually on somebody's screen? (Several are not — see "write-only".)
  2. Is this page real yet, or is a demo const standing in?

## Files that render ONLY hardcoded consts — not attack surface, not real yet

| file | const | note |
|---|---|---|
| `vehicles.rs` | `VEHICLES` `:46` | zero `api_*` calls |
| `modpacks.rs` | `MOCK_MODPACKS` `:31` | zero `api_*` calls |
| `wiki.rs` | `MANUALS` `:70` | zero `api_*` calls. `render_markdown` `:215` builds Leptos nodes, so it cannot emit attacker HTML — but it is the obvious future raw-HTML sink |
| `content.rs` | `mock_docs()` `:28` | **write surface only.** It POSTs to `/cms/announcements` `:277` but its own list is mock, so an author cannot see what they just published. See T-267 |

`arsenal.rs` is **mixed, not mock** — the `inner_html` paper-doll at `:1392` is fed by the
`Region { shape }` const tables `:1320-1366` (`&'static str`, no data path in), but the gear picker
renders `RegistryItem.display_name` from `GET /registry` (fetched `mission_editor.rs:196`).

`leaderboards.rs` is **fully API-fed** — T-195 deleted its `MOCK` ladder; only `LEADERBOARD_TABS`
(static UI copy) remains.

`mission_doc.rs` and `mission_history.rs` render **nothing** — the first is the `MissionDocCore`
host + `window.__missionDoc` bridge, the second the undo/redo driver. Neither is a page.

`orbat_manager.rs` is **CRDT-doc-fed, not REST-fed** — squad labels and slot lines come from the
local yrs graph. Only the faction library (`GET /factions` `:268`) and the vehicle picker
(`GET /registry`) are REST. That is a different write surface than any `api_post`.

## Mock values still on a real user's screen

These pages are API-fed but interleave demo constants, so they read as working data:

- `deployments.rs` — K/D `:198`, Win Rate `:199`, Fav Weapon/Asset `:210-211` are all consts
  (`:83-87`). Same page renders real `total_operations` next to them.
- `event_hub.rs` — maker/duration `:324,326`, BLUFOR/OPFOR objectives `:374,393`, vehicles `:470`.
- `missions.rs` — `thumbnail_url ?? PLACEHOLDER_ART` `:24` (a legitimate fallback).
- `server_intel.rs`, `dashboard.rs` — hero/theater art only.

## Columns with NO frontend consumer at all

Written by the API, never rendered anywhere. Do not spend a ticket "fixing" their display:

`wiki_pages.body_md` (the largest text column in the schema) · `missions.rejection_reason`
(written `approvals.rs:328`; **the author is never shown why their mission was rejected**) ·
`users.ban_reason` · `warnings.reason` · `fire_missions.fp_grid` · `fire_missions.target_grid` ·
`modpacks.workshop_url` (reaches an `<a href>` at `event_hub.rs:236` but has no writer — the
`registry_import.rs:82` INSERT omits the column).

## Ranked authored-text → other-user's-screen paths

For anyone auditing the write side, in descending order of exposure:

1. `announcements` `title`/`body`/`snippet`/`tag` — written `content.rs:277`, rendered
   `announcements.rs:80,172,175,179,282` + `dashboard.rs:440,446`. **The one real
   authoring→render pair in the whole frontend.**
2. `events.briefing`/`name_override` — `events.rs:362,391`, `event_hub.rs:211,359`,
   `orbat_selection.rs:90`.
3. `missions.briefing`/`title` — `mission_overview.rs:855,930`, `missions.rs:540,778`,
   `approvals.rs:371,399`.
4. ORBAT `squad`/`callsign`/`role`/`loadout`/`tag` — `event_hub.rs:813-1202`. Written through the
   CRDT doc + `PUT /factions/:id`, not a plain REST body.
5. `audit_logs.message`/`action`/`actor_name`/`metadata` — `audit.rs:216,217,304,316,331`.
   Server-generated, but `metadata` is free-form jsonb landing in a `<pre>`.

## Identity fields — no website form writes any of them

`username`, `discord_handle`, `arma_character` all originate from Discord OAuth or the game-server
link flow. **There is no `nickname`, `bio`, `pronouns` or `description` column anywhere in this
frontend**, and `settings.rs` has no profile-editing form — its only writes are link-code generate
and unlink. Any "sanitise the user's bio" ticket is describing a field that does not exist.
