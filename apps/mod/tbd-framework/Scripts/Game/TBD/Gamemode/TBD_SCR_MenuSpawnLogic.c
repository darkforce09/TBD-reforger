//! Slot-based deploy: overrides vanilla menu spawn to use mission slots[] position + kit.
modded class SCR_MenuSpawnLogic
{
	//------------------------------------------------------------------------------------------------
	override bool GetWaitForSpawnPoints()
	{
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm && !sm.AreSlotSpawnPointsBuilt())
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
	//! @authority server — _S = server-side spawn; routes through TBD_SpawnManager.DeployPlayer.
	override void DoSpawn_S(int playerId)
	{
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm && sm.DeployPlayer(playerId))
			return;

		super.DoSpawn_S(playerId);
	}
}
