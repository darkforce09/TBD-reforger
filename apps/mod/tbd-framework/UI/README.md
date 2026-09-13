# UI Layouts Architecture Hub (`tbd-framework/UI`)

This directory owns the visual presentation layer (Enfusion `.layout` templates and their
`.layout.meta` descriptors) for the TBD Framework.

---

## Directory Overview

```text
UI/
└── layouts/
    ├── Common/                    <-- Shared Component Library primitives (see Common/README.md)
    ├── Hud/                       <-- Persistent in-game HUD overlays
    └── Session/                   <-- Match flow screens (mirrors Scripts/Game/TBD/Session/)
        ├── Shared/                <-- Session-wide chrome: TBD_SessionTopBar, TBD_SessionBottomBar
        ├── MissionSelector/       <-- Scenario browser: dock shell + 7 sub-layouts (shipped 2026-09-12)
        ├── Lobby/                 <-- ORBAT slotting: dock shell + 9 sub-layouts (rebuilt 2026-09-13)
        ├── Briefing/              <-- Tactical briefing tabs & panels
        ├── Admin/                 <-- Mission control & referee console panels
        ├── Spectator/             <-- Broadcast glass pod & forensic trauma panels
        ├── Pause/                 <-- In-game pause & player options sidebar
        └── PostGame/              <-- Victory outcome banner & debrief AAR scoreboard
```

---

## Layout Domains

| Domain | Responsibility | Driven By Script |
|---|---|---|
| **`layouts/Common/`** | Atomic, reusable design primitives: `TBD_Panel`, `TBD_Chip`, `TBD_SearchBox`, `TBD_NavItem` + `TBD_TabStrip`, `TBD_Button`, `TBD_KeyValueRow`, `TBD_Dropdown` + `TBD_DropdownMenu`, `TBD_InsetText`, `TBD_Columns2`, plus the older `TBD_ScreenShell` and `TBD_ListRow`. Contract table: [`layouts/Common/README.md`](layouts/Common/README.md). | `Scripts/Game/TBD/UI/Core/` & `UI/Common/` |
| **`layouts/Hud/`** | Minimalist in-game tactical overlays that render over live gameplay (`TBD_ObjectiveHud.layout`, capture bars). | `Scripts/Game/TBD/UI/Hud/` |
| **`layouts/Session/`** | The human interface screens for the player & referee journey. Each folder holds a screen shell layout with empty named docks, plus the sub-layouts injected into those docks at runtime. `Shared/` holds the top/bottom bars every pre-game screen wears. | `Scripts/Game/TBD/Session/<Feature>/UI/` |

---

## The Dock Shell Contract

A screen shell is a `.layout` under 200 lines: a `Backdrop`, a `WindowFrame`, and empty, named
docks. `Scripts/Game/TBD/UI/Core/TBD_DockScreen.c` is the base class that fills them:

| Dock | Who mounts what |
|---|---|
| `TopDock` (56) | `TBD_DockScreen` → `Session/Shared/TBD_SessionTopBar` |
| `LeftDock` / `CenterDock` / `RightDock` | the screen → its panels (`Mount("LeftDock", TBD_UILayouts.PANEL)` …) |
| `BottomDock` (64) | `TBD_DockScreen` → `Session/Shared/TBD_SessionBottomBar` |
| `OverlayDock` (full-bleed, last child) | popovers / modals (`TBD_DropdownComponent`); hidden while empty |

Column widths are each shell's own business (selector 320 / 440 / rest; lobby 280 / rest / 380).
Reference shell: [`layouts/Session/MissionSelector/TBD_MissionSelector.layout`](layouts/Session/MissionSelector/TBD_MissionSelector.layout).

---

## Technical Contracts & Rules

1. **The Dock / Sub-Layout Rule:**
   No `.layout` file should exceed 1,000 lines; a shell stays under 200. Screens are shells with
   named docks; modular sub-layouts are instantiated into them dynamically.

2. **Single Source of Truth:**
   All layout resource strings are registered once in `Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`.
   Screens must always call `TBD_UILayouts.Create(TBD_UILayouts.CONST, parent)` (or
   `CreateStretched` / `CreateHandler`) rather than hardcoding bare string paths.

3. **GUID & Meta Invariance:**
   Every `.layout` has a sibling `.layout.meta`. When moving layouts, preserve the `{GUID}` prefix
   inside the `.meta` file so that existing engine references and `resourceDatabase.rdb` entries
   maintain identity. GUIDs are `7BD1A7000000XXnn` (`XX` = block, `nn` = `00` root, `01` meta,
   `02+` children); the block ledger is the header of `TBD_UILayouts.c`.

4. **Theme, not literals:**
   Colour comes from `TBD_UITheme` tokens and `TBD_EUITint`; icons from `TBD_UIIcons` keys. A
   `.layout` carries placeholder colours only; the handler repaints on attach.

5. **Workbench pass after new files:**
   New `.c` files need a Workbench cold restart; new `.layout` / `.conf` files are invisible until
   Workbench rewrites `resourceDatabase.rdb`. Headless compile: `cargo xtask mod compile`.
