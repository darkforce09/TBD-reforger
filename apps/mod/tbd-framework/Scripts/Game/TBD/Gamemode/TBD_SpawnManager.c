[ComponentEditorProps(category: "TBD/Framework", description: "Server-only: slot assignment + per-slot SCR_SpawnPoint entities from mission JSON.")]
class TBD_SpawnManagerClass : SCR_BaseGameModeComponentClass {}

//! Builds one SCR_SpawnPoint per mission slots[] entry at exact JSON coordinates.
//! Assigns each player a slot (roster identity → slotId, else round-robin).
//! @authority server — the whole manager runs server-side (slot build + assignment + deploy).
class TBD_SpawnManager : SCR_BaseGameModeComponent
{
	protected const ResourceName SPAWN_POINT_PREFAB = "{E7F4D5562F48DDE4}Prefabs/MP/Spawning/SpawnPoint_Base.et";

	//! Vertical offset (m) added to the resolved ground/JSON height so the character
	//! capsule sits feet-on-ground. Measured on a human character spawn in wb_play
	//! (T-092.1) — NOT guessed; measurement log in .ai/artifacts/t092_1_verify_log.md.
	protected const float CAPSULE_GROUND_OFFSET_M = 0.0;

	//! Warn threshold (m) between an explicit JSON y and the live terrain surface —
	//! larger deltas usually mean a stale DEM or a mis-authored slot. Start 2.0 (T-092.1).
	protected const float MAX_Y_DELTA_M = 2.0;

	protected static TBD_SpawnManager s_Instance;

	protected ref map<int, ref TBD_MissionSlotStruct> m_mPlayerSlot;
	protected ref map<string, SCR_SpawnPoint> m_mSlotSpawnPoints;
	protected int m_iRoundRobin;
	protected bool m_bSlotSpawnPointsBuilt;
	protected ref set<int> m_sDeployRequested;

	//------------------------------------------------------------------------------------------------
	void TBD_SpawnManager(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		s_Instance = this;
		m_mPlayerSlot = new map<int, ref TBD_MissionSlotStruct>();
		m_mSlotSpawnPoints = new map<string, SCR_SpawnPoint>();
		m_sDeployRequested = new set<int>();
	}

	//------------------------------------------------------------------------------------------------
	static TBD_SpawnManager GetInstance()
	{
		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	bool AreSlotSpawnPointsBuilt()
	{
		return m_bSlotSpawnPointsBuilt;
	}

	//------------------------------------------------------------------------------------------------
	//! Assign mission slot to player (roster or round-robin). Idempotent per player.
	void AssignSlotForPlayer(int playerId)
	{
		if (m_mPlayerSlot.Contains(playerId))
			return;

		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots || slots.IsEmpty())
		{
			Print("[TBD] SpawnManager: no mission slots — cannot assign player " + playerId, LogLevel.ERROR);
			return;
		}

		string slotId = ResolveSlotIdForPlayer(playerId);
		TBD_MissionSlotStruct slot = TBD_MissionLoader.GetSlotById(slotId);
		if (!slot)
		{
			// Round-robin fallback when roster slot id unknown
			int idx = m_iRoundRobin % slots.Count();
			slot = slots[idx];
			m_iRoundRobin++;
		}

		m_mPlayerSlot.Insert(playerId, slot);
		Print(string.Format("[TBD] SpawnManager: assigned slot %1 to player %2 at (%3)", slot.id, playerId, slot.x.ToString() + "," + slot.z.ToString()));
	}

	//------------------------------------------------------------------------------------------------
	protected string ResolveSlotIdForPlayer(int playerId)
	{
		if (!TBD_RosterLoader.IsLoaded())
			return string.Empty;

		string identityId = string.Format("%1", SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId));
		if (identityId.IsEmpty())
			return string.Empty;

