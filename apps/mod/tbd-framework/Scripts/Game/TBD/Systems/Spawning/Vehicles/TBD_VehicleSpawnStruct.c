/**
 * @file TBD_VehicleSpawnStruct.c
 * @brief The second-pass projection of `vehicles[]` rows that TBD_VehicleSpawnDefaults reads.
 *
 * Role: carries each roster vehicle's position, authored fuel presence and inventory rows.
 * Position: read from the held mission JSON by TBD_VehicleSpawnDefaults; TBD_MissionVehicleStruct
 * declares neither fuel presence nor inventory.
 * State: none.  Invariants: numeric fuel defaults to FUEL_ABSENT, so an authored 0 is real and an
 * omitted key is not; inventory presence is a count, never a null test, because JsonLoadContext
 * allocates a nested array for an absent key.
 */

//! One `vehicles[].inventory` row. Field names are the JSON keys.
//! @contract mission.schema.json#/$defs/entityInventory
class TBD_VehicleSpawnInventoryStruct
{
	string item; //!< JSON `item`: the item prefab
	int qty; //!< JSON `qty`: how many to insert
}

//! One `vehicles[]` row, as far as spawn defaults need it.
//! @contract mission.schema.json#/$defs/vehicle
class TBD_VehicleSpawnStruct
{
	static const float FUEL_ABSENT = -1000000; //!< sentinel for an omitted `fuel`
	static const float XZ_M = 3.0; //!< half width (m) of the box that finds the placed vehicle
	static const float Y_M = 300.0; //!< half height (m) of that box

	string uid; //!< JSON `uid`: roster identity
	string alias; //!< JSON `alias`: the `veh:` registry alias
	float x; //!< JSON `x` (m)
	float z; //!< JSON `z` (m)
	float fuel = FUEL_ABSENT; //!< JSON `fuel` (0..1); FUEL_ABSENT when omitted
	ref array<ref TBD_VehicleSpawnInventoryStruct> inventory; //!< JSON `inventory`; allocated even when absent

	//! True when the row authored `fuel`, including 0.
	bool HasFuel()
	{
		return fuel != FUEL_ABSENT;
	}

	//! True when the row authored at least one inventory row.
	bool HasInventory()
	{
		if (!inventory)
			return false;
		return inventory.Count() > 0;
	}
}

//! Root of the vehicles pass: declares `vehicles` and nothing else.
//! @contract mission.schema.json#/properties/vehicles
class TBD_VehicleSpawnDocStruct
{
	ref array<ref TBD_VehicleSpawnStruct> vehicles; //!< JSON `vehicles`; allocated even when absent
}
