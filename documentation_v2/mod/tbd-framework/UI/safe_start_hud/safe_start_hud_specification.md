**Status:** live

# Safe start HUD

What a player is told during safe start, the warm-up before a round goes live in which no one can
be hurt: the countdown and the weapons-cold and weapons-live notices. Under one life a single
negligent discharge before the start would end a player's [event](/documentation_v2/glossary/a_to_f.md#event),
so the notices must be impossible to miss. The [mod](/documentation_v2/glossary/g_to_m.md#mod) has no safe start panel of its own: the notices
are the game's pop-up banners and chat lines.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/README.md)
  (`TBD_SafestartManager.c`: the shield, the countdown, the pop-ups and the chat lines).
- Admin control: `#tbd safestart [status|go|<seconds>]`, served by `TBD_AdminService.Safestart` in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/README.md).
- Entry: `TBD_SafestartManager.OnStageChanged`, which `TBD_FrameworkManager.SetStage` calls on
  every stage change.
- Layout: none; the notices use the game's `SCR_PopUpNotification` and the chat feed.
- Related features: the [play area warning](/documentation_v2/mod/tbd-framework/UI/play_area_warning/play_area_warning_specification.md),
  which enforces nothing until the round is live; the
  [in-game menu](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md),
  whose staging-phase mockup draws a readiness panel for this phase.

## Behaviour

### The shield

1. The shield arms on `LOBBY`, `BRIEFING` and `SAFE_START` and lifts on any other stage. While
   armed, every character body has damage handling off, its weapon safety on, and every shot or
   grenade deleted the instant it exists; a re-sweep every 3 s covers bodies that appear later.
2. The server logs the first suppressed shot or throw per player as a negligent discharge, at
   warning level; the player is not told.
3. The lift restores each body's own earlier damage setting and reads it back. When any body stays
   unrestored, every player reads "[TBD] !! SAFESTART FAILED TO LIFT for some players — damage may
   still be OFF. Tell an admin NOW." and a watchdog retries every 5 s; when it recovers, "[TBD]
   Safestart lift recovered — damage is ON for everyone.".

### The countdown

1. The countdown runs on `SAFE_START` only, from the configured length: the [mission](/documentation_v2/glossary/g_to_m.md#mission)'s
   `flow.safeStartSeconds`, else 300 s; an admin's `#tbd safestart <seconds>` sets it within 5 to
   3600 s.
2. On arming, every player reads the chat line "[TBD] SAFESTART — damage OFF, weapons cold. Live
   in <MM:SS>." and each screen shows the pop-up "SAFESTART — WEAPONS COLD" with "No damage,
   rounds are suppressed. Live in <MM:SS>." for 8 s. A player who joins mid-countdown gets the same
   pop-up.
3. Chat repeats "[TBD] SAFESTART — live in <MM:SS>. Weapons cold, damage off." at 600, 300, 120,
   60, 30 and 10 s. The pop-up "SAFESTART — LIVE IN <MM:SS>" with "Weapons cold, damage off" shows
   for 3 s at those marks and also at 240, 180 and 15 s and each of the last 5 s.
4. At zero, or on `#tbd safestart go`, the stage machine moves to `LIVE`; the lift follows through
   the stage change.
5. After the lift, chat reads "[TBD] SAFESTART OVER — WEAPONS LIVE. Damage is ON, and you have ONE
   life." and the pop-up "SAFESTART OVER — WEAPONS LIVE" with "Damage is ON. You have ONE life."
   shows for 8 s.
6. `#tbd safestart <seconds>` while the countdown runs restarts it at the new length and tells
   everyone "[TBD] SAFESTART extended — live in <MM:SS>."; `#tbd safestart status` answers the
   admin with the phase, the time left, the bodies covered and the suppressed shots and grenades.

### Limits

The shield covers characters only; vehicles, static weapons and world objects keep their damage.
It cannot holster or lower a weapon: a shot still fires, with report, flash and recoil, and only
the projectile is deleted. Melee, falls, drowning and vehicle impacts are stopped by the damage
switch alone. A body that was already invulnerable before the sweep comes out of it still
invulnerable, and the lift line counts it.

## Data

The HUD makes no HTTP call and sends no RPC of its own:

- `m_iSecondsRemaining`, a replicated property of `TBD_SafestartManager`, carries the countdown;
  -1 means safe start is not running. Each client reacts in `OnCountdownReplicated`; the authority
  calls the same helper from its setter, because an authority never receives its own replication
  callback.
- The chat lines go from the server to every player through `TBD_PlayerChat.TellEveryone`.

## Design

- As built: vanilla pop-up banners, which appear wherever the game draws them, and chat lines.
  Nothing on screen persists between milestones, and nothing shows a boundary.
- Design target: the specification's wireframe; the folder has no mockup set. It draws a
  persistent 460 x 116 px glass panel 24 px below the top centre of the screen, and the built HUD
  has none of it:
  - a "SAFESTART ACTIVE" badge with a shield icon, amber while armed and green on lift;
  - a large `MM:SS` countdown that turns amber under 30 s and flashes red under 10 s;
  - a "WEAPONS COLD • DAMAGE OFF • ONE LIFE" line;
  - a staging-perimeter strip, "REMAIN INSIDE STAGING PERIMETER" with the distance to the edge,
    pulsing amber when the player strays; the mod enforces no staging boundary during safe start;
  - an admin badge, "PAUSED BY REF" or "TIME EXTENDED (+02:00)"; the mod has no pause, and an
    admin sets a new length rather than adding time;
  - a green release banner, "SAFE START ENDED — WEAPONS FREE" and "DAMAGE IS LIVE • CHECK TARGET
    IDENTIFICATION", held for 6 s with a radio chime; the built lift shows the vanilla pop-up for
    8 s with no sound;
  - the panel dimming while the player aims down sights.
- A pop-up rather than a mod screen: a mod menu preset resolves only after
  [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) regenerates `resourceDatabase.rdb`, and a
  countdown nobody can see is not a countdown.
- No open ticket covers these differences.

## Open work

None. Checked `.ai/tickets/` for open tickets on safe start, its countdown and its notices.

## Decisions

- The shield arms from `LOBBY`, not only `SAFE_START`: players stand together from the moment they
  deploy, so the window before the countdown needs the same protection.
- Damage off, projectile deletion and weapon safety stack: damage off is the enforcement, deletion
  covers bodies whose damage manager was not found, and safety is a convenience the player can
  undo.
- The lift is verified and retried, and a failed lift is broadcast: a safe start that fails to
  lift is worse than none, since bullets that do nothing are found only at first contact.
- Chat and pop-up both: the pop-up is glanceable, the chat line still there for a player who was
  looking at the map.
