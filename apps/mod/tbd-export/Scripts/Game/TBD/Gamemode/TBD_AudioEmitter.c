//! T-936.5 - positional audio emitters and music cues.
//!
//! Server reads `audio` from the raw mission JSON, arms each emitter (LIVE, or when its
//! triggerId FIRED), and fires music cues on mission_start / task_succeeded / task_failed /
//! mission_end. The server has no audio device, so playback is pushed to each client's
//! SCR_PlayerController (same transport as T-676 play_sound). Each client spawns one
//! TBD_AudioSourceEntity at the emitter origin and only raises the sound while the local
//! player is inside radiusM. loop repeats inside the radius; a one-shot fires once on first
//! enter.
//!
//! JsonLoadContext ALLOCATES nested refs when the key is absent. Presence is
//! emitters.Count() / musicCues.Count(), NOT `if (doc.audio)`.

//------------------------------------------------------------------------------------------------
class TBD_AudioEmitterStruct
{
	string id;
	float x;
	float z;
	float y;
	string sound;
	float radiusM;
	bool loop;
	string triggerId;

	void TBD_AudioEmitterStruct()
	{
		y = TBD_AudioEmitter.ABSENT;
		radiusM = 0;
		loop = false;
	}
}

//------------------------------------------------------------------------------------------------
class TBD_MusicCueStruct
{
	string id;
	string cueEvent;
	string track;
}

//------------------------------------------------------------------------------------------------
class TBD_AudioBlockStruct
{
	ref array<ref TBD_AudioEmitterStruct> emitters;
	ref array<ref TBD_MusicCueStruct> musicCues;
}

//------------------------------------------------------------------------------------------------
//! The document root for the audio pass: declares `audio` and nothing else.
class TBD_AudioDocStruct
{
	ref TBD_AudioBlockStruct audio;
}

//------------------------------------------------------------------------------------------------
[EntityEditorProps(category: "TBD/Gamemode", description: "TBD positional audio source")]
class TBD_AudioSourceEntityClass : GenericEntityClass
{
};

//! Local client sound source. Spawned by typename (no prefab). Radius is a listener gate:
//! SCR_UISoundEntity is 2D, so being inside radiusM is what makes the emitter positional.
class TBD_AudioSourceEntity : GenericEntity
{
	static const int LOOP_MS = 4000;

	string m_sId;
	string m_sSound;
	float m_fRadiusM;
	bool m_bLoop;
	bool m_bOneShotPlayed;
	float m_fNextPlayMs;

	//------------------------------------------------------------------------------------------------
	void TBD_AudioSourceEntity(IEntitySource src, IEntity parent)
	{
		SetEventMask(EntityEvent.FRAME);
		SetFlags(EntityFlags.ACTIVE, true);
	}

	//------------------------------------------------------------------------------------------------
	void Configure(string id, string sound, float radiusM, bool loop)
	{
		m_sId = id;
		m_sSound = sound;
		m_fRadiusM = radiusM;
		m_bLoop = loop;
		m_bOneShotPlayed = false;
		m_fNextPlayMs = 0;
	}

	//------------------------------------------------------------------------------------------------
	override protected void EOnFrame(IEntity owner, float timeSlice)
	{
		if (m_sSound.IsEmpty() || m_fRadiusM <= 0)
			return;

		IEntity listener = ListenerEntity();
		if (!listener)
			return;

		float dist = vector.Distance(listener.GetOrigin(), GetOrigin());
		bool inRange = dist <= m_fRadiusM;
		if (!inRange)
			return;

		float now = GetGame().GetWorld().GetWorldTime();
		if (m_bLoop)
		{
			if (now >= m_fNextPlayMs)
			{
				SCR_UISoundEntity.SoundEvent(m_sSound);
				m_fNextPlayMs = now + LOOP_MS;
			}
			return;
		}

		if (m_bOneShotPlayed)
			return;

		SCR_UISoundEntity.SoundEvent(m_sSound);
		m_bOneShotPlayed = true;
	}

	//------------------------------------------------------------------------------------------------
	protected IEntity ListenerEntity()
	{
		PlayerController pc = GetGame().GetPlayerController();
		if (!pc)
			return null;
		return pc.GetControlledEntity();
	}
}

