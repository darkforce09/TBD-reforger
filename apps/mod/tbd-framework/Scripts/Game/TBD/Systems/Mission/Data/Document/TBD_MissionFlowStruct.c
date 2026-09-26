/**
 * @file TBD_MissionFlowStruct.c
 * @brief How an event is paced (`flow`) and how its round ends (`winConditions`).
 *
 * Role: the typed form of the `flow` and `winConditions` blocks.  Position: filled by
 * `TBD_MissionLoader`'s parse; `flow` is applied by `TBD_MissionFlow` and
 * `TBD_FrameworkManager.ApplyMissionFlow`, `winConditions` is read through
 * `TBD_MissionLoader.HasEndTrigger` and checked by `TBD_MissionWinConditionChecks`.
 * State: none; plain data.  Invariants: `flow` is always allocated, so each duration is tested
 * against `ABSENT`; an authored 0 is a real value (`timeLimitSeconds: 0` means no limit); nothing
 * here validates, and an out-of-range value is reported where it is applied.
 */

//! Round-end triggers. `endOn` values are the schema enum: time_limit, all_objectives_captured,
//! faction_eliminated, objective_destroyed, hold_expired.
//! @contract mission.schema.json#/$defs/winConditions
class TBD_MissionWinConditionsStruct
{
	string mode;             //!< JSON `mode`: free-form mode label from the editor.
	ref array<string> endOn; //!< JSON `endOn`: one or more end triggers.
}

//! The numbers that pace an event. `flow` is schema-required but declares no required property, so
//! `"flow": {}` is legal and behaves as if no flow was authored. `ABSENT` is the initializer
//! `JsonLoadContext` leaves on a missing key, so it doubles as the presence flag and lets a
//! negative authored value be reported instead of read as absent.
//! @contract mission.schema.json#/$defs/flow
class TBD_MissionFlowStruct
{
	static const int ABSENT = -1000000; //!< Presence flag for an absent key, not a default.

	int briefingSeconds = ABSENT;  //!< JSON `briefingSeconds`: intended BRIEFING stage length, seconds; >= 0.
	int safeStartSeconds = ABSENT; //!< JSON `safeStartSeconds`: safestart countdown length, seconds; >= 0.
	int timeLimitSeconds = ABSENT; //!< JSON `timeLimitSeconds`: round length, seconds; 0 means no limit; >= 0.
	string jip;                    //!< JSON `jip`: "disabled", "until_safestart_end" or "always"; empty = absent.
}
