# Eden_Editor:_Actions

Source: https://community.bohemia.net/wiki/Eden_Editor:_Actions

---

Actions are engine operations used by Menu Bar and are available for scripting.

Perform an action:

do3DENAction "actionName"

Get action state, can be 0 or 1. Available only for selected actions (see the State column below); when an action doesn't have any state, -1 is returned.

get3DENActionState "actionName"

Since  2.22 some actions can also have an argument. See syntax 2 of do3DENAction.

Action

Description

State

Argument

| 

##### AddCustomConnection

| 
No implemented. Can only happen through context menu.

| 

| 

| 

##### AddRandomStart

| 
Initializes the Set Random Start action. Same as if selecting the option in the context menu. At least one marker must be selected.

| 

| 

| 

##### AddUnitToSel

| 
Add entity under cursor to selection, the same as when you press Ctrl&#32;+  on an entity

| 

| 

| 

##### AddWaypoint

| 
Add quick waypoint to currently selected entities / groups. The same as when you press &#8679; Shift&#32;+ .

| 

| 

| 

##### CenterCameraOnPlayer

| 
Move camera to player's position.

| 

| 

| 

##### ChangeSeat

| 
No implemented. Can only happen through context menu.

| 

| 

| 

##### ClearSelections

| 
Cancel the current selection.

| 

| 

| 

##### CopyUnit

| 
Copy all selected entities. The same as when you press Ctrl&#32;+ C.

| 

| 

| 

##### CreateAndChangeComment

| 
Create a comment under cursor and open its attributes window.

| 

| 

| 

##### CreateComment

| 
Create a comment under cursor.

| 

| 

| 

##### CreateCustomComposition

| 
Create a composition from selected objects.

| 

| 

| 
 2.22

##### CreateTextFile

| 
Creates a text file in the scenario folder.

| 

| 
HashMap with the following keys:

- name: String - name of the file. Maximum length 64 characters. Can only contain A-z0-9_- characters. Name is force-prefixed with generated_

- folder: String - (optional, default "") - name of the subfolder. Only one level is supported. Maximum allowed characters 16. Can only contain A-z0-9_- characters

- content: String - content of the file

private _par = createHashMap;
_par set ["name", "testfile"];
_par set ["folder", "subfolder"];
_par set ["content", "R3voing"];

do3DENAction ["CreateTextFile", _par];

| 

##### CutUnit

| 
Cut all selected entities. The same as when you press Ctrl&#32;+ X.

| 

| 

| 

##### DeleteCustomComposition

| 
Delete a composition.

| 

| 

| 

##### DeleteUnits

| 
Delete all selected entities. The same as when you press Del.

| 

| 

| 

##### EditCustomComposition

| 
Edit a composition.

| 

| 

| 

##### Exit3DEN

| 
Exit the Eden Editor, with a prompt asking for confirmation being displayed first. The same as when you press Esc.

| 

| 

| 

##### FolderMission

| 
Opens the scenario folder if it exists.

| 

| 

| 

##### ForceToFormation

| 
Move all selected group members to their default formation positions.

| 

| 

| 

##### GroupWith

| 
Initializes the Group With action. Same as if selecting the option in the context menu. At least one group must be selected.

| 

| 

| 

##### LevelOutObjects

| 
Orient all selected entities to sea normal.

| 

| 

| 

##### LevelWithSurface

| 
Orient all selected entities to terrain normal.

| 

| 

| 

##### MissionExportMP

| 
Export the scenario to multiplayer scenarios.

| 

| 

| 

##### MissionExportSP

| 
Export the scenario to scenario scenarios.

| 

| 

| 

##### MissionLoad

| 
Show "Open Scenario" window.

| 

| 

| 

##### MissionMerge

| 
Opens the merge UI.

| 

| 

| 

##### MissionNew

| 
Show "New Scenario" window.

| 

| 

| 

##### MissionPartIntro

| 
Select Intro phase.

| 

| 

| 

##### MissionPartMission

| 
Select Scenario phase.

| 

| 

| 

##### MissionPartOutroLoose

| 
Select Outro - Lose phase.

| 

| 

| 

##### MissionPartOutroWin

| 
Select Outro - Win phase.

| 

