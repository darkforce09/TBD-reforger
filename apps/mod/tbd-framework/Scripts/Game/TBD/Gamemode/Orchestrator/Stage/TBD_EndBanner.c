/**
 * @file TBD_EndBanner.c
 * @brief Decides the END banner's winner and reason, counts kills, and packs the debrief board.
 *
 * Role: holds the winner and reason an end rule named, infers them when an admin ended the round,
 * credits player kills while LIVE, packs the scoreboard, and opens the local END and DEBRIEF
 * overlays.  Position: owned by TBD_FrameworkManager, which copies the results into its replicated
 * end fields on entering END and DEBRIEF; read by TBD_DebriefScoreboard through GetKills.
 * State: the pending winner and reason and the per-player kill map; server only, not replicated.
 * Invariants: a pending reason is consumed once, on the way into END; without one the reason is
 * the objective trigger, else `faction_eliminated` when one of two or more sides survives, else
 * `admin`; LOADING and LOBBY clear everything.
 */

//! END banner decision and kill tally of one framework world.
class TBD_EndBanner : Managed
{
	protected string m_sPendingWinner; //!< winner named by the rule that ends the round; empty when none
	protected string m_sPendingReason; //!< reason named by the rule that ends the round; empty when none
	protected ref map<int, int> m_mKills = new map<int, int>(); //!< playerId -> kills this round

	//! Open or close this machine's END and DEBRIEF overlays for `stage`. Local widget work only; a
	//! machine without a workspace does nothing.
	//! @param stage the current round stage
	static void ApplyEndScreens(TBD_EGameStage stage)
	{
		if (!GetGame().GetWorkspace())
			return;

		if (stage == TBD_EGameStage.END)
			TBD_EndScreen.Open();
		else
			TBD_EndScreen.Close();

		if (stage == TBD_EGameStage.DEBRIEF)
			TBD_DebriefScreen.Open();
		else
			TBD_DebriefScreen.Close();
	}

	//! Tell every player, and the server log, who won and why.
	//! @param endWinner the banner's winner key, empty when none was named
	//! @param endReason the banner's reason
	//! @authority server
	static void AnnounceEnd(string endWinner, string endReason)
	{
		if (TBD_Authority.IsClient())
			return;

		string winner = endWinner;
		if (winner.IsEmpty())
			winner = "(none named)";

		string msg = "[TBD] END - winner=";
		msg += winner;
		msg += " reason=";
		msg += endReason;
		TBD_PlayerChat.Broadcast(TBD_MissionFlow.CH_FLOW, msg);
	}

	//! Record the winner and reason the rule ending the round names; consumed by the next Resolve.
	//! @param reason the end reason (`time_limit`, `faction_eliminated` or an objective trigger)
	//! @param winner the winning faction key, empty when none
	void SetPending(string reason, string winner)
	{
		m_sPendingReason = reason;
		m_sPendingWinner = winner;
	}

	//! Decide the banner on the way into END: the pending pair when one was set (then cleared),
	//! else inferred from the objective triggers and the survivors, else `admin`.
	//! @param winner set to the winning faction key, empty when none
	//! @param reason set to the end reason
	//! @authority server
	void Resolve(out string winner, out string reason)
	{
		if (!m_sPendingReason.IsEmpty())
		{
			reason = m_sPendingReason;
			winner = m_sPendingWinner;
			m_sPendingReason = string.Empty;
			m_sPendingWinner = string.Empty;
			return;
		}

		string objectiveWinner;
		string trigger = TBD_ObjectiveRegistry.EvaluateEndTriggers(objectiveWinner);
		if (!trigger.IsEmpty())
		{
			reason = trigger;
			winner = objectiveWinner;
			return;
		}

		int contesting;
		int stillAlive;
		TBD_FactionElimination.CountSurvivors(winner, contesting, stillAlive);
		if (stillAlive == 1 && contesting >= 2)
		{
			reason = TBD_MissionFlow.TRIGGER_FACTION_ELIMINATED;
			return;
		}

		reason = "admin";
	}

	//! Drop the pending pair and every kill count.
	void Clear()
	{
		m_sPendingWinner = string.Empty;
		m_sPendingReason = string.Empty;
		m_mKills.Clear();
	}

	//! Pack the debrief scoreboard from TBD_DebriefScoreboard.Fill.
	//! @return the packed rows (`kills\tdeaths\tfaction\trole\tname` per line)
	static string PackDebriefBoard()
	{
		array<ref TBD_DebriefRow> rows = {};
		TBD_DebriefScoreboard.Fill(rows);
		return TBD_DebriefScreen.PackRows(rows);
	}

	//! Kills credited to a player this round.
	//! @param playerId the player
	//! @return the kill count, 0 when none
	int GetKills(int playerId)
	{
		int n;
		if (m_mKills.Find(playerId, n))
			return n;
		return 0;
	}

	//! Credit one kill to `killerId`. World and AI kills (no killer) and suicides are ignored.
	//! @param killerId the killing player, 0 or less for none
	//! @param victimId the killed player
	//! @authority server
	void CreditKill(int killerId, int victimId)
	{
		if (killerId <= 0 || killerId == victimId)
			return;

		int n;
		if (!m_mKills.Find(killerId, n))
			n = 0;
		m_mKills.Set(killerId, n + 1);
	}
}
