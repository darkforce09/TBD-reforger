//! Fixed gear ResourceNames of a slot loadout (T-068.11 compiled block).
//! Empty string = slot not set (the compiler omits empty fields; JsonLoadContext
//! leaves absent keys at the initializer).
//! @contract mission.schema.json#/$defs/slot (loadout.gear)
class TBD_SlotGearStruct
{
	string primary;  //!< Primary weapon ResourceName.
	string optic;    //!< Optic ResourceName (informational until the attachments slice).
	string magazine; //!< Magazine ResourceName (informational until the attachments slice).
	string uniform;  //!< Jacket/uniform ResourceName.
	string vest;     //!< Vest ResourceName (armoredVest wins in the compiler).
	string helmet;   //!< Head cover ResourceName.
}

//! One container cargo row (loadout-export v2 {container,item,qty}).
//! @contract mission.schema.json#/$defs/slot (loadout.cargo[])
class TBD_SlotCargoStruct
{
	string container; //!< Wear container key: vest / pants / jacket / backpack.
	string item;      //!< Item ResourceName.
	int qty = 1;      //!< Units to insert (>= 1).
}

//! Optional per-slot Arsenal loadout (T-068.11): the kit prefab stays the base
//! character; T-068.12 layers this on the spawned HUMAN player.
//! @contract mission.schema.json#/$defs/slot (loadout)
class TBD_SlotLoadoutStruct
{
	ref TBD_SlotGearStruct gear;               //!< Fixed gear block (null = none).
	ref array<ref TBD_SlotCargoStruct> cargo;  //!< Container cargo rows (null = none).
}

//! One flattened ORBAT slot instance with exact spawn position (mission slots[]).
//! Field names must equal the JSON keys (JsonLoadContext maps by name).
//! @contract mission.schema.json#/$defs/slot
class TBD_MissionSlotStruct
{
	//! Sentinel for "y absent from JSON". JsonLoadContext leaves a missing key at the
	//! field initializer, and no real ASL height approaches -1e6 m, so the initializer
	//! doubles as the presence flag (standard JSON cannot carry NaN/Infinity).
	static const float Y_ABSENT = -1000000;

	string id;            //!< Stable slot id: {faction}:{groupCallsign}:{role}:{index}.
	string faction;       //!< Faction key (matches mission factions[].key).
	string groupCallsign; //!< Owning squad callsign.
	string role;          //!< Role label within the squad.
	string kit;           //!< Loadout alias (kit:<id>).
	float x;              //!< Spawn world X, metres.
	float z;              //!< Spawn world Z, metres.
	float y = -1000000;   //!< Optional spawn height, metres ASL (schema 1.2). Y_ABSENT when not in JSON.
	float headingDeg;     //!< Spawn heading, degrees.
	ref TBD_SlotLoadoutStruct loadout; //!< Optional Arsenal loadout (T-068.11; null when absent).

	//------------------------------------------------------------------------------------------------
	//! True when the mission JSON carried an explicit y for this slot.
	bool HasJsonY()
	{
		return y != Y_ABSENT;
	}
}