| 

| 

##### MissionPreview

| 
Preview the scenario in singleplayer.

| 

| 

| 

##### MissionPreviewBriefing

| 
Preview the scenario in singleplayer with briefing.

| 

| 

| 

##### MissionPreviewMP

| 
Preview the scenario in multiplayer.

| 

| 

| 

##### MissionSave

| 
Save the scenario. If it has not been saved yet, open "Save Scenario" window.

| 

| 

| 

##### MissionSaveAs

| 
Open "Save Scenario" window.

| 

| 

| 

##### MoveGridDecrease

| 
Decreases the move grid by 0.25 m. If this grid doesn't exist a new entry is added to the drop down menu in the toolbar (3.75 m (Custom)).
Also triggers OnMoveGridDecrease event.

| 

| 

| 

##### MoveGridIncrease

| 
Increases the move grid by 0.25 m. If this grid doesn't exist a new entry is added to the drop down menu in the toolbar (3.75 m (Custom)).
Also triggers OnMoveGridIncrease event.

| 

| 

| 

##### MoveGridToggle

| 
Toggle translation grid.

| 

| 

| 

##### OpenAttributes

| 
Open attributes of selected entities.

| 

| 

| 

##### OpenIntelDisplay

| 
Opens the Edit: Environment UI.

| 

| 

| 

##### OpenRequiredAddons

| 
Opens the required addons UI.

| 

| 

| 

##### OpenSteamPublishDialog

| 
Open Steam Publish Options window.

| 

| 

| 

##### OptionsAudio

| 
Open Audio Options window.

| 

| 

| 

##### OptionsControls

| 
Open Controls Options window.

| 

| 

| 

##### OptionsGame

| 
Open Game Options window.

| 

| 

| 

##### OptionsVideo

| 
Open Video Options window.

| 

| 

| 

##### PasteUnit

| 
Paste copied entities. The same as when you press Ctrl&#32;+ V.

| 

| 

| 

##### PasteUnitOrig

| 
Paste copied entities on their original positions. The same as when you press Ctrl&#32;+ &#8679; Shift&#32;+ V.

⚠Keep in mind that Surface Snapping and the selected Vertical Mode will influence the positioning when pasting!

| 

| 

| 

##### PublishCustomComposition

| 
Opens the Workshop Composition Management UI. At least one custom composition has to be selected in the asset browser (right panel).

| 

| 

| 

##### Redo

| 
Redo the undone operation.

| 

| 

| 

##### RestoreEditTreeExpansion

| 
Restores previous state of the nodes in the entity browser (left panel).

| 

| 

| 

##### RotateGridDecrease

| 
Decreases the rotation grid by 5°. If this grid doesn't exist a new entry is added to the drop down menu in the toolbar (15 ° (Custom)).
Also triggers OnRotateGridDecrease event.

| 

| 

| 

##### RotateGridIncrease

| 
Decreases the rotation grid by 5°. If this grid doesn't exist a new entry is added to the drop down menu in the toolbar (15 ° (Custom)).
Also triggers OnRotateGridIncrease event.

| 

| 

| 

##### RotateGridToggle

| 
Toggle rotation grid.

| 

| 

| 

##### ScaleGridToggle

| 
Toggle area scaling grid.

| 

| 

| 

##### SearchCreate

| 
Switch focus to asset browser search field.

| 

| 

| 

##### SearchEdit

| 
Switch focus to entity list search field.

| 

| 

| 

##### SelectAllOnScreen

| 
Select all entities currently visible on the screen.

| 

| 

| 

##### SelectGroupMode

| 
Select the Compositions mode. The same as when you press F2.

| 

| 

| 

##### SelectLayerAllDescendants

| 
Select all entities in selected layers and all sub-layers.

| 

| 

| 

##### SelectLayerChildren

| 
Select all entities directly in selected layers.

| 

| 

| 

##### SelectMarkerMode

| 
Select the Markers mode. The same as when you press F6.

| 

| 

| 

##### SelectModuleMode

| 
Select the Systems mode. The same as when you press F5.

| 

| 

| 

##### SelectObjectMode

| 
Select the Objects mode. The same as when you press F1.

| 

| 

| 

##### SelectSubmode1

