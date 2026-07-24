//! Result of a deploy attempt (spawn-authority contract, determinism slice A1).
//! Only NOT_MINE may reach the vanilla spawn path — everything else means the
//! framework owns this player and vanilla must stand down.
enum TBD_EDeployResult
{
	DEPLOYED,  //!< Bound to the slot body this call.
	ALREADY,   //!< This player is already bound.
	RETRY,     //!< Transient precondition (bodies/roster/controller) — retry shortly.
	FAILED,    //!< Permanent failure (kit resolve / body spawn) — logged ERROR, no vanilla body.
	NOT_MINE,  //!< Client side or no framework mission — vanilla may handle it.
}

[ComponentEditorProps(category: "TBD/Framework", description: "Server-only: slot-body materialization + claim/bind deploy from mission JSON.")]
class TBD_SpawnManagerClass : SCR_BaseGameModeComponentClass {}

//! Slot-body materialization (operator-approved synthesis of CRF + PlayableSelector):
//! at mission load, one numbered slot BODY per compiled slots[] entry is spawned at
//! the exact JSON transform (kit prefab, AI disabled, Arsenal loadout applied) and
//! stands in the world through the lobby. Deploy = claim + hand the player onto the
//! pre-materialized body through vanilla's POSSESS spawn request: it takes over an
//! entity that already exists, so it never creates the second body that the
//! body-creating spawn requests did (the measured double-spawn class), while still
//! running the vanilla finalize the client needs to leave the loading screen.
//! @authority server — the whole manager runs server-side.
class TBD_SpawnManager : SCR_BaseGameModeComponent
{

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

	//! Pause between death and the automatic redeploy. Vanilla's deploy menu used to be
	//! what put a killed player back in the world; with it stood down (see
	//! TBD_SCR_RespawnSystemComponent) the framework owns that too, and the delay is the
	//! respawn beat — long enough for the kill to read as a death, not a teleport.
	[Attribute("5000", desc: "Delay (ms) between death and automatic redeploy (auto-deploy worlds only).")]
	protected int m_iRedeployDelayMs;

