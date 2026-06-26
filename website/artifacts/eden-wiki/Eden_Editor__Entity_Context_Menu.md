# Eden_Editor:_Entity_Context_Menu

Source: https://community.bohemia.net/wiki/Eden_Editor:_Entity_Context_Menu

---

### Create a custom Entity Context Menu entry

- Entries for the Context Menu are defined in the class Display3DEN.

- The menu has multiple levels, the first level for example contains options like "Go Here" or folders like "Log". Those folders on the other hand can have another level, level 2.

### Config

class ctrlMenu; // Load base class
class Display3DEN
{
	class ContextMenu : ctrlMenu
	{
		class Items
		{
			items[] += {"TAG_RootFolder"}; // Root items
			class TAG_RootFolder // Level 1 item
			{
				picture = "someLogo.paa";
				text = "Custom Folder";
				value = 0;
				items[] = {"TAG_ShowSomeText"}; // Items in "folder" TAG_RootFolder
			};
			class TAG_ShowSomeText // Level 2 item
			{
				text = "Show Text";
				action = "systemChat 'Showing some text.'";
				conditionShow = "hoverObject + script1"; // Only show this entry when the object is set to be playable in multiplayer
				conditionScript1 = "_this get3DENAttribute 'ControlMP' isEqualTo [true];"
			};
		};
	};
};

#### Properties

Property
Description
Data type

| text
| Text shown in the menu. Can be localized.
| String

| picture
| Item path to a picture shown in front of the text.
| String

| action
| An SQF expression called when the item is activated.
| String

| data
| Used by the engine. Internal use only.
| String

| enable
| Used by the engine. Internal use only.
| Number

| shortcuts[]
| Each entry can have a single shortcut assigned to it. The shortcut only works when the context menu is open. This did not work prior to  2.22.
| Array

| value
| An internal bit flag. Set it to 0 to always show the item in the root of the config menu. Otherwise ignore it.
| Number

|  2.22 conditionScript1
| Scripted condition that needs to return a Boolean. It is only evaluated if simple expression script1 is used inside conditionShow.
⚠This condition is evaluated for all menu items and all selected entities entityCount * menuItemCount = evaluationCount. Keep these expressions as simple as possible!

| String

| conditionShow
| A condition represented as simple expression. Needs to return 1 (true) in order for the item to be visible. A list of all available expression is listed below.
| String

Condition

Description

| hoverFolder

| True when hovering over a folder (layer)

| hoverGroup

| True when hovering over any group

| hoverLayer

| True when hovering over a Layer

| hoverLogic

| True when hovering over any logic

| hoverLogicModule

| True when hovering over a module

| hoverMarker

| True when hovering over any marker

| hoverMarkerArea

| True when hovering over a marker of type area

| hoverObject

| True when hovering over any object

| hoverObjectAgent

| 

| hoverObjectAttached

| 

| hoverObjectBrain

| True when hovering over an object with simulation "soldier" or "UAVpilot"

| hoverObjectCanFly

| True when hovering an object which can fly

| hoverObjectFlying

| True when hovering over a object that can fly

| hoverObjectUav

| True when hovering over an UAV

| hoverObjectVehicle

| True when hovering over a vehicle

| hoverTrigger

| True when hovering over any trigger

| hoverWaypoint

| True when hovering over any waypoint

| IsInternal

| True when using internal executable. For developers only!

| isMultiplayer

| True when editing in multiplayer environment

| Map

| True when the map is open

| selected

| True when an Eden entity is selected

| selectedGroup

| True when a group is selected

| selectedLogic

| True when a logic is selected

| selectedLogicModule

| Try when a module is selected

| selectedMarker

| True when a Marker is selected

| selectedMarkerArea

| True when a marker of type area is selected

| selectedObject

| True when an object is selected

| selectedObjectAgent

| 

| selectedObjectAttached

| 

| selectedObjectBrain

| True when an object with simulation "soldier" or "UAVpilot" is selected

| selectedObjectCanFly

| True when the selected object can fly

| selectedObjectFlying

| True when the selected object is flying

| selectedObjectVehicle

| True when a vehicle is selected

| selectedTrigger

| True when a trigger is selected

| selectedWaypoint

| True when a waypoint is selected

|  2.22 script1

| Contains SQF condition defined in scriptedCondition1 property.

### Variables

Once the menu has been opened, the following variables can be accessed:

(uiNamespace getVariable "BIS_fnc_3DENEntityMenu_data") params ["_posEntity3D", "_entity"];

- 0: _posEntity3D: Vector3D - Position of the entity on which the right click happened

- 1: _entity: Object - Eden entity on which the right click happened

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Entity_Context_Menu&oldid=378463"
					Category: - Eden Editor: Modding