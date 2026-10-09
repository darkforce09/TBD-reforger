/**
 * @file TBD_DynamicSpawner.c
 * @brief Runs the authored `spawnModules[]`: waves restock, garrisons spawn once and hold.
 *
 * Role: prepares the modules of the loaded mission and ticks them during LIVE.  Position: ticked
 * and cleared by TBD_RuntimeHeartbeat every TICK_MS; each module's volley logic lives in
 * TBD_DynamicSpawnVolley.
 * State: the prepared modules with the groups they own, the mission they were built for, the LIVE
 * clock latch and the cleanup flag (static, server only).
 * Invariants: a mission change rebuilds; groups spawn only during LIVE; END or DEBRIEF deletes
 * every spawned group once; a module names x and z or a zone, never both; no module exceeds
 * MAX_ALIVE living groups.
 */

//! One prepared module plus the groups it currently owns.
class TBD_SpawnModuleRuntime
{
	string m_sId; //!< module id
	string m_sKind; //!< JSON kind: restocking or garrison
	string m_sFactionKey; //!< mission faction key
	string m_sGroupTemplate; //!< group prefab
	string m_sZoneId; //!< zone to spawn at; empty with a position
	string m_sTriggerId; //!< trigger that gates spawning; empty for none
	float m_fX; //!< spawn x (m) with a position
	float m_fZ; //!< spawn z (m) with a position
	bool m_bHasPosition; //!< true when x and z are authored
	bool m_bHasZone; //!< true when a zone is authored
	int m_iCount; //!< groups per volley, capped at MAX_ALIVE
	int m_iMaxAlive; //!< living group cap, at most MAX_ALIVE
	float m_fIntervalS; //!< interval (s) between restocking volleys
	bool m_bHasInterval; //!< true when a positive interval is authored
	bool m_bGarrisonDone; //!< true once a garrison spawned
	bool m_bMissingTriggerLogged; //!< latch for the missing-trigger warning
	float m_fLastSpawnMs; //!< world time (ms) of the last volley; 0 before the first
	ref array<SCR_AIGroup> m_aGroups; //!< groups this module spawned that may still live
}

//! Reads `spawnModules` and spawns AI groups on the authority.
class TBD_DynamicSpawner
{
	static const string CH = "Spawn"; //!< log channel
	static const string ANNOUNCE_IDLE_KEY = "Spawn.idle"; //!< TBD_AnnounceOnce key of the once-per-mission idle line

	static const float ABSENT = -1e6; //!< sentinel of an omitted numeric key
	static const int MAX_ALIVE = 32; //!< hard cap on groups per volley and living groups per module
	static const int TICK_MS = 1000; //!< heartbeat period (ms)

	protected static ref array<ref TBD_SpawnModuleRuntime> s_aModules; //!< prepared modules; null until built
	protected static bool s_bBuilt; //!< true once built for the current mission
	protected static string s_sBuiltForMission; //!< mission id the modules were built for
	protected static bool s_bLiveClockLatched; //!< true while LIVE continues
	protected static float s_fLiveStartMs; //!< world time (ms) LIVE was first seen
	protected static bool s_bCleaned; //!< true once the END cleanup ran

	//! Delete every spawned group and forget the modules.
	static void Clear()
	{
		DeleteSpawned();
		s_aModules = null;
		s_bBuilt = false;
		s_sBuiltForMission = string.Empty;
		TBD_AnnounceOnce.Rearm(ANNOUNCE_IDLE_KEY);
		s_bLiveClockLatched = false;
		s_fLiveStartMs = 0;
		s_bCleaned = false;
	}

	//! True once the modules are built for the current mission.
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//! Prepare the modules of the loaded mission once; invalid rows are warned about and skipped.
	//! @return false while no mission is loaded
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		string missionId = TBD_MissionLoader.GetMissionId();
		if (missionId.IsEmpty())
			return false;

		array<ref TBD_SpawnModuleStruct> raw = ReadWire();
		s_aModules = new array<ref TBD_SpawnModuleRuntime>();
		s_bBuilt = true;
		s_sBuiltForMission = missionId;
		s_bCleaned = false;

		if (!raw)
			return true;

		foreach (int index, TBD_SpawnModuleStruct row : raw)
		{
			if (!row)
			{
				TBD_Log.Warn(CH, string.Format("spawnModules[%1] is null - skipped", index));
				continue;
			}

			TBD_SpawnModuleRuntime prepared = Prepare(row, index);
			if (prepared)
				s_aModules.Insert(prepared);
		}

