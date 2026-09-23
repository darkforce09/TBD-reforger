//! When the runtime session runs, and with it the fleet command claims: from the moment the game
//! starts on the authority of a framework world until that world ends (TBD_RuntimeSession and
//! TBD_FleetCommandPoller, Start / Stop).
//!
//! A modded game mode self-wires without a component on `TBD_GameMode.et`, the established idiom of
//! this addon, and `IsFrameworkWorld` fences it so a vanilla scenario running this mod holds no
//! session.
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_RuntimeSessionStarted;

	//------------------------------------------------------------------------------------------------
	//! @authority server - the session belongs to the authority of a framework world.
	protected override void OnGameStart()
	{
		super.OnGameStart();

		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		// Once per game mode instance: a second start would supersede this world's own session.
		if (m_bTBD_RuntimeSessionStarted)
			return;

		m_bTBD_RuntimeSessionStarted = true;
		TBD_RuntimeSession.Start();
		TBD_FleetCommandPoller.Start();
	}

	//------------------------------------------------------------------------------------------------
	//! The world is ending. The game mode components end their player lives inside super; the
	//! session ends after them.
	override void OnGameEnd()
	{
		super.OnGameEnd();

		if (!m_bTBD_RuntimeSessionStarted)
			return;

		TBD_FleetCommandPoller.Stop();
		TBD_RuntimeSession.Stop();
	}
}
