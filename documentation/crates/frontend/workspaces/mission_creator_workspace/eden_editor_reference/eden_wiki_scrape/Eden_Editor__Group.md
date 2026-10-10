# Eden_Editor:_Group

Source: https://community.bohemia.net/wiki/Eden_Editor:_Group

---

A group is a composition of characters which are linked together in a command structure.

### General Information

#### Creation

A group is created when a new character is placed. However, if the character is near another one of the same side, it will be grouped to it instead. This functionality can be disabled using the Automatic Grouping attribute in the editor preferences.

#### Leader

A group always has a leader who is able to issue orders to the other group members. Usually, the highest ranking character is the leader, and if he dies, the second highest ranking character takes his place.
You can manually change the group leader using the grouping connection or the context menu. The character you connect others to will become the leader.

#### Side

The group side dictates its relation with other groups, e.g. characters in BLUFOR groups will engage those in OPFOR groups.
A character inherits the group side, even if its own side is different. For example, an OPFOR character grouped to a BLUFOR group will be BLUFOR as well.

### Groups in the Eden Editor

A group is represented by an icon hovering above the leader's position. Lone units do not have the icon visible unless they are selected.
Clicking on the icon will select all group members.
When zoomed out, the icons of nearby group members are stacked together with the group icon.

In the entity list, a group is represented as a folder with all its members and waypoints placed inside.
Only a whole group can be moved between layers, not its individual members. Dragging one member will move the rest of the group as well.

Sorting characters into logical groups can speed up designing your scenario, make sure to use them well.

### Attributes

Info

Development

Name

Category

Description

Property

Type

| Variable Name

| Init

| Unique system name. Can contain only letters, numbers and underscore. The name is not case sensitive, so 'someName' and 'SOMENAME' are treated as the same variables.

| Name

| String

| Init

| Init

| Expression called upon at start. In multiplayer, it is called on every machine and for each player who joins in the progress. The variable 'this' refers to the affected object.

| Init

| String

| Callsign

| Init

| Group callsign as displayed in the radio chat log.

| groupID

| String

| Placement Radius

| Init

| Placement radius in meters. The entity will start at a random position within the radius.

| placementRadius

| Number

| Combat Mode

| State

| Controls how and when the group will choose to engage enemy targets.
Available options:

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

- 0 for Wedge

- 1 for Vee

- 2 for Line

- 3 for Column

- 4 for File

- 5 for Staggered Col.

- 6 for Echelon L.

- 7 for Echelon R.

- 8 for Diamond

| formation

| Number

| SpeedMode

| State

| Default travel speed of the group. In Combat and Stealth behavior modes, group members will try to prioritize this setting.
Available options:

- Limited

- Normal

- Full

| speedMode

| String

| Enable Dynamic Simulation

| State

| Activates dynamic simulation for all characters in the group.

| dynamicSimulation

| Boolean

| Delete when Empty

| State

| Automatically delete the group when it is empty.
Note: Deletion is not immediate.

| garbageCollectGroup

| Boolean

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Group&oldid=295428"
					Category: - Eden Editor: Asset Types