		TBD_Log.Kv(CH, "built", string.Format("modules=%1", s_aModules.Count()));
		return true;
	}

	//! One heartbeat: rebuild on a mission change, clean up at END or DEBRIEF, tick every module
	//! during LIVE.
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

		if (!s_aModules || s_aModules.Count() == 0)
		{
			TBD_AnnounceOnce.Kv(CH, ANNOUNCE_IDLE_KEY, "idle", "this mission authors no spawnModules");
			return;
		}

		TBD_EGameStage stage = fm.GetStage();
		if (stage == TBD_EGameStage.END || stage == TBD_EGameStage.DEBRIEF)
		{
			if (!s_bCleaned)
			{
				DeleteSpawned();
				s_bCleaned = true;
				TBD_Log.Kv(CH, "cleanup", "mission end - spawned groups deleted");
			}
			s_bLiveClockLatched = false;
			return;
		}

		if (stage != TBD_EGameStage.LIVE)
		{
			s_bLiveClockLatched = false;
			return;
		}

		float now = GetGame().GetWorld().GetWorldTime();
		if (!s_bLiveClockLatched)
		{
			s_bLiveClockLatched = true;
			s_fLiveStartMs = now;
		}

		foreach (int index, TBD_SpawnModuleRuntime module : s_aModules)
		{
			if (!module)
				continue;
			TBD_DynamicSpawnVolley.TickModule(module, index, now);
		}
	}

	//! Validate one row into a prepared module.
	//! @return the module, or null (warned) for a row without id, kind, template, count or an
	//! exclusive position
	protected static TBD_SpawnModuleRuntime Prepare(notnull TBD_SpawnModuleStruct raw, int index)
	{
		if (raw.id.IsEmpty())
		{
			TBD_Log.Warn(CH, string.Format("spawnModules[%1] has no id - skipped", index));
			return null;
		}
		if (raw.kind != "wave" && raw.kind != "garrison")
		{
			TBD_Log.Warn(CH, string.Format("spawnModules[%1] kind '%2' is not wave|garrison - skipped", index, raw.kind));
			return null;
		}
		if (raw.groupTemplate.IsEmpty())
		{
			TBD_Log.Warn(CH, string.Format("spawnModules[%1] '%2' has no groupTemplate - skipped", index, raw.id));
			return null;
		}
		if (raw.count <= 0)
		{
			TBD_Log.Warn(CH, string.Format("spawnModules[%1] '%2' count=%3 - skipped", index, raw.id, raw.count));
			return null;
		}

		bool hasPos = (raw.x != ABSENT && raw.z != ABSENT);
		bool hasZone = !raw.zoneId.IsEmpty();
		if (hasPos == hasZone)
		{
			TBD_Log.Warn(CH, string.Format("spawnModules[%1] '%2' must name x+z XOR zoneId - skipped", index, raw.id));
			return null;
		}

		TBD_SpawnModuleRuntime module = new TBD_SpawnModuleRuntime();
		module.m_sId = raw.id;
		module.m_sKind = raw.kind;
		module.m_sFactionKey = raw.factionKey;
		module.m_sGroupTemplate = raw.groupTemplate;
		module.m_sZoneId = raw.zoneId;
		module.m_sTriggerId = raw.triggerId;
		module.m_fX = raw.x;
		module.m_fZ = raw.z;
		module.m_bHasPosition = hasPos;
		module.m_bHasZone = hasZone;
		module.m_iCount = raw.count;
		if (module.m_iCount > MAX_ALIVE)
			module.m_iCount = MAX_ALIVE;

		int cap = raw.maxAlive;
		if (cap <= 0)
			cap = module.m_iCount;
		if (cap > MAX_ALIVE)
			cap = MAX_ALIVE;
		module.m_iMaxAlive = cap;

		if (raw.intervalSeconds != ABSENT && raw.intervalSeconds > 0)
		{
			module.m_bHasInterval = true;
			module.m_fIntervalS = raw.intervalSeconds;
		}

		module.m_aGroups = new array<SCR_AIGroup>();
		return module;
	}

	//! Delete every group the modules spawned and reset their spawn state.
	protected static void DeleteSpawned()
	{
		if (!s_aModules)
			return;
		foreach (TBD_SpawnModuleRuntime module : s_aModules)
		{
			if (!module || !module.m_aGroups)
				continue;
			foreach (SCR_AIGroup group : module.m_aGroups)
			{
				if (group)
					SCR_EntityHelper.DeleteEntityAndChildren(group);
			}
			module.m_aGroups.Clear();
			module.m_bGarrisonDone = false;
			module.m_fLastSpawnMs = 0;
		}
	}

	//! The `spawnModules` rows of the held mission JSON, or null when none are authored or the
	//! document does not read.
	protected static array<ref TBD_SpawnModuleStruct> ReadWire()
	{
		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
			return null;

		TBD_SpawnModulesDocStruct doc = new TBD_SpawnModulesDocStruct();
		if (!ctx.ReadValue("", doc))
			return null;

		if (!doc.spawnModules)
			return null;
		if (doc.spawnModules.Count() == 0)
			return null;

		return doc.spawnModules;
	}
}
