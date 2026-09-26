/**
 * @file TBD_EObjectiveOnEmpty.c
 * @brief What partial capture progress does while nobody stands on the objective.
 *
 * Role: the resolved `zoneRules.onEmpty` of a capture objective.  Position: set by
 * `TBD_ObjectiveRuleResolver`; read by `TBD_ObjectiveProgression`.
 * State: none.  Invariants: the default is `HOLD`; decay applies only while the objective is
 * neutral, so an owned objective is lost only to a side standing on it.
 */

//! Empty-zone behaviour of partial capture progress. `HOLD` is the default because events are
//! one life: a squad wiped after banking progress cannot come back to redo it, and decay favours
//! the side with more bodies to feed into a zone.
enum TBD_EObjectiveOnEmpty
{
	HOLD, //!< `onEmpty: "hold"`, the default: partial progress is kept
	DECAY //!< `onEmpty: "decay"`: partial progress drains at `decayRate` per second while neutral
}
