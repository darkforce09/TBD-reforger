/**
 * @file TBD_RoundClock.c
 * @brief The authored round clock: arms at LIVE, warns in chat, ends the round when it expires.
 *
 * Role: counts `flow.timeLimitSeconds` down once a second from LIVE and ends the round with reason
 * `time_limit`.  Position: owned by TBD_FrameworkManager, which calls Arm on entering LIVE; reads
 * TBD_MissionFlow; ends the round through TBD_FrameworkManager.EndRound.
 * State: the seconds remaining and one call-queue poll; server only, not replicated (players are
 * told through chat).  Invariants: arms only when `winConditions.endOn` declares `time_limit` and
 * the duration is authored and above 0, each mismatch logged; disarms itself the moment the stage
 * leaves LIVE; the one round clock of a world (TBD_WinConditionEvaluator starts no second one).
 */

//! Authored round clock of one framework world.
class TBD_RoundClock : Managed
{
	protected static const int ROUND_CLOCK_OFF = -1; //!< clock not running; negative so it never reads as "about to expire"

	protected TBD_FrameworkManager m_Manager; //!< owning manager
	protected int m_iRoundSecondsRemaining = ROUND_CLOCK_OFF; //!< seconds left, or ROUND_CLOCK_OFF

	//! Bind the clock to its manager.
	//! @param manager the owning framework manager
	void TBD_RoundClock(TBD_FrameworkManager manager)
	{
		m_Manager = manager;
	}

	//! Stop the clock.
	void CancelCallbacks()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(TickRoundClock);
	}

	//! Start the clock at LIVE. It arms only when `winConditions.endOn` declares `time_limit` and
	//! `flow.timeLimitSeconds` is authored above 0: a duration without the trigger, or the trigger
	//! without a duration, is a WARNING, and an authored `0` is logged as an explicit no-limit.
	//! @authority server
	void Arm()
	{
		m_iRoundSecondsRemaining = ROUND_CLOCK_OFF;
		GetGame().GetCallqueue().Remove(TickRoundClock);

		int limit = TBD_MissionFlow.TimeLimitSeconds();
		bool declared = TBD_MissionLoader.HasEndTrigger(TBD_MissionFlow.TRIGGER_TIME_LIMIT);

		if (!declared)
		{
			if (limit > 0)
			{
				TBD_Log.Warn(TBD_MissionFlow.CH_FLOW, string.Format(
					"flow.timeLimitSeconds=%1 is authored but winConditions.endOn does not declare 'time_limit' -- clock NOT armed, this round will not end on time.",
					limit));
			}
			return;
		}

		if (limit == TBD_MissionFlow.UNSET)
		{
			TBD_Log.Warn(TBD_MissionFlow.CH_FLOW,
				"winConditions.endOn declares 'time_limit' but flow.timeLimitSeconds is not authored -- this round CANNOT end on time.");
			return;
		}

		if (limit == 0)
		{
			TBD_Log.Event(TBD_MissionFlow.CH_FLOW,
				"flow.timeLimitSeconds=0 (authored) -- an explicit NO LIMIT; the round will not end on time.");
			return;
		}

		m_iRoundSecondsRemaining = limit;
		GetGame().GetCallqueue().CallLater(TickRoundClock, 1000, true);

		string clock = TBD_ClockText.FormatClock(limit);
		TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "time_limit",
			string.Format("armed seconds=%1 (%2) at stage=LIVE", limit, clock));

		string msg = "[TBD] ROUND TIME LIMIT: ";
		msg += clock;
		msg += ". The round ends when it expires.";
		TBD_PlayerChat.Broadcast(TBD_MissionFlow.CH_FLOW, msg);
	}

	//! One 1 Hz tick: warns at the round clock milestones and ends the round at zero. Disarms itself
	//! when the stage leaves LIVE, so an early end or restart leaves no clock running.
	//! @authority server
	protected void TickRoundClock()
	{
		if (m_Manager.GetStage() != TBD_EGameStage.LIVE)
		{
			GetGame().GetCallqueue().Remove(TickRoundClock);
			m_iRoundSecondsRemaining = ROUND_CLOCK_OFF;
			return;
		}

		m_iRoundSecondsRemaining--;

		if (m_iRoundSecondsRemaining > 0)
		{
			if (TBD_ClockText.IsRoundClockMilestone(m_iRoundSecondsRemaining))
			{
				string warn = "[TBD] ROUND TIME REMAINING: ";
				warn += TBD_ClockText.FormatClock(m_iRoundSecondsRemaining);
				warn += ".";
				TBD_PlayerChat.Broadcast(TBD_MissionFlow.CH_FLOW, warn);
			}
			return;
		}

		GetGame().GetCallqueue().Remove(TickRoundClock);
		m_iRoundSecondsRemaining = ROUND_CLOCK_OFF;

		// The `[TBD][Win]` prefix every end rule logs under.
		Print("[TBD][Win] time_limit -- authored round clock expired");
		TBD_PlayerChat.Broadcast(TBD_MissionFlow.CH_FLOW, "[TBD] TIME. The round is over.");

		string clockWinner;
		int clockContesting;
		int clockAlive;
		TBD_FactionElimination.CountSurvivors(clockWinner, clockContesting, clockAlive);
		m_Manager.EndRound(TBD_MissionFlow.TRIGGER_TIME_LIMIT, clockWinner);

		// The clock has disarmed itself, so a refused END is reported rather than left looking ended.
		if (m_Manager.GetStage() != TBD_EGameStage.END)
		{
			TBD_Log.Error(TBD_MissionFlow.CH_FLOW,
				"time limit expired but the END transition was REFUSED -- the round is still running: " + m_Manager.GetLastStageRefusal());
		}
	}
}
