/**
 * @file TBD_RuntimeSessionLifecycle.c
 * @brief When the runtime session and the fleet command claims run: game start to world end.
 *
 * Role: starts `TBD_RuntimeSession` and `TBD_FleetCommandPoller` when the game starts on the
 * authority of a framework world, and stops them when that world ends.  Position: driven by the
 * modded `SCR_BaseGameMode` below, which self-wires without a component on `TBD_GameMode.et`.
 * State: the game mode's started flag, one per game mode instance.  Invariants: a vanilla
 * scenario running this mod holds no session (`IsFrameworkWorld`); a world starts its session at
 * most once, because a second start would supersede its own session; the session ends after the
 * game mode components have ended their player lives.
 */

//! Start and stop order of the runtime session and the fleet command claims.
//! @authority server
class TBD_RuntimeSessionLifecycle
{
	//! Whether this peer runs a runtime session: the authority of a framework world.
	//! @return true on the server of a framework world
	//! @authority server
	static bool RunsHere()
	{
		if (TBD_Authority.IsClient())
			return false;

		return TBD_FrameworkManager.IsFrameworkWorld();
	}

	//! Start the runtime session, then the fleet command claims.
	//! @authority server
	static void Begin()
	{
		TBD_RuntimeSession.Start();
		TBD_FleetCommandPoller.Start();
	}

	//! Stop the fleet command claims, then end the runtime session.
	//! @authority server
	static void End()
	{
		TBD_FleetCommandPoller.Stop();
		TBD_RuntimeSession.Stop();
	}
}

//! Game-mode hook of `TBD_RuntimeSessionLifecycle`.
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_RuntimeSessionStarted; //!< this game mode started the runtime session; default false

	//! Start the runtime session once per game mode instance on the authority of a framework world.
	//! @authority server
	protected override void OnGameStart()
	{
		super.OnGameStart();

		if (!TBD_RuntimeSessionLifecycle.RunsHere())
			return;

		// Once per game mode instance: a second start would supersede this world's own session.
		if (m_bTBD_RuntimeSessionStarted)
			return;

		m_bTBD_RuntimeSessionStarted = true;
		TBD_RuntimeSessionLifecycle.Begin();
	}

	//! The world is ending. The game mode components end their player lives inside super; the
	//! session ends after them.
	//! @authority server
	override void OnGameEnd()
	{
		super.OnGameEnd();

		if (!m_bTBD_RuntimeSessionStarted)
			return;

		TBD_RuntimeSessionLifecycle.End();
	}
}
