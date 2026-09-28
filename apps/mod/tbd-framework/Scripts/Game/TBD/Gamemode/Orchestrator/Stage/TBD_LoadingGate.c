/**
 * @file TBD_LoadingGate.c
 * @brief Carries the round from LOADING to LOBBY once the mission, roster and loadouts are settled.
 *
 * Role: polls the mission load, puts the valid document into force (its entities, settings,
 * environment and gadget hook through TBD_MissionWorldApplier, then its flow, weather and latched
 * settings), loads the registry, materializes the slot bodies, then waits for the event roster and
 * the loadout settle before asking for LOBBY.  Position: owned by TBD_FrameworkManager, which calls
 * Begin from OnPostInit on the server; reads TBD_MissionLoader, TBD_RosterLoader and
 * TBD_SpawnManager.
 * State: the roster settle tick counter and two call-queue polls; server only.
 * Invariants: the valid document is put into force only here, on the main thread from the call
 * queue, once per load and after the world has created its entities, never from the parse or a
 * request's answer; LOBBY is requested exactly once per load; an invalid mission stays in LOADING;
 * the roster force-settles after 4 ticks (2 s) and the loadout settle is waited on for up to 24
 * ticks.
 */

//! LOADING to LOBBY gate of one framework world.
class TBD_LoadingGate : Managed
{
	protected TBD_FrameworkManager m_Manager; //!< owning manager
	protected int m_iRosterSettleTicks; //!< roster settle polls elapsed (500 ms each)

	//! Bind the gate to its manager.
	//! @param manager the owning framework manager
	void TBD_LoadingGate(TBD_FrameworkManager manager)
	{
		m_Manager = manager;
	}

	//! Start polling the mission load once a second.
	//! @authority server
	void Begin()
	{
		GetGame().GetCallqueue().CallLater(TickLoading, 1000, true);
	}

	//! Stop both polls.
	void CancelCallbacks()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		queue.Remove(TickLoading);
		queue.Remove(TickRosterSettle);
	}

	//! One load poll: once the mission is loaded and valid, put it into force and start the roster
	//! settle. Stops itself when the stage has left LOADING.
	//! @authority server
	protected void TickLoading()
	{
		if (m_Manager.GetStage() != TBD_EGameStage.LOADING)
		{
			GetGame().GetCallqueue().Remove(TickLoading);
			return;
		}

		if (!TBD_MissionLoader.IsLoaded())
			return;

		if (!TBD_MissionLoader.IsValid())
		{
			Print("[TBD] Mission loaded but invalid -- staying in LOADING.", LogLevel.ERROR);
			return;
		}

		GetGame().GetCallqueue().Remove(TickLoading);

		// The document's own world effects before anything that reads them: the authored wind
		// direction below overrides the environment, and the slot bodies claim the placed vehicles.
		TBD_MissionWorldApplier.Apply();

		// Flow next: `flow.safeStartSeconds` reaches the safe start while SAFE_START cannot yet run.
		TBD_MissionFlowReport.Apply();
		TBD_StageEnvironment.ApplyAuthoredWeather();
		m_Manager.LatchAuthoredSettings();

		TBD_Registry.Load();

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm)
			sm.MaterializeSlotBodies();

		// The roster settles before LOBBY, so slot assignment reads settled state and the 250 ms
		// deploy pass cannot race the roster fetch.
		TBD_RosterLoader.BeginLoad();
		m_iRosterSettleTicks = 0;
		GetGame().GetCallqueue().CallLater(TickRosterSettle, 500, true);
	}

	//! One roster settle poll (500 ms): force-settles the roster at the 2 s deadline, waits while
	//! the slot loadouts settle, then asks for LOBBY once. SetStage refuses LOBBY through
	//! TBD_SpawnManager.StageRefusalFor while loadout delivery is pending or refused.
	//! @authority server
	protected void TickRosterSettle()
	{
		m_iRosterSettleTicks++;

		if (!TBD_RosterLoader.IsLoaded() && m_iRosterSettleTicks < 4)
			return;

		if (!TBD_RosterLoader.IsLoaded())
			TBD_RosterLoader.ForceSettle();

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		// The loadout settle can outlast the 2 s roster deadline; it times out on its own and refuses.
		if (sm && sm.IsLoadoutSettlePending() && m_iRosterSettleTicks < 24)
			return;

		GetGame().GetCallqueue().Remove(TickRosterSettle);

		Print(string.Format("[TBD][Spawn] roster settled=%1 assignments=%2",
			TBD_RosterLoader.GetSettleReason(), TBD_RosterLoader.GetAssignmentCount()));

		m_Manager.SetStage(TBD_EGameStage.LOBBY);
	}
}
