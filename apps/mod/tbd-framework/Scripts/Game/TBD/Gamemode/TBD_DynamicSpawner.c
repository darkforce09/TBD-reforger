//! T-936.6 - run authored `spawnModules[]`: waves restock, garrisons spawn once and hold.
//!
//! Server-only. Groups are world entities; clients see replication. JsonLoadContext ALLOCATES
//! the `spawnModules` array when the key is absent. Presence is `Count()`, not
//! `if (doc.spawnModules)`.
//!
//! Wave: spawn `count` groups at interval (or after triggerId FIRED) while living groups <
//! maxAlive (omitted maxAlive => count; hard cap 32). Garrison: spawn once, do not restock.
//! Cleanup deletes every spawned group when the round reaches END.

//------------------------------------------------------------------------------------------------
class TBD_SpawnModuleStruct
{
	string id;
	string kind;
	string factionKey;
	string groupTemplate;
	float x;
	float z;
	string zoneId;
	int count;
	float intervalSeconds;
	int maxAlive;
	string triggerId;

	void TBD_SpawnModuleStruct()
	{
		x = TBD_DynamicSpawner.ABSENT;
		z = TBD_DynamicSpawner.ABSENT;
		intervalSeconds = TBD_DynamicSpawner.ABSENT;
		maxAlive = 0;
		count = 0;
	}
}

//------------------------------------------------------------------------------------------------
//! The document root for the spawn-modules pass: declares `spawnModules` and nothing else.
class TBD_SpawnModulesDocStruct
{
	ref array<ref TBD_SpawnModuleStruct> spawnModules;
}

//------------------------------------------------------------------------------------------------
//! One prepared module plus the groups it currently owns.
class TBD_SpawnModuleRuntime
{
	string m_sId;
	string m_sKind;
	string m_sFactionKey;
	string m_sGroupTemplate;
	string m_sZoneId;
	string m_sTriggerId;
	float m_fX;
	float m_fZ;
	bool m_bHasPosition;
	bool m_bHasZone;
	int m_iCount;
	int m_iMaxAlive;
	float m_fIntervalS;
	bool m_bHasInterval;
	bool m_bGarrisonDone;
	bool m_bMissingTriggerLogged;
	float m_fLastSpawnMs;
	ref array<SCR_AIGroup> m_aGroups;
}

//------------------------------------------------------------------------------------------------
//! Reads `spawnModules` and spawns AI groups on the authority.
class TBD_DynamicSpawner
{
	static const string CH = "Spawn";

	static const float ABSENT = -1e6;
	static const int MAX_ALIVE = 32;
	static const int TICK_MS = 1000;

	protected static ref array<ref TBD_SpawnModuleRuntime> s_aModules;
	protected static bool s_bBuilt;
	protected static string s_sBuiltForMission;
	protected static bool s_bAnnounced;
	protected static bool s_bLiveClockLatched;
	protected static float s_fLiveStartMs;
	protected static bool s_bCleaned;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		DeleteSpawned();
		s_aModules = null;
		s_bBuilt = false;
		s_sBuiltForMission = string.Empty;
		s_bAnnounced = false;
		s_bLiveClockLatched = false;
		s_fLiveStartMs = 0;
		s_bCleaned = false;
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

		if (!s_aModules || s_aModules.Count() == 0)
		{
			AnnounceEmptyOnce();
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
			TickModule(module, index, now);
		}
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static void TickModule(notnull TBD_SpawnModuleRuntime module, int index, float nowMs)
	{
		PruneDead(module);

		if (!module.m_sTriggerId.IsEmpty() && !TriggerHasFired(module))
			return;

		if (module.m_sKind == "garrison")
		{
			if (module.m_bGarrisonDone)
				return;
			SpawnVolley(module, index);
			module.m_bGarrisonDone = true;
			module.m_fLastSpawnMs = nowMs;
			return;
		}

		int living = LivingCount(module);
		if (living >= module.m_iMaxAlive)
			return;

		if (module.m_fLastSpawnMs > 0 && module.m_bHasInterval)
		{
			float elapsedS = (nowMs - module.m_fLastSpawnMs) / 1000.0;
			if (elapsedS < module.m_fIntervalS)
				return;
		}
		else if (module.m_fLastSpawnMs > 0 && !module.m_bHasInterval)
		{
			return;
		}

		SpawnVolley(module, index);
		module.m_fLastSpawnMs = nowMs;
	}

	//------------------------------------------------------------------------------------------------
	protected static void SpawnVolley(notnull TBD_SpawnModuleRuntime module, int index)
	{
		vector origin;
		if (!ResolveOrigin(module, origin))
			return;

		int living = LivingCount(module);
		int want = module.m_iCount;
		int room = module.m_iMaxAlive - living;
		if (room < want)
			want = room;
		if (want <= 0)
			return;

		for (int i = 0; i < want; i++)
		{
			SCR_AIGroup group = SpawnGroup(module, origin);
			if (!group)
				continue;
			module.m_aGroups.Insert(group);
		}

		TBD_Log.Kv(CH, "spawn", string.Format("id='%1' kind='%2' spawned=%3 alive=%4 max=%5",
			module.m_sId, module.m_sKind, want, LivingCount(module), module.m_iMaxAlive));
	}

