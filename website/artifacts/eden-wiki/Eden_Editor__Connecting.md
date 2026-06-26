# Eden_Editor:_Connecting

Source: https://community.bohemia.net/wiki/Eden_Editor:_Connecting

---

Connecting can bind certain entities together and affect their functionality.

### Connecting Entities

To connect entities together, click the right mouse button on an entity, expand the Connect folder and pick a connection type to start the connecting operation.
Click the left mouse button on the desired target to connect all selected entities to the target. You can also press the right mouse button to cancel the operation.

### Disconnecting Entities

To remove an existing connection, hover over the connection line in the scene and press Del. Alternatively, clicking into an empty space while connecting will remove any connection of the selected type.

### Additional Notes

It is also possible to group or sync multiple units with another entity at once.

Connections are shown as lines, both in the scene and on the map. Each connection type is represented by a different color.
Unless selected, the line will be obstructed by the terrain and environmental settings.

### Connection Types

#### Grouping

Character & Character

Characters grouped together become a group with a leader. Subordinates follow the leader and carry out issued orders.
Grouping can also be quickly accessed by holding Ctrl and dragging a line from one character to another.

Read more: Eden Editor: Group

#### Syncing

Character & Object

A generic connection without any inherent functionality. Often used by scripted systems. One of the connected entities has to be a character, otherwise the connection will not be recognized once the scenario starts.

#### Setting Trigger Owner

Trigger & Character

Trigger owner changes a trigger's activation from being side based (e.g., activated by BLUFOR) to object based (e.g., activated by the owner or its group members).

Read more: Eden Editor: Trigger

#### Setting Random Start

Object & Marker

When the scenario starts, an object will appear randomly either on its default position, or on the position of any connected marker. Each restart will bring different results.

#### Setting Waypoint Activation

Waypoint & Trigger

A Waypoint will only be completed once all connected triggers are activated. This is useful, for example, for sending reinforcements once enemy locations are cleared.

Read more: Eden Editor: Waypoint

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Connecting&oldid=296499"
					Category: - Eden Editor: Editing