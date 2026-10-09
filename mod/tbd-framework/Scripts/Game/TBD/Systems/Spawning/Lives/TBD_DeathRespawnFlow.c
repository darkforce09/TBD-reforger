/**
 * @file TBD_DeathRespawnFlow.c
 * @brief What a death means: ONE LIFE spent, or a timed redeploy; and the admin respawn back.
 *
 * Role: handles a player's death and the admin respawn, the only way back for a spent life.
 * Position: TBD_SpawnManager.OnPlayerKilled and AdminRespawn call it on the authority;
 * TBD_DeployWaves.RetryDeploy and TBD_SpawnDeploymentGate settle admin respawns through
 * FinishAdminRespawn.
 * State: none of its own; it writes the one-life ledger and re-arms the deploy bookkeeping.
 * Invariants: under ONE LIFE the death mark is written before anything else can deploy the
 * player, and the seat stays claimed; an admin respawn gives the life back only once DEPLOYED,
 * so a player is never both alive in the ledger and without a body.
 */

//! Death and admin respawn handling of the spawn manager.
class TBD_DeathRespawnFlow : Managed
{
	protected TBD_SpawnManager m_Spawn; //!< owning manager

	//! Bind the flow to its manager.
	void TBD_DeathRespawnFlow(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
	}

	//! Cancel the pending redeploys.
	void CancelCallbacks()
	{
		GetGame().GetCallqueue().Remove(RedeployAfterDeath);
	}

	//! Re-arm the killed player's deploy bookkeeping and close their spawn ticket; then either spend
	//! the life (ONE LIFE: the seat stays claimed, only an admin brings them back) or schedule the
	//! redeploy after the respawn delay (automatic deploy worlds).
	//! @authority server
	void OnPlayerKilled(notnull SCR_InstigatorContextData instigatorContextData)
	{
		int playerId = instigatorContextData.GetVictimPlayerID();
		if (playerId <= 0)
			return;

		m_Spawn.GetDeployExecutor().ForgetRequest(playerId);
		m_Spawn.GetDeployWaves().ForgetHolder(playerId);
		m_Spawn.GetDeployWaves().ForgetRetries(playerId);
		m_Spawn.GetDeployWatchdog().ForgetSpawnSeen(playerId);
		m_Spawn.GetTickets().ForgetPlayer(playerId);
		m_Spawn.GetLives().ClearAdminRespawnPending(playerId);

		if (m_Spawn.IsOneLife())
		{
			m_Spawn.GetLives().MarkLifeSpent(playerId);
			Print(string.Format("[TBD][Spawn] player=%1 KILLED -- one life spent (key=%2), slot retained, awaiting admin",
				playerId, m_Spawn.GetIdentity().PlayerBindKey(playerId)));
			return;
		}

		Print(string.Format("[TBD][Spawn] player=%1 killed -- re-armed for respawn (slot retained)", playerId));

		if (m_Spawn.IsAutoDeploy())
			GetGame().GetCallqueue().CallLater(RedeployAfterDeath, m_Spawn.RedeployDelayMs(), false, playerId, m_Spawn.GetEpochs().EnsureConnectEpoch(playerId));
	}

	//! Put a killed player back on their slot; the deploy finds the body dead and rematerializes a
	//! fresh one. Does nothing once the connection moved on, the player left or is already back.
	//! @authority server
	protected void RedeployAfterDeath(int playerId, int epoch)
	{
		if (!m_Spawn.GetEpochs().IsSameConnection(playerId, epoch))
			return;

		if (!GetGame().GetPlayerManager().GetPlayerController(playerId))
			return;  // Disconnected during the respawn beat.

		if (m_Spawn.GetDeployExecutor().HasRequested(playerId))
			return;  // Already back in the world by another path.

		TBD_EDeployResult r = m_Spawn.DeployPlayerEx(playerId);
		Print(string.Format("[TBD][Spawn] path=redeploy player=%1 result=%2", playerId, typename.EnumToString(TBD_EDeployResult, r)));
		if (r == TBD_EDeployResult.RETRY)
			m_Spawn.GetDeployWaves().ScheduleDeployRetry(playerId);
	}

	//! The escape hatch for a glitch death: a fresh dressed body on the player's own slot, past the
	//! ONE LIFE guard. Refuses a player who is not dead or not connected. The result is recorded.
	//! The caller owns the permission check (TBD_AdminCommands).
	//! @authority server
	TBD_EDeployResult AdminRespawn(int playerId, string byAdmin)
	{
		if (TBD_Authority.IsClient())
			return TBD_EDeployResult.NOT_MINE;

		if (!m_Spawn.IsPlayerDead(playerId))
		{
			Print(string.Format("[TBD][Admin] respawn REFUSED player=%1 by=%2 -- not dead", playerId, byAdmin), LogLevel.WARNING);
			return TBD_EDeployResult.ALREADY;
		}

		if (!GetGame().GetPlayerManager().GetPlayerController(playerId))
		{
			Print(string.Format("[TBD][Admin] respawn REFUSED player=%1 by=%2 -- disconnected", playerId, byAdmin), LogLevel.WARNING);
			return TBD_EDeployResult.FAILED;
		}

		m_Spawn.GetDeployExecutor().ForgetRequest(playerId);
		m_Spawn.GetDeployWaves().ForgetHolder(playerId);
		m_Spawn.GetDeployWaves().ForgetRetries(playerId);
		m_Spawn.GetDeployWatchdog().ForgetSpawnSeen(playerId);

		TBD_EDeployResult r = m_Spawn.GetDeployExecutor().DeployPlayerInternal(playerId, true, true);
		Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 result=%3",
			playerId, byAdmin, typename.EnumToString(TBD_EDeployResult, r)));

		FinishAdminRespawn(playerId, r, byAdmin);
		m_Spawn.GetDeployExecutor().RecordDeployResult(playerId, r);
		return r;
	}

	//! Settle an admin respawn attempt: DEPLOYED gives the life back; RETRY and AUTHORIZING keep the
	//! player dead with the admin override pending; anything else leaves them dead, logged ERROR.
	void FinishAdminRespawn(int playerId, TBD_EDeployResult r, string byAdmin)
	{
		TBD_OneLifeLedger lives = m_Spawn.GetLives();
		if (r == TBD_EDeployResult.AUTHORIZING)
		{
			lives.SetAdminRespawnPending(playerId);
			Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 - awaiting the platform's deployment decision, player stays DEAD until it allows the new life", playerId, byAdmin));
			return;
		}

		if (r == TBD_EDeployResult.DEPLOYED)
		{
			lives.ClearLifeSpent(playerId);
			lives.ClearAdminRespawnPending(playerId);
			Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 -- back in the world, life restored", playerId, byAdmin));
			return;
		}

		if (r == TBD_EDeployResult.RETRY)
		{
			lives.SetAdminRespawnPending(playerId);
			m_Spawn.GetDeployWaves().ScheduleDeployRetry(playerId);
			Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 -- RETRY queued, player stays DEAD until a body lands", playerId, byAdmin));
			return;
		}

		lives.ClearAdminRespawnPending(playerId);
		Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 did NOT deploy (%3) -- player REMAINS dead, run '#tbd respawn %1' again",
			playerId, byAdmin, typename.EnumToString(TBD_EDeployResult, r)), LogLevel.ERROR);
	}
}
