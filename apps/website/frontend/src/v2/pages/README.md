# Platform Pages Hub (`src/v2/pages`)

All platform document pages and the persistent navigation frame are organized here.

---

## Directory Structure

```text
pages/
├── navigation/        <-- Persistent platform navigation frame (Sidebar, TopNav, NavConfig)
│
├── command_center/    <-- Hub 1: Dashboard, Server Intel, Announcements
├── operations/        <-- Hub 2: Operations Schedule, Event Dossier & Slotting
├── mission_hub/       <-- Hub 3: Mission Library, Overview Dossier, Scenario Creator
├── field_tools/       <-- Hub 4: Mortar Ballistics Calculator, Debug Viewers
├── doctrine_and_info/ <-- Hub 5: Wiki Knowledgebase, Vehicle Index, Modpacks
└── administration/    <-- Hub 6: Event Admin, Server Control, Personnel, Approvals, CMS
```

Every page route adheres to the **Page-as-a-Folder** pattern: a main `page.rs` layout file composing focused sub-panel files (< 300 LOC each).
