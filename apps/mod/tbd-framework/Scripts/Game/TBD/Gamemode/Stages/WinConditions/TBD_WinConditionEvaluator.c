/**
 * @file TBD_WinConditionEvaluator.c
 * @brief Reads the authored win rule once per world and ends the round on it exactly once.
 *
 * Role: gives `winConditions.mode` runtime meaning: `extraction` and `vip` are evaluated through
 * TBD_WinConditionModes; `attrition`, `objective` and `timeout` end through TBD_FactionElimination,
 * TBD_ObjectiveRegistry and TBD_RoundClock, and this class only warns when `endOn` cannot satisfy
 * them.  Position: TBD_RuntimeHeartbeat calls Clear on the way into a world and Tick every TICK_MS
 * until HasEnded; reads the mission through its own TBD_MissionJsonPass read into
 * TBD_WinConditionsStruct.  State: static rule, read flag and end latch, cleared on the way in
 * because statics outlive a world; server only (clients hold no mission document).
 * Invariants: the latch is set before SetStage(END), so a condition that stays true ends the round
 * once; no second round timer is started for `timeout`.
 */

//! The authored win rule of the loaded mission.
class TBD_WinConditionEvaluator
{
	static const string CH = "Win"; //!< log channel of every `[TBD][Win]` line
	static const int TICK_MS = 2000; //!< heartbeat period (ms), the same granularity as TBD_FactionElimination
	protected static const string ANNOUNCE_KEY = "Win.rule"; //!< TBD_AnnounceOnce key of the once-per-world rule report

	protected static ref TBD_WinConditionsStruct s_Rule; //!< the parsed rule; null when none was authored or before Read
	protected static bool s_bRead; //!< true once Read ran for this world
	protected static bool s_bEnded; //!< the end latch; true once this evaluator ended the round

	//! Reset for a new world; runs on the way in, so it does not depend on a tidy teardown.
	static void Clear()
	{
		s_Rule = null;
		s_bRead = false;
		s_bEnded = false;
		TBD_AnnounceOnce.Rearm(ANNOUNCE_KEY);
		TBD_AnnounceOnce.Rearm(TBD_WinConditionModes.VIP_MISSING_KEY);
	}

	//! The end latch.
	//! @return true once this evaluator ended the round
	static bool HasEnded()
	{
		return s_bEnded;
	}

	//! Parse the rule; only the first call after Clear does work.
	//! @return true when the mission authored a win rule
	static bool Read()
	{
		if (s_bRead)
			return s_Rule != null;

		s_bRead = true;
		s_Rule = null;

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
			return false;

		TBD_WinConditionDocStruct doc = new TBD_WinConditionDocStruct();
		if (!ctx.ReadValue("", doc))
			return false;

		// The JSON reader allocates the block even when absent; `mode` is schema-required inside it.
		if (!doc.winConditions || doc.winConditions.mode.IsEmpty())
			return false;

		s_Rule = doc.winConditions;
		return true;
	}

	//! The authored mode.
	//! @return the mode, or empty when the mission authored no win rule
	static string Mode()
	{
		if (!Read())
			return string.Empty;

		return s_Rule.mode;
	}

	//! One evaluation while LIVE; safe to call at any stage. Reports the rule once, then evaluates
	//! `extraction` or `vip` and ends the round when either fires.
	//! @authority server
	static void Tick()
	{
		if (s_bEnded)
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
			return;

		string mode = Mode();
		if (mode.IsEmpty())
			return;

		if (TBD_AnnounceOnce.Claim(ANNOUNCE_KEY))
			TBD_WinConditionModes.ReportRule(s_Rule);

		string winner;

		if (mode == TBD_WinConditionModes.MODE_EXTRACTION)
		{
			if (TBD_WinConditionModes.EvaluateExtraction(s_Rule, winner))
				EndRound("extraction", winner);
			return;
		}

		if (mode == TBD_WinConditionModes.MODE_VIP)
		{
			string reason;
			if (TBD_WinConditionModes.EvaluateVip(s_Rule, winner, reason))
				EndRound(reason, winner);
			return;
		}

		// attrition, objective, timeout and grandfathered golden modes end elsewhere.
	}

	//! End the round once: the latch is set before SetStage(END), so a re-entrant call cannot pass.
	//! @param reason the log label (`extraction`, `vip_down` or `vip_extracted`)
	//! @param winnerFaction the winning side, empty when none
	//! @authority server
	protected static void EndRound(string reason, string winnerFaction)
	{
		if (s_bEnded)
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		s_bEnded = true;

		string winnerLabel = winnerFaction;
		if (winnerLabel.IsEmpty())
			winnerLabel = "(none named)";

		TBD_Log.Event(CH, string.Format("[TBD][Win] %1 - winner=%2", reason, winnerLabel));
		fm.SetStage(TBD_EGameStage.END);
	}
}
