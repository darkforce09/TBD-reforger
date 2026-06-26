# Eden_Editor:_Scenario_Attributes

Source: https://community.bistudio.com/wiki/Eden_Editor:_Scenario_Attributes

---

Scenario Attributes are used to set, customize and define numerous scenario settings.

### Attributes

##### General

Info

Development

Name

Category

Description

Property

Type

| Title

| Presentation

| Scenario name. Appears in the scenarios menu, multiplayer lobby and loading screens. Use @STR_ prefix to link to localization keys.

| IntelBriefingName

| String

| Author

| Presentation

| Scenario author. Appears in the scenarios menu and in loading screens.

| Author

| String

| Picture

| Overview

| Path to overview picture visible in the scenarios menu. When no loading screen picture is defined, this will be used in loading screen instead.

| OverviewPicture

| String

| Text

| Overview

| Short overview text visible in the scenarios menu. When no loading screen text is defined, this will be used in loading screens instead. Use @STR_ prefix to link to localization keys.

| OverviewText

| String

| DLC

| Overview

| When set, the overview image will be overlaid with a DLC frame indicating the scenario belongs to the DLC.

| AppId

| Number

| Require DLC

| Overview

| Sets if the DLC is required. When checked, the player will not be able to play the scenario if the DLC is not owned.

| AssetType

| String

| Picture

| Overview (Locked)

| Path to the overview picture visible in the scenarios menu. When no loading screen picture is defined, this will be used in the loading screen instead.

| OverviewPictureLocked

| String

| Text

| Overview (Locked)

| Short overview text visible in the scenarios menu when the scenario is locked (see 'Unlock' category for more details). Use @STR_ prefix to link to localization keys.

| OverviewTextLocked

| String

| Picture

| Loading Screen

| Path to the picture visible in the loading screens before and during the scenario.

| LoadScreen

| String

| Text

| Loading Screen

| Short text visible in loading screens before and during the scenario. Use @STR_ prefix to link to localization keys.

| OnLoadMission

| String

| Show Briefing

| States

| When disabled, the scenario will start automatically after the loading, without showing the briefing. Valid only for singleplayer scenarios.

| Briefing

| Boolean

| Show Debriefing

| States

| When disabled, the debriefing screen will not be shown when the scenario is completed.

| Debriefing

| Boolean

| Enable Saving

| States

| When disabled, the 'SAVE' option in the pause menu will be disabled and scripted autosaves will not do anything.

| Saving

| Boolean

| Show Map

| States

| When disabled, a black screen is shown instead of the map. Tasks, notes and other information will remain accessible. Can also be achieved by removing the map from the player's inventory.

| ShowMap

| Boolean

| Show Compass

| States

| When disabled, the compass will not be available either on the map, or in the scene after pressing the 'Compass' key. Can also be achieved by removing the compass from the player's inventory.

| ShowCompass

| Boolean

| Show Watch

| States

| When disabled, the watch will not be available in the scene after pressing the 'Watch' key. Can also be achieved by removing the watch from the player's inventory.

| ShowWatch

| Boolean

| Show GPS

| States

| When disabled, the GPS minimap will not be available in the scene after pressing the 'GPS' key. Can also be achieved by removing the GPS from the player's inventory.

| ShowGPS

| Boolean

| Show HUD

| States

| When disabled, on-screen information such as weapon information or the command menu will be hidden.

| ShowHUD

| Boolean

| Show UAV Feed

| States

| When disabled, the UAV feed will not be available in the scene after pressing the 'AV Camera' key.

| ShowUAVFeed

| Boolean

| Advanced Flight Model

| States

| When enabled, all player controlled helicopters will use the advanced flight model.

| ForceRotorLibSimulation

| Boolean

| Debug Console

| States

| Determines debug console availability
Available options:

- Available only in editor

- Available for the host or logged-in admin

- Available for everyone

| EnableDebugConsole

| Number

| Unlocked Keys

| Unlock

| Keys needed to mark the scenario as completed in the scenarios menu. Can be multiple words divided by a semicolon. Key can be activated using 'activateKey' scripting command. Does not affect the multiplayer and campaign scenarios, and the scenarios downloaded from Steam.

