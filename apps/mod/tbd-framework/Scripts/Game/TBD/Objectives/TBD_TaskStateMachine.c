//! T-936.2 - the server-authoritative task state machine.
//!
//! The Enfusion half of `mission.schema.json#/properties/tasks`. Tasks OBSERVE T-676 trigger
//! completion (`TBD_TriggerRuntime`); they never fire `winConditions.endOn` and they never
//! duplicate T-212 objective logic.
//!
//! Transitions: assigned -> succeeded, assigned -> failed. Anything else is logged and ignored.
//! A linked trigger that FIRED succeeds the task. A linked trigger that is INERT, or an id that
//! names no prepared trigger once the runtime is built, fails the task. A task with no triggerId
//! stays assigned until something calls TryTransition.
//!
//! Clients hold no mission document, so state is pushed through TBD_TaskHud's player-controller
//! RPC (the existing marker icon path draws only assigned tasks).
//! @contract mission.schema.json#/$defs/task

//------------------------------------------------------------------------------------------------
//! One `tasks[]` entry on the wire. Field names must equal the JSON keys - JsonLoadContext maps
//! by name. Empty strings are the absent-key sentinel (schema minLength 1, so they cannot be
//! authored).
class TBD_TaskStruct
{
	string id;
	string title;
	string tier;         //!< primary | secondary | optional
	string state;        //!< assigned | succeeded | failed
	string triggerId;
	string markerId;
	string description;
}

//------------------------------------------------------------------------------------------------
//! The document root for the TASK pass: declares `tasks` and nothing else.
class TBD_TaskDocStruct
{
	ref array<ref TBD_TaskStruct> tasks;
}

//------------------------------------------------------------------------------------------------
enum TBD_ETaskState
{
	ASSIGNED,
	SUCCEEDED,
	FAILED
}

//------------------------------------------------------------------------------------------------
enum TBD_ETaskTier
{
	PRIMARY,
	SECONDARY,
	OPTIONAL
}

//------------------------------------------------------------------------------------------------
//! One prepared task. Server-owned; clients see a snapshot, not this object.
class TBD_Task
{
	string m_sId;
	string m_sTitle;
	TBD_ETaskTier m_eTier;
	TBD_ETaskState m_eState;
	string m_sTriggerId;
	string m_sMarkerId;
	string m_sDescription;
	int m_iWorldX;
	int m_iWorldZ;
	bool m_bHasPosition;
}

//------------------------------------------------------------------------------------------------
//! Reads `tasks[]`, applies the transition table, and asks TBD_TaskHud to replicate assigned
//! markers to clients.
class TBD_TaskStateMachine
{
	static const string CH = "Task";

	static const string STATE_ASSIGNED  = "assigned";
	static const string STATE_SUCCEEDED = "succeeded";
	static const string STATE_FAILED    = "failed";

	static const string TIER_PRIMARY   = "primary";
	static const string TIER_SECONDARY = "secondary";
	static const string TIER_OPTIONAL  = "optional";

	static const string DEFAULT_ICON = "objective_marker";

	static const int TICK_MS = 1000;

	protected static ref array<ref TBD_Task> s_aTasks;
	protected static bool s_bBuilt;
	protected static string s_sBuiltForMission;
	protected static bool s_bDirty;
	protected static bool s_bAnnounced;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		s_aTasks = null;
		s_bBuilt = false;
		s_sBuiltForMission = string.Empty;
		s_bDirty = false;
		s_bAnnounced = false;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//------------------------------------------------------------------------------------------------
	static array<ref TBD_Task> GetAll()
	{
		return s_aTasks;
	}

	//------------------------------------------------------------------------------------------------
	//! Parse. Returns false while there is no document yet so the heartbeat keeps waiting.
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		string missionId = CurrentMissionId();
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

	//------------------------------------------------------------------------------------------------
	static void Tick()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		string liveId = CurrentMissionId();
		if (s_bBuilt && !s_sBuiltForMission.IsEmpty() && liveId != s_sBuiltForMission)
			Clear();

		if (!Build())
			return;

