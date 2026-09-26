/**
 * @file TBD_Task.c
 * @brief One prepared mission task and its state and tier vocabularies.
 *
 * Role: the runtime record of one `tasks[]` row: identity, text, tier, state, linked trigger,
 * HUD position and schedule window.  Position: prepared and advanced by `TBD_TaskStateMachine`
 * and `TBD_TaskSchedule`; read by `TBD_TaskHud` and `TBD_AudioEmitter`.
 * State: server state for the world that built it; clients receive a snapshot, never this
 * object.  Invariants: the state moves at most once, from ASSIGNED.
 */

//! A task's lifecycle state.
enum TBD_ETaskState
{
	ASSIGNED, //!< `assigned`: the start state
	SUCCEEDED, //!< `succeeded`: the linked trigger fired
	FAILED //!< `failed`: the trigger is inert or missing, or the window closed
}

//! A task's tier.
enum TBD_ETaskTier
{
	PRIMARY, //!< `primary`, the default
	SECONDARY, //!< `secondary`
	OPTIONAL //!< `optional`
}

//! One prepared task.
class TBD_Task
{
	string m_sId; //!< `id`
	string m_sTitle; //!< `title`
	TBD_ETaskTier m_eTier; //!< `tier`; default PRIMARY
	TBD_ETaskState m_eState; //!< current state; starts at the authored `state`, default ASSIGNED
	string m_sTriggerId; //!< `triggerId`; empty = no linked trigger
	string m_sMarkerId; //!< `markerId`; empty = the default icon
	string m_sDescription; //!< `description`
	int m_iWorldX; //!< HUD marker world X, metres, from the linked trigger's zone
	int m_iWorldZ; //!< HUD marker world Z, metres
	bool m_bHasPosition; //!< the linked trigger has a zone, so the task has a marker position
	bool m_bHasSchedule; //!< a valid `schedule` window was authored
	int m_iStartAfterS; //!< window start, mission seconds
	int m_iWindowS; //!< window length, seconds
	bool m_bWindowOpened; //!< the window has opened (logged once)
}
