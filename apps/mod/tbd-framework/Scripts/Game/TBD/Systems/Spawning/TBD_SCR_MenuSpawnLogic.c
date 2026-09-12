//! Slot-based deploy: overrides vanilla menu spawn to use mission slots[] position + kit.
//!
//! Spawn-authority contract (determinism program, slice A1): when a framework mission is
//! active, TBD_SpawnManager is the ONLY thing that may spawn a player. `DeployPlayerEx`
//! returns a tri-state result and this hook NEVER falls through to `super.DoSpawn_S`
//! except on NOT_MINE (client side / no framework mission) — the silent fall-through on
//! "already deployed" was the double-body / slot-transfer / vanilla-kit bug.
//!
//! Registration/audit are swallowed on framework worlds (see TBD_SCR_RespawnSystemComponent
//! for the measured faction-churn defect): the `super.OnPlayerAuditSuccess_S` call that used
//! to run here handed the player straight to the vanilla spawn logic, which then spent the
//! rest of the session re-rolling his faction looking for a spawn point that no longer
//! exists. `DoSpawn_S` and `GetWaitForSpawnPoints` stay live and unchanged: on a plain
//! vanilla world (this mod loads world-globally) they are the fall-through that keeps
//! ordinary scenarios working.
modded class SCR_MenuSpawnLogic
{
	//------------------------------------------------------------------------------------------------
	//! Never wait for spawn points on a framework world: slot bodies replaced them, so
	//! there are zero SCR_SpawnPoint entities and vanilla's own answer here is "keep
	//! waiting" forever — which is what pinned the client on the loading screen even
	//! after TBD_SpawnManager had bound it to a body (measured 2026-07-25).
	override bool GetWaitForSpawnPoints()
	{
		if (TBD_FrameworkManager.IsFrameworkWorld())
			return false;

		return super.GetWaitForSpawnPoints();
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — vanilla's per-player entry into the spawn logic; swallowed on
	//! framework worlds so the deploy flow never starts hunting a faction.
	override void OnPlayerRegistered_S(int playerId)
	{
		if (TBD_FrameworkManager.IsFrameworkWorld())
			return;

		super.OnPlayerRegistered_S(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the _S suffix is vanilla's server-side audit hook.
	//!
	//! T-181.22 — DEAD ON A FRAMEWORK WORLD, and nothing here should be relied on. Vanilla only
	//! reaches this through `SCR_RespawnSystemComponent.OnPlayerAuditSuccess_S ->
	//! m_SpawnLogic.OnPlayerAuditSuccess_S(playerId)` (vanilla SCR_RespawnSystemComponent.c:196-199),
	//! and TBD_SCR_RespawnSystemComponent swallows that call on framework worlds. So the
	//! `AssignSlotForPlayer` below runs on VANILLA worlds only — where TBD_SpawnManager does not
	//! exist and the guard short-circuits anyway. It is kept purely so a framework mission left
	//! loaded on a non-framework world still seats people.
	//!
	//! The join-time work that must actually happen on a framework world lives in
	//! TBD_SpawnManager.OnPlayerAuditSuccess (a SCR_BaseGameModeComponent virtual the game mode
	//! drives directly, unaffected by the suppression above): bind-key resolve, and handing a
	//! returning spent life its seat back.
	override void OnPlayerAuditSuccess_S(int playerId)
	{
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm && sm.AreSlotBodiesMaterialized())
			sm.AssignSlotForPlayer(playerId);

		if (TBD_FrameworkManager.IsFrameworkWorld())
			return;

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

		// DEPLOYED / ALREADY / FAILED / DENIED: never let vanilla spawn a second body on a
		// framework mission. FAILED keeps the player on the wait screen (logged ERROR).
		// T-181.21 — DENIED means the player has spent their one life. It is NOT retried here,
		// on purpose: retrying a policy decision would just re-log the refusal forever, and the
		// only legitimate way back in is an admin (TBD_SpawnManager.AdminRespawn). Nothing extra
		// is needed at this call site — DeployPlayerEx already refused, and the vanilla request
		// route is refused independently by TBD_SCR_RespawnSystemComponent.CanRequestSpawn_S.
	}
}
