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
//! below refuses, on a framework world, anything TBD_SpawnManager did not authorize.
//!
//! T-181.22 — CORRECTION TO THE ABOVE. It said "every spawn request on the server passes through
//! it". It does not. SCR_PossessSpawnHandlerComponent overrides CanRequestSpawn_S as
//! `m_bIgnoreConditions || super...` and that attribute ships defaulted to "1", so `||`
//! short-circuits and a POSSESS request never reaches this class — and possess is the ONLY
//! request type TBD_PlayerController.et leaves enabled. The possess route is now gated in
//! TBD_SCR_PossessSpawnHandlerComponent, at a call vanilla cannot skip; this file keeps every
//! other handler honest. Read the block on CanRequestSpawn_S below before changing either.
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
	//! On a framework world a request is refused unless TBD_SpawnManager issued it, for the exact
	//! entity it names. TBD_SpawnManager only issues one from DeployPlayerInternal, which is where
	//! ONE LIFE is enforced.
	//!
	//! ── T-181.22: WHAT THIS OVERRIDE DOES *NOT* COVER, AND WHY THAT IS NOW FINE ────────────────
	//! The chain above claims "every spawn request funnels through this one call". That was wrong
	//! for the one handler that matters. Vanilla SCR_PossessSpawnHandlerComponent.c:98-101 is:
	//!
	//!     override bool CanRequestSpawn_S(...)
	//!     { return m_bIgnoreConditions || super.CanRequestSpawn_S(requestComponent, data, result); }
	//!
	//! `||` short-circuits, and `m_bIgnoreConditions` ships defaulted to "1" — so for a POSSESS
	//! request `super` (and therefore this override) may never be reached. Since
	//! Prefabs/Systems/TBD_PlayerController.et disables the free-spawn and spawn-point request
	//! components (`Enabled 0`), POSSESS was BOTH the only live request type and the only one that
	//! could walk past this guard. That is why TBD_SCR_PossessSpawnHandlerComponent now gates the
	//! possess route at CanHandleRequest_S/HandleRequest_S, which cannot be short-circuited.
	//!
	//! This override remains load-bearing for every OTHER handler (free spawn, spawn point, and
	//! anything a future prefab re-enables): none of them override CanRequestSpawn_S, so they all
	//! still funnel through here. It also still covers POSSESS if `m_bIgnoreConditions` turns out
	//! to be 0 — belt and braces, and neither depends on knowing which.
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
			if (!TBD_IsSpawnAuthorized(requestComponent, data))
			{
				result = SCR_ESpawnResult.SPAWN_NOT_ALLOWED;
				return false;
			}
		}

		return super.CanRequestSpawn_S(requestComponent, handlerComponent, data, result);
	}

	//------------------------------------------------------------------------------------------------
	//! Did TBD_SpawnManager authorize this specific request, for this specific entity?
	//!
	//! T-181.22 — the entity half is new, and it is what makes a refusal here meaningful rather
	//! than decorative. A handler whose SCR_SpawnData names no entity (free spawn, spawn point)
	//! resolves to null, and null never matches a ticket — correct, because TBD only ever deploys
	//! through POSSESS, so a non-possess request on a framework world is by definition not ours.
	//!
	//! Fails CLOSED. A framework world whose spawn manager is missing must not quietly fall back
	//! to vanilla spawning — that is the double-spawn/vanilla-kit class this mod exists to
	//! prevent — so "cannot ask" is refused, loudly, rather than allowed.
	protected bool TBD_IsSpawnAuthorized(SCR_SpawnRequestComponent requestComponent, SCR_SpawnData data)
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

		IEntity target = TBD_SpawnManager.ResolveSpawnDataEntity(data);

		bool logOnce;
		if (sm.IsSpawnAuthorizedFor(playerId, target, logOnce))
			return true;

		if (logOnce)
			Print(string.Format("[TBD][Spawn] vanilla spawn request REFUSED player=%1 target=%2 — TBD_SpawnManager did not authorize it (deploy goes through DeployPlayerEx)", playerId, target), LogLevel.WARNING);

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
