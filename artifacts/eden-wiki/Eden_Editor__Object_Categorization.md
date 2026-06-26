# Eden_Editor:_Object_Categorization

Source: https://community.bohemia.net/wiki/Eden_Editor:_Object_Categorization

---

### Hierarchy

#### Mode Sorting

- Mode - All objects placed in Objects mode

- Submode - Characters and vehicles are sorted per side (e.g., BLUFOR), the rest is under Props

#### Object Sorting

- Category - the topmost tree hierarchy item. Characters and vehicles are usually sorted by faction (e.g., NATO), while props are sorted by some shared functionality (e.g., Furniture)

- Subcategory - more detailed categorization specific to the main category.

- Entity - the object asset which can be actually placed. Identified by an icon; categories don't have one.

### Configuration

#### Object

For the list of all config properties, see CfgVehicles Config Reference

// MODE
// All objects are configured in CfgVehicles (even those which are not technically vehicles)
class CfgVehicles
{
	class MyObject : Car
	{
		// SUBMODE
		// The side decides under which submode (e.g., BLUFOR) is the object sorted
		// When the class doesn't inherit from AllVehicles or its child classes (e.g., Car),
		// the side is ignored and the object will be a Prop
		side = 1;

		// CATEGORY
		// Two ways how to set it exist. When both are present, editorCategory has priority.
		editorCategory = "MyCategory"; // Class from CfgEditorCategories. Usually used for props.
		faction = "MyCategory"; // Class from CfgFactionClasses. Usually used for characters and vehicles.

		// SUBCATEGORY
		// Two ways how to set it exist. When both are present, editorSubcategory has priority.
		editorSubcategory = "MySubcategory"; // Class from CfgEditorSubcategories. Should be used everywhere.
		vehicleClass = "MySubcategory"; // Class from CfgVehicleClasses. Deprecated, should not be used anymore.
	};
};

#### Categories

When you find out that default categories and subcategories don't fit your needs, you can add your own ones. However, we recommend to stick to existing categorization as much as possible.

class CfgEditorCategories
{
	class MyCategory // Category class, you point to it in editorCategory property
	{
		displayName = "My Category"; // Name visible in the list
	};
};

class CfgEditorSubcategories
{
	class MySubcategory // Category class, you point to it in editorSubcategory property
	{
		displayName = "My Subcategory"; // Name visible in the list
	};
};

### Categories and Subcategories

#### Characters and Vehicles

##### Faction

Example configuration:

faction = "OPF_F";

Class

Display Name

Side

| OPF_F

| CSAT

| OPFOR

| OPF_G_F

| FIA

| OPFOR

| BLU_F

| NATO

| BLUFOR

| BLU_G_F

| FIA

| BLUFOR

| IND_F

| AAF

| Independent

| IND_G_F

| FIA

| Independent

| CIV_F

| Civilians

| Civilian

##### Subcategory

Example configuration:

editorSubcategory = "EdSubcat_AAs";

Class

Display Name

Contents

| EdSubcat_AAs

| Anti-Air

| Vehicles specifically equipped for neutralizing air threats.

| EdSubcat_APCs

| APCs

| Vehicles which were designed as Armored Vehicle Carriers. A tank with passenger seats (e.g., M4A2 Slammer) is not an APC, because such functionality is not its main purpose.

| EdSubcat_Artillery

| Artillery

| Vehicles which were designed to engage distant targets. Generally reserved only with vehicles where gunner can access the ballistic computer.

| EdSubcat_Boats

| Boats

| Naval vehicles.

| EdSubcat_Cars

| Cars

| Lightly armored wheeled vehicles.

| EdSubcat_Drones

| Drones

| Vehicles which doesn't need crew and are controlled using UAV interface.

| EdSubcat_Helicopters

| Helicopters

| Rotary wing aircrafts.

| EdSubcat_Personnel

| Men

| Default characters folder.

| EdSubcat_Personnel_African

| Men (African)

| Civilians with African identity sets.

