# Eden_Editor:_System

Source: https://community.bohemia.net/wiki/Eden_Editor:_System

---

A system is a virtual object which can help expand your scenario with a new functionality.

### Modules

A module provides complex functionality which would otherwise have to be scripted. It can usually be configured using custom attributes, and can sometimes be affected by synchronization connections. For example, many modules are activated only once all the synchronized triggers are active. To see more about the individual modules, explore their attributes.

### Logic Entities

A logic entity is simply a virtual object without any inherent functionality. It is mainly used in cooperation with modules or scripts, for example, to mark positions and their relations.

### Virtual Entities

Some virtual entities are playable. They are used in multiplayer for abstract roles such as a spectator or headless client.

Be sure to investigate all the available systems, especially Modules. Because each has its own rules, do not forget to also check their attributes.

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

| 

| itemClass

| String

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

| Position

| Transformation

| World coordinates in meters. X goes from West to East, Y from South to North and Z is height above terrain.

| position

| Position3D

| Rotation

| Transformation

| Local rotation in degrees. X is pitch, Y is roll and Z is yaw.

| rotation

| Array

| Size

| Transformation

| Area size in meters.

| size2

| Array

| Size

| Transformation

| Area size in meters.

| size3

| Array

| Shape

| Transformation

| Area shape.
Available options:

- Ellipse

- Rectangle

| IsRectangle

| Boolean

| Placement Radius

| Transformation

| Placement radius in meters. The entity will start at a random position within the radius.

| placementRadius

| Number

| Player

| Control

| Player in singleplayer. When enabled, the character will also be available in multiplayer and team switch ('Playable' status cannot be disabled individually in such case).

| ControlSP

| Boolean

| Playable

| Control

| When enabled, the character will appear as a slot in the multiplayer scenario lobby and in the list of roles available for team switch.

| ControlMP

| Boolean

| Role Description

| Control

| Multiplayer role description visible in the multiplayer lobby. When undefined, the object type name will be used by default.

| description

| String

| Probability of Presence

| Presence

| Probability of presence evaluated at the scenario start. When it fails, the object is not created at all.

| presence

| Number

| Condition of Presence

| Presence

| Condition of presence evaluated at the scenario start, must return boolean expression. When false, the object is not created at all.

| presenceCondition

| String

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_System&oldid=338898"
					Category: - Eden Editor: Asset Types