| DoneKeys

| Array

| Required Keys

| Unlock

| Keys required for mission to be available for playing from the scenarios menu. Can be multiple words divided by a semicolon. Does not affect the multiplayer and campaign scenarios.

| Keys

| Array

| Required Keys Limit

| Unlock

| The number of required keys to be active for the mission to be unlocked.

| KeysLimit

| Number

| Init

| Init

| Expression called upon at start. In multiplayer, it is called on every machine and for each player who joins in the progress. The variable 'this' refers to the affected object. ⚠Despite the tooltip saying that 'this' refers to the affected object, it does not. There are no arguments passed to this attribute! This attribute behaves similar to the init.sqf.

| Init

| String

| Independents Allegiance

| Misc

| Sets who the Independent side will be friendly to. The value is shared across all scenario phases.

| IntelIndepAllegiance

| Array in format [west:Number,east:Number]

| Binarize the Scenario File

| Misc

| When true, the *.sqm file will be binarized. The process saves loading time, but makes the file uneditable in a text editor.

| SaveBinarized

| Boolean

##### Environment

Info

Development

Name

Category

Description

Property

Type

| Date

| Date

| Starting date.

| IntelDate

| Array in format [year, month, day]

| Time

| Date

| Starting time of the day. Please note that the sunrise and sunset times are influenced by the date (e.g., days are shorter in winter) and the longitude and latitude of the terrain.

| IntelTime

| Number

| Time of Changes

| Weather Forecast

| Delay until all forecasted weather values take effect.

| IntelTimeOfChanges

| Number

| Overcast Start

| Overcast

| Initial cloud cover. Unless set manually, this value also affects rain, lightning, waves, wind and gusts values.

| IntelWeatherStart

| Number

| Overcast Forecast

| Overcast

| Forecasted cloud cover.

| IntelWeatherForecast

| Number

| Fog Start

| Fog

| Initial fog. Strength affects not only how far player can see, but also limits the sight of AI entities.

| IntelFogStart

| Number

| Fog Forecast

| Fog

| Forecasted fog.

| IntelFogForecast

| Number

| Manual Override

| Rain

| Enables manual settings in this category. When disabled, the engine will calculate the values automatically.

| IntelRainIsForced

| Boolean

| Rain Start

| Rain

| Initial rain strength. Applied only when the cloud cover is already bad, i.e., a high value is ignored when the weather is sunny.

| IntelRainStart

| Number

| Rain Forecast

| Rain

| Forecasted rain strength.

| IntelRainForecast

| Number

| Manual Override

| Lightnings

| Enables manual settings in this category. When disabled, the engine will calculate the values automatically.

| IntelLightningIsForced

| Boolean

| Lightnings Start

| Lightnings

| Initial frequency of lightning. Applied only when the cloud cover is already bad, i.e., a high value is ignored when the weather is sunny.

| IntelLightningStart

| Number

| Lightnings Forecast

| Lightnings

| Forecasted frequency of lightning.

| IntelLightningForecast

| Number

| Manual Override

| Waves

| Enables manual settings in this category. When disabled, the engine will calculate the values automatically.

| IntelWavesIsForced

| Boolean

| Waves Start

| Waves

| Initial waves size. Applied independently of cloud cover settings.

| IntelWavesStart

| Number

| Waves Forecast

| Waves

| Forecasted waves size.

| IntelWavesForecast

| Number

| Manual Override

| Wind

| Enables manual settings in this category. When disabled, the engine will calculate the values automatically.

| IntelWindIsForced

| Boolean

| Wind Start

| Wind

| Initial wind strength. Together with wind direction, it influences how clouds, smoke, vegetation or flags are moving.

| IntelWindStart

| Number

| Wind Forecast

| Wind

| Forecasted wind strength.

| IntelWindForecast

| Number

| Gusts Start

| Wind

| Initial gusts strength. Gusts are randomly changing wind direction. The worse the cloud cover, the stronger and more frequent the changes.

| IntelWindGustStart

| Number

| Gusts Forecast

| Wind

