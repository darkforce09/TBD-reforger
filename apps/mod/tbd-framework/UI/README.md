# UI Layouts Architecture Hub (`tbd-framework/UI`)

This directory owns the visual presentation layer (Enfusion `.layout` templates and their `.layout.meta` descriptors) for the TBD Framework.

---

## Directory Overview

```text
UI/
└── layouts/
    ├── Common/                    <-- Shared Component Library primitives
    ├── Hud/                       <-- Persistent in-game HUD overlays
    └── Session/                   <-- Match flow screens (mirrors Scripts/Game/TBD/Session/)
        ├── Shared/                <-- Session-wide modals & bars (VoicePanel, PlayersModal)
        ├── MissionSelector/       <-- Scenario browser & mission card layouts
        ├── Lobby/                 <-- ORBAT slotting screen & dock sub-layouts
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
| **`layouts/Common/`** | Atomic, reusable design primitives (`TBD_ScreenShell`, `TBD_ListRow`, and upcoming 9-slice buttons, cards, and tag chips). Reused across multiple screens. | `Scripts/Game/TBD/UI/Core/` & `UI/Common/` |
| **`layouts/Hud/`** | Minimalist in-game tactical overlays that render over live gameplay (`TBD_ObjectiveHud.layout`, capture bars). | `Scripts/Game/TBD/UI/Hud/` |
| **`layouts/Session/`** | The human interface screens for the player & referee journey. Each folder holds a screen shell layout with empty named docks, plus the sub-layouts injected into those docks at runtime. | `Scripts/Game/TBD/Session/<Feature>/UI/` |

---

## Technical Contracts & Rules

1. **The Dock / Sub-Layout Rule:**
   No `.layout` file should exceed 1,000 lines. Screens are built as lightweight shells containing empty named docks (`HeaderDock`, `SidebarDock`, `ContentSlot`), and modular sub-layouts are instantiated into them dynamically.

2. **Single Source of Truth:**
   All layout resource strings are registered once in `Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`. Screens must always call `TBD_UILayouts.Create(TBD_UILayouts.CONST, parent)` rather than hardcoding bare string paths.

3. **GUID & Meta Invariance:**
   Every `.layout` has a sibling `.layout.meta`. When moving layouts, preserve the `{GUID}` prefix inside the `.meta` file so that existing engine references and `resourceDatabase.rdb` entries maintain identity.