| 
Select the first sub-mode. Based on the selected mode; for example it is BLUFOR when Objects mode is selected.

| 

| 

| 

##### SelectSubmode2

| 
Select the second submode. Based on the selected mode; for example it is OPFOR when Objects mode is selected.

| 

| 

| 

##### SelectSubmode3

| 
Select the third sub-mode. Based on the selected mode; for example it is Independent when Objects mode is selected.

| 

| 

| 

##### SelectSubmode4

| 
Select the fourth sub-mode. Based on the selected mode; for example it is Civilian when Objects mode is selected.

| 

| 

| 

##### SelectSubmode5

| 
Select the fifth sub-mode. Based on the selected mode; for example it is Props when Objects mode is selected.

| 

| 

| 

##### SelectSubmode6

| 
Select the sixth sub-mode. Based on the selected mode; for example it is Custom when Compositions mode is selected.

| 

| 

| 

##### SelectTriggerMode

| 
Select the Triggers mode. The same as when you press F3.

| 

| 

| 

##### SelectUnit

| 
Select the entity under cursor.

| 

| 

| 

##### SelectWaypointMode

| 
Select the Waypoints mode. The same as when you press F4.

| 

| 

| 

##### SetFlyingHeight

| 
Move selected entities to default flying height. Only available for flying objects, i.e., planes and helicopters.

| 

| 

| 

##### SetTriggerOwner

| 
Initializes the Set Trigger Owner action. Same as if selecting the option in the context menu. At least one trigger must be selected.

| 

| 

| 

##### SetWaypointActivation

| 
Initializes the Set Waypoint Activation action. Same as if selecting the option in the context menu. At least one waypoint must be selected.

| 

| 

| 

##### SnapToSurface

| 
Snap selected entities to the ground.

| 

| 

| 

##### SubmodeToggle

| 
Toggle sub-mode (browse through all available ones).

| 

| 

| 

##### SurfaceSnapDisable

| 
Toggle surface snap off.

| 

| 

| 

##### SurfaceSnapEnable

| 
Toggle surface snap on.

| 

| 

| 

##### SurfaceSnapToggle

| 
Toggle surface snap on/off.

| 

| 

| 

##### SyncWith

| 
Initializes the Synch With action. Same as if selecting the option in the context menu. At least syncable entity must be selected.

| 

| 

| 

##### ToggleMap

| 
Toggle map on/off.

| 

| 

| 

##### ToggleMapIDs

| 
Toggle map IDs on/off.

| 

| 

| 

##### ToggleMapTextures

| 
Toggle map textures on/off.

| 

| 

| 

##### TogglePlaceEmptyVehicle

| 
Toggle place empty vehicles on/off.

| 

| 

| 

##### ToggleUnitSel

| 
Toggle entity selection on/off.

| 

| 

| 

##### ToggleVegetion

| 
Toggles vegetation on/off.

| 

| 

| 

##### Undo

| 
Undo the last operation.

| 

| 

| 

##### Unknown

| 
Engine placeholder. Not implemented.

| 

| 

| 

##### VerticalASL

| 
Set vertical mode to ASL (Above Sea Level).

| 

| 

| 

##### VerticalATL

| 
Set vertical mode to ATL (Above Terrain Level).

| 

| 

| 

##### VerticalToggle

| 
Toggle vertical mode.

| 

| 

| 

##### WidgetArea

| 
Toggle area widget on.

| 

| 

| 

##### WidgetCoord

| 
Toggle widget coordinate space between global and local.

| 

| 

| 

##### WidgetNone

| 
Toggle widget off.

| 

| 

| 

##### WidgetRotation

| 
Toggle rotation widget on.

| 

| 

| 

##### WidgetScale

| 
Toggle scaling widget on.

| 

| 

| 

##### WidgetSpaceLocal

| 
Set widget coordinate space to local.

| 

| 

| 

##### WidgetSpaceWorld

| 
Set widget coordinate space to global.

| 

| 

| 

##### WidgetToggle

| 
Toggle widget (browse through all available widget types).

| 

| 

| 

##### WidgetTranslation

| 
Toggle translation widget on.

| 

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Actions&oldid=378784"
					Category: - Eden Editor: Modding