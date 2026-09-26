/**
 * @file TBD_TaskStateMachine.c
 * @brief Runs the mission's `tasks[]`: assigned, then succeeded or failed from triggers and windows.
 *
 * Role: reads `tasks[]` once per mission, moves each task at most once (assigned to succeeded
 * when its linked editor trigger fires, to failed when that trigger is inert or missing or its
 * window closes), resolves marker positions, and asks `TBD_TaskHud` to push the assigned tasks on
 * a change.  Position: `TBD_RuntimeHeartbeat` calls `Tick` every `TICK_MS` on the server and
 * `Clear` per world; reads `TBD_TriggerRuntime` and the held mission JSON; read by `TBD_TaskHud`
 * and `TBD_AudioEmitter`.
 * State: the prepared tasks, the mission they were built for and the dirty flag, for the current
 * world.  Invariants: tasks observe triggers and never fire `winConditions.endOn`; every
 * transition logs `[TBD][Task] id=<n> t=<s> -> <state>`; an illegal transition is logged and
 * ignored; a mission id change rebuilds the tasks.
 */

//! Server-authoritative `tasks[]` state machine.
class TBD_TaskStateMachine
{
	static const string CH = "Task"; //!< log channel `[TBD][Task]`

	static const string STATE_ASSIGNED  = "assigned"; //!< `tasks[].state` value
	static const string STATE_SUCCEEDED = "succeeded"; //!< `tasks[].state` value
	static const string STATE_FAILED    = "failed"; //!< `tasks[].state` value

	static const string TIER_PRIMARY   = "primary"; //!< `tasks[].tier` value
	static const string TIER_SECONDARY = "secondary"; //!< `tasks[].tier` value
	static const string TIER_OPTIONAL  = "optional"; //!< `tasks[].tier` value

	static const string DEFAULT_ICON = "objective_marker"; //!< HUD icon of a task with no `markerId`

	static const int TICK_MS = 1000; //!< heartbeat cadence of `Tick`, milliseconds
	static const string ANNOUNCE_IDLE_KEY = "Task.idle"; //!< `TBD_AnnounceOnce` key of the once-per-mission idle line

	protected static ref array<ref TBD_Task> s_aTasks; //!< prepared tasks; null until `Build`
	protected static bool s_bBuilt; //!< `Build` completed since the last `Clear`
	protected static string s_sBuiltForMission; //!< `meta.id` of the mission the tasks were built for
	protected static bool s_bDirty; //!< a task changed since the last HUD push

	//! Drop every task and reset the clock; the next tick rebuilds.
	static void Clear()
	{
		s_aTasks = null;
		s_bBuilt = false;
		s_sBuiltForMission = string.Empty;
		s_bDirty = false;
		TBD_AnnounceOnce.Rearm(ANNOUNCE_IDLE_KEY);
		TBD_TaskSchedule.Clear();
	}

	//! Whether `Build` has completed since the last `Clear`.
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//! The prepared tasks.
	//! @return the tasks, or null before `Build`
	static array<ref TBD_Task> GetAll()
	{
		return s_aTasks;
	}

	//! Prepare every `tasks[]` row of the loaded mission; a row without an id or title is skipped
	//! with a warning. Idempotent until `Clear`.
	//! @return false while no mission is loaded, so the heartbeat keeps waiting
	//! @authority server
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		string missionId = TBD_MissionLoader.GetMissionId();
		if (missionId.IsEmpty())
			return false;

		array<ref TBD_TaskStruct> raw = ReadWire();
		s_aTasks = new array<ref TBD_Task>();
		s_bBuilt = true;
		s_sBuiltForMission = missionId;
		s_bDirty = true;

		if (!raw)
			return true;

		foreach (int index, TBD_TaskStruct rawTask : raw)
		{
			if (!rawTask)
			{
				TBD_Log.Warn(CH, string.Format("tasks[%1] is null - skipped", index));
				continue;
			}

			TBD_Task task = Prepare(rawTask, index);
			if (task)
				s_aTasks.Insert(task);
		}

