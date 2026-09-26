/**
 * @file TBD_AudioEmitter.c
 * @brief Server side of mission audio: reads the `audio` block, arms emitters, fires music cues.
 *
 * Role: builds the authored emitters and cues once per mission id, arms each emitter at LIVE (or
 * once its `triggerId` has fired) and fires cues on mission_start, task_succeeded, task_failed and
 * mission_end.  Position: `TBD_RuntimeHeartbeat` calls `Tick` every `TICK_MS` on the server and
 * `Clear` at world start; delivery goes to every player through the modded `SCR_PlayerController`.
 * State: static registry, owned by the server.  Invariants: each emitter is pushed once per
 * mission; mission_start and mission_end cue once each; a task cue fires only on a change of a
 * task's state after it was first seen; a dangling `triggerId` warns once and stays silent.
 */

//! The audio registry and its tick.
//! @authority server
class TBD_AudioEmitter
{
	static const string CH = "Audio"; //!< log channel
	static const float ABSENT = -1e6; //!< sentinel for an omitted emitter `y`
	static const int TICK_MS = 1000; //!< heartbeat period of `Tick` in milliseconds

	static const string EV_START = "mission_start"; //!< cue event on entering LIVE
	static const string EV_TASK_OK = "task_succeeded"; //!< cue event when a task succeeds
	static const string EV_TASK_FAIL = "task_failed"; //!< cue event when a task fails
	static const string EV_END = "mission_end"; //!< cue event on entering END

	protected static const string ANNOUNCE_IDLE_KEY = "Audio.idle"; //!< `TBD_AnnounceOnce` key of the no-audio line

	protected static ref array<ref TBD_AudioEmitterStruct> s_aEmitters; //!< valid authored emitters; null until built
	protected static ref array<ref TBD_MusicCueStruct> s_aCues; //!< valid authored cues; null until built
	protected static ref array<string> s_aArmedIds; //!< ids of emitters already pushed
	protected static ref array<string> s_aMissingTriggers; //!< ids of emitters whose dangling trigger was reported
	protected static ref array<string> s_aTaskSeen; //!< `taskId<TAB>state` rows of the last seen task states
	protected static bool s_bBuilt; //!< true once `Build` ran for the current mission
	protected static string s_sBuiltForMission; //!< mission id the registry was built for
	protected static bool s_bStartCued; //!< true once the mission_start cues fired
	protected static bool s_bEndCued; //!< true once the mission_end cues fired

	//! Drop the registry, the client's spawned sources and the cue flags. Runs at world start and
	//! when the loaded mission id changes.
	static void Clear()
	{
		TBD_AudioLocalSources.Clear();

		s_aEmitters = null;
		s_aCues = null;
		s_aArmedIds = null;
		s_aMissingTriggers = null;
		s_aTaskSeen = null;
		s_bBuilt = false;
		s_sBuiltForMission = string.Empty;
		TBD_AnnounceOnce.Rearm(ANNOUNCE_IDLE_KEY);
		s_bStartCued = false;
		s_bEndCued = false;
	}

	//! @return true once the registry is built for the current mission
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//! Read and validate the `audio` block once per mission. Rows without an id, a sound or track,
	//! or a positive radius are skipped with a warning.
	//! @return true when the registry is built (possibly empty); false while no mission id is held
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		string missionId = TBD_MissionLoader.GetMissionId();
		if (missionId.IsEmpty())
			return false;

		TBD_AudioBlockStruct block = ReadWire();
		s_aEmitters = new array<ref TBD_AudioEmitterStruct>();
		s_aCues = new array<ref TBD_MusicCueStruct>();
		s_aArmedIds = new array<string>();
		s_aMissingTriggers = new array<string>();
		s_aTaskSeen = new array<string>();
		s_bBuilt = true;
		s_sBuiltForMission = missionId;

		if (!block)
			return true;

		if (block.emitters)
		{
			foreach (int index, TBD_AudioEmitterStruct raw : block.emitters)
			{
				if (!raw)
				{
					TBD_Log.Warn(CH, string.Format("audio.emitters[%1] is null - skipped", index));
					continue;
				}
				if (raw.id.IsEmpty() || raw.sound.IsEmpty())
				{
					TBD_Log.Warn(CH, string.Format("audio.emitters[%1] missing id/sound - skipped", index));
					continue;
				}
				if (raw.radiusM <= 0)
				{
					TBD_Log.Warn(CH, string.Format("audio.emitters[%1] id='%2' radiusM=%3 - skipped (must be > 0)",
						index, raw.id, raw.radiusM));
					continue;
				}
				s_aEmitters.Insert(raw);
			}
		}

