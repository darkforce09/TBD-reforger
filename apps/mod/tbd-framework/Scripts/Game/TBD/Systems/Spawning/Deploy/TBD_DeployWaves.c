/**
 * @file TBD_DeployWaves.c
 * @brief Scheduled deploys: the BRIEFING holder deploy, the automatic seating, slot changes and the retry ladder.
 *
 * Role: runs the deploys that are not a direct player request and keeps their per-player
 * bookkeeping.  Position: owned by TBD_SpawnManager; driven by stage changes, loadout settle, lobby
 * claims, the join audit, the vanilla pull path and platform decisions; every deploy goes through
 * TBD_SpawnManager.DeployPlayerEx or, for an admin respawn retry, TBD_DeployExecutor.DeployPlayerInternal.
 * State: the players already deployed as slot holders and the per-player retry count, server only.
 * Invariants: bodies wait for BRIEFING (no deploy during LOBBY); each holder deploys once until a
 * death, re-arm or disconnect clears it; a retry ladder is at most 20 x 500 ms, quotes the
 * connection epoch and carries the admin override only for a pending admin respawn.
 */

//! Scheduled deploys of the spawn manager.
class TBD_DeployWaves : Managed
{
	protected const int WAVE_DELAY_MS = 250; //!< settle (ms) before the BRIEFING holder deploy runs
	protected const int RETRY_DELAY_MS = 500; //!< delay (ms) between retry attempts
	protected const int RETRY_MAX_ATTEMPTS = 20; //!< retry attempts before the ladder gives up

	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected ref map<int, bool> m_mDeployedHolders; //!< players who already received their holder body
	protected ref map<int, int> m_mRetryCount; //!< playerId to retry attempts made

	//! Bind the waves to their manager.
	void TBD_DeployWaves(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
		m_mDeployedHolders = new map<int, bool>();
		m_mRetryCount = new map<int, int>();
	}

	//! Cancel the pending waves and retries.
	void CancelCallbacks()
	{
		GetGame().GetCallqueue().Remove(DeployClaimedHolders);
		GetGame().GetCallqueue().Remove(RetryDeploy);
	}

	//! True when `playerId` already received their holder body.
	bool IsHolderDeployed(int playerId)
	{
		return m_mDeployedHolders.Contains(playerId);
	}

	//! Record that `playerId` received their holder body.
	void MarkHolderDeployed(int playerId)
	{
		m_mDeployedHolders.Set(playerId, true);
	}

	//! Clear the holder mark of `playerId`, so the next scheduled deploy seats them again.
	void ForgetHolder(int playerId)
	{
		m_mDeployedHolders.Remove(playerId);
	}

	//! Reset the retry count of `playerId`.
	void ForgetRetries(int playerId)
	{
		m_mRetryCount.Remove(playerId);
	}

	//! Deploy `playerId`, log `[TBD][Spawn] path=<path>` with the result, mark a delivered holder and
	//! schedule a retry for RETRY.
	//! @return the deploy result
	//! @authority server
	TBD_EDeployResult DeployOnPath(int playerId, string path)
	{
		TBD_EDeployResult r = m_Spawn.DeployPlayerEx(playerId);
		Print(string.Format("[TBD][Spawn] path=%3 player=%1 result=%2", playerId, typename.EnumToString(TBD_EDeployResult, r), path));
		if (r == TBD_EDeployResult.DEPLOYED || r == TBD_EDeployResult.ALREADY)
			MarkHolderDeployed(playerId);
		if (r == TBD_EDeployResult.RETRY)
			ScheduleDeployRetry(playerId);

		return r;
	}

	//! React to a stage change: LOBBY deploys nobody; LOBBY to BRIEFING deploys every claimed holder.
	//! @authority server
	void OnStageChanged(TBD_EGameStage previous, TBD_EGameStage stage)
	{
		if (stage == TBD_EGameStage.LOBBY)
		{
			PrintFormat("[TBD][Spawn] LOBBY: no bodies this phase -- claimed holders deploy on BRIEFING (T-941.2). m_bAutoDeploy=%1 seats unclaimed players at briefing only.",
				m_Spawn.IsAutoDeploy());
			return;
		}

		if (previous == TBD_EGameStage.LOBBY && stage == TBD_EGameStage.BRIEFING)
		{
			PrintFormat("[TBD][Spawn] BRIEFING: deploying each claimed slot holder once (T-941.2).");
			ScheduleDeployClaimedHolders();
		}
	}

	//! Schedule the holder deploy once the lineup is materialized; TBD_SlotLoadoutSettle schedules it
	//! again when the lineup opens during BRIEFING.
	//! @authority server
	void ScheduleDeployClaimedHolders()
	{
		if (TBD_Authority.IsClient())
			return;

		if (!m_Spawn.GetBodies().AreMaterialized())
			return;

		GetGame().GetCallqueue().CallLater(DeployClaimedHolders, WAVE_DELAY_MS, false);
	}

