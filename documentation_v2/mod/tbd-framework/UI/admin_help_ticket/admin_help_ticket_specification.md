**Status:** live

# Admin help ticket

A designed, unbuilt way for a player to ask the admins for help without leaving the round: a
request dialog with a category, a description and the player's position attached, and a tickets
module in the admin menu where an admin claims, resolves and acts on each request. No code
implements either; this document records the design target and what exists instead.

## Where it lives

- Code: none. What a stuck player can use today is the admin screen and the `#tbd` commands in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/README.md),
  run by an admin, and the game's own chat.
- Reference: the design follows the admin menu of the Coalition Reforger Framework (CRF), whose
  scripts sit in the gitignored local reference copy `apps/mod/crf_framework/`
  (`CRF_AdminMenuManager.c`, `CRF_PlayerRplToAuthorityManager.c`, `CRF_RplBroadcastManager.c`);
  TBD reads that code and never copies it.
- Related features: the [in-game menu](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md),
  whose designed admin sidebar has a Tickets entry and whose pause sidebar has "Contact Admin".

## Behaviour

Nothing is built. Today a player who needs an admin types in chat, and a listed admin acts with the
admin screen (respawn or deploy the player) or a `#tbd` command; the rest of the target's powers
(teleport, heal, uniform repair) do not exist in the [mod](/documentation_v2/glossary/g_to_m.md#mod).

### Known discrepancies

- The target opens the dialog with F8 — but F8 is bound to the `TBD_AdminMenu` action, which
  toggles the admin screen (`apps/mod/tbd-framework/Configs/System/Actions/TBD_AdminMenu.conf`).

## Data

None: no RPC, [API](/documentation_v2/glossary/a_to_f.md#api) call or storage exists for a ticket.

## Design

The design target, from the specification's wireframe and the
[admin tickets panel mockup](/documentation_v2/mod/tbd-framework/UI/admin_help_ticket/visual_references/admin_tickets_panel_mockup/README.md),
a design-phase reference.

### Player side

1. The player opens "REQUEST REFEREE ASSISTANCE" from the pause menu ("HELP TICKET") or a key,
   alive or spectating. The dialog sits centred over a darkened backdrop and takes the input.
2. Controls: a close cross (Esc); a category ("Stuck in Terrain", "Medical / Uniform Glitch",
   "Spectator Bug" or "Rule Violation / Admin Request"); a description of up to 256 characters with
   a live counter; a read-only card of attached data (callsign and side, grid and elevation, life
   state, vehicle, time and the number of admins online); "CANCEL" and "SUBMIT TICKET".
3. Submit sends the ticket to the server over a reliable RPC. It is disabled while the player has
   an open ticket and for 60 s after the last one.
4. When an admin claims the ticket the player sees "TICKET ACKNOWLEDGED: Referee <name> is
   reviewing your request."; on closure, a confirmation banner.

### Admin side

1. Every admin gets the ticket in their admin chat, "[TICKET #12] <player> (<side> @ <grid>)
   [<category>]: <description>", with a chime.
2. The tickets module lists tickets filtered by All, Pending and Resolved, each with its number,
   category, state, player, side, grid or claiming referee, and age.
3. The selected ticket shows its category and priority, submission time, the player with "Teleport
   To" and "Spectate", the grid, health, posture and transport, the description and quick tags.
4. Actions: "Unstick to Surface" (moves the player 1.5 m to the nearest open ground), "Full Heal &
   Revive", "Fix Uniform/Vest" (resyncs the containers without losing items), "Dispatch Toast to
   Player & Close" and "Resolve & Notify".

### Purpose the target serves

Players clipped into terrain or buildings, uniforms or vests that do not show, broken bleeding or
unconscious states, a spectator camera stuck on black, and rule breaches (safe start, theft,
deliberate team kills) reported with a position.

No open ticket covers the feature.

## Open work

None. Checked `.ai/tickets/` for open tickets on help tickets, player reports and the admin
tickets module.

## Decisions

None recorded: nothing is built.
