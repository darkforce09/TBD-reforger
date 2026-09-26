/**
 * @file TBD_DeployWatchdog.c
 * @brief After a deploy: arrival watchdog, spawn bookkeeping, transform log and the body census.
 *
 * Role: notices spawns landing, re-arms a deploy whose body never arrived, logs the landed
 * transform and counts characters against slot bodies and players.  Position: armed by
 * TBD_DeployExecutor.HandPlayerOntoBody; OnPlayerSpawnedHook is subscribed to the game mode's spawn
 * invoker by TBD_SpawnManager.
 * State: the per-player spawn-seen set and the census debounce and counter, server only.
 * Invariants: every deferred callback quotes the connection epoch it was armed under and does
 * nothing once it moved on; the census runs at most once per 5 s window.
 */

//! Post-deploy watchdog of the spawn manager.
class TBD_DeployWatchdog : Managed
{
	protected const int ARRIVAL_TIMEOUT_MS = 10000; //!< how long (ms) a deploy may take to put the player in control
	protected const int CENSUS_DELAY_MS = 5000; //!< settle time (ms) between the first spawn of a burst and the census

	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected ref map<int, bool> m_mSpawnSeen; //!< players whose requested spawn was observed to land
	protected bool m_bCensusScheduled; //!< true while a census is pending
	protected int m_iCensusCount; //!< characters counted by the running census

	//! Bind the watchdog to its manager.
	void TBD_DeployWatchdog(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
		m_mSpawnSeen = new map<int, bool>();
	}

	//! Cancel every pending check, log and census.
	void CancelCallbacks()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		queue.Remove(CheckSpawnArrived);
		queue.Remove(LogDeployedTransform);
		queue.Remove(RunCensus);
	}

	//! Forget that a spawn of `playerId` was seen.
	void ForgetSpawnSeen(int playerId)
	{
		m_mSpawnSeen.Remove(playerId);
	}

	//! Arm the arrival check for a deploy of `playerId` that was just handed a body.
	//! @authority server
	void Arm(int playerId)
	{
		GetGame().GetCallqueue().CallLater(CheckSpawnArrived, ARRIVAL_TIMEOUT_MS, false, playerId, m_Spawn.GetEpochs().EnsureConnectEpoch(playerId));
	}

	//! Spawn invoker sink: marks the spawn seen, spends the spawn ticket, schedules the transform
	//! log and the census. Bookkeeping only; dressing belongs to the slot body spawn.
	//! @authority server
	void OnPlayerSpawnedHook(int playerId, IEntity controlledEntity)
	{
		if (TBD_Authority.IsClient() || !controlledEntity)
			return;

		m_mSpawnSeen.Set(playerId, true);
		m_Spawn.GetTickets().RevokeSpawnAuthorization(playerId);
		GetGame().GetCallqueue().CallLater(LogDeployedTransform, 500, false, playerId, m_Spawn.GetEpochs().EnsureConnectEpoch(playerId));
		ScheduleCensus();
	}

	//! Re-arm a deploy whose spawn never landed, so the next attempt deploys instead of answering
	//! ALREADY forever.
	protected void CheckSpawnArrived(int playerId, int epoch)
	{
		if (!m_Spawn.GetEpochs().IsSameConnection(playerId, epoch))
			return;

		if (m_mSpawnSeen.Contains(playerId))
			return;
		if (GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId))
			return;

		Print(string.Format("[TBD][Spawn] watchdog player=%1 -- spawn request never materialized, re-arming", playerId), LogLevel.WARNING);
		m_Spawn.GetDeployExecutor().ForgetRequest(playerId);
		m_Spawn.GetDeployWaves().ForgetHolder(playerId);
	}

	//! Log the landed character's feet height against the live terrain (the calibration source of
	//! TBD_SlotBodyMaterializer.CAPSULE_GROUND_OFFSET_M) and mark the spawn seen.
	protected void LogDeployedTransform(int playerId, int epoch)
	{
		if (!m_Spawn.GetEpochs().IsSameConnection(playerId, epoch))
			return;

		IEntity ent = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (!ent)
		{
			Print(string.Format("[TBD][Spawn] deployed player=%1 -- no controlled entity yet (spawn pending?)", playerId), LogLevel.WARNING);
			return;
		}
		m_mSpawnSeen.Set(playerId, true);

		vector org = ent.GetOrigin();
		float surfaceY = GetGame().GetWorld().GetSurfaceY(org[0], org[2]);
		float groundDelta = org[1] - surfaceY;
		float yaw = ent.GetYawPitchRoll()[0];

		string slotId = "-";
		TBD_MissionSlotStruct slot = m_Spawn.GetAssignedSlot(playerId);
		if (slot)
			slotId = slot.id;

		Print(string.Format("[TBD][Spawn] deployed player=%1 slot=%2 pos=%3 feetY=%4 surfaceY=%5 groundDelta=%6 yaw=%7",
			playerId, slotId, org.ToString(), org[1], surfaceY, groundDelta, yaw));
	}

	//! Schedule one census CENSUS_DELAY_MS from now unless one is pending.
	protected void ScheduleCensus()
	{
		if (m_bCensusScheduled)
			return;
		m_bCensusScheduled = true;
		GetGame().GetCallqueue().CallLater(RunCensus, CENSUS_DELAY_MS, false);
	}

	//! Log characters, slot bodies and players; characters above players points at an orphan body.
	protected void RunCensus()
	{
		m_iCensusCount = 0;
		BaseWorld world = GetGame().GetWorld();
		if (world)
			world.QueryEntitiesByAABB(Vector(-1000, -2000, -1000), Vector(20000, 4000, 20000), CensusAddEntity);

		array<int> players = {};
		int playerCount = GetGame().GetPlayerManager().GetPlayers(players);
		Print(string.Format("[TBD][Audit] characters=%1 bodies=%2 players=%3", m_iCensusCount, m_Spawn.GetBodies().CountBodies(), playerCount));
		m_bCensusScheduled = false;
	}

	//! Census query callback: counts every character.
	protected bool CensusAddEntity(IEntity ent)
	{
		if (ChimeraCharacter.Cast(ent))
			m_iCensusCount++;
		return true;
	}
}