		return TBD_RosterLoader.GetSlotForIdentity(identityId);
	}

	//------------------------------------------------------------------------------------------------
	TBD_MissionSlotStruct GetAssignedSlot(int playerId)
	{
		return m_mPlayerSlot.Get(playerId);
	}

	//------------------------------------------------------------------------------------------------
	SCR_SpawnPoint GetSpawnPointForSlot(string slotId)
	{
		return m_mSlotSpawnPoints.Get(slotId);
	}

	//------------------------------------------------------------------------------------------------
	//! Engine faction key for mission faction key.
	string EngineFactionKey(string missionFactionKey)
	{
		switch (missionFactionKey)
		{
			case "blufor": return "US";
			case "opfor": return "USSR";
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Authority-only: one SCR_SpawnPoint per mission slots[] at exact JSON coordinates.
	void BuildMissionSlotSpawnPoints()
	{
		if (m_bSlotSpawnPointsBuilt)
			return;

		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots || slots.IsEmpty())
		{
			Print("[TBD] SpawnManager: no mission slots — cannot build spawn points.", LogLevel.ERROR);
			return;
		}

		Resource resource = Resource.Load(SPAWN_POINT_PREFAB);
		if (!resource || !resource.IsValid())
		{
			Print("[TBD] SpawnManager: spawn point prefab failed to load.", LogLevel.ERROR);
			return;
		}

		int built = 0;
		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;

			string engineKey = EngineFactionKey(slot.faction);
			if (engineKey.IsEmpty())
				continue;

			float x = slot.x;
			float z = slot.z;

			// Spawn height policy (T-092.1): explicit JSON y wins, else live terrain
			// surface; both get the measured capsule offset on top.
			float surfaceY = GetGame().GetWorld().GetSurfaceY(x, z);
			float spawnY = surfaceY;
			float delta = 0;
			string jsonYLabel = "-";
			if (slot.HasJsonY())
			{
				spawnY = slot.y;
				delta = Math.AbsFloat(slot.y - surfaceY);
				jsonYLabel = slot.y.ToString();
				if (delta > MAX_Y_DELTA_M)
					Print(string.Format("[TBD][Spawn] slot=%1 jsonY=%2 deviates %3 m from surfaceY=%4 (> %5 m) — stale DEM or mis-authored slot?",
						slot.id, slot.y, delta, surfaceY, MAX_Y_DELTA_M), LogLevel.WARNING);
			}
			spawnY += CAPSULE_GROUND_OFFSET_M;

			vector pos = Vector(x, spawnY, z);

			EntitySpawnParams params = new EntitySpawnParams();
			params.TransformMode = ETransformMode.WORLD;
			Math3D.MatrixIdentity4(params.Transform);
			params.Transform[3] = pos;

			// Apply heading from JSON (yaw around Y)
			float yawRad = slot.headingDeg * Math.DEG2RAD;
			params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
			params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

			IEntity ent = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
			SCR_SpawnPoint sp = SCR_SpawnPoint.Cast(ent);
			if (!sp)
			{
				Print("[TBD] SpawnManager: failed to spawn SCR_SpawnPoint for " + slot.id, LogLevel.ERROR);
				continue;
			}

			sp.SetFactionKey(engineKey);
			m_mSlotSpawnPoints.Insert(slot.id, sp);
			built++;
			Print(string.Format("[TBD] SpawnManager: built slot spawn %1 (%2) kit %3 at %4", slot.id, engineKey, slot.kit, pos.ToString()));
			Print(string.Format("[TBD][Spawn] slot=%1 Y=%2 jsonY=%3 surfaceY=%4 delta=%5 heading=%6",
				slot.id, spawnY, jsonYLabel, surfaceY, delta, slot.headingDeg));
		}

		if (built > 0)
		{
			m_bSlotSpawnPointsBuilt = true;
			ScheduleDeployAllConnectedPlayers();
		}
	}

	//------------------------------------------------------------------------------------------------
	void OnStageChanged(TBD_EGameStage stage)
	{
		if (stage == TBD_EGameStage.LOBBY)
			ScheduleDeployAllConnectedPlayers();
	}

	//------------------------------------------------------------------------------------------------
	protected void ScheduleDeployAllConnectedPlayers()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!m_bSlotSpawnPointsBuilt)
			return;

		GetGame().GetCallqueue().CallLater(DeployAllConnectedPlayers, 250, false);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — deploys every connected player from the server.
	protected void DeployAllConnectedPlayers()
	{
		// Authority only — spawning happens on the server.
		if (RplSession.Mode() == RplMode.Client)
			return;

		array<int> players = {};
		int count = GetGame().GetPlayerManager().GetPlayers(players);
		for (int i = 0; i < count; i++)
			DeployPlayer(players[i]);
	}

	//------------------------------------------------------------------------------------------------
	//! Authority: assign slot + request spawn at mission JSON position with kit prefab.
	//! @authority server
	bool DeployPlayer(int playerId)
	{
		// Authority only — slot assignment + spawn run on the server.
		if (RplSession.Mode() == RplMode.Client)
			return false;

		if (!m_bSlotSpawnPointsBuilt)
			return false;

		if (m_sDeployRequested.Contains(playerId))
			return false;

		AssignSlotForPlayer(playerId);

		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (!slot)
			return false;

		SCR_SpawnPoint sp = GetSpawnPointForSlot(slot.id);
		if (!sp)
		{
			Print("[TBD] SpawnManager: no spawn point for slot " + slot.id, LogLevel.ERROR);
			return false;
		}

		bool kitOk;
		ResourceName prefab = TBD_Registry.Resolve(slot.kit, kitOk);
		if (!kitOk || prefab.IsEmpty())
		{
			Print("[TBD] SpawnManager: kit resolve failed: " + slot.kit, LogLevel.ERROR);
			return false;
		}

		PlayerController pc = GetGame().GetPlayerManager().GetPlayerController(playerId);
		if (pc)
		{
			SCR_PlayerFactionAffiliationComponent factionComp = SCR_PlayerFactionAffiliationComponent.Cast(
				pc.FindComponent(SCR_PlayerFactionAffiliationComponent));
			if (factionComp)
			{
				string engineKey = EngineFactionKey(slot.faction);
				factionComp.SetAffiliatedFactionByKey(engineKey);
			}
		}

		RplComponent rpl = RplComponent.Cast(sp.FindComponent(RplComponent));
		if (!rpl)
		{
			Print("[TBD] SpawnManager: spawn point missing RplComponent for " + slot.id, LogLevel.ERROR);
			return false;
		}

		SCR_RespawnComponent respawn = SCR_RespawnComponent.SGetPlayerRespawnComponent(playerId);
		if (!respawn)
		{
			Print("[TBD] SpawnManager: no respawn component for player " + playerId, LogLevel.ERROR);
			return false;
		}

		SCR_SpawnPointSpawnData data = new SCR_SpawnPointSpawnData(prefab, rpl.Id());
		if (!respawn.RequestSpawn(data))
		{
			Print("[TBD] SpawnManager: RequestSpawn failed for slot " + slot.id, LogLevel.ERROR);
			return false;
		}

		m_sDeployRequested.Insert(playerId);
		Print(string.Format("[TBD] SpawnManager: spawn requested player %1 slot %2 kit %3", playerId, slot.id, slot.kit));
		GetGame().GetCallqueue().CallLater(LogDeployedTransform, 3000, false, playerId);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Post-deploy diagnostic (T-092.1): logs the spawned character's actual feet height
	//! against the live terrain — groundDelta is the measured capsule/ground offset on a
	//! human character spawn, the calibration source for CAPSULE_GROUND_OFFSET_M.
	protected void LogDeployedTransform(int playerId)
	{
		IEntity ent = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (!ent)
		{
			Print(string.Format("[TBD][Spawn] deployed player=%1 — no controlled entity yet (spawn pending?)", playerId), LogLevel.WARNING);
			return;
		}

		vector org = ent.GetOrigin();
		float surfaceY = GetGame().GetWorld().GetSurfaceY(org[0], org[2]);
		float groundDelta = org[1] - surfaceY;
		float yaw = ent.GetYawPitchRoll()[0];

		string slotId = "-";
		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (slot)
			slotId = slot.id;

		Print(string.Format("[TBD][Spawn] deployed player=%1 slot=%2 pos=%3 feetY=%4 surfaceY=%5 groundDelta=%6 yaw=%7",
			playerId, slotId, org.ToString(), org[1], surfaceY, groundDelta, yaw));
	}
}
