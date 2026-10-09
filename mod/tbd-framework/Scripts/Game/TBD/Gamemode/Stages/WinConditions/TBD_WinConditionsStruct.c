/**
 * @file TBD_WinConditionsStruct.c
 * @brief The `winConditions` parameters the win rule reads in its own JSON pass.
 *
 * Role: typed target of the second JSON read of `winConditions` (`mode` and its parameters).
 * Position: filled by TBD_WinConditionEvaluator.Read from TBD_MissionJsonPass; read by
 * TBD_WinConditionEvaluator and TBD_WinConditionModes.  State: none.
 * Invariants: field names equal the JSON keys; `endOn` is not declared, TBD_MissionLoader.HasEndTrigger
 * is the one reader of it; an absent block reads as an empty `mode` (schema-required inside it).
 */

//! @contract mission.schema.json#/$defs/winConditions partial
class TBD_WinConditionsStruct
{
	static const int ABSENT_INT = -1; //!< initializer marking an absent integer key; `timeoutMinutes` has `minimum: 1`

	string mode; //!< JSON `mode`: attrition, objective, extraction, vip or timeout; empty = absent
	string extractionZoneId; //!< JSON `extractionZoneId`: a `zones[].id`; required by extraction, optional on vip; empty = absent
	string vipSlotId; //!< JSON `vipSlotId`: the protected player's `slots[].uid`; empty = absent
	int timeoutMinutes = ABSENT_INT; //!< JSON `timeoutMinutes`: round length (min), reported only; ABSENT_INT when absent
}

//! Document root of the win-rule pass; declares `winConditions` only, so no other key is read.
//! @contract mission.schema.json#/ partial
class TBD_WinConditionDocStruct
{
	ref TBD_WinConditionsStruct winConditions; //!< JSON `winConditions`; allocated even when absent
}