//------------------------------------------------------------------------------------------------
//! Reads `audio`, arms emitters, fires cues. Server-authoritative; clients play.
class TBD_AudioEmitter
{
	static const string CH = "Audio";
	static const float ABSENT = -1e6;
	static const int TICK_MS = 1000;

	static const string EV_START = "mission_start";
	static const string EV_TASK_OK = "task_succeeded";
	static const string EV_TASK_FAIL = "task_failed";
	static const string EV_END = "mission_end";

	protected static ref array<ref TBD_AudioEmitterStruct> s_aEmitters;
	protected static ref array<ref TBD_MusicCueStruct> s_aCues;
	protected static ref array<string> s_aArmedIds;
	protected static ref array<string> s_aMissingTriggers;
	protected static ref array<string> s_aTaskSeen;
	protected static ref array<TBD_AudioSourceEntity> s_aLocalSources;
	protected static bool s_bBuilt;
	protected static string s_sBuiltForMission;
	protected static bool s_bAnnounced;
	protected static bool s_bStartCued;
	protected static bool s_bEndCued;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		if (s_aLocalSources)
		{
			foreach (TBD_AudioSourceEntity src : s_aLocalSources)
			{
				if (src)
					SCR_EntityHelper.DeleteEntityAndChildren(src);
			}
		}

		s_aEmitters = null;
		s_aCues = null;
		s_aArmedIds = null;
		s_aMissingTriggers = null;
		s_aTaskSeen = null;
		s_aLocalSources = null;
		s_bBuilt = false;
		s_sBuiltForMission = string.Empty;
		s_bAnnounced = false;
		s_bStartCued = false;
		s_bEndCued = false;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//------------------------------------------------------------------------------------------------
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		string missionId = CurrentMissionId();
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

