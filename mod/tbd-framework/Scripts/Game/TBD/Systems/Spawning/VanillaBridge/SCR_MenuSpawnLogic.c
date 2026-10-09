/**
 * @file SCR_MenuSpawnLogic.c
 * @brief Vanilla's menu spawn logic routed to TBD_SpawnManager on a framework world.
 *
 * Role: stands down vanilla registration and spawn-point waiting on a framework world and sends
 * the vanilla pull spawn to TBD_SpawnManager.DeployPlayerEx.  Position: vanilla spawn logic of the
 * respawn system; the real join work happens in TBD_SpawnManager.OnPlayerAuditSuccess.
 * State: none.  Invariants: on a framework mission only a NOT_MINE result falls through to vanilla,
 * so vanilla never creates a second body; a plain vanilla world (the mod loads world-globally)
 * keeps vanilla behaviour.
 */

//! Framework routing of vanilla's menu spawn logic.
modded class SCR_MenuSpawnLogic
{
	//! Never wait for spawn points on a framework world: slot bodies replace them, there are no
	//! SCR_SpawnPoint entities, and waiting would pin the client on the loading screen.
	override bool GetWaitForSpawnPoints()
	{
		if (TBD_FrameworkManager.IsFrameworkWorld())
			return false;

		return super.GetWaitForSpawnPoints();
	}

	//! Vanilla's per-player entry into the spawn logic; swallowed on a framework world so the
	//! deploy flow never hunts a faction.
	//! @authority server
	override void OnPlayerRegistered_S(int playerId)
	{
		if (TBD_FrameworkManager.IsFrameworkWorld())
			return;

		super.OnPlayerRegistered_S(playerId);
	}

	//! Vanilla's audit hook. The modded SCR_RespawnSystemComponent swallows the call that reaches it
	//! on a framework world, so it seats a player only when a framework mission is loaded on a
	//! non-framework world; the framework join lives in TBD_SpawnManager.OnPlayerAuditSuccess.
	//! @authority server
	override void OnPlayerAuditSuccess_S(int playerId)
	{
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm && sm.AreSlotBodiesMaterialized())
			sm.AssignSlotForPlayer(playerId);

		if (TBD_FrameworkManager.IsFrameworkWorld())
			return;

		super.OnPlayerAuditSuccess_S(playerId);
	}

	//! Vanilla's server-side pull spawn, routed through TBD_SpawnManager.DeployPlayerEx; only
	//! NOT_MINE (or no manager) reaches vanilla, RETRY schedules a retry.
	//! @authority server
	override void DoSpawn_S(int playerId)
	{
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (!sm)
		{
			super.DoSpawn_S(playerId);
			return;
		}

		TBD_EDeployResult r = sm.DeployPlayerEx(playerId);
		Print(string.Format("[TBD][Spawn] path=pull player=%1 result=%2", playerId, typename.EnumToString(TBD_EDeployResult, r)));

		if (r == TBD_EDeployResult.NOT_MINE)
		{
			Print(string.Format("[TBD][Spawn] path=vanilla-fallthrough player=%1", playerId));
			super.DoSpawn_S(playerId);
			return;
		}

		if (r == TBD_EDeployResult.RETRY)
			sm.GetDeployWaves().ScheduleDeployRetry(playerId);

		// Every other result keeps vanilla out: FAILED leaves the player on the wait screen, DENIED
		// (a spent life) is never retried, and a platform decision finishes AUTHORIZING by itself.
	}
}
