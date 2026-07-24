//! Slot-based deploy: overrides vanilla menu spawn to use mission slots[] position + kit.
//!
//! Spawn-authority contract (determinism program, slice A1): when a framework mission is
//! active, TBD_SpawnManager is the ONLY thing that may spawn a player. `DeployPlayerEx`
//! returns a tri-state result and this hook NEVER falls through to `super.DoSpawn_S`
//! except on NOT_MINE (client side / no framework mission) — the silent fall-through on
//! "already deployed" was the double-body / slot-transfer / vanilla-kit bug.
modded class SCR_MenuSpawnLogic
{
	//------------------------------------------------------------------------------------------------
	//! Hold the vanilla wait screen until slots exist AND the roster race is settled —
	//! the pull path must not outrun assignment determinism (A5 settles roster pre-LOBBY;
	//! this is the belt-and-braces for early auditors).
	override bool GetWaitForSpawnPoints()
	{
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm && (!sm.AreSlotSpawnPointsBuilt() || !TBD_RosterLoader.IsLoaded()))
			return true;

		return super.GetWaitForSpawnPoints();
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the _S suffix is vanilla's server-side audit hook.
	override void OnPlayerAuditSuccess_S(int playerId)
	{
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm && sm.AreSlotSpawnPointsBuilt())
			sm.AssignSlotForPlayer(playerId);

		super.OnPlayerAuditSuccess_S(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — _S = server-side spawn; routes through TBD_SpawnManager.
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
			sm.ScheduleDeployRetry(playerId);

		// DEPLOYED / ALREADY / FAILED: never let vanilla spawn a second body on a
		// framework mission. FAILED keeps the player on the wait screen (logged ERROR).
	}
}