		if (!s_aTasks || s_aTasks.Count() == 0)
		{
			AnnounceEmptyOnce();
			return;
		}

		SyncFromTriggers();
		ResolvePositions();

		if (s_bDirty)
		{
			s_bDirty = false;
			TBD_TaskHud.PushToPlayers();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! The ONE mutation. Illegal pairs are logged and ignored.
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

		TBD_ETaskState from = task.m_eState;
		task.m_eState = to;
		s_bDirty = true;
		TBD_Log.Kv(CH, "transition", string.Format(
			"id=%1 from=%2 to=%3",
			task.m_sId,
			StateName(from),
			StateName(to)));
		return true;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static void SyncFromTriggers()
	{
		if (!TBD_TriggerRuntime.IsBuilt())
			return;

		foreach (TBD_Task task : s_aTasks)
		{
			if (!task)
				continue;

			if (task.m_eState != TBD_ETaskState.ASSIGNED)
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

	//------------------------------------------------------------------------------------------------
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

	protected static int RoundToInt(float value)
	{
		if (value >= 0)
			return value + 0.5;

		return value - 0.5;
	}

	//------------------------------------------------------------------------------------------------
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

			task.m_iWorldX = RoundToInt(x);
			task.m_iWorldZ = RoundToInt(z);
			task.m_bHasPosition = true;
		}
	}

	//------------------------------------------------------------------------------------------------
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
		return task;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_ETaskTier ParseTier(string raw)
	{
		if (raw == TIER_SECONDARY)
			return TBD_ETaskTier.SECONDARY;

		if (raw == TIER_OPTIONAL)
			return TBD_ETaskTier.OPTIONAL;

		return TBD_ETaskTier.PRIMARY;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_ETaskState ParseState(string raw)
	{
		if (raw == STATE_SUCCEEDED)
			return TBD_ETaskState.SUCCEEDED;

		if (raw == STATE_FAILED)
			return TBD_ETaskState.FAILED;

		return TBD_ETaskState.ASSIGNED;
	}

	//------------------------------------------------------------------------------------------------
	static string StateName(TBD_ETaskState state)
	{
		if (state == TBD_ETaskState.SUCCEEDED)
			return STATE_SUCCEEDED;

		if (state == TBD_ETaskState.FAILED)
			return STATE_FAILED;

		return STATE_ASSIGNED;
	}

	//------------------------------------------------------------------------------------------------
	static string IconFor(notnull TBD_Task task)
	{
		if (!task.m_sMarkerId.IsEmpty())
			return task.m_sMarkerId;

		return DEFAULT_ICON;
	}

	//------------------------------------------------------------------------------------------------
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;

		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	protected static array<ref TBD_TaskStruct> ReadWire()
	{
		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return null;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
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

	//------------------------------------------------------------------------------------------------
	protected static void AnnounceEmptyOnce()
	{
		if (s_bAnnounced)
			return;

		s_bAnnounced = true;
		TBD_Log.Kv(CH, "idle", "this mission authors no tasks");
	}
}

//------------------------------------------------------------------------------------------------
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_TaskTickArmed;

	//------------------------------------------------------------------------------------------------
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_TaskStateMachine.Clear();
		TBD_TaskHud.Clear();

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		if (m_bTBD_TaskTickArmed)
			return;

		m_bTBD_TaskTickArmed = true;

		if (RplSession.Mode() == RplMode.Client)
		{
			GetGame().GetCallqueue().CallLater(TBD_TaskHudTick, TBD_TaskStateMachine.TICK_MS, false);
			return;
		}

		GetGame().GetCallqueue().CallLater(TBD_TaskTick, TBD_TaskStateMachine.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	void TBD_TaskTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_TaskStateMachine.Tick();
		GetGame().GetCallqueue().CallLater(TBD_TaskTick, TBD_TaskStateMachine.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	void TBD_TaskHudTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_TaskHud.RequestLocal();
		GetGame().GetCallqueue().CallLater(TBD_TaskHudTick, TBD_TaskStateMachine.TICK_MS, false);
	}
}
