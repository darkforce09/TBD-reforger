/**
 * @file TBD_MatchTelemetryTally.c
 * @brief The round's per-player combat counters: kills, team kills, longest kill, vehicles destroyed.
 *
 * Role: counts what each player did this round, for the results revision's `counters` block and
 * the DEBRIEF scoreboard.  Position: credited by `TBD_MatchEventCapture` from the same engine
 * facts that become detailed events, while `TBD_MatchEventRecorder` records; read by
 * `TBD_ResultsPayload` and `TBD_DebriefScoreboard`; cleared by `TBD_MatchEventRecorder` when a round
 * goes LIVE and when a game mode starts.
 * State: one line per player id; server statics.  Invariants: a kill is credited to the killing
 * player only, never on a suicide; a team kill is counted apart and never as a kill and never
 * toward the longest kill; the longest kill is whole metres; an unknown player reads as zeros.
 */

//! One player's counters this round.
class TBD_MatchTallyLine
{
	int m_iKills; //!< enemy players and AI characters killed; default 0
	int m_iTeamKills; //!< friendly players and AI characters killed; default 0
	int m_iLongestKillM; //!< longest enemy kill, in whole metres; default 0
	int m_iVehiclesDestroyed; //!< vehicles this player instigated the destruction of; default 0
}

//! The round's per-player combat counters, keyed by player id.
//! @authority server
class TBD_MatchTelemetryTally
{
	protected static ref map<int, ref TBD_MatchTallyLine> s_mLines; //!< player id -> counters; null until the first credit

	//! Forget every counter.
	static void Clear()
	{
		s_mLines = null;
	}

	//! Credit one kill of an enemy to `playerId`.
	//! @param distanceM killer to victim distance, in whole metres
	//! @authority server
	static void CreditKill(int playerId, int distanceM)
	{
		TBD_MatchTallyLine line = Ensure(playerId);
		if (!line)
			return;

		line.m_iKills++;
		if (distanceM > line.m_iLongestKillM)
			line.m_iLongestKillM = distanceM;
	}

	//! Credit one kill of a friendly character to `playerId`.
	//! @authority server
	static void CreditTeamKill(int playerId)
	{
		TBD_MatchTallyLine line = Ensure(playerId);
		if (line)
			line.m_iTeamKills++;
	}

	//! Credit one destroyed vehicle to `playerId`.
	//! @authority server
	static void CreditVehicleDestroyed(int playerId)
	{
		TBD_MatchTallyLine line = Ensure(playerId);
		if (line)
			line.m_iVehiclesDestroyed++;
	}

	//! Kills credited to `playerId` this round.
	//! @return the count, 0 when none
	static int GetKills(int playerId)
	{
		TBD_MatchTallyLine line = Find(playerId);
		if (!line)
			return 0;

		return line.m_iKills;
	}

	//! Team kills credited to `playerId` this round.
	//! @return the count, 0 when none
	static int GetTeamKills(int playerId)
	{
		TBD_MatchTallyLine line = Find(playerId);
		if (!line)
			return 0;

		return line.m_iTeamKills;
	}

	//! The longest enemy kill of `playerId` this round.
	//! @return whole metres, 0 when none
	static int GetLongestKillM(int playerId)
	{
		TBD_MatchTallyLine line = Find(playerId);
		if (!line)
			return 0;

		return line.m_iLongestKillM;
	}

	//! Vehicles `playerId` destroyed this round.
	//! @return the count, 0 when none
	static int GetVehiclesDestroyed(int playerId)
	{
		TBD_MatchTallyLine line = Find(playerId);
		if (!line)
			return 0;

		return line.m_iVehiclesDestroyed;
	}

	//! The counters of `playerId`, created on first use.
	//! @return the line, or null for an id that names no player
	protected static TBD_MatchTallyLine Ensure(int playerId)
	{
		if (playerId <= 0)
			return null;

		if (!s_mLines)
			s_mLines = new map<int, ref TBD_MatchTallyLine>();

		TBD_MatchTallyLine line = s_mLines.Get(playerId);
		if (line)
			return line;

		line = new TBD_MatchTallyLine();
		s_mLines.Set(playerId, line);
		return line;
	}

	//! The counters of `playerId`.
	//! @return the line, or null when nothing was credited to it
	protected static TBD_MatchTallyLine Find(int playerId)
	{
		if (!s_mLines)
			return null;

		return s_mLines.Get(playerId);
	}
}
