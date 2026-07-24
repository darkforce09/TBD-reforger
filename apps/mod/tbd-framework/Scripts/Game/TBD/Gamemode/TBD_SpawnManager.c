//! Result of a deploy attempt (spawn-authority contract, determinism slice A1).
//! Only NOT_MINE may reach the vanilla spawn path — everything else means the
//! framework owns this player and vanilla must stand down.
enum TBD_EDeployResult
{
	DEPLOYED,  //!< Spawn requested this call.
	ALREADY,   //!< A spawn request is already in flight / completed for this player.
	RETRY,     //!< Transient precondition (slots/roster/spawn-point) — retry shortly.
	FAILED,    //!< Permanent failure (kit/Rpl/RequestSpawn) — logged ERROR, no vanilla body.
	NOT_MINE,  //!< Client side or no framework mission — vanilla may handle it.
}

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

	//! A1 — the LOBBY auto-deploy wave (PIE/dev convenience: deploy everyone on stage
	//! entry without the deploy menu). The T-068.13 slot picker will default this off;
	//! the pull path (SCR_MenuSpawnLogic → DeployPlayerEx) is the production entry.
	[Attribute("1", desc: "Auto-deploy all connected players on LOBBY (PIE/dev wave; slot picker turns this off).")]
	protected bool m_bAutoDeploy;

	protected ref map<int, ref TBD_MissionSlotStruct> m_mPlayerSlot;
	protected ref map<string, SCR_SpawnPoint> m_mSlotSpawnPoints;
	protected int m_iRoundRobin;
	protected bool m_bSlotSpawnPointsBuilt;
	protected ref set<int> m_sDeployRequested;
	//! A1 — pull-path retry bookkeeping (transient RETRY results; cap = 20 × 500 ms).
	protected ref map<int, int> m_mRetryCount;
	//! A1 — watchdog: players whose requested spawn has been observed to materialize.
	protected ref set<int> m_sSpawnSeen;
	//! A2 — per-spawn equip idempotency: last entity each player was equipped on.
	protected ref map<int, EntityID> m_mLastEquippedEntity;
	//! A2 — last body delivered per player. The vanilla spawn pipeline (measured:
	//! run4 vs run2 of the determinism gate) can fire OnPlayerSpawned TWICE for one
	//! RequestSpawn with DIFFERENT bodies ~1 s apart — the abandoned first body is
	//! the operator's "kitted AI next to me" + ground litter. Superseded LIVE bodies
	//! are reaped; dead ones stay (corpses are gameplay).
	protected ref map<int, IEntity> m_mLastBody;
	//! A6 — identityId → slot key, so a reconnect reclaims the same slot (dedicated
	//! servers reuse numeric playerIds; identity is the durable key).
	protected ref map<string, string> m_mIdentityReclaim;
	//! A7 — settle-census debounce + counter.
	protected bool m_bCensusScheduled;
	protected int m_iCensusCount;
	//! T-068.12 — strong refs to in-flight loadout applications (CallLater holds none);
	//! pruned of completed apps whenever a new one starts.
	protected ref array<ref TBD_LoadoutApplication> m_aLoadoutApps = {};

	//------------------------------------------------------------------------------------------------
	void TBD_SpawnManager(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		s_Instance = this;
		m_mPlayerSlot = new map<int, ref TBD_MissionSlotStruct>();
		m_mSlotSpawnPoints = new map<string, SCR_SpawnPoint>();
		m_sDeployRequested = new set<int>();
		m_mRetryCount = new map<int, int>();
		m_sSpawnSeen = new set<int>();
		m_mLastEquippedEntity = new map<int, EntityID>();
		m_mLastBody = new map<int, IEntity>();
		m_mIdentityReclaim = new map<string, string>();
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

		// A6 — reconnect reclaim beats roster/round-robin: same identity → same slot.
		TBD_MissionSlotStruct slot;
		string identityId = string.Format("%1", SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId));
		if (!identityId.IsEmpty())
		{
			string reclaimId;
			if (m_mIdentityReclaim.Find(identityId, reclaimId))
				slot = TBD_MissionLoader.GetSlotById(reclaimId);
		}

		if (!slot)
		{
			string slotId = ResolveSlotIdForPlayer(playerId);
			slot = TBD_MissionLoader.GetSlotById(slotId);
		}
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
			m_mSlotSpawnPoints.Insert(slot.Key(), sp); // B1 — durable key (uid-else-id)
			built++;
			Print(string.Format("[TBD] SpawnManager: built slot spawn %1 (%2) kit %3 at %4", slot.id, engineKey, slot.kit, pos.ToString()));
			Print(string.Format("[TBD][Spawn] slot=%1 Y=%2 jsonY=%3 surfaceY=%4 delta=%5 heading=%6",
				slot.id, spawnY, jsonYLabel, surfaceY, delta, slot.headingDeg));
		}

		if (built > 0)
		{
			// A1: no deploy wave here — spawn points existing is not a deploy trigger.
			// The single push wave fires on LOBBY (roster settled by then, A5), and the
			// pull path handles everyone else.
			m_bSlotSpawnPointsBuilt = true;
		}
	}

	//------------------------------------------------------------------------------------------------
	void OnStageChanged(TBD_EGameStage stage)
	{
		if (stage == TBD_EGameStage.LOBBY && m_bAutoDeploy)
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
		{
			TBD_EDeployResult r = DeployPlayerEx(players[i]);
			Print(string.Format("[TBD][Spawn] path=push player=%1 result=%2", players[i], typename.EnumToString(TBD_EDeployResult, r)));
			if (r == TBD_EDeployResult.RETRY)
				ScheduleDeployRetry(players[i]);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Authority: assign slot + request spawn at mission JSON position with kit prefab.
	//! @authority server — back-compat bool wrapper over DeployPlayerEx; true only when
	//! a spawn was requested by THIS call.
	bool DeployPlayer(int playerId)
	{
		return DeployPlayerEx(playerId) == TBD_EDeployResult.DEPLOYED;
	}

	//------------------------------------------------------------------------------------------------
	//! Authority: assign slot + request spawn at mission JSON position with kit prefab.
	//! Tri-state spawn-authority contract (A1): NOT_MINE is the only result that may
	//! reach vanilla spawn; ALREADY/FAILED mean "vanilla stands down"; RETRY = transient.
	//! @authority server
	TBD_EDeployResult DeployPlayerEx(int playerId)
	{
		// Authority only — slot assignment + spawn run on the server.
		if (RplSession.Mode() == RplMode.Client)
			return TBD_EDeployResult.NOT_MINE;

		// No valid framework mission → vanilla owns spawning entirely.
		if (!TBD_MissionLoader.IsLoaded() || !TBD_MissionLoader.IsValid())
			return TBD_EDeployResult.NOT_MINE;

		if (!m_bSlotSpawnPointsBuilt || !TBD_RosterLoader.IsLoaded())
			return TBD_EDeployResult.RETRY;

		if (m_sDeployRequested.Contains(playerId))
			return TBD_EDeployResult.ALREADY;

		AssignSlotForPlayer(playerId);

		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (!slot)
			return TBD_EDeployResult.RETRY;

		SCR_SpawnPoint sp = GetSpawnPointForSlot(slot.Key());
		if (!sp)
		{
			Print("[TBD] SpawnManager: no spawn point for slot " + slot.id, LogLevel.ERROR);
			return TBD_EDeployResult.RETRY;
		}

		bool kitOk;
		ResourceName prefab = TBD_Registry.Resolve(slot.kit, kitOk);
		if (!kitOk || prefab.IsEmpty())
		{
			Print("[TBD] SpawnManager: kit resolve failed: " + slot.kit, LogLevel.ERROR);
			return TBD_EDeployResult.FAILED;
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
			return TBD_EDeployResult.FAILED;
		}

		SCR_RespawnComponent respawn = SCR_RespawnComponent.SGetPlayerRespawnComponent(playerId);
		if (!respawn)
		{
			Print("[TBD] SpawnManager: no respawn component for player " + playerId, LogLevel.ERROR);
			return TBD_EDeployResult.RETRY;
		}

		SCR_SpawnPointSpawnData data = new SCR_SpawnPointSpawnData(prefab, rpl.Id());
		if (!respawn.RequestSpawn(data))
		{
			Print("[TBD] SpawnManager: RequestSpawn failed for slot " + slot.id, LogLevel.ERROR);
			return false;
		}

		m_sDeployRequested.Insert(playerId);
		Print(string.Format("[TBD] SpawnManager: spawn requested player %1 slot %2 kit %3", playerId, slot.id, slot.kit));
		m_mRetryCount.Remove(playerId);
		m_sSpawnSeen.Remove(playerId);
		// A2: the transform log + Arsenal equip now fire from OnPlayerSpawned with the
		// ACTUAL spawned entity — no fixed timers, no GetPlayerControlledEntity polling.
		// A1 watchdog: if the requested spawn never materializes, re-arm so the next
		// pull attempt can deploy instead of wedging on ALREADY forever.
		GetGame().GetCallqueue().CallLater(CheckSpawnArrived, 10000, false, playerId);
		return TBD_EDeployResult.DEPLOYED;
	}

	//------------------------------------------------------------------------------------------------
	//! A2 — subscribe the spawn invoker (SCR_BaseGameModeComponent has no
	//! OnPlayerSpawned virtual in 1.7 — measured compile error; the vanilla
	//! SCR_BaseGameMode ScriptInvoker is the supported seam).
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(owner);
		if (gm)
			gm.GetOnPlayerSpawned().Insert(OnPlayerSpawnedHook);
	}

	//------------------------------------------------------------------------------------------------
	override void OnDelete(IEntity owner)
	{
		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(owner);
		if (gm)
			gm.GetOnPlayerSpawned().Remove(OnPlayerSpawnedHook);

		super.OnDelete(owner);
	}

	//------------------------------------------------------------------------------------------------
	//! A2 — the deterministic equip trigger: fires with the entity the player actually
	//! received (every spawn, including respawns — the operator-locked re-equip rule).
	//! @authority server
	protected void OnPlayerSpawnedHook(int playerId, IEntity controlledEntity)
	{
		if (RplSession.Mode() == RplMode.Client || !controlledEntity)
			return;

		m_sSpawnSeen.Insert(playerId);

		// A2 — reap a superseded LIVE body (vanilla double-spawn): the player just got
		// a NEW body while the previous one is alive and abandoned. Dead bodies stay
		// (death → respawn keeps the corpse).
		IEntity prev;
		if (m_mLastBody.Find(playerId, prev) && prev && prev != controlledEntity)
		{
			bool dead = false;
			ChimeraCharacter prevChar = ChimeraCharacter.Cast(prev);
			if (prevChar)
			{
				CharacterControllerComponent ccc = prevChar.GetCharacterController();
				dead = ccc && ccc.IsDead();
			}
			if (!dead)
			{
				// Abort any in-flight equip on the doomed body BEFORE deleting it —
				// its verify would otherwise fire against a dead handle (measured
				// in determinism-gate run 1).
				foreach (TBD_LoadoutApplication app : m_aLoadoutApps)
				{
					if (!app.IsDone() && app.GetCharacter() == prev)
						app.Cancel("body superseded");
				}
				Print(string.Format("[TBD][Spawn] reaping superseded body player=%1 (vanilla double-spawn)", playerId), LogLevel.WARNING);
				SCR_EntityHelper.DeleteEntityAndChildren(prev);
			}
		}
		m_mLastBody.Set(playerId, controlledEntity);

		GetGame().GetCallqueue().CallLater(LogDeployedTransform, 500, false, playerId);
		ScheduleCensus();

		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (!slot || !slot.loadout)
			return; // kit-only slot — Phase-1 semantics, deliberate silent skip

		// Per-spawn idempotency: equip each body exactly once (respawn = new entity id).
		EntityID entId = controlledEntity.GetID();
		EntityID last;
		if (m_mLastEquippedEntity.Find(playerId, last) && last == entId)
			return;
		m_mLastEquippedEntity.Set(playerId, entId);

		// Prune completed applications before starting a new one (strong-ref hygiene).
		for (int i = m_aLoadoutApps.Count() - 1; i >= 0; i--)
		{
			if (m_aLoadoutApps[i].IsDone())
				m_aLoadoutApps.Remove(i);
		}

		Print(string.Format("[TBD][Loadout][Player] applying loadout player=%1 slot=%2", playerId, slot.id));
		TBD_LoadoutApplication app = new TBD_LoadoutApplication(controlledEntity, slot.loadout, "[TBD][Loadout][Player]", slot.id);
		m_aLoadoutApps.Insert(app);
		app.Run();
	}

	//------------------------------------------------------------------------------------------------
	//! A6 — death re-arms the deploy guard; the slot assignment survives, so the
	//! vanilla respawn flow (pull path) redeploys the SAME slot and the spawn hook
	//! re-equips the new body. (1.7 component virtual takes SCR_InstigatorContextData
	//! — the CRF Rally precedent.)
	//! @authority server
	override void OnPlayerKilled(notnull SCR_InstigatorContextData instigatorContextData)
	{
		super.OnPlayerKilled(instigatorContextData);

		if (RplSession.Mode() == RplMode.Client)
			return;

		int playerId = instigatorContextData.GetVictimPlayerID();
		if (playerId <= 0)
			return;

		m_sDeployRequested.Remove(playerId);
		m_mRetryCount.Remove(playerId);
		m_sSpawnSeen.Remove(playerId);
		Print(string.Format("[TBD][Spawn] player=%1 killed — re-armed for respawn (slot retained)", playerId));
	}

	//------------------------------------------------------------------------------------------------
	//! A6 — disconnect clears all per-player state; the identity → slot pairing is
	//! remembered so a reconnecting player (dedicated servers reuse numeric playerIds)
	//! reclaims the same slot ahead of roster/round-robin.
	//! @authority server
	override void OnPlayerDisconnected(int playerId, KickCauseCode cause, int timeout)
	{
		super.OnPlayerDisconnected(playerId, cause, timeout);

		if (RplSession.Mode() == RplMode.Client)
			return;

		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (slot)
		{
			string identityId = string.Format("%1", SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId));
			if (!identityId.IsEmpty())
				m_mIdentityReclaim.Set(identityId, slot.id);
		}

		m_mPlayerSlot.Remove(playerId);
		m_sDeployRequested.Remove(playerId);
		m_mRetryCount.Remove(playerId);
		m_sSpawnSeen.Remove(playerId);
		m_mLastEquippedEntity.Remove(playerId);
		m_mLastBody.Remove(playerId);
		Print(string.Format("[TBD][Spawn] player=%1 disconnected — state cleared, slot reclaim recorded", playerId));
	}

	//------------------------------------------------------------------------------------------------
	//! A7 — settle census (~5 s after the first spawn of a wave): the orphan-body
	//! oracle. characters != players means a duplicate/abandoned body exists.
	protected void ScheduleCensus()
	{
		if (m_bCensusScheduled)
			return;
		m_bCensusScheduled = true;
		GetGame().GetCallqueue().CallLater(RunCensus, 5000, false);
	}

	//------------------------------------------------------------------------------------------------
	protected void RunCensus()
	{
		m_iCensusCount = 0;
		BaseWorld world = GetGame().GetWorld();
		if (world)
			world.QueryEntitiesByAABB(Vector(-1000, -2000, -1000), Vector(20000, 4000, 20000), CensusAddEntity);

		array<int> players = {};
		int playerCount = GetGame().GetPlayerManager().GetPlayers(players);
		Print(string.Format("[TBD][Audit] characters=%1 players=%2", m_iCensusCount, playerCount));
		m_bCensusScheduled = false;
	}

	//------------------------------------------------------------------------------------------------
	protected bool CensusAddEntity(IEntity ent)
	{
		if (ChimeraCharacter.Cast(ent))
			m_iCensusCount++;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! A1 — pull-path retry for transient RETRY results (500 ms cadence, cap 20 = 10 s;
	//! cap-hit logs ERROR and stops — the vanilla wait screen keeps the player parked).
	void ScheduleDeployRetry(int playerId)
	{
		GetGame().GetCallqueue().CallLater(RetryDeploy, 500, false, playerId);
	}

	//------------------------------------------------------------------------------------------------
	protected void RetryDeploy(int playerId)
	{
		int n;
		m_mRetryCount.Find(playerId, n);
		if (n >= 20)
		{
			Print(string.Format("[TBD][Spawn] path=retry player=%1 gave up after %2 attempts", playerId, n), LogLevel.ERROR);
			m_mRetryCount.Remove(playerId);
			return;
		}
		m_mRetryCount.Set(playerId, n + 1);

		TBD_EDeployResult r = DeployPlayerEx(playerId);
		Print(string.Format("[TBD][Spawn] path=retry player=%1 attempt=%2 result=%3", playerId, n + 1, typename.EnumToString(TBD_EDeployResult, r)));
		if (r == TBD_EDeployResult.RETRY)
			ScheduleDeployRetry(playerId);
		else
			m_mRetryCount.Remove(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! A1 watchdog — a DEPLOYED request whose spawn never arrived re-arms the player.
	//! Spawn-seen is marked by the transform log today (A2 moves it to OnPlayerSpawned).
	protected void CheckSpawnArrived(int playerId)
	{
		if (m_sSpawnSeen.Contains(playerId))
			return;
		if (GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId))
			return;

		Print(string.Format("[TBD][Spawn] watchdog player=%1 — spawn request never materialized, re-arming", playerId), LogLevel.WARNING);
		m_sDeployRequested.Remove(playerId);
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
		m_sSpawnSeen.Insert(playerId);

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
