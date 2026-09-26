# Framework interface textures

The images the framework's screens draw that the game's data cannot supply: the disc the rounded
shapes are cut from, the masks and fade of the
[mission](/documentation_v2/glossary/g_to_m.md#mission) inspector's hero band, the hero art itself,
and the icon set. Vanilla texture GUIDs live only inside the game's packed data, so every image the
interface needs is a committed PNG that [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
imports here.

## Contents

```text
apps/mod/tbd-framework/UI/Textures/TBD/
├── Icons/                the 38 white interface icons `TBD_UIIcons` loads by key
├── TBD_Disc_UI.*         filled white disc for rounded-shape corners, `{1F2DC726318EC5AF}`
├── TBD_DiscInv_UI.*      clear disc in an opaque square for photo corners, `{8DF41982E2A2BBA5}`
├── TBD_FadeDown_UI.*     vertical fade under the hero title, `{5DB4948B38B2380A}`
├── TBD_Hero_Everon_UI.*  Everon's satellite band for the inspector hero, `{9D92B49A28C50256}`
└── TBD_HeroTopo_UI.*     grid art for terrains without imagery, `{B546577F58DCE62A}`
```

## How it works

Each texture is three files: the source `.png`, the `.edds` that Workbench writes when it imports
the PNG, and the `.edds.meta` that holds the resource GUID. `TBD_UILayouts`
(`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`) pins each GUID in a constant,
and `TBD_UILayouts.LoadTexture` puts a texture on an image widget; when the texture does not
resolve it hides the widget and logs the miss once, so a missing import shows an empty slot, never
a white square.

| Texture | Size | Constant | Where it shows |
|---|---|---|---|
| `TBD_Disc_UI` | 128 x 128 | `CORNER_DISC` | the four corner images of the eight `TBD_Rounded*.layout` shapes in `apps/mod/tbd-framework/UI/layouts/Common/`, which name it by GUID |
| `TBD_DiscInv_UI` | 128 x 128 | `CORNER_DISC_INV` | clipped quarters over the top corners of the mission inspector's hero photo, painted in the colour behind it (`MountCornerMask`) |
| `TBD_FadeDown_UI` | 8 x 128 | `FADE_DOWN` | the inspector hero's `HeroFade`, tinted to the colour it fades into |
| `TBD_Hero_Everon_UI` | 1024 x 140 | `HERO_EVERON` | the inspector hero for Everon, named by the terrain entry of the mock catalog |
| `TBD_HeroTopo_UI` | 1024 x 176 | `HERO_TOPO` | the inspector hero for Arland and Kolguyev, and the grid behind the 3D previews of the kit inspector and the briefing pages |

The shapes are white and the screens tint them: `TBD_UITheme` paints the image widgets, so one
disc serves every colour. The Everon band is cropped from Everon's satellite mosaic, 4096 x 560
pixels from y 1600, scaled to 1024 x 140.

## Format

- File type: the `.png` is an RGBA source image; the `.edds` is
  [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion)'s imported texture; the
  `.edds.meta` is a `MetaFileClass` whose `Name` holds `{GUID}UI/Textures/TBD/<name>.edds` and whose
  per-platform `PNGResourceClass` entries inherit the vanilla `TextureUI.conf`.
- Resource GUID: the `.edds.meta` file. The GUID is written once by the import and copied into
  `TBD_UILayouts`; a referenced GUID never changes.
- Naming: `TBD_<Subject>_UI`, the `_UI` suffix marking an interface texture; icons go in `Icons/`.
- Adding a texture: commit the PNG, import it in Workbench with `TextureUI.conf`, commit the
  `.edds` and `.edds.meta` it writes, and pin the new GUID in a `TBD_UILayouts` constant. Until the
  import, the bare path resolves nothing and `LoadTexture` hides the slot.

## Referenced by

- `apps/mod/tbd-framework/UI/layouts/Common/TBD_Rounded5.layout` to `TBD_Rounded12.layout` name
  `TBD_Disc_UI` by resource GUID in their `DiscTL`, `DiscTR`, `DiscBL` and `DiscBR` images.
- `TBD_UILayouts` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c` names all
  five by GUID and path; the callers use its constants:
  - `TBD_MissionInspectorPanel` in
    `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/` loads `FADE_DOWN`,
    `CORNER_DISC_INV` and the terrain's hero image, falling back to `HERO_TOPO`;
  - `TBD_MissionSelectorMock` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/` gives Everon
    `HERO_EVERON` and the other terrains `HERO_TOPO`;
  - `TBD_KitInspectorPanel` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/` and
    `TBD_BriefingPage` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/` load
    `HERO_TOPO` behind their 3D previews.
- `TBD_UIIcons` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UIIcons.c` loads the icons
  by path.

## Boundaries

- Depends on: the vanilla `TextureUI.conf` import settings that each `.meta` inherits.
- Used by: the rounded shapes in `apps/mod/tbd-framework/UI/layouts/Common/` and the screens listed
  above, through `TBD_UILayouts` and `TBD_UIIcons`.
- Rules: every image the framework owns is committed here as a PNG with its `.edds` and `.meta`;
  the three files of a texture are committed together; a GUID in `TBD_UILayouts` matches its
  `.meta`; shapes stay white, because the theme supplies the colour; a new texture is visible to
  the game only after Workbench has rewritten `resourceDatabase.rdb`.

## Related documentation

- [Mod UI structure](/documentation_v2/mod/tbd-framework/UI/README.md) — where each UI file goes
- [Mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md)
  — the mission inspector and its hero band
