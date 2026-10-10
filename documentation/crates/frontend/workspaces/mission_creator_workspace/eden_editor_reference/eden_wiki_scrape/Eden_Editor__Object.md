# Eden_Editor:_Object

Source: https://community.bohemia.net/wiki/Eden_Editor:_Object

---

An object is an entity with physical representation in the world. The player can see it as a model with textures, while the game recognizes its technical properties like the simulation type, collision geometry or damage.

Objects can be divided into three main categories as listed below.

### Characters

A Character is a living entity controlled either by a player, or by an AI (Artificial Intelligence; sometimes also known as NPC - Non-Playable Character).

### Vehicles

A Vehicle is an object which can have a crew composed of characters. A crewed vehicle has an effective commander, who can issue orders to the rest of the crew. The vehicle also inherits the side of the commander (i.e., BLUFOR pilot will render a helicopter BLUFOR as well).

### Props

A Prop is any other object without special functionality, for example, houses, fences or signs. Empty vehicles are not props. Animals are, mainly because they cannot be considered characters.

The object is the entity type you will be editing the most, so make sure to familiarize yourself with all its options.

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

| Object type. Can be changed only to another type of the same side,
e.g., you can change BLUFOR Car to BLUFOR Helicopter, but not to OPFOR Car or a Prop.

| ItemClass

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

| N/A

| Pylons Settings

| 

| Pylons

| 

| Position

| Transformation

| World coordinates in meters. X goes from West to East, Y from South to North and Z is height above terrain.

| position

| Position3D

| Rotation

| Transformation

| Local rotation in degrees. X is pitch, Y is roll and Z is yaw.

| rotation

| Number

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

| Lock

| States

| Vehicle lock. When locked, characters outside of the vehicle will be unable to get in and those already inside will be forbidden from leaving.
Available options:

- Unlocked - Anyone can get in

- Default - Group leaders and anyone who was ordered in by their group leader can get in

- Locked - No one can get in

- Locked for players - Anyone except for players can get in

| lock

| Number

| Skill

| States

| General AI skill. The attribute does not allow for decreasing it below 20%, because AI behavior would be too simplified.

| skill

| Number

| Health / Armor

| States

| Object health / armor. When close to 0%, the object will be destroyed.

| Health

| Number

| Fuel

| States

| Vehicle fuel.

| fuel

| Number

| Ammunition

| States

| General vehicle ammo state.

| ammo

| Number

| Rank

| States

| Character rank. When a group leader is killed, the subordinate with the highest rank will take over.
Available options:

- Private

- Corporal

- Sergeant

- Lieutenant

- Captain

- Major

- Colonel

| rank

| String

| Stance

| States

| 
Available options:

- Default Stance

- Lie Down

- Crouch

- Stand Up

| unitPos

| 

| Enable Dynamic Simulation

| Special States

| Entity simulation is enabled only if the player or an enemy unit is nearby.
Note: Does not work on simple objects and it overwrites basic simulation settings.

| dynamicSimulation

| Boolean

| Wake-Up Dynamic Simulation

| Special States

| Controls unit capability to activate dynamically simulated entities.

| addToDynSimGrid

| Number

| Enable Simulation

| Special States

| When disabled, the object will freeze and ignore any input or collisions.
Note: This option does not have any effect on dynamically simulated objects.

| enableSimulation

| Boolean

| Simple Object

| Special States

| When enabled, the object will behave like a map object (e.g. rocks or trees), which significantly saves performance. This option is available only for objects where it leads to improved performance.
Warning: If set, the setting is enforced at the start of the scenario and is irreversible during its runtime!

| objectIsSimple

| Boolean

| Show Model

| Special States

| Show model and collisions. Even when disabled, the object will be simulated normally (e.g., soldiers will still be able to move and shoot).

| hideObject

| Boolean

| Enable Damage

| Special States

| Set if the object can receive any damage. When a vehicle is invincible, its crew can still be killed.

| allowDamage

| Boolean

| Enable Stamina

| Special States

| Set whether the character should become tired when moving or not. When disabled for player, the stamina bar will be hidden completely.

| enableStamina

| Boolean

| Revive Enabled

| Special States

| Enable revive for this unit.

| EnableRevive

| Boolean

| Doors States

| Special States

| Set closed, locked or opened state for terrain object doors.
LMB - cycle between states
RMB - close door (reset to default state)
LMB + Alt - open door
LMB + Shift - lock door
LMB + Ctrl - close door

| DoorStates

| 

| Local Only

| Special States

| When enabled, the object will exist as a local instance on every client. It will not be synchronized over the network. Use this primarily for static (decorative) objects in order to optimize large-scale scenarios.

| isLocalOnly

| Boolean

| Name

| Identity

| Character name, by default automatically generated based on faction.

| unitName

| String

| Face

| Identity

| Character face.

| face

| String

| Call Sign

| Identity

| Call sign used in radio protocol (e.g., leader will order 'Kerry, fall back' instead of generic '2, fall back'). It does not affect character's actual name; consider changing it as well so they match together.
Available options:

- No Call Sign

- Miller

- Reynolds

- Armstrong

- Nichols

- Tanny

- Frost

- Lacey

- Larkin

- Kerry

- Jackson

- McKendrick

- Levine

- James

- McKay

- Hardy

- Northgate

- Adams

- Bennett

- Campbell

- Dixon

- Everett

- Franklin

- Givens

- Hawkins

- Lopez

- Martinez

- OConnor

- Ryan

- Patterson

- Sykes

- Taylor

- Walker

- Amin

- Masood

- Fahim

- Habibi

- Kushan

- Jawadi

- Nazari

- Siddiqi

- Takhtar

- Wardak

- Yousuf

- Anthis

- Costa

- Dimitirou

- Elias

- Gekas

- Kouris

- Leventis

- Markos

- Nikas

- Nicolo

- Panas

- Petros

- Rosi

- Samaras

- Stavrou

- Ghost

- Stranger

- Fox

- Snake

- Razer

- Jester

- Nomad

- Viper

- Korneedler