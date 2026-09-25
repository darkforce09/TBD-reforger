**Status:** live

# Eden toolbar, shortcuts, actions and status bar

The Arma 3 Eden editor's toolbar entries, its keyboard shortcuts, the engine actions behind its
clipboard and transform commands, and the entries of its status bar, each as an ID with its Eden
wiki source. The toolbar's buttons and the status bar's read-outs are described panel by panel in
the [UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md).

## TOOLBAR — Index

The toolbar's buttons are listed in [UI anatomy § Toolbar](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md#toolbar-toolbar).
IDs: `TOOLBAR-NEW-001` … `TOOLBAR-TUTORIAL-001` (New, Open, Save, Workshop, Undo, Redo, widgets,
snap, grids, intel, map, flashlight, vision, phase, tutorials).

Wiki: https://community.bistudio.com/wiki/Eden_Editor:_Toolbar

## KEY shortcuts

| ID | Keys | Effect |
|----|------|--------|
| KEY-WP-001 | Shift+RMB | Quick waypoint |
| KEY-WIDGET-001 | Space | Cycle widget |
| KEY-GRID-001 | `;` | Translation grid toggle |
| KEY-HIDE-UI-001 | Backspace | Hide UI (screenshot) |

## ACTION appendix

Eden names 90+ `do3DENAction` actions; the scraped page is
`.ai/artifacts/eden-wiki/Eden_Editor__Actions.md`. The clipboard and transform actions:

| ID | Action | Effect |
|----|--------|--------|
| ACTION-COPY-001 | `CopyUnit` | Copy selection to clipboard |
| ACTION-CUT-001 | `CutUnit` | Cut selection |
| ACTION-PASTE-001 | `PasteUnit` | Paste at cursor |
| ACTION-PASTE-ORIG-001 | `PasteUnitOrig` | Paste at original position |
| ACTION-LEVEL-001 | `LevelWithSurface` | Align to terrain |
| ACTION-SNAP-001 | `SnapToSurface` | Snap to ground |
| ACTION-SEAT-001 | `ChangeSeat` | Crew seat change |
| ACTION-FORM-001 | `ForceToFormation` | Snap group to formation positions |
| ACTION-TOGGLE-SEL-001 | `ToggleUnitSel` | Toggle unit in selection |
| ACTION-WP-QUICK-001 | Shift+RMB | Quick MOVE waypoint |

Also: `MissionSave`, `SelectObjectMode`, `SyncWith`, `GroupWith`, `OpenAttributes`, `SearchEdit`, …

**Note:** the scraped entity context menu page,
`.ai/artifacts/eden-wiki/Eden_Editor__Entity_Context_Menu.md`, is primarily **modding/config**
(`class Cfg3DENContextMenu`); the end-user menu items are better sourced from the Switching from
2D Editor and Connecting pages and the per-type pages the other topic files cite.

## STATUS bar

| ID | Entry |
|----|-------|
| STATUS-X-001 | Cursor X |
| STATUS-Y-001 | Cursor Y |
| STATUS-Z-001 | Elevation |
| STATUS-ZOOM-001 | Zoom/resolution |
| STATUS-VER-001 | Game version |
| STATUS-MOD-001 | Mods loaded |
| STATUS-SRV-001 | MP server |

Wiki: https://community.bistudio.com/wiki/Eden_Editor:_Status_Bar
