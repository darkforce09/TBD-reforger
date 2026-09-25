**Status:** live

# Spectator

Where a dead player spends the rest of the [event](/documentation_v2/glossary.md#event) under one
life: a free, follow or first-person camera, and a roster of the living players they may watch.
The [mission](/documentation_v2/glossary.md#mission)'s spectator policy decides who may be watched and when.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/README.md)
  (the camera, the client controller, the roster rules and the server streaming host) and
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/README.md)
  (`TBD_SpectatorScreen`, the roster).
- Layout: the shared list shell, `apps/mod/tbd-framework/UI/layouts/Common/TBD_ScreenShell.layout`,
  repainted transparent; [`apps/mod/tbd-framework/UI/layouts/Session/Spectator/`](/apps/mod/tbd-framework/UI/layouts/Session/Spectator/README.md)
  holds no layout.
- Entry: `TBD_SpectatorComponent` on `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`,
  which starts the client controller and the server host.
- Input: vanilla's `ManualCameraContext` for flight, and the `TBD_SpectatorContext` actions in
  `apps/mod/tbd-framework/Configs/System/`.
- Related features: the [play area warning](/documentation_v2/mod/tbd-framework/UI/play_area_warning/play_area_warning_specification.md),
  whose kill penalty sends a player here; the
  [in-game menu](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md),
  whose admin screen gives a spent life back.

## Behaviour

### Entering and leaving

1. Four times a second the client asks whether it controls a living character. When it no longer
   does, after it had one, it enters spectating at the corpse; a player who joins with a spent
   life and no body enters after 20 s.
2. The mission's `settings.spectatorPolicy` decides the rest:
   - `none`: the player stays on the game's death view with no spectator camera;
   - `own_side_delayed_60s`: the camera appears 60 s after death, and the roster lists the
     viewer's own side only;
   - `free`: the camera appears at once and the roster lists every side;
   - no policy: the camera appears at once, own side only.
3. When an admin gives the life back (`#tbd respawn` or the admin screen's respawn), the next check
   finds a living body and the player leaves spectating.

### The camera

1. Free flight uses the keys the player already has bound for the game's manual camera: move,
   climb, turn, and the speed wheel, from 0.15x to 12x of 18 m/s. The camera stays at least 0.6 m
   and at most 2500 m above the ground.
2. Following orbits the chosen player at 1.5 to 60 m (the speed wheel sets the distance); first
   person looks through their eyes.
3. Keys: Tab opens or closes the roster, Right and Left cycle to the next or previous player, V
   toggles first person, F returns to free flight. Each is also a click on the roster.

### The roster

1. The roster, titled "SPECTATOR", opens over the live camera and never pauses it. Its subtitle
   reads "Your life is spent. You may watch your own side." or "Your life is spent. All sides
   visible.".
2. Every second it lists the players the viewer may watch, grouped by faction and group; the one
   being followed shows "FOLLOWING" or "FIRST PERSON". Players out of streaming range are counted
   as "<n> more not in view — fly closer".
3. Clicking a player follows them; clicking the followed player again toggles first person;
   "FREE CAMERA" returns to free flight. When the followed player dies: "That player is no longer
   alive — back to free camera.".
4. An empty list explains itself: "Your faction could not be resolved — no targets shown.",
   "Nobody alive nearby. Fly toward the AO to pick players up." or "Nobody left alive to watch.".

### What the server does

1. The game streams the world around the controlled entity, not the camera, so the server gives
   each dead player an inert, damage-free host entity to possess and moves it to the position the
   client reports every half second.
2. Each reported position is clamped to the configured range from the death position: 2000 m by
   default, and 0 or less also means 2000 m on the component. A host is never a character and
   never damageable, so it cannot become a way back into the world.

### Known discrepancies

- The range is never unlimited (`TBD_SpectatorComponent.c`, `m_fHostMaxRangeM`) — but
  `TBD_SpectatorHost` still logs "0 = unlimited" and skips the clamp when its own maximum is 0 or
  less; the component always hands it a positive value, so the branch is unreachable today.
- The own-side restriction runs on the client (`TBD_SpectatorTargets`): it is a discipline
  measure, not a security boundary; the game's replication range is the hard limit.

## Data

The feature makes no HTTP call. Its wire, on the modded `SCR_PlayerController` (the spectator
README's [Authority](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/README.md#authority)
lists it):

- `TBD_RpcAsk_SpectatorHostAt(vector)`, Unreliable, Server: the camera position. The server moves
  the host of the calling player only, clamped to the range, and trusts no other position.
- The spectator policy arrives as a replicated `TBD_FrameworkManager` property.
- The spent life is `TBD_SpawnManager`'s one-life record; the roster reads factions and groups
  from what the client has streamed.

## Design

- As built: the list shell with a transparent backdrop and a `SURFACE_GLASS` panel, listing the
  roster with one primary action, "FREE CAMERA", over the live camera. Nothing else is drawn over
  the world.
- Design target: the four Stitch sets in
  [visual_references](/documentation_v2/mod/tbd-framework/UI/spectator/visual_references/README.md)
  (a broadcast top bar, a split-faction bottom bar, a tactical roster sidebar and a combat details
  panel), design-phase references, and the Arma 3 captures in
  [reference_screenshots](/documentation_v2/mod/tbd-framework/UI/spectator/visual_references/reference_screenshots/README.md)
  the design started from. The built spectator has none of the following:
  - a bottom rail with the watched player in their faction's colour, the round clock, each side's
    living count and each side's "Kills to win";
  - a top bar with the same counts and each objective's owner and capture state;
  - a sidebar roster with per-side counts, squads, roles and vehicle seats, where the built roster
    is a flat list grouped by faction and group;
  - death forensics: the killer, the fatal weapon, calibre and range, the hits taken by body part,
    the viewer's own kills and shot accuracy, and a 3D beacon over the killer;
  - world billboards: nameplates for the living, vehicle occupant tags and mine markers;
  - a UI toggle for a clean view, a map key, night vision and thermal cycling, and nameplate
    toggles;
  - a spectator voice channel apart from the living players' radios; the [mod](/documentation_v2/glossary.md#mod) changes no voice
    routing on death.
- No open ticket covers these differences.

## Open work

- [T-946.50 — SpectatorHost still treats 0 as unlimited](/.ai/tickets/T-946.50.toml) (idea, no
  plan): `TBD_SpectatorHost` drops the unlimited branch and its log text, matching the component.

## Decisions

- The spectator camera is its own `SCR_CameraBase`, not the game's manual camera: that camera's
  behaviour lives in a prefab, it is the editor camera, and it cannot follow a player.
- Free flight reuses the game's manual-camera actions, so it needs no new input resource; the TBD
  keys are also clicks on the roster, so every action still works before
  [Workbench](/documentation_v2/glossary.md#workbench) registers the new ones.
- The client polls for a living body instead of hooking a death event: whether it controls a living
  character is a question the client answers itself, reconnects included.
- The range defaults to 2000 m and is never unlimited: a modified client could otherwise stream the
  whole map through its host.
