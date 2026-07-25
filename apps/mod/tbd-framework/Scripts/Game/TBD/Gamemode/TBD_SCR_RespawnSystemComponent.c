//! Stands the vanilla respawn system down on framework worlds.
//!
//! Measured defect (2026-07-24 session logs): with slot bodies replacing spawn points,
//! the vanilla respawn/spawn-logic flow registers the player, finds nothing it can spawn
//! him on, and re-rolls his faction roughly once a second forever — 138 engine
//! "has switched from faction" lines cycling US/USSR/FIA/CIV in one session, each new
//! faction dragging a PlayableGroup + "not found in SCR_AIWorld" warning behind it, while
//! the player sits on the deploy/loading screen even though TBD_SpawnManager had already
//! bound him to his body. The spawn never finalizes because SetInitialMainEntity bypasses
//! the vanilla state machine, so the hunt never ends.
//!
//! Fix (PlayableSelector's design, mirrored — never its code): the framework decides who
//! spawns where, so vanilla's own player registration must never start. Registration and
//! audit are swallowed here, which is what ends the faction hunt (measured: 138 switch
//! lines → 0). Deploy itself still goes THROUGH vanilla, via the possess spawn request in
//! TBD_SpawnManager — that route takes over an existing body rather than creating one, so
//! it cannot resurrect the double-spawn, and it runs the finalize the client waits on.
//!
//! T-181.21 — the two registration overrides only covered the JOIN doors. The DEATH door
//! (OnPlayerKilled_S -> ... -> NotifyReadyForSpawn_S -> a client spawn request) was still wide
//! open, which meant the ONE LIFE invariant could be walked straight around. CanRequestSpawn_S
//! below is the answer: every spawn request on the server passes through it, and on a framework
//! world it now refuses anything TBD_SpawnManager did not authorize.
//!
//! Every override is GUARDED on TBD_FrameworkManager.IsFrameworkWorld(): this mod is loaded
//! world-globally, and on a plain vanilla world the whole class must behave as if absent.
modded class SCR_RespawnSystemComponent
{
	//! Tri-state cache of the framework guard: 0 unresolved, 1 framework world, 2 vanilla.
	//! Resolved lazily — the game mode is not reliably reachable at component init time.
	protected int m_iTbdManaged;
	protected bool m_bTbdSuppressionLogged;

	//------------------------------------------------------------------------------------------------
	protected bool TBD_IsManaged()
	{
		if (m_iTbdManaged == 0)
		{
			if (!GetGame().GetGameMode())
				return false;  // Too early to decide — ask again next call.

			if (TBD_FrameworkManager.IsFrameworkWorld())
				m_iTbdManaged = 1;
			else
				m_iTbdManaged = 2;
		}

		return m_iTbdManaged == 1;
	}

	//------------------------------------------------------------------------------------------------
	protected void TBD_LogSuppressionOnce()
	{
		if (m_bTbdSuppressionLogged)
			return;
		m_bTbdSuppressionLogged = true;
		Print("[TBD][Spawn] vanilla respawn system suppressed (framework world)");
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the registration that hands a player to the spawn logic (and
	//! opens the vanilla deploy flow). Swallowed on framework worlds: TBD_SpawnManager
	//! deploys from the stage machine instead.
	override void OnPlayerRegistered_S(int playerId)
	{
		if (TBD_IsManaged())
		{
			TBD_LogSuppressionOnce();
			return;
		}

		super.OnPlayerRegistered_S(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — audit success is the second door into the same flow.
	override void OnPlayerAuditSuccess_S(int playerId)
	{
		if (TBD_IsManaged())
		{
			TBD_LogSuppressionOnce();
			return;
		}

		super.OnPlayerAuditSuccess_S(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — THE DEATH DOOR. Do not remove this; read why it exists first.
	//!
	//! The two overrides above only stand down the JOIN doors. Vanilla has a second, entirely
	//! separate route back into the world that they never touched:
	//!
	//!     SCR_BaseGameMode.OnPlayerKilled
	//!       -> SCR_RespawnSystemComponent.OnPlayerKilled_S            (vanilla src :209-212)
	//!         -> SCR_SpawnLogic.OnPlayerKilled_S                      (vanilla src :178-186)
	//!           -> OnPlayerEntityLost_S
	//!             -> SCR_MenuSpawnLogic.OnPlayerEntityLost_S          (vanilla src :42-46)
	//!               -> SCR_RespawnComponent.NotifyReadyForSpawn_S     -- "you may spawn now"
	//!                 -> client asks -> SCR_SpawnRequestComponent.RequestRespawn
	//!                   -> ProcessRequest_S -> SCR_SpawnHandlerComponent.HandleRequest_S
	//!                     -> CanHandleRequest_S -> CanRequestSpawn_S  <-- WE ARE HERE
	//!
	//! Every spawn request on the server funnels through this one call — death-driven, JIP,
	//! deploy menu, diag menu, and our own possess request. So instead of chasing each chain
	//! (which is how the guard ended up on ClaimSlot/ReleaseSlot, functions that cannot put
	//! anybody in the world), the rule is inverted: on a framework world a request is refused
	//! unless TBD_SpawnManager issued it. TBD_SpawnManager only issues one from DeployPlayerEx,
	//! which is where ONE LIFE is enforced — so the invariant is checked once, in one place, and
	//! this closes every route around it.
	//!
	//! Why not just override IsRespawnEnabled() to false? Because vanilla :149 makes it the
	//! blanket gate on CanRequestSpawn_S, so it would also reject OUR possess request — the only
	//! route that runs the client-side finalize and releases the loading screen. That is why the
	//! note below still says leave it alone. This override is the per-request version of the
	//! same idea: policy, not a blanket switch.
	//! @authority server
	override bool CanRequestSpawn_S(SCR_SpawnRequestComponent requestComponent, SCR_SpawnHandlerComponent handlerComponent, SCR_SpawnData data, out SCR_ESpawnResult result = SCR_ESpawnResult.SPAWN_NOT_ALLOWED)
	{
		if (TBD_IsManaged())
		{
			if (!TBD_IsSpawnAuthorized(requestComponent))
			{
				result = SCR_ESpawnResult.SPAWN_NOT_ALLOWED;
				return false;
			}
		}

		return super.CanRequestSpawn_S(requestComponent, handlerComponent, data, result);
	}

	//------------------------------------------------------------------------------------------------
	//! Did TBD_SpawnManager authorize this specific request?
	//!
	//! Fails CLOSED. A framework world whose spawn manager is missing must not quietly fall back
	//! to vanilla spawning — that is the double-spawn/vanilla-kit class this mod exists to
	//! prevent — so "cannot ask" is refused, loudly, rather than allowed.
	protected bool TBD_IsSpawnAuthorized(SCR_SpawnRequestComponent requestComponent)
	{
		if (!requestComponent)
			return false;

		int playerId = requestComponent.GetPlayerId();

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (!sm)
		{
			Print(string.Format("[TBD][Spawn] vanilla spawn request REFUSED player=%1 — framework world with no TBD_SpawnManager", playerId), LogLevel.ERROR);
			return false;
		}

		bool logOnce;
		if (sm.IsSpawnAuthorized(playerId, logOnce))
			return true;

		if (logOnce)
			Print(string.Format("[TBD][Spawn] vanilla spawn request REFUSED player=%1 — TBD_SpawnManager did not authorize it (deploy goes through DeployPlayerEx)", playerId), LogLevel.WARNING);

		return false;
	}

	// NOTE: IsRespawnEnabled()/IsFactionChangeAllowed() are deliberately NOT overridden.
	// Reporting respawn "off" reads well but makes the authority reject our own possess
	// request (CanRequestSpawn_S consults it), and that request is how the player takes
	// over its slot body through the vanilla pipeline — the only path that fires the
	// client-side spawn finalize and lets go of the loading screen. Suppressing
	// registration above is what stops the faction hunt; the policy getters are not
	// needed for it, and the harness asserts the churn stays dead.
}
