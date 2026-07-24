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

	// NOTE: IsRespawnEnabled()/IsFactionChangeAllowed() are deliberately NOT overridden.
	// Reporting respawn "off" reads well but makes the authority reject our own possess
	// request (CanRequestSpawn_S consults it), and that request is how the player takes
	// over its slot body through the vanilla pipeline — the only path that fires the
	// client-side spawn finalize and lets go of the loading screen. Suppressing
	// registration above is what stops the faction hunt; the policy getters are not
	// needed for it, and the harness asserts the churn stays dead.
}
