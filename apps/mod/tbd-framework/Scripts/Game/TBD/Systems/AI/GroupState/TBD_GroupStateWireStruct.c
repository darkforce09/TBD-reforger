/**
 * @file TBD_GroupStateWireStruct.c
 * @brief The group AI attributes of the mission document, read by a second JSON pass.
 *
 * Role: JSON DTOs for `orbat.<faction>.groups[]` `callsign`, `combatMode`, `behaviour`,
 * `formation` and `speedMode`, and nothing else. `TBD_MissionOrbatGroupStruct` declares none of
 * the four attributes and `JsonLoadContext` binds only declared names, so the group-state reader
 * parses the raw document again with this root.  Position: filled by `TBD_GroupState` through
 * `TBD_MissionJsonPass.LoadRoot`.
 * State: none.  Invariants: field names are the JSON keys; the four attributes are strings, so
 * presence is an emptiness test.
 */

//! One `$defs/group` object: the callsign and the four AI attributes.
//! @contract mission.schema.json#/$defs/group
class TBD_GroupStateWireStruct
{
	string callsign; //!< JSON `callsign`: the join key onto flattened slots
	string combatMode; //!< JSON `combatMode`, optional: blue|green|white|yellow|red
	string behaviour; //!< JSON `behaviour`, optional: careless|safe|aware|combat|stealth
	string formation; //!< JSON `formation`, optional: a schema formation token
	string speedMode; //!< JSON `speedMode`, optional: limited|normal|full

	//! @return true when any of the four attributes is authored
	bool HasAnyAttr()
	{
		if (!combatMode.IsEmpty())
			return true;
		if (!behaviour.IsEmpty())
			return true;
		if (!formation.IsEmpty())
			return true;
		if (!speedMode.IsEmpty())
			return true;
		return false;
	}
}

//! One orbat faction's groups.
//! @contract mission.schema.json#/$defs/orbatFaction
class TBD_GroupStateFactionWireStruct
{
	ref array<ref TBD_GroupStateWireStruct> groups; //!< JSON `groups`
}

//! Root of the group-state pass: declares `orbat` and nothing else.
//! @contract mission.schema.json#/
class TBD_GroupStateDocStruct
{
	ref map<string, ref TBD_GroupStateFactionWireStruct> orbat; //!< JSON `orbat`: faction key -> faction
}
