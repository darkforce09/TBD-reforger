/**
 * @file TBD_ResultsPayload.c
 * @brief The facts of a finished round: outcome, per-player lines, identity census, match key.
 *
 * Role: tallies the finished round from the spawn manager, the framework manager and the mission
 * document into what `TBD_MatchResultsRevision` reports.  Position: called once per round by
 * `TBD_ResultsReporter` on the server; reads `TBD_SpawnManager`, `TBD_MatchTelemetryTally`,
 * `TBD_MissionLoader`, `TBD_DeployedMission` and `TBD_PlayerIdentity`; the winner comes from
 * `TBD_FactionElimination.CountSurvivors`, the rule the END banner uses.
 * State: none; pure functions over the ended round.  Invariants: `outcome` is always a member of
 * the backend's set; a player without an engine identity is dropped, never sent with an empty
 * `arma_id`; every line has a non-empty role and line key, which the API requires.
 */

//! Stateless tallies of a finished round.
//! @authority server
class TBD_ResultsPayload
{
	static const string UNNAMED_ROLE = "unassigned"; //!< `role_played` of a slot authored without a role; the API requires one

	//! The `matches.outcome` value, always a member of the backend's set
	//! (`success | failure | aborted | pending`; anything else is a 400). `success` only when
	//! the mission declares `faction_eliminated` and exactly one of at least two fielded sides
	//! survived; everything else is `aborted`. Events are PvP, so `failure` has no side-independent
	//! meaning and is never sent; `winning_faction` carries who won.
	//! @return `success` or `aborted`
	static string ResolveOutcome(string winner, int contesting)
	{
		if (winner.IsEmpty())
			return "aborted";

		if (contesting < 2)
			return "aborted";

		if (!TBD_MissionLoader.HasEndTrigger("faction_eliminated"))
			return "aborted";

		return "success";
	}

	//! One line per connected player holding a claimed slot. A player who disconnected is not
	//! reported (the engine stops resolving their identity), though their seat still counts in
	//! `TBD_FactionElimination.CountSurvivors`. A player with no engine identity is dropped, not
	//! sent with an empty `arma_id`: the API keys lines on `(arma_id, source_event_id)` and
	//! `users.arma_id` is UNIQUE, so empty ids would collapse into one line. Deaths come from
	//! `TBD_SpawnManager` (one life, so 0 or 1); kills, team kills, the longest kill and vehicles
	//! destroyed from `TBD_MatchTelemetryTally`.
	//! @param sourceMatchId the round's source match id, the line key when no event is deployed
	//! @param outLines receives the lines
	//! @param durable players sent under a backend identity
	//! @param synthetic players sent under a name-derived identity
	//! @param unresolved slotted players dropped for having no identity
	//! @param unslotted connected players without a slot
	//! @param deaths players dead this round
	//! @authority server
	static void CollectPlayers(string sourceMatchId, notnull array<ref TBD_MatchPlayerLine> outLines, out int durable,
		out int synthetic, out int unresolved, out int unslotted, out int deaths)
	{
		durable = 0;
		synthetic = 0;
		unresolved = 0;
		unslotted = 0;
		deaths = 0;

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (!sm)
			return;

		string lineKey = LineKey(sourceMatchId);

		array<int> players = {};
		int count = GetGame().GetPlayerManager().GetPlayers(players);
		for (int i = 0; i < count; i++)
		{
			int playerId = players[i];

			TBD_MissionSlotStruct slot = sm.GetAssignedSlot(playerId);
			if (!slot)
			{
				unslotted++;
				continue;
			}

			string armaId = TBD_PlayerIdentity.GetArmaId(playerId);
			if (armaId.IsEmpty())
			{
				unresolved++;
				continue;
			}

			if (TBD_PlayerIdentity.IsDurable(armaId))
				durable++;
			else
				synthetic++;

			TBD_MatchPlayerLine line = new TBD_MatchPlayerLine();
			line.m_sArmaId = armaId;
			line.m_sRolePlayed = slot.role;
			if (line.m_sRolePlayed.IsEmpty())
				line.m_sRolePlayed = UNNAMED_ROLE;

			line.m_sSourceEventId = lineKey;
			line.m_iKills = TBD_MatchTelemetryTally.GetKills(playerId);
			line.m_iTeamKills = TBD_MatchTelemetryTally.GetTeamKills(playerId);
			line.m_iLongestKillM = TBD_MatchTelemetryTally.GetLongestKillM(playerId);
			line.m_iVehiclesDestroyed = TBD_MatchTelemetryTally.GetVehiclesDestroyed(playerId);

			// One life makes this exact: a player is dead or not, with no second death to miss.
			if (sm.IsPlayerDead(playerId))
			{
				line.m_iDeaths = 1;
				deaths++;
			}

			outLines.Insert(line);
		}
	}

	//! The `source_event_id` of every line of the round: the deployed event, or the round's source
	//! match id when no event is deployed, because the API requires a non-empty key.
	//! @return the line key
	protected static string LineKey(string sourceMatchId)
	{
		string eventId = TBD_DeployedMission.GetEventId();
		if (!eventId.IsEmpty())
			return eventId;

		return sourceMatchId;
	}

	//! Log once per round how many sent identities are durable, synthetic or missing, and warn
	//! about each gap. "Resolved" means the engine issued an identity, not that the backend has
	//! `users.arma_id` for it; linking is `TBD_IdentityLink`. Keeps an unlinked no-op visible.
	static void LogIdentityCensus(int durable, int synthetic, int unresolved, int unslotted)
	{
		TBD_Log.Kv(TBD_ResultsReporter.CH_RESULTS, "identities", string.Format(
			"sent=%1 durable=%2 synthetic=%3 noIdentity=%4 unslotted=%5",
			durable + synthetic, durable, synthetic, unresolved, unslotted));

		if (synthetic > 0)
		{
			TBD_Log.Warn(TBD_ResultsReporter.CH_RESULTS, string.Format(
				"%1 player(s) reported under a NAME-DERIVED identity (vanilla's 00bbbddd- fallback, listen/hosted host). Those ids are not durable -- a rename makes a new person and a shared name makes one. Run events on a dedicated server.",
				synthetic));
		}

		if (unresolved > 0)
		{
			TBD_Log.Warn(TBD_ResultsReporter.CH_RESULTS, string.Format(
				"%1 slotted player(s) had NO engine identity and were dropped from the report. On a dedicated server this means the backend identity service is not configured.",
				unresolved));
		}

		TBD_Log.Event(TBD_ResultsReporter.CH_RESULTS,
			"NOTE: attendance / user-stat recompute / leaderboard refresh only hit players with users.arma_id set. Link via `#tbd link <code>` (TBD_IdentityLink, Arm()'d by MissionLoader -> POST /api/v1/ingest/link-confirm). An engine-resolved identity without that link still matches nobody.");
	}

	//! The per-round source match id `<missionId>@<startedAt>#<tick>`, computed once when the round
	//! goes LIVE and named by its registration and every report of it. The tick count separates two
	//! rounds of one mission started in the same second.
	//! @return the key
	static string BuildSourceMatchId(string startedAtUtc)
	{
		string missionId = TBD_DeployedMission.GetMissionId();
		if (missionId.IsEmpty())
			missionId = "unknown-mission";

		return string.Format("%1@%2#%3", missionId, startedAtUtc, System.GetTickCount());
	}
}
