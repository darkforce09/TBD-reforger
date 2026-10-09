/**
 * @file TBD_MissionEntityStruct.c
 * @brief One mission-placed world object from the `entities[]` array.
 *
 * Role: the typed form of `entities[]` rows.  Position: filled by `TBD_MissionLoader`'s parse;
 * spawned by `TBD_MissionWorldApplier.SpawnMissionEntities`; filtered by the variant filter.
 * State: none; plain data.  Invariants: field names equal the JSON keys; the optional `inventory`
 * key has no field and is not read.
 */

//! One mission-placed world object. `uid` is the join key to the `vehicles[]` roster row of the
//! same authored vehicle: one authored vehicle emits both rows, and `TBD_MissionVehicleRoster.ClaimTwin`
//! joins them so the vehicle spawns once.
//! @contract mission.schema.json#/$defs/entity
class TBD_MissionEntityStruct
{
	string alias;     //!< JSON `alias`: registry alias (`prop:`, `comp:`, ...); schema-required.
	string uid;       //!< JSON `uid`: the editor's id for the authored object; optional, empty when not carried.
	float x;          //!< JSON `x`: world X, metres; schema-required.
	float z;          //!< JSON `z`: world Z, metres; schema-required.
	float headingDeg; //!< JSON `headingDeg`: yaw, degrees 0..360; 0 when absent.
	string faction;   //!< JSON `faction`: faction key; empty when absent.
	string variantId; //!< JSON `variantId`: variant gate; empty = unconditional row.
}
