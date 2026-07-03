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

	//------------------------------------------------------------------------------------------------
	//! True when the mission JSON carried an explicit y for this slot.
	bool HasJsonY()
	{
		return y != Y_ABSENT;
	}
}