| EdSubcat_Personnel_Asian

| Men (Asian)

| Civilians with Asian identity sets.

| EdSubcat_Personnel_Camo_Arctic

| Men (Arctic)

| Characters in arctic (snow) camouflage uniforms.

| EdSubcat_Personnel_Camo_Arid

| Men (Arid)

| Characters in arid camouflage uniforms.

| EdSubcat_Personnel_Camo_Desert

| Men (Desert)

| Characters in desert camouflage uniforms.

| EdSubcat_Personnel_Camo_Jungle

| Men (Jungle)

| Characters in jungle camouflage uniforms.

| EdSubcat_Personnel_Camo_Urban

| Men (Urban)

| Characters in urban camouflage uniforms.

| EdSubcat_Personnel_Camo_Woodland

| Men (Woodland)

| Characters in woodland camouflage uniforms.

| EdSubcat_Personnel_European

| Men (European)

| Civilians with European identity sets.

| EdSubcat_Personnel_SpecialForces

| Men (Special Forces)

| Characters with special purpose (e.g., recon, snipers, divers, etc.)

| EdSubcat_Personnel_Story

| Men (Story)

| Special story characters, usually with specific names and identities already assigned.

| EdSubcat_Personnel_VR

| Men (Virtual Reality)

| Virtual characters.

| EdSubcat_Planes

| Planes

| Fixed wing aircrafts.

| EdSubcat_Submersibles

| Submersibles

| Vehicles operating underwater.

| EdSubcat_Tanks

| Tanks

| Armored vehicles with main purpose to neutralize other armored vehicles.

| EdSubcat_Targets

| Targets

| Virtual targets for AI.

| EdSubcat_Turrets

| Turrets

| Static weapons, e.g., fixed machine guns or mortars. This subcategory has priority over others (e.g., mortar is an artillery as well, but it is a turret first).

#### Props

##### Category

Example configuration:

editorCategory = "EdCat_Animals";

Class

Display Name

Contents

| EdCat_Animals

| Animals

| Animals, i.e., objects inherited from class Animal.

| EdCat_Default

| Other

| Default category where unsorted objects belong. Ideally, you should never use it and pick one of the more specific categories instead.

| EdCat_Equipment

| Equipment

| Character equipment (i.e., uniforms, vests, backpacks and headgear) which player can pick. Usually inherited from weapon class WeaponHolder and vehicle class Backpack.

| EdCat_Fences

| Fences

| Fence is defined as a structure that encloses an area, typically outdoors, and is usually constructed from posts that are connected by boards, wire, rails or netting.[1] A fence differs from a wall in not having a solid foundation along its whole length. Includes matching gates and corner elements.

| EdCat_Furniture

| Furniture

| Furniture is defined as the movable objects intended to support various human activities such as seating (e.g., chairs, stools and sofas) and sleeping (e.g., beds)

| EdCat_Ruins

| Ruins

| Ruins of objects categorized in Structures class. Not destroyed vehicles, they should be placed in Wrecks.

| EdCat_Ruins_Altis

| Ruins (Altis)

| Ruins of objects categorized in Structures (Altis) class.

| EdCat_Ruins_Tanoa

| Ruins (Tanoa)

| Ruins of objects categorized in Structures (Tanoa) class.

| EdCat_Signs

| Signs

| Signs, wall and ground decals and helper objects.

| EdCat_Structures

| Structures

| Immovable man-made objects, like buildings, industrial objects or utility network.

| EdCat_Structures_Altis

| Structures (Altis)

| Structures belonging specifically to Altis (i.e., Mediterranean) themed terrains.

| EdCat_Structures_Tanoa

| Structures (Tanoa)

| Structures belonging specifically to Tanoa (i.e., Pacific) themed terrains.

| EdCat_Supplies

| Supplies

| Equipment containers which can store weapons, magazines, items and equipment. Used for cargo crates, equipment which are visualized directly (e.g., specific rifle) belong to Equipment and Weapon Attachmentsand Weapons categories.