		int nEm = 0;
		int nCue = 0;
		if (s_aEmitters)
			nEm = s_aEmitters.Count();
		if (s_aCues)
			nCue = s_aCues.Count();
		if (nEm == 0 && nCue == 0)
		{
			AnnounceEmptyOnce();
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

	//------------------------------------------------------------------------------------------------
	static void SpawnLocalSource(string id, float x, float y, float z, float radiusM, bool loop, string sound)
	{
		if (id.IsEmpty() || sound.IsEmpty())
			return;

		if (!s_aLocalSources)
			s_aLocalSources = new array<TBD_AudioSourceEntity>();

		foreach (TBD_AudioSourceEntity existing : s_aLocalSources)
		{
			if (existing && existing.m_sId == id)
				return;
		}

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return;

		float spawnY = y;
		if (y == ABSENT)
			spawnY = world.GetSurfaceY(x, z);

		vector pos = Vector(x, spawnY, z);
		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = pos;

		IEntity spawned = GetGame().SpawnEntity(TBD_AudioSourceEntity, world, params);
		TBD_AudioSourceEntity src = TBD_AudioSourceEntity.Cast(spawned);
		if (!src)
		{
			TBD_Log.Warn(CH, string.Format("could not spawn source id='%1'", id));
			if (spawned)
				SCR_EntityHelper.DeleteEntityAndChildren(spawned);
			return;
		}

		src.Configure(id, sound, radiusM, loop);
		s_aLocalSources.Insert(src);
		TBD_Log.Kv(CH, "source", string.Format("id='%1' radiusM=%2 loop=%3", id, radiusM, loop));
	}

	//------------------------------------------------------------------------------------------------
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
			if (!raw.triggerId.IsEmpty() && !TriggerHasFired(raw.id, raw.triggerId))
				continue;

			s_aArmedIds.Insert(raw.id);
			PushEmitter(raw);
			TBD_Log.Kv(CH, "armed", string.Format("id='%1' trigger='%2'", raw.id, raw.triggerId));
		}
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static bool TriggerHasFired(string emitterId, string triggerId)
	{
		array<ref TBD_Trigger> all = TBD_TriggerRuntime.GetAll();
		if (!all)
			return false;

		foreach (TBD_Trigger t : all)
		{
			if (!t)
				continue;
			if (t.m_sId != triggerId)
				continue;
			return t.m_eState == TBD_ETriggerState.FIRED;
		}

		if (!s_aMissingTriggers)
			s_aMissingTriggers = new array<string>();
		bool seen = false;
		foreach (string miss : s_aMissingTriggers)
		{
			if (miss == emitterId)
			{
				seen = true;
				break;
			}
		}
		if (!seen)
		{
			s_aMissingTriggers.Insert(emitterId);
			TBD_Log.Warn(CH, string.Format("emitter '%1' triggerId '%2' is not a prepared trigger - stays silent",
				emitterId, triggerId));
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static string TaskStateName(TBD_ETaskState st)
	{
		if (st == TBD_ETaskState.SUCCEEDED)
			return "succeeded";
		if (st == TBD_ETaskState.FAILED)
			return "failed";
		return "assigned";
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;
		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_AudioBlockStruct ReadWire()
	{
		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return null;

		// `event` is an Enforce keyword (`proto event void`), so the struct field is
		// `cueEvent`. Rewrite the JSON key before JsonLoadContext binds by member name.
		// Format copies: Replace mutates in place and must not touch MissionLoader's cache.
		string rewritten = string.Format("%1", raw);
		rewritten.Replace("\"event\":", "\"cueEvent\":");

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(rewritten))
			return null;

		TBD_AudioDocStruct doc = new TBD_AudioDocStruct();
		if (!ctx.ReadValue("", doc))
			return null;

		// NOT `if (doc.audio)`. JsonLoadContext ALLOCATES a nested ref even when the
		// key is absent. Presence is Count() on the arrays.
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

	//------------------------------------------------------------------------------------------------
	protected static void AnnounceEmptyOnce()
	{
		if (s_bAnnounced)
			return;
		s_bAnnounced = true;
		TBD_Log.Kv(CH, "idle", "this mission authors no audio");
	}
}

//------------------------------------------------------------------------------------------------
//! Eighth modded SCR_PlayerController block (mission browser, briefing, lobby, markers, radio,
//! spectator host, triggers, now audio). Overrides no vanilla method; every symbol is TBD_-prefixed.
modded class SCR_PlayerController
{
	//------------------------------------------------------------------------------------------------
	//! @authority server - start one positional emitter on the addressed client.
	void TBD_PushAudioEmitter(string id, float x, float y, float z, float radiusM, bool loop, string sound)
	{
		if (id.IsEmpty() || sound.IsEmpty())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			TBD_AudioEmitter.SpawnLocalSource(id, x, y, z, radiusM, loop, sound);
			return;
		}

		Rpc(TBD_RpcDo_AudioEmitter, id, x, y, z, radiusM, loop, sound);
	}

	//------------------------------------------------------------------------------------------------
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_AudioEmitter(string id, float x, float y, float z, float radiusM, bool loop, string sound)
	{
		TBD_AudioEmitter.SpawnLocalSource(id, x, y, z, radiusM, loop, sound);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server - 2D music cue on the addressed client.
	void TBD_PushAudioCue(string track)
	{
		if (track.IsEmpty())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			SCR_UISoundEntity.SoundEvent(track);
			return;
		}

		Rpc(TBD_RpcDo_AudioCue, track);
	}

	//------------------------------------------------------------------------------------------------
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_AudioCue(string track)
	{
		SCR_UISoundEntity.SoundEvent(track);
	}
}

//------------------------------------------------------------------------------------------------
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_AudioTickArmed;

	//------------------------------------------------------------------------------------------------
	//! @authority server - the mission document lives here. Clients only spawn local sources
	//! via the owner RPC.
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_AudioEmitter.Clear();

		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		if (m_bTBD_AudioTickArmed)
			return;

		m_bTBD_AudioTickArmed = true;
		GetGame().GetCallqueue().CallLater(TBD_AudioTick, TBD_AudioEmitter.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	void TBD_AudioTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_AudioEmitter.Tick();
		GetGame().GetCallqueue().CallLater(TBD_AudioTick, TBD_AudioEmitter.TICK_MS, false);
	}
}