	protected ref map<int, ref TBD_MissionSlotStruct> m_mPlayerSlot;
	//! Slot key (uid-else-id) → the materialized slot body standing in the world.
	protected ref map<string, IEntity> m_mSlotBodies;
	protected int m_iRoundRobin;
	protected bool m_bSlotBodiesMaterialized;
	protected ref map<int, bool> m_mDeployRequested;
	//! A1 — pull-path retry bookkeeping (transient RETRY results; cap = 20 × 500 ms).
	protected ref map<int, int> m_mRetryCount;
	//! A1 — watchdog: players whose requested spawn has been observed to materialize.
	protected ref map<int, bool> m_mSpawnSeen;
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
		m_mSlotBodies = new map<string, IEntity>();
		m_mDeployRequested = new map<int, bool>();
		m_mRetryCount = new map<int, int>();
		m_mSpawnSeen = new map<int, bool>();
		m_mIdentityReclaim = new map<string, string>();
	}

	//------------------------------------------------------------------------------------------------
	static TBD_SpawnManager GetInstance()
	{
		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	bool AreSlotBodiesMaterialized()
	{
		return m_bSlotBodiesMaterialized;
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
	//! The materialized body standing on a slot (null when never materialized).
	IEntity GetSlotBody(string slotKey)
	{
		return m_mSlotBodies.Get(slotKey);
	}

	//------------------------------------------------------------------------------------------------
	//! PS-shaped server claim guard (backend for the T-068.13 picker): a slot can be
	//! claimed when unclaimed, already ours, or its previous claimant disconnected.
	//! Rejected when a DIFFERENT live player holds it. Round-robin/roster auto-claim
	//! goes through AssignSlotForPlayer as before.
	bool ClaimSlot(int playerId, string slotKey)
	{
		TBD_MissionSlotStruct slot = TBD_MissionLoader.GetSlotById(slotKey);
		if (!slot)
			return false;

		foreach (int otherId, TBD_MissionSlotStruct assigned : m_mPlayerSlot)
		{
			if (!assigned || assigned.Key() != slot.Key() || otherId == playerId)
				continue;
			// Slot held by another CONNECTED player → reject (first-come guard).
			if (GetGame().GetPlayerManager().GetPlayerController(otherId))
			{
				Print(string.Format("[TBD][Spawn] claim rejected player=%1 slot=%2 (held by player %3)", playerId, slot.Key(), otherId));
				return false;
			}
		}

		m_mPlayerSlot.Set(playerId, slot);
		Print(string.Format("[TBD][Spawn] claim player=%1 slot=%2", playerId, slot.Key()));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Engine faction key a materialized body was built with (kit prefab affiliation) —
	//! the fallback when a mission faction key has no mapping above.
	protected string BodyFactionKey(IEntity body)
	{
		if (!body)
			return string.Empty;

		FactionAffiliationComponent affiliation = FactionAffiliationComponent.Cast(
			body.FindComponent(FactionAffiliationComponent));
		if (!affiliation)
			return string.Empty;

		Faction faction = affiliation.GetDefaultAffiliatedFaction();
		if (!faction)
			return string.Empty;

		return faction.GetFactionKey();
	}

	//------------------------------------------------------------------------------------------------
	//! Engine faction key for mission faction key.
	string EngineFactionKey(string missionFactionKey)
	{
		switch (missionFactionKey)
		{
			case "blufor": return "US";
			case "opfor": return "USSR";
			case "indfor": return "FIA";
			case "civ": return "CIV";
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Authority-only: materialize one slot BODY per mission slots[] entry at the exact
	//! JSON transform — kit prefab, AI disabled (CRF pattern), Arsenal loadout applied.
	//! The numbered lineup stands in the world through the lobby; deploy binds onto it.
	void MaterializeSlotBodies()
	{
		if (m_bSlotBodiesMaterialized)
			return;

		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots || slots.IsEmpty())
		{
			Print("[TBD] SpawnManager: no mission slots — cannot materialize bodies.", LogLevel.ERROR);
			return;
		}

		int built = 0;
		int loadouts = 0;
		int number = 0;
		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;
			number++;

			IEntity body = SpawnSlotBody(slot, number);
			if (!body)
				continue;

			m_mSlotBodies.Set(slot.Key(), body);
			built++;
			if (slot.loadout)
				loadouts++;
		}

		if (built > 0)
			m_bSlotBodiesMaterialized = true;

		Print(string.Format("[TBD][Slots] materialized %1 bodies (%2 loadouts applied)", built, loadouts));
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn one slot body at the slot's JSON transform: kit prefab → AI off →
	//! Arsenal loadout (when authored). Also the respawn path (fresh body per life —
	//! operator-locked). Returns null on kit/prefab failure (logged ERROR).
	protected IEntity SpawnSlotBody(TBD_MissionSlotStruct slot, int number)
	{
		bool kitOk;
		ResourceName prefab = TBD_Registry.Resolve(slot.kit, kitOk);
		if (!kitOk || prefab.IsEmpty())
		{
			Print("[TBD] SpawnManager: kit resolve failed: " + slot.kit, LogLevel.ERROR);
			return null;
		}

		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			Print("[TBD] SpawnManager: kit prefab failed to load: " + prefab, LogLevel.ERROR);
			return null;
		}

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

		IEntity body = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (!body)
		{
			Print("[TBD] SpawnManager: failed to spawn slot body for " + slot.id, LogLevel.ERROR);
			return null;
		}

		// CRF pattern: deactivate once + next-frame re-check. No repeating hammer —
		// created-at-load bodies don't fight the PS parked-AI reactivation bug.
		DisableBodyAI(body);

		Print(string.Format("[TBD][Slots] Slot-%1 %2 (%3) kit %4 at %5",
			number, slot.Key(), slot.id, slot.kit, pos.ToString()));
		Print(string.Format("[TBD][Spawn] slot=%1 Y=%2 jsonY=%3 surfaceY=%4 delta=%5 heading=%6",
			slot.id, spawnY, jsonYLabel, surfaceY, delta, slot.headingDeg));

		if (slot.loadout)
		{
			PruneDoneLoadoutApps();
			TBD_LoadoutApplication app = new TBD_LoadoutApplication(body, slot.loadout, "[TBD][Loadout][Slot]", slot.id);
			m_aLoadoutApps.Insert(app);
			app.Run();
		}

		return body;
	}

	//------------------------------------------------------------------------------------------------
	//! CRF_PlayerCharacter.DisableAI port: deactivate the agent + one next-frame re-check.
	protected void DisableBodyAI(IEntity body)
	{
		AIControlComponent aiComponent = AIControlComponent.Cast(body.FindComponent(AIControlComponent));
		if (!aiComponent)
			return;

		AIAgent agent = aiComponent.GetAIAgent();
		if (agent)
			agent.DeactivateAI();

		GetGame().GetCallqueue().Call(DisableBodyAIRecheck, aiComponent);
	}

	//------------------------------------------------------------------------------------------------
	protected void DisableBodyAIRecheck(AIControlComponent aiComponent)
	{
		if (!aiComponent)
			return;
		AIAgent agent = aiComponent.GetAIAgent();
		if (agent)
			agent.DeactivateAI();
	}

	//------------------------------------------------------------------------------------------------
	protected void PruneDoneLoadoutApps()
	{
		for (int i = m_aLoadoutApps.Count() - 1; i >= 0; i--)
		{
			if (m_aLoadoutApps[i].IsDone())
				m_aLoadoutApps.Remove(i);
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

		if (!m_bSlotBodiesMaterialized)
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
	//! @authority server — back-compat bool wrapper over DeployPlayerEx; true only when
	//! a bind happened in THIS call.
	bool DeployPlayer(int playerId)
	{
		return DeployPlayerEx(playerId) == TBD_EDeployResult.DEPLOYED;
	}

	//------------------------------------------------------------------------------------------------
	//! Authority: claim the player's slot and BIND them onto its pre-materialized body
	//! via SCR_PlayerController.SetInitialMainEntity — the CRF/PlayableSelector-proven
	//! takeover; the vanilla RequestSpawn pipeline (measured double-spawn source) is
	//! never used. Tri-state spawn-authority contract (A1): NOT_MINE is the only
	//! result that may reach vanilla spawn; ALREADY/FAILED mean "vanilla stands down".
	//! @authority server
	TBD_EDeployResult DeployPlayerEx(int playerId)
	{
		// Authority only — slot assignment + binding run on the server.
		if (RplSession.Mode() == RplMode.Client)
			return TBD_EDeployResult.NOT_MINE;

		// No valid framework mission → vanilla owns spawning entirely.
		if (!TBD_MissionLoader.IsLoaded() || !TBD_MissionLoader.IsValid())
			return TBD_EDeployResult.NOT_MINE;

		if (!m_bSlotBodiesMaterialized || !TBD_RosterLoader.IsLoaded())
			return TBD_EDeployResult.RETRY;

		if (m_mDeployRequested.Contains(playerId))
			return TBD_EDeployResult.ALREADY;

		AssignSlotForPlayer(playerId);

		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (!slot)
			return TBD_EDeployResult.RETRY;

		// Body must exist and be alive; a dead one (previous life) is replaced by a
		// fresh dressed body at the slot transform — the corpse stays where it fell.
		IEntity body = m_mSlotBodies.Get(slot.Key());
		if (!body || IsBodyDead(body))
		{
			body = SpawnSlotBody(slot, 0);
			if (!body)
				return TBD_EDeployResult.FAILED;
			m_mSlotBodies.Set(slot.Key(), body);
			Print(string.Format("[TBD][Slots] rematerialized body for slot %1 (respawn)", slot.Key()));
		}

		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (!pc)
		{
			Print("[TBD] SpawnManager: no player controller for player " + playerId, LogLevel.ERROR);
			return TBD_EDeployResult.RETRY;
		}

		SCR_PlayerFactionAffiliationComponent factionComp = SCR_PlayerFactionAffiliationComponent.Cast(
			pc.FindComponent(SCR_PlayerFactionAffiliationComponent));
		if (factionComp)
		{
			// Mission key first; if it maps to nothing (modded kit faction, unmapped side)
			// fall back to whatever faction the body itself was built as, so the player is
			// never registered under an empty key.
			string engineKey = EngineFactionKey(slot.faction);
			if (engineKey.IsEmpty())
				engineKey = BodyFactionKey(body);

			if (!engineKey.IsEmpty())
			{
				factionComp.SetAffiliatedFactionByKey(engineKey);
				// Vanilla only learns about the affiliation through the manager (the
				// PlayableSelector finalize); without it the player is faction-correct
				// locally but invisible to faction-keyed vanilla systems.
				SCR_FactionManager fm = SCR_FactionManager.Cast(GetGame().GetFactionManager());
				if (fm)
					fm.UpdatePlayerFaction_S(factionComp);
			}
			else
			{
				Print(string.Format("[TBD][Spawn] slot=%1 faction=%2 has no engine mapping — affiliation left untouched",
					slot.id, slot.faction), LogLevel.WARNING);
			}
		}

		// The takeover. Preferred route is vanilla's POSSESS spawn request: it is the
		// engine's own "this player takes over an entity that already exists" path, so it
		// creates no second body (the double-spawn class stays fixed) while running the
		// full spawn finalize — including the client-side notification the loading screen
		// waits on. A raw SetInitialMainEntity possesses the body and gives it a camera,
		// but the client is never told a spawn happened and sits on the loading screen
		// forever (measured 2026-07-25). SetInitialMainEntity stays as the fallback for
		// when the request component is missing or refuses.
		bool possessed = PossessSlotBody(pc, body, playerId);
		if (!possessed)
			pc.SetInitialMainEntity(body);

		m_mDeployRequested.Set(playerId, true);
		m_mRetryCount.Remove(playerId);
		m_mSpawnSeen.Remove(playerId);
		Print(string.Format("[TBD] SpawnManager: bound player %1 to slot %2 body (kit %3)", playerId, slot.Key(), slot.kit));

		// Announce the spawn ourselves ONLY on the fallback route. The possess pipeline
		// fires the game mode's spawn invoker itself, and our hook is subscribed to it —
		// self-announcing there notified every listener twice (measured: two
		// "deployed player=" diagnostics per bind).
		if (!possessed)
			NotifySpawnedManually(playerId, body);

		// A1 watchdog: if control never materializes, re-arm so the next pull
		// attempt can deploy instead of wedging on ALREADY forever.
		GetGame().GetCallqueue().CallLater(CheckSpawnArrived, 10000, false, playerId);
		return TBD_EDeployResult.DEPLOYED;
	}

	//------------------------------------------------------------------------------------------------
	//! Hand the player to its slot body through vanilla's possess spawn request.
	//! Returns false when the route is unavailable, so the caller can fall back.
	protected bool PossessSlotBody(SCR_PlayerController pc, IEntity body, int playerId)
	{
		SCR_PossessSpawnRequestComponent request = SCR_PossessSpawnRequestComponent.Cast(
			pc.FindComponent(SCR_PossessSpawnRequestComponent));
		if (!request)
		{
			Print(string.Format("[TBD][Spawn] player=%1 has no possess request component — falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		SCR_PossessSpawnData data = SCR_PossessSpawnData.FromEntity(body);
		if (!data)
		{
			Print(string.Format("[TBD][Spawn] player=%1 possess data build failed — falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		if (!request.RequestRespawn(data))
		{
			Print(string.Format("[TBD][Spawn] player=%1 possess request refused — falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		Print(string.Format("[TBD][Spawn] player=%1 possess request accepted", playerId));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! FALLBACK ROUTE ONLY (the possess pipeline announces its own spawns).
	//! SetInitialMainEntity bypasses the vanilla spawn pipeline, so nothing fires the
	//! usual spawn notifications (the CRF finding). Fire the game mode's own invoker
	//! rather than calling our hook directly: our hook is subscribed to it (OnPostInit),
	//! so our bookkeeping still runs exactly once, and the vanilla listeners that assume
	//! a spawn always announces itself finally hear it too (the PlayableSelector finalize).
	//! Server-side only — a dedicated server also needs the client-side invoke, which is
	//! the named follow-up in the verify log.
	//! (CRF also notifies its own MODDED data collector here — vanilla
	//! SCR_DataCollectorComponent has no such entry point; stats integration is a
	//! future slice if the platform ever consumes vanilla session stats.)
	protected void NotifySpawnedManually(int playerId, IEntity body)
	{
		// Fire only once the player ACTUALLY controls the body: SetInitialMainEntity hands
		// over asynchronously, and listeners that react to a spawn (the client-side ones
		// that take the player off the loading screen among them) check the controlled
		// entity and bail when it is still null. PlayableSelector fires from
		// OnControlledEntityChanged for the same reason.
		FinalizeSpawnWhenControlled(playerId, 0);
	}

	//------------------------------------------------------------------------------------------------
	//! Poll until possession lands (200 ms × 25 = 5 s ceiling), then announce the spawn.
	protected void FinalizeSpawnWhenControlled(int playerId, int attempt)
	{
		IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (!controlled)
		{
			if (attempt < 25)
				GetGame().GetCallqueue().CallLater(FinalizeSpawnWhenControlled, 200, false, playerId, attempt + 1);
			else
				Print(string.Format("[TBD][Spawn] player=%1 never took control of its body — spawn not announced", playerId), LogLevel.WARNING);
			return;
		}

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(GetOwner());
		if (gm)
			gm.GetOnPlayerSpawned().Invoke(playerId, controlled);
		else
			OnPlayerSpawnedHook(playerId, controlled);
	}

	//------------------------------------------------------------------------------------------------
	//! True when a materialized body is destroyed/dead (corpse — respawn replaces it).
	protected static bool IsBodyDead(IEntity body)
	{
		ChimeraCharacter character = ChimeraCharacter.Cast(body);
		if (!character)
			return true;
		CharacterControllerComponent ccc = character.GetCharacterController();
		return ccc && ccc.IsDead();
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
	//! Spawn-notify sink: fired by NotifySpawnedManually on every bind (and by the
	//! vanilla invoker for any non-framework spawn). Bookkeeping only — dressing is
	//! owned by materialization (SpawnSlotBody dresses both initial and respawn
	//! bodies), so no equip runs here; the reaper died with the vanilla RequestSpawn
	//! pipeline (nothing can double-spawn any more).
	//! @authority server
	protected void OnPlayerSpawnedHook(int playerId, IEntity controlledEntity)
	{
		if (RplSession.Mode() == RplMode.Client || !controlledEntity)
			return;

		m_mSpawnSeen.Set(playerId, true);
		GetGame().GetCallqueue().CallLater(LogDeployedTransform, 500, false, playerId);
		ScheduleCensus();
	}

	//------------------------------------------------------------------------------------------------
	//! A6 — death re-arms the deploy guard; the slot assignment survives, so the next
	//! deploy finds the slot body dead and REMATERIALIZES a fresh dressed one at the
	//! slot transform (operator-locked re-equip-every-spawn; corpse stays). (1.7
	//! component virtual takes SCR_InstigatorContextData — the CRF Rally precedent.)
	//! @authority server
	override void OnPlayerKilled(notnull SCR_InstigatorContextData instigatorContextData)
	{
		super.OnPlayerKilled(instigatorContextData);

		if (RplSession.Mode() == RplMode.Client)
			return;

		int playerId = instigatorContextData.GetVictimPlayerID();
		if (playerId <= 0)
			return;

		m_mDeployRequested.Remove(playerId);
		m_mRetryCount.Remove(playerId);
		m_mSpawnSeen.Remove(playerId);
		Print(string.Format("[TBD][Spawn] player=%1 killed — re-armed for respawn (slot retained)", playerId));

		// Re-arming alone used to be enough because the vanilla deploy menu asked again;
		// it is stood down now, so the framework drives the next life itself.
		if (m_bAutoDeploy)
			GetGame().GetCallqueue().CallLater(RedeployAfterDeath, m_iRedeployDelayMs, false, playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! Puts a killed player back on his slot: DeployPlayerEx finds the slot body dead and
	//! rematerializes a fresh dressed one (re-equip every spawn — operator-locked; the
	//! corpse stays where it fell).
	//! @authority server
	protected void RedeployAfterDeath(int playerId)
	{
		if (!GetGame().GetPlayerManager().GetPlayerController(playerId))
			return;  // Disconnected during the respawn beat.

		if (m_mDeployRequested.Contains(playerId))
			return;  // Already back in the world by another path.

		TBD_EDeployResult r = DeployPlayerEx(playerId);
		Print(string.Format("[TBD][Spawn] path=redeploy player=%1 result=%2", playerId, typename.EnumToString(TBD_EDeployResult, r)));
		if (r == TBD_EDeployResult.RETRY)
			ScheduleDeployRetry(playerId);
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
		m_mDeployRequested.Remove(playerId);
		m_mRetryCount.Remove(playerId);
		m_mSpawnSeen.Remove(playerId);
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
		Print(string.Format("[TBD][Audit] characters=%1 bodies=%2 players=%3", m_iCensusCount, m_mSlotBodies.Count(), playerCount));
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
		if (m_mSpawnSeen.Contains(playerId))
			return;
		if (GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId))
			return;

		Print(string.Format("[TBD][Spawn] watchdog player=%1 — spawn request never materialized, re-arming", playerId), LogLevel.WARNING);
		m_mDeployRequested.Remove(playerId);
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
		m_mSpawnSeen.Set(playerId, true);

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