		if (block.musicCues)
		{
			foreach (int index, TBD_MusicCueStruct raw : block.musicCues)
			{
				if (!raw)
				{
					TBD_Log.Warn(CH, string.Format("audio.musicCues[%1] is null - skipped", index));
					continue;
				}
				if (raw.id.IsEmpty() || raw.track.IsEmpty() || raw.cueEvent.IsEmpty())
				{
					TBD_Log.Warn(CH, string.Format("audio.musicCues[%1] incomplete - skipped", index));
					continue;
				}
				s_aCues.Insert(raw);
			}
		}

		TBD_Log.Kv(CH, "built", string.Format("emitters=%1 cues=%2", s_aEmitters.Count(), s_aCues.Count()));
		return true;
	}

	//! One heartbeat: rebuild on a mission change, fire the stage cues, and during LIVE follow task
	//! states and arm emitters. Logs the idle line once when the mission authors no audio.
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

		int nEm = 0;
		int nCue = 0;
		if (s_aEmitters)
			nEm = s_aEmitters.Count();
		if (s_aCues)
			nCue = s_aCues.Count();
		if (nEm == 0 && nCue == 0)
		{
			TBD_AnnounceOnce.Kv(CH, ANNOUNCE_IDLE_KEY, "idle", "this mission authors no audio");
			return;
		}

		TBD_EGameStage stage = fm.GetStage();
		if (stage == TBD_EGameStage.LIVE && !s_bStartCued)
		{
			s_bStartCued = true;
			FireCues(EV_START);
		}
		if (stage == TBD_EGameStage.END && !s_bEndCued)
		{
			s_bEndCued = true;
			FireCues(EV_END);
		}

		if (stage == TBD_EGameStage.LIVE)
		{
			WatchTasks();
			ArmEmitters();
		}
	}

	//! Push every emitter that is not yet armed and whose trigger, when it names one, has fired.
	//! @authority server
	protected static void ArmEmitters()
	{
		if (!s_aEmitters)
			return;

		foreach (TBD_AudioEmitterStruct raw : s_aEmitters)
		{
			if (!raw)
				continue;
			if (IsArmed(raw.id))
				continue;
			if (!raw.triggerId.IsEmpty() && !IsTriggerFired(raw.id, raw.triggerId))
				continue;

			s_aArmedIds.Insert(raw.id);
			PushEmitter(raw);
			TBD_Log.Kv(CH, "armed", string.Format("id='%1' trigger='%2'", raw.id, raw.triggerId));
		}
	}

	//! @return true when the emitter with this id was already pushed
	protected static bool IsArmed(string id)
	{
		if (!s_aArmedIds)
			return false;
		foreach (string armed : s_aArmedIds)
		{
			if (armed == id)
				return true;
		}
		return false;
	}


	//! Compare every task's state with the last one seen and fire the task cues on a change to
	//! succeeded or failed. A task's first sighting fires nothing.
	//! @authority server
	protected static void WatchTasks()
	{
		array<ref TBD_Task> tasks = TBD_TaskStateMachine.GetAll();
		if (!tasks)
			return;

		foreach (TBD_Task task : tasks)
		{
			if (!task)
				continue;

			string now = TaskStateName(task.m_eState);
			string prev = SeenTaskState(task.m_sId);
			RememberTask(task.m_sId, now);
			if (prev.IsEmpty() || prev == now)
				continue;
			if (now == "succeeded")
				FireCues(EV_TASK_OK);
			else if (now == "failed")
				FireCues(EV_TASK_FAIL);
		}
	}

	//! @return `succeeded`, `failed` or `assigned` for a task state
	protected static string TaskStateName(TBD_ETaskState st)
	{
		if (st == TBD_ETaskState.SUCCEEDED)
			return "succeeded";
		if (st == TBD_ETaskState.FAILED)
			return "failed";
		return "assigned";
	}

	//! @return the last state recorded for the task, or empty when it was never seen
	protected static string SeenTaskState(string id)
	{
		if (!s_aTaskSeen)
			return string.Empty;
		string prefix = id + "\t";
		foreach (string row : s_aTaskSeen)
		{
			if (row.IndexOf(prefix) == 0)
				return row.Substring(prefix.Length(), row.Length() - prefix.Length());
		}
		return string.Empty;
	}

	//! Record the task's current state, replacing any earlier row for it.
	protected static void RememberTask(string id, string state)
	{
		if (!s_aTaskSeen)
			s_aTaskSeen = new array<string>();
		string prefix = id + "\t";
		for (int i = 0; i < s_aTaskSeen.Count(); i++)
		{
			if (s_aTaskSeen[i].IndexOf(prefix) == 0)
			{
				s_aTaskSeen[i] = prefix + state;
				return;
			}
		}
		s_aTaskSeen.Insert(prefix + state);
	}

	//! Push every cue bound to the event to all players and log how many fired.
	//! @param eventName one of the `EV_*` names
	//! @authority server
	protected static void FireCues(string eventName)
	{
		if (!s_aCues)
			return;

		int n = 0;
		foreach (TBD_MusicCueStruct cue : s_aCues)
		{
			if (!cue || cue.cueEvent != eventName)
				continue;
			PushCue(cue.track);
			n++;
		}
		if (n > 0)
			TBD_Log.Kv(CH, "cue", string.Format("event='%1' tracks=%2", eventName, n));
	}

	//! Hand the emitter to every player's controller and log how many received it.
	//! @authority server
	protected static void PushEmitter(notnull TBD_AudioEmitterStruct raw)
	{
		float y = raw.y;
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> ids = new array<int>();
		players.GetPlayers(ids);
		int sent = 0;
		foreach (int playerId : ids)
		{
			SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(playerId));
			if (!controller)
				continue;
			controller.TBD_PushAudioEmitter(raw.id, raw.x, y, raw.z, raw.radiusM, raw.loop, raw.sound);
			sent++;
		}
		TBD_Log.Kv(CH, "pushEmitter", string.Format("id='%1' players=%2 sent=%3", raw.id, ids.Count(), sent));
	}

	//! Hand the cue track to every player's controller.
	//! @authority server
	protected static void PushCue(string track)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> ids = new array<int>();
		players.GetPlayers(ids);
		foreach (int playerId : ids)
		{
			SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(playerId));
			if (!controller)
				continue;
			controller.TBD_PushAudioCue(track);
		}
	}

	//! Whether the emitter's trigger has fired; a trigger id the registry does not hold warns once
	//! per emitter and reads as not fired.
	//! @return true when the trigger is FIRED
	protected static bool IsTriggerFired(string emitterId, string triggerId)
	{
		bool unknownId;
		if (TBD_TriggerRuntime.HasFired(triggerId, unknownId))
			return true;

		if (!unknownId)
			return false;

		if (!s_aMissingTriggers)
			s_aMissingTriggers = new array<string>();
		if (s_aMissingTriggers.Find(emitterId) < 0)
		{
			s_aMissingTriggers.Insert(emitterId);
			TBD_Log.Warn(CH, string.Format("emitter '%1' triggerId '%2' is not a prepared trigger - stays silent",
				emitterId, triggerId));
		}
		return false;
	}

	//! Read the `audio` block from the held mission JSON. `event` is an Enforce keyword, so the
	//! key is renamed to `cueEvent` before the typed read.
	//! @return the block, or null when no document is held, it does not parse, or it authors no
	//! emitters and no cues
	protected static TBD_AudioBlockStruct ReadWire()
	{
		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome, "event", "cueEvent");
		if (!ctx)
			return null;

		TBD_AudioDocStruct doc = new TBD_AudioDocStruct();
		if (!ctx.ReadValue("", doc))
			return null;

		// JsonLoadContext allocates a nested ref even when its key is absent: presence is the
		// arrays' Count().
		if (!doc.audio)
			return null;
		int nEm = 0;
		int nCue = 0;
		if (doc.audio.emitters)
			nEm = doc.audio.emitters.Count();
		if (doc.audio.musicCues)
			nCue = doc.audio.musicCues.Count();
		if (nEm == 0 && nCue == 0)
			return null;

		return doc.audio;
	}
}

