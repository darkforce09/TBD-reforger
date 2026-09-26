/**
 * @file SCR_PossessSpawnHandlerComponent.c
 * @brief The possess door: a possess request succeeds only for the body a spawn ticket names.
 *
 * Role: gates vanilla's POSSESS spawn handler on a framework world.  Position: vanilla routes both
 * the ask (SCR_SpawnRequestComponent.ProcessCanRequest_S) and the real request (ProcessRequest_S
 * to HandleRequest_S) through CanHandleRequest_S; tickets come from TBD_PossessTicketLedger.
 * State: a one-time latch for the m_bIgnoreConditions report.
 * Invariants: POSSESS is the only request type TBD_PlayerController.et leaves enabled, and its RPC
 * takes a client-supplied RplId, so every request must match a ticket for that exact body; the gate
 * does not rely on CanRequestSpawn_S, which vanilla short-circuits while m_bIgnoreConditions is
 * set; it fails closed without a TBD_SpawnManager; on a plain vanilla world the class behaves as
 * vanilla.
 */

//! Framework gate on vanilla's possess spawn handler.
modded class SCR_PossessSpawnHandlerComponent
{
	protected bool m_bTbdIgnoreConditionsLogged; //!< One-time latch for the m_bIgnoreConditions report below.

	//! Asked by vanilla before it spawns or possesses anything, for the probe and the real request.
	//! Never spends the ticket: the probe must leave it for the real request.
	//! @authority server
	override SCR_ESpawnResult CanHandleRequest_S(SCR_SpawnRequestComponent requestComponent, SCR_SpawnData data)
	{
		SCR_ESpawnResult gate = TBD_GateRequest(requestComponent, data);
		if (gate != SCR_ESpawnResult.OK)
			return gate;

		return super.CanHandleRequest_S(requestComponent, data);
	}

	//! The request that hands the body over: gated again, then the ticket is spent once vanilla
	//! reports OK, so a request failing vanilla's own checks keeps the in-flight deploy's ticket.
	//! @authority server
	override SCR_ESpawnResult HandleRequest_S(SCR_SpawnRequestComponent requestComponent, SCR_SpawnData data, out IEntity spawnedEntity)
	{
		SCR_ESpawnResult gate = TBD_GateRequest(requestComponent, data);
		if (gate != SCR_ESpawnResult.OK)
			return gate;

		SCR_ESpawnResult result = super.HandleRequest_S(requestComponent, data, spawnedEntity);
		if (result != SCR_ESpawnResult.OK)
			return result;

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm && TBD_FrameworkManager.IsFrameworkWorld())
			sm.GetTickets().ConsumeSpawnAuthorization(TBD_PlayerIdOf(requestComponent), TBD_PossessTicketLedger.ResolveSpawnDataEntity(data));

		return result;
	}

	//! The gate: OK off a framework world; SPAWN_NOT_ALLOWED without a TBD_SpawnManager or without a
	//! ticket naming the requested body (the first refusal per ticket is logged).
	//! @authority server
	protected SCR_ESpawnResult TBD_GateRequest(SCR_SpawnRequestComponent requestComponent, SCR_SpawnData data)
	{
		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return SCR_ESpawnResult.OK;

		// The attribute lives in a packed prefab, so the server reports it once per session; the
		// gate does not depend on it. Logged here so a plain vanilla world stays silent.
		if (!m_bTbdIgnoreConditionsLogged)
		{
			m_bTbdIgnoreConditionsLogged = true;
			Print(string.Format("[TBD][Spawn] possess handler m_bIgnoreConditions=%1 (the TBD gate is independent of it)", m_bIgnoreConditions));
		}

		int playerId = TBD_PlayerIdOf(requestComponent);

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (!sm)
		{
			Print(string.Format("[TBD][Spawn] possess request REFUSED player=%1 -- framework world with no TBD_SpawnManager", playerId), LogLevel.ERROR);
			return SCR_ESpawnResult.SPAWN_NOT_ALLOWED;
		}

		// Entity-keyed: the ticket names the exact body TBD_SpawnManager put this player on, so a
		// client-supplied RplId pointing at somebody else's slot body matches nothing.
		IEntity target = TBD_PossessTicketLedger.ResolveSpawnDataEntity(data);

		bool logOnce;
		if (sm.GetTickets().IsSpawnAuthorizedFor(playerId, target, logOnce))
			return SCR_ESpawnResult.OK;

		if (logOnce)
			Print(string.Format("[TBD][Spawn] possess request REFUSED player=%1 target=%2 -- TBD_SpawnManager did not authorize this body (deploy goes through DeployPlayerEx)", playerId, target), LogLevel.WARNING);

		return SCR_ESpawnResult.SPAWN_NOT_ALLOWED;
	}

	//! The requesting player's id, or 0 without a request component.
	protected int TBD_PlayerIdOf(SCR_SpawnRequestComponent requestComponent)
	{
		if (!requestComponent)
			return 0;

		return requestComponent.GetPlayerId();
	}
}