| Forecasted gusts value.

| IntelWindGustForecast

| Number

| Direction Start

| Wind

| Initial wind direction.

| IntelWindDirectionStart

| Number

| Direction Forecast

| Wind

| Forecasted wind direction.

| IntelWindDirectionForecast

| Number

##### Multiplayer

Info

Development

Name

Category

Description

Property

Type

| Game Type

| Type

| Scenario type shown in the server browser and in the loading screen.

| GameType

| String

| Min Players

| Type

| Minimum number of required players.

| MinPlayers

| Number

| Max Players

| Type

| Minimum number of allowed players.

| MaxPlayers

| Number

| Summary

| Lobby

| Short summary shown in the scenario lobby.

| IntelOverviewText

| Structured Text

| Enable AI

| Lobby

| When AI is enabled, all playable characters are created at the scenario start, controlled by AI. Upon joining, a player takes control of an existing character.
When AI is disabled, a new character is created for each connecting player.
Server host can disable AI even when it is allowed by default.

| DisabledAI

| Boolean

| Auto Assign Slots

| Lobby

| When enabled, arriving players will be first assigned to the side with the lowest number of players. Valid only for scenarios with multiple playable sides involved.

| JoinUnassigned

| Boolean

| Respawn

| Respawn

| Character respawn type. Defines what happens to players and playable characters after they die in multiplayer scenarios. Has no effect in singleplayer scenarios.
Available options:

- Disabled - No respawn.

- Respawn on Custom Position - Respawn on a position defined by the marker with a specific prefix in the name. When multiple ones are available, a random one is selected.

- respawn - position available for all sides

- respawn_west - BLUFOR respawn

- respawn_east - OPFOR respawn

- respawn_guerrila - Independent respawn

- respawn_civilian - Civilian respawn

Because these are prefixes, anything can follow them, e.g., respawn_1, respawn_west_base, etc.
- Respawn on Position of Death - Respawn where killed, even if the dead body moved.

- Switch to Group Member - Take control of the next available group member. When none are left, 'Switch to Spectator' respawn is used instead.

- Switch to Side Member - Pick a character on the same side to take control of. When none are left, 'Switch to Spectator' respawn is used instead.

- Switch to Spectator - Spectate the scenario without being directly involved.

| Respawn

| Number

| Rulesets

| Respawn

| Specific respawn rulesets based on currently selected 'Respawn'.

| RespawnTemplates

| Array

| Respawn Delay

| Respawn

| Time in seconds after which players respawn.

| RespawnDelay

| Number

| Vehicle Respawn Delay

| Respawn

| Time in seconds after which vehicles respawn.

| RespawnVehicleDelay

| Number

| Show Scoreboard

| Respawn

| When enabled, the scoreboard is shown while waiting for respawn. May be subdued by some respawn rulesets.

| RespawnDialog

| Boolean

| Allow Manual Respawn

| Respawn

| When enabled, players will have the option to manually respawn in the pause menu.

| RespawnButton

| Boolean

| Enable Team Switch

| Respawn

| When enabled, players will be able to manually switch to any other playable character in the scenario not controlled by another player.

| EnableTeamSwitch

| Boolean

| Allow AI Score

| Respawn

| When enabled, the score of playable characters controlled by AI will appear on the scoreboard.

| AIKills

| Boolean

| Shared Objectives

| Tasks

| Sets if tasks assigned to friendlies are visualized to player and who can assign/unassign tasks.
Available options:

- Disabled - Tasks assigned by friendlies are not marked.

- Enabled - Tasks assigned by friendlies are marked to player.

- Enabled with Task Propagation - Tasks assigned by friendlies are marked to player, but only group leaders can assign and unassign tasks. Task assignment changes are automatically propagated to subordinates.

| SharedObjectives

| String

| Revive Mode

| Revive

| Controls who can be incapacitated and who can revive others.
Available options:

- Disabled - Revive is completely disabled. All players will die immediately.

- Enabled for all players - Revive is enabled for all players.

- Controlled by player Attributes - Only players with Revive enabled in their Attributes are effected.

| ReviveMode

| String

| Required Trait

