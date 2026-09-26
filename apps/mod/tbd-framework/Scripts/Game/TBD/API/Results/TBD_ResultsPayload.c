/**
 * @file TBD_ResultsPayload.c
 * @brief The end-of-round results: winner, outcome, per-player rows and the match-results JSON.
 *
 * Role: tallies the finished round from the spawn manager and the mission document and builds the
 * exact bytes `TBD_ResultsReporter` posts.  Position: called once per round by
 * `TBD_ResultsReporter.Report` on the server; reads `TBD_SpawnManager`, `TBD_MissionLoader`,
 * `TBD_DeployedMission` and `TBD_PlayerIdentity`.
 * State: none; pure functions over the ended round.  Invariants: `outcome` is always a member of
 * the backend's set; a player without an engine identity is dropped, never sent with an empty
 * `arma_id`; every string field goes through `TBD_BackendText.JsonEscape`.
 */

//! Stateless builders for the match-results payload.
//! @authority server
class TBD_ResultsPayload
{
	//! Who won, with the survivor arithmetic of `TBD_FrameworkManager` over the same two
	//! `TBD_SpawnManager` primitives: a fielded side (claimed slots above 0) contests, and the one
	//! contesting side with anyone alive wins. The stage machine exposes no winner, so it is
	//! recomputed; the counts do not change after END, so the poll's lag cannot change the answer.
	//! @param winner the winning faction key, or empty unless exactly one side survives
	//! @param contesting how many sides were fielded
	//! @param stillAlive how many fielded sides have anyone alive
	static void ResolveWinner(out string winner, out int contesting, out int stillAlive)
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

			// 0 claimed means the side was never fielded, which is not the same as eliminated.
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

	//! The `matches.outcome` value, always a member of the backend's set
	//! (`"" | success | failure | aborted | pending`; anything else is a 400). `success` only when
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

	//! One JSON row per connected player holding a claimed slot. A player who disconnected is not
	//! reported (the engine stops resolving their identity), though their seat still counts in
	//! `ResolveWinner`. A player with no engine identity is dropped, not sent with an empty
	//! `arma_id`: the backend dedupes on `(match_id, arma_id, source_event_id)` and
	//! `users.arma_id` is UNIQUE, so empty ids would collapse into one row.
	//! @param outRows receives the rows
	//! @param durable players sent under a backend identity
	//! @param synthetic players sent under a name-derived identity
	//! @param unresolved slotted players dropped for having no identity
	//! @param unslotted connected players without a slot
	//! @param deaths players dead this round
	//! @authority server
	static void CollectPlayers(notnull array<string> outRows, out int durable, out int synthetic,
		out int unresolved, out int unslotted, out int deaths)
	{
		durable = 0;
		synthetic = 0;
		unresolved = 0;
		unslotted = 0;
		deaths = 0;

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (!sm)
			return;

		string sourceEventId = TBD_DeployedMission.GetEventId();

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

			// One life makes this exact: a player is dead or not, with no second death to miss.
			int playerDeaths = 0;
			if (sm.IsPlayerDead(playerId))
			{
				playerDeaths = 1;
				deaths++;
			}

			outRows.Insert(BuildPlayerRow(armaId, slot.role, playerDeaths, sourceEventId));
		}
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

	// PAYLOAD

	//! The match-results body, hand-built so the bytes are fixed by code rather than by a
	//! serializer, and assembled in steps because a long `+` chain fails with
	//! `Formula too complex`.
	//! @param sourceMatchId JSON key `source_match_id`, the idempotency key
	//! @param startedAtUtc JSON key `started_at`
	//! @param endedAtUtc JSON key `ended_at`
	//! @param outcome JSON key `outcome`, from `ResolveOutcome`
	//! @param winner JSON key `winning_faction`
	//! @param playerRows the `players[]` entries from `CollectPlayers`
	//! @return the JSON body
	static string BuildPayload(string sourceMatchId, string startedAtUtc, string endedAtUtc, string outcome, string winner, notnull array<string> playerRows)
	{
		string json = "{\"match\":{";
		json += string.Format("\"source_match_id\":\"%1\"", TBD_BackendText.JsonEscape(sourceMatchId));
		json += string.Format(",\"event_id\":\"%1\"", TBD_BackendText.JsonEscape(TBD_DeployedMission.GetEventId()));

		// The catalog mission of the running deployment (TBD_DeployedMission), a UUID. A world
		// running no deployed mission, or a hand-staged cached artifact with an id that is not a
		// UUID, sends what it has, and `parse_uuid_opt` stores NULL for anything that is not a
		// UUID: no mission row matches it, so NULL is the truthful answer.
		json += string.Format(",\"mission_id\":\"%1\"", TBD_BackendText.JsonEscape(TBD_DeployedMission.GetMissionId()));
		json += string.Format(",\"terrain\":\"%1\"", TBD_BackendText.JsonEscape(GetTerrain()));
		json += string.Format(",\"started_at\":\"%1\"", startedAtUtc);
		json += string.Format(",\"ended_at\":\"%1\"", endedAtUtc);
		json += string.Format(",\"outcome\":\"%1\"", outcome);
		json += string.Format(",\"winning_faction\":\"%1\"", TBD_BackendText.JsonEscape(winner));
		json += "},\"players\":[";

		foreach (int i, string row : playerRows)
		{
			if (i > 0)
				json += ",";
			json += row;
		}

		json += "]}";
		return json;
	}

	//! One `players[]` entry. The nested `counters` block is complete, unmeasured counters as 0,
	//! false or null, because ingest reads a present `counters` as the full scoreline; the flat
	//! `deaths` key carries the same count.
	//! @return the JSON object
	protected static string BuildPlayerRow(string armaId, string role, int deaths, string sourceEventId)
	{
		string row = "{";
		row += string.Format("\"arma_id\":\"%1\"", TBD_BackendText.JsonEscape(armaId));
		row += string.Format(",\"role_played\":\"%1\"", TBD_BackendText.JsonEscape(role));
		row += string.Format(",\"deaths\":%1", deaths);
		row += string.Format(",\"source_event_id\":\"%1\"", TBD_BackendText.JsonEscape(sourceEventId));
		row += string.Format(",\"counters\":{\"kills\":0,\"deaths\":%1,\"team_kills\":0,\"longest_kill_m\":0,\"vehicles_destroyed\":0,\"is_command\":false,\"command_win\":null}", deaths);
		row += "}";
		return row;
	}

	//! Terrain key from the mission header, or empty. The backend allowlists `everon|arland|custom`
	//! and stores NULL for anything else, so an unexpected key degrades rather than 400s.
	//! @return the terrain key, or empty without a mission
	protected static string GetTerrain()
	{
		TBD_MissionDocumentStruct mission = TBD_MissionLoader.GetMission();
		if (!mission || !mission.meta)
			return string.Empty;

		return mission.meta.terrain;
	}

	//! The per-round idempotency key `<missionId>@<startedAt>#<tick>`, computed once when the round
	//! goes LIVE and reused by every retry so the backend upserts one match row. The tick count
	//! separates two rounds of one mission started in the same second.
	//! @return the key
	static string BuildSourceMatchId(string startedAtUtc)
	{
		string missionId = TBD_DeployedMission.GetMissionId();
		if (missionId.IsEmpty())
			missionId = "unknown-mission";

		return string.Format("%1@%2#%3", missionId, startedAtUtc, System.GetTickCount());
	}
}
