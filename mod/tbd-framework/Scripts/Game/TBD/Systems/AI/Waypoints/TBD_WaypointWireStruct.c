/**
 * @file TBD_WaypointWireStruct.c
 * @brief The `group.waypoints[]` subset of the mission document, read by a second JSON pass.
 *
 * Role: JSON DTOs for `orbat.<faction>.groups[].callsign` and `.waypoints[]`, and nothing else.
 * `TBD_MissionOrbatGroupStruct` declares no `waypoints` field and `JsonLoadContext` binds only
 * declared names, so the waypoint reader parses the raw document again with this root.
 * Position: filled by `TBD_WaypointRuntime` through `TBD_MissionJsonPass.LoadRoot`; each waypoint
 * is spawned by `TBD_WaypointFactory`.
 * State: none.  Invariants: field names are the JSON keys; `waypoints` presence is a null-or-empty
 * test; numeric keys that may be authored as 0 carry an ABSENT sentinel (`y` shares
 * `TBD_MissionSlotStruct`'s value).
 */

//! One `$defs/waypoint` object.
//! @contract mission.schema.json#/$defs/waypoint
class TBD_WaypointWireStruct
{
	static const float Y_ABSENT = -1000000; //!< `y` value when the key is omitted; same as `TBD_MissionSlotStruct`
	static const float RADIUS_ABSENT = -1; //!< `radiusM` value when the key is omitted; 0 is an authored radius

	string type; //!< JSON `type`, required: move|attack|defend|patrol|cycle|hold|get_in|get_out|seek_and_destroy|sentry
	float x; //!< JSON `x`, required: world X in metres
	float z; //!< JSON `z`, required: world Z in metres
	float y = -1000000; //!< JSON `y`, optional: metres ASL; `Y_ABSENT` when omitted
	string vehicleUid; //!< JSON `vehicleUid`, optional: get_in / get_out roster vehicle; empty boards by proximity
	float radiusM = -1; //!< JSON `radiusM`, optional: completion radius in metres; `RADIUS_ABSENT` when omitted
	string behaviour; //!< JSON `behaviour`, optional: careless|safe|aware|combat|stealth
	string speedMode; //!< JSON `speedMode`, optional: limited|normal|full

	//! @return true when `y` was authored
	bool HasJsonY()
	{
		return y != Y_ABSENT;
	}

	//! @return true when `radiusM` was authored
	bool HasRadius()
	{
		return radiusM != RADIUS_ABSENT;
	}
}

//! The `$defs/group` keys the waypoint pass reads.
//! @contract mission.schema.json#/$defs/group partial
class TBD_WaypointGroupWireStruct
{
	string callsign; //!< JSON `callsign`: the join key onto flattened slots
	ref array<ref TBD_WaypointWireStruct> waypoints; //!< JSON `waypoints`: null or empty when none are authored
}

//! One orbat faction's groups.
//! @contract mission.schema.json#/$defs/orbatFaction
class TBD_WaypointFactionWireStruct
{
	ref array<ref TBD_WaypointGroupWireStruct> groups; //!< JSON `groups`
}

//! Root of the waypoint pass: declares `orbat` and nothing else.
//! @contract mission.schema.json#/ partial
class TBD_WaypointDocStruct
{
	ref map<string, ref TBD_WaypointFactionWireStruct> orbat; //!< JSON `orbat`: faction key -> faction
}
