//! T-181.22 — THE POSSESS DOOR. This file exists because the T-181.21 backstop never ran.
//!
//! WHAT WENT WRONG, from vanilla source (SCR_PossessSpawnHandlerComponent.c:8-9, :98-101):
//!
//!     [Attribute("1", desc: "When enabled, conditions like respawn time will not be checked.")]
//!     protected bool m_bIgnoreConditions;
//!
//!     override bool CanRequestSpawn_S(...)
//!     {
//!         return m_bIgnoreConditions || super.CanRequestSpawn_S(requestComponent, data, result);
//!     }
//!
//! `||` short-circuits. With the attribute at its shipped default of "1", `super` is never
//! called — so `SCR_SpawnHandlerComponent.CanRequestSpawn_S` (:148-156) never asks
//! `SCR_RespawnSystemComponent.CanRequestSpawn_S`, and TBD's override of THAT
//! (TBD_SCR_RespawnSystemComponent) never sees a possess request. The one-life backstop
//! T-181.21 shipped was, for this handler, dead code.
//!
//! WHY THAT MATTERED. Prefabs/Systems/TBD_PlayerController.et disables
//! SCR_FreeSpawnRequestComponent and SCR_SpawnPointRespawnRequestComponent (`Enabled 0`), so
//! POSSESS is the only live request type on a framework world — i.e. the only route around the
//! backstop was also the only route anybody uses. And
//! SCR_PossessSpawnRequestComponent.Rpc_RequestRespawn_S is `[RplRpc(..., RplRcver.Server)]`
//! taking a client-supplied RplId, while vanilla's own checks (:33-58) only require the target
//! to be an ALIVE, UNCONTROLLED ChimeraCharacter — which is exactly what every un-deployed TBD
//! slot body is. A modded client could therefore possess any slot body it liked: a one-life
//! bypass and a slot-theft vector in one call.
//!
//! WHY `CanHandleRequest_S` / `HandleRequest_S` AND NOT `CanRequestSpawn_S`. Overriding
//! `CanRequestSpawn_S` here would work only while `m_bIgnoreConditions` is false, and its value
//! cannot be read: it is set on GameMode_Plain.et, which ships inside a compressed pak (grep for
//! "IgnoreConditions" across data*.pak = 0 hits; "GameMode_Plain" appears only as a path string).
//! So the fix must not depend on it. `CanHandleRequest_S` is the choke point that cannot be
//! skipped — vanilla routes BOTH the ask (SCR_SpawnRequestComponent.ProcessCanRequest_S:208)
//! and the real request (ProcessRequest_S -> HandleRequest_S:71) through it, unconditionally,
//! before anything is spawned or possessed.
//!
//! Guarded on TBD_FrameworkManager.IsFrameworkWorld(): this mod loads world-globally, and on a
//! plain vanilla world the class must behave exactly as if this file did not exist.
modded class SCR_PossessSpawnHandlerComponent
{
	//! One-time latch for the m_bIgnoreConditions report below.
	protected bool m_bTbdIgnoreConditionsLogged;

	//------------------------------------------------------------------------------------------------
	//! @authority server — vanilla asks this before it will spawn/possess anything, from both
	//! ProcessCanRequest_S ("may I?") and HandleRequest_S ("do it"). Non-consuming: answering the
	//! "may I?" probe must never spend the ticket the real request still needs.
	override SCR_ESpawnResult CanHandleRequest_S(SCR_SpawnRequestComponent requestComponent, SCR_SpawnData data)
	{
		SCR_ESpawnResult gate = TBD_GateRequest(requestComponent, data);
		if (gate != SCR_ESpawnResult.OK)
			return gate;

		return super.CanHandleRequest_S(requestComponent, data);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the request that actually hands the body over. Gated again (cheap, and
	//! it means neither entry point trusts the other), then the ticket is SPENT — but only once
	//! vanilla reports OK, so a request that fails vanilla's own checks does not burn the ticket
	//! belonging to the deploy that is still in flight.
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
			sm.ConsumeSpawnAuthorization(TBD_PlayerIdOf(requestComponent), TBD_SpawnManager.ResolveSpawnDataEntity(data));

		return result;
	}

	//------------------------------------------------------------------------------------------------
	//! The gate itself. FAILS CLOSED on a framework world: a missing TBD_SpawnManager means the
	//! authority cannot tell an authorized deploy from a forged RPC, and "cannot tell" must never
	//! resolve to "allow" for the invariant the design doc calls non-negotiable.
	//! @authority server
	protected SCR_ESpawnResult TBD_GateRequest(SCR_SpawnRequestComponent requestComponent, SCR_SpawnData data)
	{
		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return SCR_ESpawnResult.OK;

		// Report the attribute whose short-circuit caused all this, once, on the first request of
		// a framework session. The packed prefab cannot be read offline (see the header), so the
		// honest answer is to have the server say it out loud on a real run. The gate does NOT
		// depend on the value — this only tells an operator whether the vanilla condition chain is
		// live underneath it. Logged here rather than in OnPostInit so a plain vanilla world stays
		// completely silent, as the header promises.
		if (!m_bTbdIgnoreConditionsLogged)
		{
			m_bTbdIgnoreConditionsLogged = true;
			Print(string.Format("[TBD][Spawn] possess handler m_bIgnoreConditions=%1 (the TBD gate is independent of it)", m_bIgnoreConditions));
		}

		int playerId = TBD_PlayerIdOf(requestComponent);

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (!sm)
		{
			Print(string.Format("[TBD][Spawn] possess request REFUSED player=%1 — framework world with no TBD_SpawnManager", playerId), LogLevel.ERROR);
			return SCR_ESpawnResult.SPAWN_NOT_ALLOWED;
		}

		// Entity-keyed: the ticket names the exact body TBD_SpawnManager put this player on, so a
		// client-supplied RplId pointing at somebody else's slot body matches nothing.
		IEntity target = TBD_SpawnManager.ResolveSpawnDataEntity(data);

		bool logOnce;
		if (sm.IsSpawnAuthorizedFor(playerId, target, logOnce))
			return SCR_ESpawnResult.OK;

		if (logOnce)
			Print(string.Format("[TBD][Spawn] possess request REFUSED player=%1 target=%2 — TBD_SpawnManager did not authorize this body (deploy goes through DeployPlayerEx)", playerId, target), LogLevel.WARNING);

		return SCR_ESpawnResult.SPAWN_NOT_ALLOWED;
	}

	//------------------------------------------------------------------------------------------------
	//! Vanilla dereferences requestComponent.GetPlayerController() without a null check; we do not.
	protected int TBD_PlayerIdOf(SCR_SpawnRequestComponent requestComponent)
	{
		if (!requestComponent)
			return 0;

		return requestComponent.GetPlayerId();
	}
}
