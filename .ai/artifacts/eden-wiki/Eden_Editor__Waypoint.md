# Eden_Editor:_Waypoint

Source: https://community.bohemia.net/wiki/Eden_Editor:_Waypoint

---

A waypoint defines the destination for a group. Waypoints are ordered sequentially, and only one can be active at any given time.

### Waypoint Types

Each waypoint has a specific type, which decides what the group has to do to complete the waypoint. A group led by an AI character will pursue the waypoint automatically, but the player is expected to issue necessary orders when in charge of the group.

You can use any type of waypoint from the Waypoints list in the asset browser. Because waypoints are added to groups, the existing character, group or any of its waypoints has to be selected in order for new waypoint to be added.

#### MOVE Waypoint Shortcut

Alternatively, you can quickly add a MOVE waypoint by holding Shift and pressing the right mouse button.

### Attaching Waypoints

A waypoint can also be attached to an object. Some waypoint types even require it, like DESTROY or GET IN.
To attach a waypoint, drag it to an object. Similarly, you can detach it by dragging it away. Moving the object around will move all the attached waypoints as well.
The attached waypoint is visualized by a different icon outline ( instead of ).

### Waypoint Completion

The waypoint is completed when requirements set by its type are met, but additional conditions can be added. A scripted condition can be set in attributes, but you can also connect the waypoint to one or more triggers using Set Waypoint Activation connection. The waypoint will be completed only once all the connected triggers are active.

Once a waypoint is completed, the group will move to the next one. If there is none, the group will remain where it is.
You can find more about the waypoint in the tooltips of their attributes.

### Attributes

Info

Development

Name

Category

Description

Property

Type

| Type

| Type

| Waypoint type defines what the group will do when the waypoint becomes active, and the condition when it becomes completed. Applies mainly to AI-led groups, as players will not be prompted to perform any specific actions, even though the waypoint completion condition is the same.
Some waypoints have special attributes, which are only available the next time you access the attribute window.

| itemClass

| String

| Description

| Init

| Text visible for the player next to the waypoint icon in the scene.

| description

| String

| Order

| Init

| Order in which waypoints follow. When changing order, the waypoint will take position of the selected one, pushing all other further down.

| order

| 

| Identified

| Init

| System name of the waypoint, used for identification in scripts.

| name

| String

| Position

| Transformation

| World coordinates in meters. X goes from West to East, Y from South to North and Z is height above terrain.

| position

| Position3D

| Placement Radius

| Transformation

| Placement radius in meters. The entity will start at a random position within the radius.

| placementRadius

| Number

| Completion Radius

| Transformation

| Distance in meters in which a group member has to be in order for the waypoint to be considered completed.

| completionRadius

| Number

| Combat Mode

| State

| Controls how and when the group will choose to engage enemy targets.
Available options:

- Unchanged - No change compared to the previous state.

- Forced Hold Fire - The group will never fire under any circumstances.

- Do Not Fire Unless Fired Upon, Keep Formation - Group members will hold fire unless directly threatened.

- Do Not Fire Unless Fired Upon - Group members will move into positions from which they can shoot at the enemy, but will hold fire unless directly threatened.

- Open Fire, Keep Formation - Default combat mode. Group members will fire upon any target in range, while staying in formation. The group leader may order individual members to engage specific targets.

- Open Fire - Group members will fire at any suitable target in range, leaving the formation to find a suitable shooting position.

| combatMode

| String

| Behavior

| State

| Behavior pattern of the group.
Available options:

- Unchanged - No change compared to the previous state.

- Careless - The same as 'Safe', except that it will not be switched automatically after spotting a threat. Use with caution, player may perceive AIs in this behavior as defective.

- Safe - Non-combat behavior. Characters have weapons lowered. Vehicles follow roads and use lights. They automatically switch to 'Aware' upon spotting a threat.

- Aware - Default behavior. Characters are in ready position. Vehicles prefer roads, do not use lights and their passengers will disembark to counter threats.

- Combat - Firefight behavior. Characters break formation and take cover. Vehicles ignore roads and do not use lights.

- Stealth - Characters break formation and move cautiously, preferring cover and going prone. Vehicles prefer roads, but do not use lights.

| behaviour

| String

| Formation

| State

| Default group formation. Based on the combat mode, group members may ignore the formation in 'Combat' and 'Stealth' modes.
Available options:

- Unchanged

- Wedge

- Vee

- Line

- Column

- File

- Staggered Col.

- Echelon L.

- Echelon R.

- Diamond

| formation

| String

| SpeedMode

| State

| Default travel speed of the group. In Combat and Stealth behavior modes, group members will try to prioritize this setting.
Available options:

- Unchanged

- Limited

- Normal

- Full

| speedMode

| String

| Condition

| Expression

| Repeatedly calculated condition, must return boolean expression. When the waypoint type conditions are met and this expression returns true, the waypoint is completed.
Passed variables are:

- this - group leader

- thisList - array with all group members

| condition

| String

| On Activation

| Expression

| Expression called when the waypoint is completed.
Passed variables are:

- this - group leader

- thisList - array with all group members

| onActivation

| String

| Script

| Expression

| Script executed when 'SCRIPTED' waypoint type is selected. The waypoint will be completed once the script is finished.

| script

| String

| Map Visibility

| Visibility

| Make the waypoint visible for the player on the map.

| show2D

| Boolean

| Scene Visibility

| Visibility

| Make the waypoint visible for the player in the scene.

| show3D

| Boolean

| Timer Values

| Timer

| Time in seconds passed between when the waypoint would be considered complete and when it actually completes. Selected randomly in a range from Min to Max, gravitating towards Mid.

| timeout

| Array in format [min, mid, max]

| Effect Condition

| Effects

| Condition for effects to be played, must return boolean expression.

| effectCondition

| String

| Sound

| Effects

| Sound played upon activation.

| sound

| String

| Voice

| Effects

| Sound spoken by the first unit which activated the trigger.

| voice

| String

| Environment

| Effects

| Environment sounds played upon activation.

| soundEnvironment

| String

| Music

| Effects

| Music played upon activation. Replaces previously playing music track.

| music

| String

| UI Overlay

| Effects

| User interface overlay shown upon activation.

| title

| String

### See Also

- Waypoints

- Data Type Waypoint

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Waypoint&oldid=296772"
					Category: - Eden Editor: Asset Types