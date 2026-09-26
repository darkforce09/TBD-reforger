/**
 * @file TBD_MissionOrbatFactionStruct.c
 * @brief One faction's ORBAT: its groups (squads) and each group's roles.
 *
 * Role: the typed form of the `orbat` map values.  Position: filled by `TBD_MissionLoader`'s
 * parse; read by `TBD_MissionOrbatQuery`, the variant filter and the validator's ORBAT parity check.
 * State: none; plain data.  Invariants: a callsign is unique only within one faction;
 * `leaderSlotId` carries a slot `uid`, never a derived slot id.
 */

//! One ORBAT role line.
//! @contract mission.schema.json#/$defs/role
class TBD_MissionOrbatRoleStruct
{
	string slot; //!< JSON `slot`: role label within the squad ("Squad Leader"); schema-required.
	string kit;  //!< JSON `kit`: loadout alias (`kit:<id>`); schema-required.
	int count;   //!< JSON `count`: number of slots this role materializes.
}

//! One ORBAT group (squad) and its roles. The squad leader is a per-group fact, so one copy lives
//! here rather than one per seat; the compiler drops a `leaderSlotId` naming no seat of the group,
//! and `TBD_MissionOrbatQuery.GetSquadLeaderSlot` re-checks membership for documents it did not
//! compile.
//! @contract mission.schema.json#/$defs/group
class TBD_MissionOrbatGroupStruct
{
	string callsign;                               //!< JSON `callsign`: squad callsign ("Alpha"); schema-required.
	string type;                                   //!< JSON `type`: group type label ("infantry_squad"); schema-required.
	string variantId;                              //!< JSON `variantId`: variant gate; empty = unconditional row.
	ref array<ref TBD_MissionOrbatRoleStruct> roles; //!< JSON `roles`: the role lines.
	string leaderSlotId;                           //!< JSON `leaderSlotId`: `slots[].uid` of the seat that leads the squad; empty when none is named.
}

//! One faction's ORBAT, keyed by faction key in the document's `orbat` map.
//! @contract mission.schema.json#/$defs/orbatFaction
class TBD_MissionOrbatFactionStruct
{
	ref array<ref TBD_MissionOrbatGroupStruct> groups; //!< JSON `groups`: the faction's squads.
}
