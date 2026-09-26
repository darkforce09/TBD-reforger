/**
 * @file TBD_ValidatorSecondPassStruct.c
 * @brief The validator's second-pass view of a mission: only the values the primary document does
 * not declare.
 *
 * Role: `layers`, `factions[].tickets` and `orbat.*.groups[].roles[].radio`, read by a second
 * `JsonLoadContext` pass over the same text.  Position: filled and read by
 * `TBD_MissionUnconsumedKeyCheck`; `TBD_MissionDocumentStruct` stays the single source of every
 * consumed field.
 * State: none; plain data.  Invariants: field names equal the JSON keys and every other key is
 * invisible to this pass; a `ref array<>` field is null when its key is absent, which tells absent
 * from authored-empty; `tickets` has no sentinel, because absent and 0 both mean one-life. When a
 * key gains a consumer, its field leaves this pass for the primary document.
 */

//! One `orbat.*.groups[].roles[]` entry, radio only.
//! @contract mission.schema.json#/$defs/role
class TBD_ValidatorRawRoleStruct
{
	ref array<string> radio; //!< JSON `radio`: null = key absent; empty = authored `[]`; else authored net ids.
}

//! One `orbat.*.groups[]` entry, roles only.
//! @contract mission.schema.json#/$defs/group
class TBD_ValidatorRawGroupStruct
{
	ref array<ref TBD_ValidatorRawRoleStruct> roles; //!< JSON `roles`: the group's roles.
}

//! One faction's ORBAT, groups only.
//! @contract mission.schema.json#/$defs/orbatFaction
class TBD_ValidatorRawOrbatFactionStruct
{
	ref array<ref TBD_ValidatorRawGroupStruct> groups; //!< JSON `groups`: the faction's groups.
}

//! One `factions[]` entry: the key, so a finding can name the side, and the tickets.
//! @contract mission.schema.json#/$defs/faction
class TBD_ValidatorRawFactionStruct
{
	string key;      //!< JSON `key`: the faction key.
	int tickets = 0; //!< JSON `tickets`: respawn pool; absent reads 0, the same as authored one-life.
}

//! Document root for the second pass: the three vocabularies, nothing else.
//! @contract mission.schema.json#/
class TBD_ValidatorSecondPassStruct
{
	ref array<string> layers;                                    //!< JSON `layers`: null = key absent; empty = authored `[]`.
	ref array<ref TBD_ValidatorRawFactionStruct> factions;       //!< JSON `factions`: key and tickets per faction.
	ref map<string, ref TBD_ValidatorRawOrbatFactionStruct> orbat; //!< JSON `orbat`: groups per faction key.
}
