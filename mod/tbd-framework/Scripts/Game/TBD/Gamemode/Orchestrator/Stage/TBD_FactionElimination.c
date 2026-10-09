/**
 * @file TBD_FactionElimination.c
 * @brief Ends a LIVE round on an objective end trigger or when one fielded side is left alive.
 *
 * Role: the 2 s end check armed at LIVE: first TBD_ObjectiveRegistry.EvaluateEndTriggers, then
 * `faction_eliminated`; also the survivor count the END banner, the round clock and the match
 * results report use.
 * Position: owned by TBD_FrameworkManager, which calls Arm on entering LIVE; ends the round
 * through TBD_FrameworkManager.EndRound.  State: one call-queue poll; server only.
 * Invariants: a side that never claimed a slot is never eliminated; elimination needs at least two
 * contesting sides; the poll stops itself when the stage leaves LIVE or once it ends the round.
 */

//! Objective and elimination end check of one framework world.
class TBD_FactionElimination : Managed
{
	protected static const int TICK_MS = 2000; //!< poll period (ms); an elimination is not time-critical

	protected TBD_FrameworkManager m_Manager; //!< owning manager

	//! Bind the check to its manager.
	//! @param manager the owning framework manager
	void TBD_FactionElimination(TBD_FrameworkManager manager)
	{
		m_Manager = manager;
	}

	//! Stop the poll.
	void CancelCallbacks()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);
	}

	//! Start the 2 s poll when `winConditions.endOn` declares `faction_eliminated` or the end
	//! trigger of any objective kind; a mission that declares none runs until an admin ends it.
	//! @authority server
	void Arm()
	{
		bool anyTrigger = TBD_MissionLoader.HasEndTrigger(TBD_MissionFlow.TRIGGER_FACTION_ELIMINATED);
		for (int i = 0; i < TBD_ObjectiveKindBehaviour.Count(); i++)
		{
			TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.At(i);
			if (TBD_MissionLoader.HasEndTrigger(behaviour.EndTrigger()))
				anyTrigger = true;
		}

		if (!anyTrigger)
		{
			Print("[TBD][Win] no faction_eliminated trigger in mission -- round runs until admin ends it");
			return;
		}

		GetGame().GetCallqueue().Remove(Tick);
		GetGame().GetCallqueue().CallLater(Tick, TICK_MS, true);
	}

	//! Count the sides that claimed at least one slot and those of them with a living player: the
	//! one survivor rule of the end check, the END banner, the round clock and the match results
	//! report (TBD_ResultsReporter).
	//! @param winner set to the one surviving side's key, empty unless exactly one survives
	//! @param contesting set to the number of sides with a claimed slot
	//! @param stillAlive set to the number of those sides with a living player
	static void CountSurvivors(out string winner, out int contesting, out int stillAlive)
	{
		winner = string.Empty;
		contesting = 0;
		stillAlive = 0;

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		array<ref TBD_MissionFactionStruct> factions = TBD_MissionLoader.GetFactions();
		if (!sm || !factions)
			return;

		foreach (TBD_MissionFactionStruct faction : factions)
		{
			if (!faction || faction.key.IsEmpty())
				continue;

			if (sm.CountClaimedForFaction(faction.key) == 0)
				continue;

			contesting++;
			if (sm.CountAliveForFaction(faction.key) > 0)
			{
				stillAlive++;
				winner = faction.key;
			}
		}

		if (stillAlive != 1)
			winner = string.Empty;
	}

	//! One poll while LIVE: ends the round on the first objective end trigger, else when at least
	//! two sides fielded players and at most one still has a living player. Stops itself outside LIVE.
	//! @authority server
	protected void Tick()
	{
		if (m_Manager.GetStage() != TBD_EGameStage.LIVE)
		{
			GetGame().GetCallqueue().Remove(Tick);
			return;
		}

		// Objective triggers first; the registry gates each on HasEndTrigger and answers empty when
		// it never built.
		string objectiveWinner;
		string objectiveTrigger = TBD_ObjectiveRegistry.EvaluateEndTriggers(objectiveWinner);
		if (!objectiveTrigger.IsEmpty())
		{
			GetGame().GetCallqueue().Remove(Tick);
			PrintFormat("[TBD][Win] %1 -- winner=%2", objectiveTrigger, objectiveWinner);
			m_Manager.EndRound(objectiveTrigger, objectiveWinner);
			return;
		}

		string lastAlive;
		int contesting;
		int stillAlive;
		CountSurvivors(lastAlive, contesting, stillAlive);

		if (contesting < 2)
			return;

		if (stillAlive > 1)
			return;

		GetGame().GetCallqueue().Remove(Tick);
		Print(string.Format("[TBD][Win] faction_eliminated -- winner=%1 (%2 factions contested)",
			lastAlive, contesting));
		m_Manager.EndRound(TBD_MissionFlow.TRIGGER_FACTION_ELIMINATED, lastAlive);
	}
}