		return true;
	}

	//! One heartbeat pass: rebuild on a mission change, advance the schedule windows and the
	//! trigger-linked transitions, resolve positions, and push the HUD when anything changed.
	//! @authority server
	static void Tick()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		string liveId = TBD_MissionLoader.GetMissionId();
		if (s_bBuilt && !s_sBuiltForMission.IsEmpty() && liveId != s_sBuiltForMission)
			Clear();

		if (!Build())
			return;

		if (!s_aTasks || s_aTasks.Count() == 0)
		{
			TBD_AnnounceOnce.Kv(CH, ANNOUNCE_IDLE_KEY, "idle", "this mission authors no tasks");
			return;
		}

		int t = TBD_TaskSchedule.ElapsedS();
		TBD_TaskSchedule.Evaluate(s_aTasks, t);
		SyncFromTriggers(t);
		ResolvePositions();

		if (s_bDirty)
		{
			s_bDirty = false;
			TBD_TaskHud.PushToPlayers();
		}
	}

	//! The one mutation: move `task` to `to` when the pair is legal, mark the HUD dirty and log.
	//! @return false, with a warning, for an illegal pair
	static bool TryTransition(notnull TBD_Task task, TBD_ETaskState to)
	{
		if (!IsLegal(task.m_eState, to))
		{
			TBD_Log.Warn(CH, string.Format(
				"illegal task transition %1 -> %2 on '%3' - ignored",
				StateName(task.m_eState),
				StateName(to),
				task.m_sId));
			return false;
		}

		task.m_eState = to;
		s_bDirty = true;
		TBD_Log.Event(CH, string.Format(
			"id=%1 t=%2 -> %3",
			task.m_sId,
			TBD_TaskSchedule.LastElapsedS(),
			StateName(to)));
		return true;
	}

	//! Whether `from` -> `to` is legal: only ASSIGNED moves, to SUCCEEDED or FAILED.
	static bool IsLegal(TBD_ETaskState from, TBD_ETaskState to)
	{
		if (from != TBD_ETaskState.ASSIGNED)
			return false;

		if (to == TBD_ETaskState.SUCCEEDED)
			return true;

		if (to == TBD_ETaskState.FAILED)
			return true;

		return false;
	}

	//! For every assigned, evaluating task with a `triggerId`: fail it when the trigger is missing
	//! or inert, succeed it when the trigger fired. Waits until the trigger runtime is built.
	protected static void SyncFromTriggers(int t)
	{
		if (!TBD_TriggerRuntime.IsBuilt())
			return;

		foreach (TBD_Task task : s_aTasks)
		{
			if (!task)
				continue;

			if (task.m_eState != TBD_ETaskState.ASSIGNED)
				continue;

			if (!TBD_TaskSchedule.IsEvaluating(task, t))
				continue;

			if (task.m_sTriggerId.IsEmpty())
				continue;

			TBD_Trigger trigger = FindTriggerById(task.m_sTriggerId);
			if (!trigger)
			{
				TryTransition(task, TBD_ETaskState.FAILED);
				continue;
			}

			if (trigger.m_eState == TBD_ETriggerState.INERT)
			{
				TryTransition(task, TBD_ETaskState.FAILED);
				continue;
			}

			if (trigger.m_eState == TBD_ETriggerState.FIRED)
				TryTransition(task, TBD_ETaskState.SUCCEEDED);
		}
	}

	//! The prepared trigger with this id.
	//! @return the trigger, or null
	protected static TBD_Trigger FindTriggerById(string id)
	{
		array<ref TBD_Trigger> triggers = TBD_TriggerRuntime.GetAll();
		if (!triggers)
			return null;

		foreach (TBD_Trigger trigger : triggers)
		{
			if (trigger && trigger.m_sId == id)
				return trigger;
		}

		return null;
	}

	//! Place each task with a linked trigger's zone at that zone's centre (a circle's centre, else
	//! the bounds' midpoint), rounded to whole metres.
	protected static void ResolvePositions()
	{
		foreach (TBD_Task task : s_aTasks)
		{
			if (!task)
				continue;

			task.m_bHasPosition = false;

			if (task.m_sTriggerId.IsEmpty())
				continue;

			TBD_Trigger trigger = FindTriggerById(task.m_sTriggerId);
			if (!trigger || !trigger.m_Zone)
				continue;

			TBD_Zone zone = trigger.m_Zone;
			float x;
			float z;
			if (zone.m_eShape == TBD_EZoneShapeKind.CIRCLE)
			{
				x = zone.m_fCx;
				z = zone.m_fCz;
			}
			else
			{
				x = (zone.m_fMinX + zone.m_fMaxX) * 0.5;
				z = (zone.m_fMinZ + zone.m_fMaxZ) * 0.5;
			}

			task.m_iWorldX = TBD_Rounding.RoundToInt(x);
			task.m_iWorldZ = TBD_Rounding.RoundToInt(z);
			task.m_bHasPosition = true;
		}
	}

	//! Prepare one row; a missing id or title skips it with a warning.
	//! @return the task, or null when skipped
	protected static TBD_Task Prepare(notnull TBD_TaskStruct raw, int index)
	{
		if (raw.id.IsEmpty())
		{
			TBD_Log.Warn(CH, string.Format("tasks[%1] has no id - skipped", index));
			return null;
		}

		if (raw.title.IsEmpty())
		{
			TBD_Log.Warn(CH, string.Format("task '%1' has no title - skipped", raw.id));
			return null;
		}

		TBD_Task task = new TBD_Task();
		task.m_sId = raw.id;
		task.m_sTitle = raw.title;
		task.m_sTriggerId = raw.triggerId;
		task.m_sMarkerId = raw.markerId;
		task.m_sDescription = raw.description;
		task.m_eTier = ParseTier(raw.tier);
		task.m_eState = ParseState(raw.state);
		task.m_bHasSchedule = false;
		task.m_iStartAfterS = 0;
		task.m_iWindowS = 0;
		task.m_bWindowOpened = false;
		TBD_TaskSchedule.Bind(task, raw);
		return task;
	}

	//! The tier of a `tasks[].tier` string; anything else is PRIMARY.
	protected static TBD_ETaskTier ParseTier(string raw)
	{
		if (raw == TIER_SECONDARY)
			return TBD_ETaskTier.SECONDARY;

		if (raw == TIER_OPTIONAL)
			return TBD_ETaskTier.OPTIONAL;

		return TBD_ETaskTier.PRIMARY;
	}

	//! The state of a `tasks[].state` string; anything else is ASSIGNED.
	protected static TBD_ETaskState ParseState(string raw)
	{
		if (raw == STATE_SUCCEEDED)
			return TBD_ETaskState.SUCCEEDED;

		if (raw == STATE_FAILED)
			return TBD_ETaskState.FAILED;

		return TBD_ETaskState.ASSIGNED;
	}

	//! The `tasks[].state` string of a state.
	static string StateName(TBD_ETaskState state)
	{
		if (state == TBD_ETaskState.SUCCEEDED)
			return STATE_SUCCEEDED;

		if (state == TBD_ETaskState.FAILED)
			return STATE_FAILED;

		return STATE_ASSIGNED;
	}

	//! The HUD icon of a task: its `markerId`, else `DEFAULT_ICON`.
	static string IconFor(notnull TBD_Task task)
	{
		if (!task.m_sMarkerId.IsEmpty())
			return task.m_sMarkerId;

		return DEFAULT_ICON;
	}

	//! Bind `tasks[]` from the held mission JSON.
	//! @return the rows, or null when there is no document or no non-empty `tasks[]`
	protected static array<ref TBD_TaskStruct> ReadWire()
	{
		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
			return null;

		TBD_TaskDocStruct doc = new TBD_TaskDocStruct();
		if (!ctx.ReadValue("", doc))
			return null;

		// NOT `if (doc.tasks)`. JsonLoadContext ALLOCATES a nested ref array even when the key is
		// absent. Presence is Count().
		if (!doc.tasks)
			return null;

		if (doc.tasks.Count() == 0)
			return null;

		return doc.tasks;
	}
}
