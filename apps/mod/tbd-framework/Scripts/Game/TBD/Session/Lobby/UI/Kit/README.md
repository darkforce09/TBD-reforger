# Lobby KIT INSPECTOR column

The right-hand column of the Lobby screen: the chosen seat's headline and chips, a 3D doll wearing
the seat's kit that turns on a drag and zooms on the wheel, and one card per kit section. The
briefing's ORBAT page reuses it beside its read-only roster.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/Kit/
├── TBD_KitInspectorCells.c     static builders of the stat-cell grids and weapon cards
├── TBD_KitInspectorPanel.c     the column: header, preview card and section cards per seat
└── TBD_KitPreviewComponent.c   the 3D doll: dressed character or vehicle, drag to turn, wheel to zoom
```

## How it works

`TBD_KitInspectorPanel.Show(slot, squad)` destroys the previous preview, clears the cards and
rebuilds from `TBD_LobbyCatalog.GetKit`: the header chips, a preview card, then GEAR, WEAPONS,
GRENADES, GADGETS, TOOLS, MEDICAL and MISC, skipping empty sections, and scrolls back to the top.
`TBD_KitInspectorCells` lays kit lines out two, three or four wide on `TBD_StatCell` and builds up
to three weapon cards with attachment and ammunition rows. `TBD_KitPreviewComponent` asks
`TBD_LoadoutPreviewDresser` for a preview entity wearing the kit's base prefab and loadout, shows it
through the engine's `ItemPreviewManagerEntity` with the character's full-body framing, turns it on
a left drag (polled at 30 Hz) and zooms it on the wheel; when anything is missing it captions
"PREVIEW UNAVAILABLE" and logs one WARNING. `ShowPrefab` and `ShowVehicle` serve the briefing's
uniform and vehicle cards.

## Authority

- Server: nothing.
- Client: everything; the column and preview run on the local player's machine.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_LobbyCatalog` and the kit models in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/Catalog/`; `TBD_LoadoutPreviewDresser` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/Preview/`; `TBD_UILayouts`,
  `TBD_UITheme`, `TBD_UIIcons` and `TBD_UIScrollBar` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; `TBD_PanelComponent`, `TBD_ChipComponent` and
  `TBD_KeyValueRowComponent` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Session/Lobby/`; the engine's `ItemPreviewManagerEntity`.
- Used by: `TBD_LobbyScreen` in the parent folder; `TBD_BriefingOrbatPage`,
  `TBD_BriefingUniformsPage` and `TBD_BriefingAssetsPage` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/Pages/`.
- Rules: the preview is destroyed before its card is cleared and unhooks its input; lines added
  stay ASCII and `cargo xtask mod compile` checks that the scripts compile.