	//! Deploy every slot holder not yet deployed and not dead, then, with automatic deploy on, every
	//! other connected player.
	//! @authority server
	protected void DeployClaimedHolders()
	{
		if (TBD_Authority.IsClient())
			return;

		foreach (int playerId, TBD_MissionSlotStruct slot : m_Spawn.GetSlots().GetPlayerSlots())
		{
			if (!slot)
				continue;
			if (m_mDeployedHolders.Contains(playerId))
				continue;
			if (m_Spawn.IsOneLife() && m_Spawn.IsPlayerDead(playerId))
			{
				Print(string.Format("[TBD][Spawn] path=briefing-holder player=%1 skipped -- one life spent", playerId));
				continue;
			}

			DeployOnPath(playerId, "briefing-holder");
		}

		if (m_Spawn.IsAutoDeploy())
			DeployAllConnectedPlayers();
	}

	//! Leave the current body and deploy onto the newly claimed slot.
	//! @authority server
	void RedeployHolderToClaimedSlot(int playerId)
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (pc && pc.IsPossessing())
			pc.SetPossessedEntity(null);

		m_Spawn.GetDeployExecutor().ForgetRequest(playerId);
		m_mDeployedHolders.Remove(playerId);
		m_Spawn.GetTickets().RevokeSpawnAuthorization(playerId);

		DeployOnPath(playerId, "slot-change");
	}

	//! Deploy every connected player not yet deployed as a holder and not dead.
	//! @authority server
	protected void DeployAllConnectedPlayers()
	{
		if (TBD_Authority.IsClient())
			return;

		array<int> players = {};
		int count = GetGame().GetPlayerManager().GetPlayers(players);
		for (int i = 0; i < count; i++)
		{
			if (m_mDeployedHolders.Contains(players[i]))
				continue;

			if (m_Spawn.IsOneLife() && m_Spawn.IsPlayerDead(players[i]))
			{
				Print(string.Format("[TBD][Spawn] path=push player=%1 skipped -- one life spent", players[i]));
				continue;
			}

			DeployOnPath(players[i], "push");
		}
	}

	//! Retry a transient RETRY every RETRY_DELAY_MS, stamped with the connection epoch.
	//! @authority server
	void ScheduleDeployRetry(int playerId)
	{
		GetGame().GetCallqueue().CallLater(RetryDeploy, RETRY_DELAY_MS, false, playerId, m_Spawn.GetEpochs().EnsureConnectEpoch(playerId));
	}

	//! One retry attempt: gives up (logged ERROR, and attributed for an admin respawn) after
	//! RETRY_MAX_ATTEMPTS; carries the admin override only while an admin respawn is pending and
	//! settles that respawn on any final result. A spent life ends the ladder with DENIED.
	//! @authority server
	void RetryDeploy(int playerId, int epoch)
	{
		if (!m_Spawn.GetEpochs().IsSameConnection(playerId, epoch))
			return;

		bool adminRespawn = m_Spawn.GetLives().IsAdminRespawnPending(playerId);

		int n;
		m_mRetryCount.Find(playerId, n);
		if (n >= RETRY_MAX_ATTEMPTS)
		{
			Print(string.Format("[TBD][Spawn] path=retry player=%1 gave up after %2 attempts", playerId, n), LogLevel.ERROR);
			m_mRetryCount.Remove(playerId);
			if (adminRespawn)
			{
				m_Spawn.GetLives().ClearAdminRespawnPending(playerId);
				Print(string.Format("[TBD][Admin] respawn player=%1 gave up after %2 attempts -- player REMAINS dead, run '#tbd respawn %1' again",
					playerId, n), LogLevel.ERROR);
			}
			return;
		}
		m_mRetryCount.Set(playerId, n + 1);

		TBD_EDeployResult r = m_Spawn.GetDeployExecutor().DeployPlayerInternal(playerId, adminRespawn, adminRespawn);
		Print(string.Format("[TBD][Spawn] path=retry player=%1 attempt=%2 admin=%3 result=%4",
			playerId, n + 1, adminRespawn, typename.EnumToString(TBD_EDeployResult, r)));

		if (r == TBD_EDeployResult.RETRY)
		{
			ScheduleDeployRetry(playerId);
			return;
		}

		m_mRetryCount.Remove(playerId);
		if (r == TBD_EDeployResult.DEPLOYED || r == TBD_EDeployResult.ALREADY)
			MarkHolderDeployed(playerId);
		if (adminRespawn)
			m_Spawn.GetDeathFlow().FinishAdminRespawn(playerId, r, "retry");
	}
}
