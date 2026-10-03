/**
 * @file TBD_EObjectiveKind.c
 * @brief Which of the schema's three objective zone types an objective runs as.
 *
 * Role: the resolved `zones[].type` of an objective zone, and the identity key of its kind
 * behaviour.  Position: `TBD_ObjectiveRegistry.Build` sets it from the behaviour
 * `TBD_ObjectiveKindBehaviour.ForZoneType` finds; `TBD_ObjectiveKindBehaviour.For` turns it back
 * into that behaviour for the objective runtime, `TBD_ZoneVolume` and `TBD_MissionValidator`.
 * State: none.  Invariants: every zone that is not an objective zone resolves to `NONE`, so a
 * walk over all prepared zones needs no string comparison per call site.
 */

//! Objective kind resolved from `zones[].type`.
enum TBD_EObjectiveKind
{
	NONE, //!< not an objective zone (spawn, boundary, base_protection)
	CAPTURE, //!< a capture objective zone; `TBD_ObjectiveCaptureBehaviour`
	DESTROY, //!< a destroy objective zone; `TBD_ObjectiveDestroyBehaviour`
	HOLD_UNTIL //!< a hold objective zone; `TBD_ObjectiveHoldUntilBehaviour`
}
