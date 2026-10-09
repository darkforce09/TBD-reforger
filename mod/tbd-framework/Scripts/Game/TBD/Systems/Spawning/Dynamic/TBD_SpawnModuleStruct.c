/**
 * @file TBD_SpawnModuleStruct.c
 * @brief One authored `spawnModules[]` row and the document root of that pass.
 *
 * Role: the typed read of a dynamic spawn module.  Position: read from the held mission JSON by
 * TBD_DynamicSpawner.ReadWire; prepared into TBD_SpawnModuleRuntime.
 * State: none.  Invariants: absent `x`, `z` and `intervalSeconds` read as TBD_DynamicSpawner.ABSENT;
 * absent `count` and `maxAlive` read as 0; `spawnModules` presence is a count, because
 * JsonLoadContext allocates the array for an absent key.
 */

//! One `spawnModules[]` row. Field names are the JSON keys.
//! @contract mission.schema.json#/$defs/spawnModule
class TBD_SpawnModuleStruct
{
	string id; //!< JSON `id`: stable module id
	string kind; //!< JSON `kind`: the restocking token or `garrison`
	string factionKey; //!< JSON `factionKey`: blufor, opfor, indfor or civ
	string groupTemplate; //!< JSON `groupTemplate`: the group prefab
	float x; //!< JSON `x` (m); ABSENT when omitted
	float z; //!< JSON `z` (m); ABSENT when omitted
	string zoneId; //!< JSON `zoneId`: spawn at this zone's centre instead of x and z
	int count; //!< JSON `count`: groups per volley
	float intervalSeconds; //!< JSON `intervalSeconds` (s) between restocking volleys; ABSENT when omitted
	int maxAlive; //!< JSON `maxAlive`: living group cap; 0 when omitted (the count applies)
	string triggerId; //!< JSON `triggerId`: stay idle until this trigger fired

	//! Seed the ABSENT sentinels the JSON read overwrites when a key is present.
	void TBD_SpawnModuleStruct()
	{
		x = TBD_DynamicSpawner.ABSENT;
		z = TBD_DynamicSpawner.ABSENT;
		intervalSeconds = TBD_DynamicSpawner.ABSENT;
		maxAlive = 0;
		count = 0;
	}
}

//! Root of the spawn-modules pass: declares `spawnModules` and nothing else.
//! @contract mission.schema.json#/ partial
class TBD_SpawnModulesDocStruct
{
	ref array<ref TBD_SpawnModuleStruct> spawnModules; //!< JSON `spawnModules`; allocated even when absent
}
