/**
 * @file TBD_MatchResultsRevision.c
 * @brief One revision of a round's results: the player lines and the `MatchResultsRevision` body.
 *
 * Role: issues the next results revision of a match from the queue state, builds the exact body
 * and queues it.  Position: called by `TBD_ResultsReporter` at the end of a round with the lines
 * `TBD_ResultsPayload.CollectPlayers` gathered; queues through `TBD_TelemetryQueue`.
 * State: none; each player line is a value object.  Invariants: the revision comes from the
 * persisted per-match counter, so a later revision of a match is always higher; every line carries
 * a complete `counters` block and no flat counter key; optional match fields are sent only when
 * known (`mission_id` and `event_id` only as UUIDs), since an absent field keeps the stored value
 * while a malformed one refuses the whole report; every string goes through
 * `TBD_BackendText.JsonEscape`.
 */

//! One player line of a results revision.
class TBD_MatchPlayerLine
{
	string m_sArmaId; //!< JSON `arma_id`, from `TBD_PlayerIdentity.GetArmaId`
	string m_sRolePlayed; //!< JSON `role_played`: the claimed slot's role
	string m_sSourceEventId; //!< JSON `source_event_id`: the line key within the match
	int m_iKills; //!< JSON `counters.kills`: enemy kills credited this round
	int m_iDeaths; //!< JSON `counters.deaths`: 0 or 1 in a one-life round
	int m_iTeamKills; //!< JSON `counters.team_kills`: friendly kills this round
	int m_iLongestKillM; //!< JSON `counters.longest_kill_m`: longest enemy kill, whole metres
	int m_iVehiclesDestroyed; //!< JSON `counters.vehicles_destroyed`: vehicles destroyed this round
}

//! Builder and enqueuer of results revisions.
//! @authority server
class TBD_MatchResultsRevision
{
	//! Issue the next revision of `sourceMatchId`, build its body and queue it.
	//! @param outcome `success`, `failure`, `aborted` or `pending`
	//! @param winner the winning faction key, or empty
	//! @param lines the player lines
	//! @return the revision queued, or 0 when the entry could not be written
	static int Enqueue(string sourceMatchId, string outcome, string startedAtUtc, string endedAtUtc, string winner,
		notnull array<ref TBD_MatchPlayerLine> lines)
	{
		int revision = TBD_TelemetryQueue.NextResultsRevision(sourceMatchId);
		string body = BuildBody(revision, sourceMatchId, outcome, startedAtUtc, endedAtUtc, winner, lines);
		if (!TBD_TelemetryQueue.EnqueueResults(sourceMatchId, body))
			return 0;

		return revision;
	}

	//! The results revision body, hand-built so the bytes are fixed by code and assembled in steps,
	//! because a long `+` chain fails with `Formula too complex`.
	//! @param revision JSON `revision`, at least 1
	//! @param sourceMatchId JSON `match.source_match_id`
	//! @param outcome JSON `match.outcome`
	//! @param startedAtUtc JSON `match.started_at`, RFC 3339 UTC, omitted when empty
	//! @param endedAtUtc JSON `match.ended_at`, RFC 3339 UTC, omitted when empty
	//! @param winner JSON `match.winning_faction`, omitted when empty
	//! @param lines the `players[]` entries
	//! @return the JSON body
	//! @contract match-telemetry.schema.json#/definitions/MatchResultsRevision
	static string BuildBody(int revision, string sourceMatchId, string outcome, string startedAtUtc, string endedAtUtc,
		string winner, notnull array<ref TBD_MatchPlayerLine> lines)
	{
		string json = string.Format("{\"revision\":%1,\"match\":{", revision);
		json += string.Format("\"source_match_id\":\"%1\"", TBD_BackendText.JsonEscape(sourceMatchId));
		json += string.Format(",\"outcome\":\"%1\"", outcome);

		string eventId = TBD_DeployedMission.GetEventId();
		if (TBD_BackendText.IsUuid(eventId))
			json += string.Format(",\"event_id\":\"%1\"", eventId);

		string missionId = TBD_DeployedMission.GetMissionId();
		if (TBD_BackendText.IsUuid(missionId))
			json += string.Format(",\"mission_id\":\"%1\"", missionId);

		string terrain = TBD_MatchRegistration.MissionTerrain();
		if (!terrain.IsEmpty())
			json += string.Format(",\"terrain\":\"%1\"", TBD_BackendText.JsonEscape(terrain));

		if (!startedAtUtc.IsEmpty())
			json += string.Format(",\"started_at\":\"%1\"", startedAtUtc);

		if (!endedAtUtc.IsEmpty())
			json += string.Format(",\"ended_at\":\"%1\"", endedAtUtc);

		if (!winner.IsEmpty())
			json += string.Format(",\"winning_faction\":\"%1\"", TBD_BackendText.JsonEscape(winner));

		json += "},\"players\":[";
		foreach (int i, TBD_MatchPlayerLine line : lines)
		{
			if (i > 0)
				json += ",";
			json += BuildLine(line);
		}

		json += "]}";
		return json;
	}

	//! One `players[]` entry with a complete `counters` block: the command counters, which this
	//! round does not measure, are false and null, because a present block is read as the whole
	//! scoreline.
	//! @return the JSON object
	//! @contract match-telemetry.schema.json#/definitions/PlayerLine
	protected static string BuildLine(notnull TBD_MatchPlayerLine line)
	{
		string row = string.Format("{\"arma_id\":\"%1\"", TBD_BackendText.JsonEscape(line.m_sArmaId));
		row += string.Format(",\"role_played\":\"%1\"", TBD_BackendText.JsonEscape(line.m_sRolePlayed));
		row += string.Format(",\"source_event_id\":\"%1\"", TBD_BackendText.JsonEscape(line.m_sSourceEventId));
		row += string.Format(",\"counters\":{\"kills\":%1,\"deaths\":%2", line.m_iKills, line.m_iDeaths);
		row += string.Format(",\"team_kills\":%1,\"longest_kill_m\":%2,\"vehicles_destroyed\":%3", line.m_iTeamKills,
			line.m_iLongestKillM, line.m_iVehiclesDestroyed);
		row += ",\"is_command\":false,\"command_win\":null}}";
		return row;
	}
}