| EdCat_Things

| Things

| Movable man-made objects.

| EdCat_VRObjects

| VR Objects

| Virtual Reality themed objects, including structures and walls (this category has larger priority than others).

| EdCat_Walls

| Walls

| Objects for area enclosure with solid foundation. Includes matching gates and corner elements.

| EdCat_WeaponAttachments

| Weapon Attachments

| Weapon attachments which player can pick. Usually inherited from class WeaponHolder.

| EdCat_Weapons

| Weapons

| Weapons attachments which player can pick. Usually inherited from class WeaponHolder.

| EdCat_Wrecks

| Wrecks

| Destroyed vehicles. Should use the subcategories as unamaged vehicles have.

##### Subcategory

Example configuration:

editorSubcategory = "EdSubcat_Advertisements";

Class

Display Name

Intended Category

Contents

| EdSubcat_Advertisements

| Advertisements

| Signs

| Company signs and billboards.

| EdSubcat_Airports

| Airport

| 

| Airport themed objects.

| EdSubcat_Aquatic

| Aquatic

| Animals

| Aquatic animals (e.g., fish, sharks or turtles)

| EdSubcat_AssaultRifles

| Assault Rifles

| Weapons

| Assault rifles which can be picked up from the ground.

| EdSubcat_Avian

| Avian

| Animals

| Avian animals (e.g., birds)

| EdSubcat_Backpacks

| Backpacks

| Equipment

| Backpacks which can be picked up from the ground.

| EdSubcat_Beach

| Beach

| 

| Beach themed objects.

| EdSubcat_BlankSigns

| Blanks

| Signs

| Blank signs to which deigner can map a texture using setObjectTexture.

| EdSubcat_Blocks

| Blocks

| VR Objects

| Virtual Reality blocks.

| EdSubcat_BottomSlot

| Bipods

| Weapon Attachments

| Weapon attachments which can be picked up from the ground.

| EdSubcat_Camping

| Camping

| 

| Objects with touristic and camping theme.

| EdSubcat_Cemetery

| Cemetery

| 

| Cemetery themed objects.

| EdSubcat_Chemlights

| Chemlights

| Equipment

| Chemlights which can be picked up from the ground.

| EdSubcat_ConstructionSites

| Construction Sites

| 

| Construction themed objects.

| EdSubcat_Decals

| Decals

| Signs

| Semi-transparent flat signs which can be placed on the ground or on walls.

| EdSubcat_Default

| Other

| 

| Defaul subcategory. Ideally, you should never use it and pick one of the more specific subcategories instead.

| EdSubcat_DismantledWeapons

| Dismantled Weapons

| Equipment

| Backpacks with dismantled weapons (e.g., mortar) which can be picked up from the ground.

| EdSubcat_Electronics

| Electronics

| 

| Electronics and appliances.

| EdSubcat_Explosives

| Explosives

| Weapons

| Explosives and mines which can be picked up from the ground. They are inventory items, not explosives that can be immediatelly detonated.

| EdSubcat_Flags

| Flags

| Signs

| Flags on a flagpole.

| EdSubcat_Food

| Food

| 

| Food related objects.

| EdSubcat_FrontSlot

| Suppressors

| Weapon Attachments

| Front slot attachments which can be picked up from the ground.

| EdSubcat_Garbage

| Junk

| 

| Garbage and junk themed objects.

| EdSubcat_Graffiti

| Graffiti

| Signs

| Graffiti themed decals which can be placed on a wall.

| EdSubcat_Hats

| Hats

| Equipment

| Hats which can be picked up from the ground.

| EdSubcat_Helipads

| Helipads

| Signs

| Landing zone decals which can be placed on the ground.

| EdSubcat_Helmets

| Helmets

| Equipment

| Helmets which can be picked up from the ground.

| EdSubcat_Helpers

| Helpers

| Signs

| Abstract helper symbols and marks.

| EdSubcat_Historical

| Historical

| 

