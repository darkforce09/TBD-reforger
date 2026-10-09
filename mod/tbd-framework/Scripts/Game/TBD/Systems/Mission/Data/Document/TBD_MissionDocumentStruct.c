/**
 * @file TBD_MissionDocumentStruct.c
 * @brief The root of the typed mission document, its header and its playable factions.
 *
 * Role: the primary `JsonLoadContext` target for a mission document; every consumed top-level key
 * is a field named exactly as its JSON key.  Position: filled by `TBD_MissionLoader` from the
 * verified artifact bytes; read by the validator, the variant filter and every system through
 * `TBD_MissionLoader.GetMission`.
 * State: none; plain data.  Invariants: `JsonLoadContext` allocates a nested `ref <class>` field
 * even when its key is absent, so presence is tested on content (sentinels, empty strings,
 * `Count()`), never on a null reference; a JSON key no field declares is invisible to this parse.
 */

//! The mission header: identity, display name and terrain.
//! @contract mission.schema.json#/$defs/meta partial
class TBD_MissionMetaStruct
{
	string id;      //!< JSON `id`: the content-hash id a published mission carries.
	string name;    //!< JSON `name`: display name; empty when absent.
	string terrain; //!< JSON `terrain`: terrain key the mission runs on; empty when absent.
}

//! One playable faction from the mission `factions[]` array.
//! @contract mission.schema.json#/$defs/faction
class TBD_MissionFactionStruct
{
	string key;         //!< JSON `key`: the faction key slots, zones and the ORBAT reference.
	string displayName; //!< JSON `displayName`: lobby label; empty when absent.
	string presetId;    //!< JSON `presetId`: registry preset the faction builds from; empty when absent.
}

//! The full mission document. `schemaVersion` is the canonical string ("1.0" to "1.3"), distinct
//! from the editor's integer export version; every 1.3 document is a valid 1.2 one with extra
//! optional keys. Nested objects are always allocated after a parse, so each consumer tests their
//! fields, never the reference.
//! @contract mission.schema.json#/
class TBD_MissionDocumentStruct
{
	string schemaVersion;                                        //!< JSON `schemaVersion`; `TBD_MissionValidator` lists the accepted values.
	ref TBD_MissionMetaStruct meta;                              //!< JSON `meta`: the mission header.
	ref array<ref TBD_MissionFactionStruct> factions;            //!< JSON `factions`: playable factions.
	ref array<ref TBD_MissionZoneStruct> zones;                  //!< JSON `zones`: spawn, objective and boundary zones.
	ref map<string, ref TBD_MissionOrbatFactionStruct> orbat;    //!< JSON `orbat`: the ORBAT keyed by faction key.
	ref array<ref TBD_MissionSlotStruct> slots;                  //!< JSON `slots`: flattened spawn slots (schema 1.1 and later).
	ref array<ref TBD_MissionEntityStruct> entities;             //!< JSON `entities`: mission-placed world objects; optional.
	ref array<ref TBD_MissionVehicleStruct> vehicles;            //!< JSON `vehicles`: the vehicle crew roster, joined to `entities` on `uid`; optional.
	ref TBD_MissionWinConditionsStruct winConditions;            //!< JSON `winConditions`: round-end triggers.
	ref TBD_MissionFlowStruct flow;                              //!< JSON `flow`: event pacing; test fields against `TBD_MissionFlowStruct.ABSENT`.
	ref map<string, ref TBD_MissionBriefingStruct> briefings;    //!< JSON `briefings`: written orders keyed by faction key; null when absent.
	ref TBD_MissionRadioPlanStruct radioPlan;                    //!< JSON `radioPlan`: presence is a content test on its nets inside `TBD_RadioPlan`.
	ref TBD_MissionSettingsStruct settings;                      //!< JSON `settings`: respawn, spectator and night-vision policy.
	ref TBD_MissionEnvironmentStruct environment;                //!< JSON `environment`: fog, wind and view distance; ABSENT sentinels mark unauthored fields.
	ref array<ref TBD_MissionParamStruct> missionParams;         //!< JSON `missionParams`: launch-time parameters; presence is `Count()`.
	ref array<ref TBD_MissionVariantStruct> variants;            //!< JSON `variants`: the named-variant registry; key presence is read off the raw body.
}
