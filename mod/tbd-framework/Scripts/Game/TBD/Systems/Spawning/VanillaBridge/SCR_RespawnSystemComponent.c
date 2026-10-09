/**
 * @file SCR_RespawnSystemComponent.c
 * @brief Vanilla's respawn system stood down on a framework world, and every spawn request gated.
 *
 * Role: swallows vanilla player registration and audit (which otherwise re-roll a player's faction
 * forever looking for a spawn point) and refuses every spawn request TBD_SpawnManager did not
 * issue.  Position: vanilla respawn system component of the game mode; tickets come from
 * TBD_PossessTicketLedger; deploys run through vanilla's POSSESS request from TBD_DeployExecutor.
 * State: the lazily resolved framework-world flag and a one-time suppression log latch.
 * Invariants: on a plain vanilla world (the mod loads world-globally) every override behaves as
 * vanilla; the request gate fails closed without a TBD_SpawnManager; a request whose data names no
 * entity never matches a ticket; IsRespawnEnabled and IsFactionChangeAllowed stay vanilla, because
 * CanRequestSpawn_S consults them and would reject the framework's own POSSESS request.
 */

//! Framework stand-down of vanilla's respawn system.
modded class SCR_RespawnSystemComponent
{
	protected int m_iTbdManaged; //!< framework guard cache: 0 unresolved, 1 framework world, 2 vanilla
	protected bool m_bTbdSuppressionLogged; //!< latch for the one suppression log line

	//! True on a framework world; resolved lazily, because the game mode is not reliably reachable
	//! at component init time (false until it is).
	protected bool TBD_IsManaged()
	{
		if (m_iTbdManaged == 0)
		{
			if (!GetGame().GetGameMode())
				return false;  // Too early to decide -- ask again next call.

			if (TBD_FrameworkManager.IsFrameworkWorld())
				m_iTbdManaged = 1;
			else
				m_iTbdManaged = 2;
		}

		return m_iTbdManaged == 1;
	}

	//! Log once that the vanilla respawn system is suppressed.
	protected void TBD_LogSuppressionOnce()
	{
		if (m_bTbdSuppressionLogged)
			return;
		m_bTbdSuppressionLogged = true;
		Print("[TBD][Spawn] vanilla respawn system suppressed (framework world)");
	}

	//! Tear down vanilla's loading placeholder on a framework world. Vanilla builds it as a raw
	//! workspace widget above every menu and mutes SFX, and only its own spawn pipeline (which a
	//! framework world never uses) destroys it. The game mode prefab also blanks the layout; this
	//! call, idempotent and the one that resumes SFX, covers a world where that prefab setting is
	//! missing. Runs after super, which sets up the spawn logic. The guard reads the owner, because
	//! the game mode is not reliably reachable at init time.
	override void OnInit(IEntity owner)
	{
		super.OnInit(owner);

		if (!owner || !owner.FindComponent(TBD_FrameworkManager))
			return;

		DestroyLoadingPlaceholder();
		Print("[TBD][Spawn] vanilla loading placeholder torn down (framework world) -- it is a raw workspace widget that outranks every menu, and nothing on our spawn path would ever have destroyed it");
	}

	//! The registration that hands a player to the vanilla spawn logic; swallowed on a framework
	//! world, where TBD_SpawnManager deploys instead.
	//! @authority server
	override void OnPlayerRegistered_S(int playerId)
	{
		if (TBD_IsManaged())
		{
			TBD_LogSuppressionOnce();
			return;
		}

		super.OnPlayerRegistered_S(playerId);
	}

	//! Audit success, the second door into the same vanilla flow; swallowed on a framework world.
	//! @authority server
	override void OnPlayerAuditSuccess_S(int playerId)
	{
		if (TBD_IsManaged())
		{
			TBD_LogSuppressionOnce();
			return;
		}

		super.OnPlayerAuditSuccess_S(playerId);
	}

	//! Refuse, on a framework world, every spawn request TBD_SpawnManager did not issue for the
	//! entity it names. Every handler that does not override CanRequestSpawn_S funnels through
	//! here, including the death route (OnPlayerKilled_S to NotifyReadyForSpawn_S to a client
	//! request); the possess handler is gated separately at CanHandleRequest_S, because vanilla
	//! short-circuits its CanRequestSpawn_S while m_bIgnoreConditions is set.
	//! @authority server
	override bool CanRequestSpawn_S(SCR_SpawnRequestComponent requestComponent, SCR_SpawnHandlerComponent handlerComponent, SCR_SpawnData data, out SCR_ESpawnResult result = SCR_ESpawnResult.SPAWN_NOT_ALLOWED)
	{
		if (TBD_IsManaged())
		{
			if (!TBD_IsSpawnAuthorized(requestComponent, data))
			{
				result = SCR_ESpawnResult.SPAWN_NOT_ALLOWED;
				return false;
			}
		}

		return super.CanRequestSpawn_S(requestComponent, handlerComponent, data, result);
	}

	//! True when a spawn ticket covers this request's player and entity. Fails closed (logged
	//! ERROR) without a TBD_SpawnManager; logs the first refusal per ticket.
	//! @authority server
	protected bool TBD_IsSpawnAuthorized(SCR_SpawnRequestComponent requestComponent, SCR_SpawnData data)
	{
		if (!requestComponent)
			return false;

		int playerId = requestComponent.GetPlayerId();

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (!sm)
		{
			Print(string.Format("[TBD][Spawn] vanilla spawn request REFUSED player=%1 -- framework world with no TBD_SpawnManager", playerId), LogLevel.ERROR);
			return false;
		}

		IEntity target = TBD_PossessTicketLedger.ResolveSpawnDataEntity(data);

		bool logOnce;
		if (sm.GetTickets().IsSpawnAuthorizedFor(playerId, target, logOnce))
			return true;

		if (logOnce)
			Print(string.Format("[TBD][Spawn] vanilla spawn request REFUSED player=%1 target=%2 -- TBD_SpawnManager did not authorize it (deploy goes through DeployPlayerEx)", playerId, target), LogLevel.WARNING);

		return false;
	}
}