| Historical objects, e.g., ancient or medieval themed objects.

| EdSubcat_Industrial

| Industrial

| 

| Industrial themed objects.

| EdSubcat_Intel

| Intel

| 

| Intel items, e.g., documents or maps.

| EdSubcat_InventoryItems

| Inventory Items

| Equipment

| Misc inventory items which can be picked up from the ground.

| EdSubcat_Lamps

| Lamps

| 

| Lamps illuminating their surroundings.

| EdSubcat_Launchers

| Launchers

| Weapons

| Rocket launchers which can be picked up from the ground.

| EdSubcat_MachineGuns

| Machine Guns

| Weapons

| Machine guns which can be picked up from the ground.

| EdSubcat_Machines

| Machines

| 

| Various machines and devices.

| EdSubcat_Magazines

| Magazines

| 

| Magazines which can be picked up from the ground.

| EdSubcat_Market

| Market

| 

| Outdoor market themed objects.

| EdSubcat_Medicine

| Medicine

| 

| Healthcare themed objects.

| EdSubcat_Military

| Military

| 

| Military themed objects.

| EdSubcat_Obstacles

| Obstacles

| 

| Infantry and vehicle obstacles, mainly for training courses.

| EdSubcat_Office

| Office

| 

| Office themed objects.

| EdSubcat_Pistols

| Pistols

| Weapons

| Handguns which can be picked up from the ground.

| EdSubcat_Posters

| Posters

| Signs

| Posters which can be placed on a wall.

| EdSubcat_Racing

| Racing

| 

| Racing themed objects.

| EdSubcat_Religious

| Religious

| 

| Religious objects, with specific religion undefined.

| EdSubcat_Residential_City

| City

| 

| City themed objects.

| EdSubcat_Residential_Village

| Village

| 

| Village themed objects.

| EdSubcat_RoadSigns

| Road Signs

| 

| Road signs and barriers.

| EdSubcat_Seaports

| Seaport

| 

| Seaport themed objects.

| EdSubcat_Services

| Services

| 

| Commercial zone objects.

| EdSubcat_ShootHouse

| Shoot House

| 

| Objects with a theme of plywood training shoot house.

| EdSubcat_Shops

| Shop

| 

| Shop themed objects.

| EdSubcat_SideSlot

| Accessories

| Weapon Attachments

| Side attachments which can be picked up from the ground.

| EdSubcat_SniperRifles

| Sniper & Marksmen Rifles

| Weapons

| Sniper rifles which can be picked up from the ground.

| EdSubcat_Sports

| Sport & Recreation

| 

| Sport and recreation themed objects.

| EdSubcat_Storage

| Storage

| 

| Storage facilities and objects. Not weapon and equipment storage.

| EdSubcat_SubMachineGuns

| SMGs

| Weapons

| Sub-machine guns which can be picked up from the ground.

| EdSubcat_Terrestrial

| Terrestrial

| Animals

| Terrestrial animals (e.g., dog or rabbit).

| EdSubcat_Tools

| Tools

| 

| Tools and small devices.

| EdSubcat_TopSlot_Collimators

| Collimator Sights

| Weapon Attachments

| Collimator sight attachments which can be picked up from the ground.

| EdSubcat_TopSlot_Optics

| Optic Sights

| Weapon Attachments

| Optic sight attachments which can be picked up from the ground.

| EdSubcat_Trains

| Trains

| 

| Railroad themed objects.

| EdSubcat_Transportation

| Transportation

| 

| Transportation themed objects.

| EdSubcat_Uniforms

| Uniforms

| Equipment

| Uniforms which can be picked up from the ground.

| EdSubcat_Utilities

| Utilities

| 

| Utility objects, e.g., power, gas or water distribution.

| EdSubcat_Vests

| Vests

| Equipment

| Vests which can be picked up from the ground.

| EdSubcat_WarningSigns

| Warnings

| Signs

| Warning signs.

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Object_Categorization&oldid=363844"
					Category: - Eden Editor: Modding