| Revive

| Controls the specialty required to revive others.
Available options:

- None - Anyone can revive.

- Medic - Only players with the 'Medic' trait can revive.

| ReviveRequiredTrait

| String

| Required Items

| Revive

| Controls the item required to revive others.
Available options:

- None - No special item is required in order to revive.

- Medikit - The 'Medikit' item is required in order to revive.

- First Aid Kit / Medikit - 'Medikit' or 'First Aid Kit' items are required in order to revive.

| ReviveRequiredItems

| String

| Revive Duration

| Revive

| Controls the time it takes to revive someone.

| ReviveDelay

| String

| Medic Speed Multiplier

| Revive

| Controls how much faster a medic revives someone.

| ReviveMedicSpeedMultiplier

| String

| Force Respawn Duration

| Revive

| Controls how long it takes to force respawn while incapacitated.

| ReviveForceRespawnDelay

| String

| Incapacitation Mode

| Revive

| Controls the level of simulation involved in triggering incapacitation.
Available options:

- Basic - Players will always enter the incapacitated state.

- Advanced - Extensive wounds will kill players instantly.

| ReviveUnconsciousStateMode

| String

| Bleed Out Duration

| Revive

| Controls how long it takes an incapacitated unit to bleed out.

| ReviveBleedOutDelay

| String

##### Garbage Collection

Info

Development

Name

Category

Description

Property

Type

| Minimum distance

| Garbage Collection

| The minimum distance from any player. Set the value to 0 to remove the min. distance.

| MinPlayerDistance

| Number

| Character Corpses

| Garbage Collection

| 

| AttributeSystemSubcategory

| 

| Mode

| Garbage Collection

| Garbage collecting mode.
Available options:

- None

- All scenario objects

- Only objects that can respawn

- Only objects that cannot respawn

| CorpseManagerMode

| Number

| Limit

| Garbage Collection

| Maximum allowed number of dead bodies in the scenario.

| CorpseLimit

| Number

| Min Delay

| Garbage Collection

| Time in seconds before a dead body is removed when the number of dead bodies exceeds the 'Limit'.

| CorpseRemovalMinTime

| Number

| Max Delay

| Garbage Collection

| Time in seconds before a dead body is removed when the number of dead bodies is below or equals the 'Limit'.

| CorpseRemovalMaxTime

| Number

| Vehicle Wrecks

| Garbage Collection

| 

| AttributeSystemSubcategory

| 

| Mode

| Garbage Collection

| Garbage collecting mode.
Available options:

- None

- All scenario objects

- Only objects that can respawn

- Only objects that cannot respawn

| WreckManagerMode

| Number

| Limit

| Garbage Collection

| Maximum allowed number of dead bodies in the scenario.

| WreckLimit

| Number

| Min Delay

| Garbage Collection

| Time in seconds before a dead body is removed when the number of dead bodies exceeds the 'Limit'.

| WreckRemovalMinTime

| Number

| Max Delay

| Garbage Collection

| Time in seconds before a dead body is removed when the number of dead bodies is below or equals the 'Limit'.

| WreckRemovalMaxTime

| Number

| Enable Dynamic Simulation

| Dynamic Simulation

| Turns on the 'Dynamic Simulation' system. Only entities and groups with the 'Dynamic Simulation' will be affected.

| DynSimEnabled

| 

| Characters

| Dynamic Simulation

| 

| DynSimDistGroup

| 

| Manned Vehicles

| Dynamic Simulation

| 

| DynSimDistVehicle

| 

| Props

| Dynamic Simulation

| 

| DynSimDistProp

| 

| Empty Vehicles

| Dynamic Simulation

| 

| DynSimDistEmptyVehicle

| 

| Is Moving

| Dynamic Simulation

| Multiplies activation distance of non-static entity by set value.

| DynSimMovingCoef

| 

| Limit by View Distance

| Dynamic Simulation

| Limits all activation distances to player's object view distance. Dynamically simulated entities beyond the object view distance will be disabled.

| DynSimSaturateByObjDist

| 

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Scenario_Attributes&oldid=364855"
					Category: - Eden Editor: Editing