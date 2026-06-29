//! One flattened ORBAT slot instance with exact spawn position (mission slots[]).
//! Field names must equal the JSON keys (JsonLoadContext maps by name).
//! @contract mission.schema.json#/$defs/slot
class TBD_MissionSlotStruct
{
	string id;            //!< Stable slot id: {faction}:{groupCallsign}:{role}:{index}.
	string faction;       //!< Faction key (matches mission factions[].key).
	string groupCallsign; //!< Owning squad callsign.
	string role;          //!< Role label within the squad.
	string kit;           //!< Loadout alias (kit:<id>).
	float x;              //!< Spawn world X, metres.
	float z;              //!< Spawn world Z, metres.
	float headingDeg;     //!< Spawn heading, degrees.
}
