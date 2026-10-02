/**
 * @file TBD_EObjectiveKind.c
 * @brief Which of the schema's three objective zone types an objective runs as.
 *
 * Role: the resolved `zones[].type` of an objective zone.  Position: set by
 * `TBD_ObjectiveRegistry.KindOf`; read by the objective runtime, `TBD_ZoneVolume` and
 * `TBD_MissionValidator`.
 * State: none.  Invariants: every zone that is not an objective zone resolves to `NONE`, so a
 * walk over all prepared zones needs no string comparison per call site.
 */

//! Objective kind resolved from `zones[].type`.
enum TBD_EObjectiveKind
{
	NONE, //!< not an objective zone (spawn, boundary, base_protection)
	CAPTURE, //!< `objective_capture`
	DESTROY, //!< `objective_destroy`
	HOLD_UNTIL //!< `objective_hold_until`
}