	//------------------------------------------------------------------------------------------------
	protected static bool ResolveOrigin(notnull TBD_SpawnModuleRuntime module, out vector origin)
	{
		float x;
		float z;
		if (module.m_bHasZone)
		{
			TBD_Zone zone = FindZone(module.m_sZoneId);
			if (!zone)
			{
				TBD_Log.Warn(CH, string.Format("module '%1' zoneId '%2' is not a usable zone - skipped",
					module.m_sId, module.m_sZoneId));
				return false;
			}
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
		}
		else
		{
			x = module.m_fX;
			z = module.m_fZ;
		}

		float y = GetGame().GetWorld().GetSurfaceY(x, z);
		origin = Vector(x, y, z);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_Zone FindZone(string zoneId)
	{
		if (zoneId.IsEmpty())
			return null;

		array<ref TBD_Zone> zones = TBD_ZoneRegistry.GetAll();
		if (!zones)
			return null;

		foreach (TBD_Zone zone : zones)
		{
			if (!zone || !zone.IsUsable())
				continue;
			if (zone.m_sId == zoneId)
				return zone;
		}
		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected static SCR_AIGroup SpawnGroup(notnull TBD_SpawnModuleRuntime module, vector origin)
	{
		ResourceName prefab = module.m_sGroupTemplate;
		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			TBD_Log.Warn(CH, string.Format("module '%1' groupTemplate failed to load", module.m_sId));
			return null;
		}

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = origin;

		IEntity ent = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		SCR_AIGroup group = SCR_AIGroup.Cast(ent);
		if (!group)
		{
			if (ent)
				SCR_EntityHelper.DeleteEntityAndChildren(ent);
			TBD_Log.Warn(CH, string.Format("module '%1' prefab is not an SCR_AIGroup", module.m_sId));
			return null;
		}

		ApplyFaction(group, module.m_sFactionKey);
		return group;
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyFaction(notnull SCR_AIGroup group, string factionKey)
	{
		string engineKey;
		switch (factionKey)
		{
			case "blufor": engineKey = "US"; break;
			case "opfor": engineKey = "USSR"; break;
			case "indfor": engineKey = "FIA"; break;
			case "civ": engineKey = "CIV"; break;
		}
		if (engineKey.IsEmpty())
			return;

		SCR_FactionManager fm = SCR_FactionManager.Cast(GetGame().GetFactionManager());
		if (!fm)
			return;
		Faction faction = fm.GetFactionByKey(engineKey);
		if (faction)
			group.SetFaction(faction);
	}

	//------------------------------------------------------------------------------------------------
	protected static int LivingCount(notnull TBD_SpawnModuleRuntime module)
	{
		if (!module.m_aGroups)
			return 0;
		int n = 0;
		foreach (SCR_AIGroup group : module.m_aGroups)
		{
			if (GroupIsLiving(group))
				n++;
		}
		return n;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool GroupIsLiving(SCR_AIGroup group)
	{
		if (!group)
			return false;
		array<AIAgent> agents = {};
		group.GetAgents(agents);
		if (!agents)
			return false;
		foreach (AIAgent agent : agents)
		{
			if (!agent)
				continue;
			IEntity body = agent.GetControlledEntity();
			if (body)
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected static void PruneDead(notnull TBD_SpawnModuleRuntime module)
	{
		if (!module.m_aGroups)
			return;
		for (int i = module.m_aGroups.Count() - 1; i >= 0; i--)
		{
			if (!GroupIsLiving(module.m_aGroups[i]))
				module.m_aGroups.Remove(i);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static bool TriggerHasFired(notnull TBD_SpawnModuleRuntime module)
	{
		array<ref TBD_Trigger> all = TBD_TriggerRuntime.GetAll();
		if (!all)
			return false;

		foreach (TBD_Trigger t : all)
		{
			if (!t)
				continue;
			if (t.m_sId != module.m_sTriggerId)
				continue;
			return t.m_eState == TBD_ETriggerState.FIRED;
		}

		if (!module.m_bMissingTriggerLogged)
		{
			module.m_bMissingTriggerLogged = true;
			TBD_Log.Warn(CH, string.Format("module '%1' triggerId '%2' is not a prepared trigger - stays idle",
				module.m_sId, module.m_sTriggerId));
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;
		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	protected static array<ref TBD_SpawnModuleStruct> ReadWire()
	{
		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return null;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
			return null;

		TBD_SpawnModulesDocStruct doc = new TBD_SpawnModulesDocStruct();
		if (!ctx.ReadValue("", doc))
			return null;

		// NOT `if (doc.spawnModules)`. JsonLoadContext ALLOCATES a nested ref even when the
		// key is absent. Presence is Count().
		if (!doc.spawnModules)
			return null;
		if (doc.spawnModules.Count() == 0)
			return null;

		return doc.spawnModules;
	}

	//------------------------------------------------------------------------------------------------
	protected static void AnnounceEmptyOnce()
	{
		if (s_bAnnounced)
			return;
		s_bAnnounced = true;
		TBD_Log.Kv(CH, "idle", "this mission authors no spawnModules");
	}
}

//------------------------------------------------------------------------------------------------
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_SpawnTickArmed;

	//------------------------------------------------------------------------------------------------
	//! @authority server - AI groups spawn here. Clients follow replication.
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_DynamicSpawner.Clear();

		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		if (m_bTBD_SpawnTickArmed)
			return;

		m_bTBD_SpawnTickArmed = true;
		GetGame().GetCallqueue().CallLater(TBD_SpawnTick, TBD_DynamicSpawner.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	void TBD_SpawnTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_DynamicSpawner.Tick();
		GetGame().GetCallqueue().CallLater(TBD_SpawnTick, TBD_DynamicSpawner.TICK_MS, false);
	}
}
