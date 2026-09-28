/**
 * @file TBD_TaskStruct.c
 * @brief The `tasks[]` wire shapes the task pass binds from the mission JSON.
 *
 * Role: one `tasks[]` row, its optional `schedule`, and the document root that declares only
 * `tasks`.  Position: bound by `TBD_TaskStateMachine.ReadWire`; prepared into `TBD_Task`.
 * State: none.  Invariants: member names equal the JSON keys, because `JsonLoadContext` binds by
 * member name; an empty string is an absent key (the schema requires `minLength` 1); the nested
 * `schedule` is always allocated, so its ABSENT sentinels are the presence test.
 */

//! One `tasks[]` row.
//! @contract mission.schema.json#/$defs/task
class TBD_TaskStruct
{
	string id; //!< `id`
	string title; //!< `title`
	string tier;         //!< primary | secondary | optional
	string state;        //!< assigned | succeeded | failed
	string triggerId; //!< `triggerId`: the editor trigger whose firing succeeds the task
	string markerId; //!< `markerId`: the HUD icon; empty uses the default
	string description; //!< `description`
	ref TBD_TaskScheduleStruct schedule; //!< `schedule`, always allocated
}

//! Optional `tasks[].schedule`: the task's evaluation window on the mission clock.
//! @contract mission.schema.json#/$defs/taskSchedule
class TBD_TaskScheduleStruct
{
	static const int ABSENT = -1000000; //!< "key absent" sentinel; `startAfterS` may legally be 0
	int startAfterS = -1000000; //!< `startAfterS`: mission seconds before the window opens
	int windowS = -1000000; //!< `windowS`: window length in seconds
}

//! The document root of the task pass; declares `tasks` only.
//! @contract mission.schema.json#/ partial
class TBD_TaskDocStruct
{
	ref array<ref TBD_TaskStruct> tasks; //!< `tasks[]`
}
