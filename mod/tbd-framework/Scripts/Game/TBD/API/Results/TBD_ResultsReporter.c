/**
 * @file TBD_ResultsReporter.c
 * @brief Registers each round when it goes LIVE and queues its results when it ends.
 *
 * Role: watches the round's stage; a round going LIVE gets a fresh source match id and its
 * registration (`TBD_MatchRegistration`) and starts its detailed-event recording
 * (`TBD_MatchEventRecorder`), and the first END or DEBRIEF after LIVE flushes the recording and
 * queues one results revision (`TBD_MatchResultsRevision`).  Position: armed by
 * `TBD_MissionLoader.ParseMissionJson` next to `TBD_IdentityLink.Arm()`; polls
 * `TBD_FrameworkManager.GetStage()` once a second; `TBD_ResultsPayload` tallies the round; the
 * durable `TBD_TelemetryQueue` and `TBD_TelemetryDelivery` carry every report to the API.
 * State: round flags, timestamps and the source match id; server statics that outlive a world.
 * Invariants: nothing here sends a request, so the round ends the same whether or not the backend
 * answers; the report carries what the mod measures (outcome, winning faction, times, terrain,
 * mission and event, and per player `arma_id`, role, one-life deaths and the
 * `TBD_MatchTelemetryTally` kills, team kills, longest kill and vehicles destroyed) inside a
 * complete `counters` block.
 *
 * IDENTITY LINKING SHIPS: attendance, the user-stat recompute and the leaderboard refresh join on
 * `users.arma_id`, written by `TBD_IdentityLink` (`#tbd link <code>`, POST
 * `/api/v1/ingest/link-confirm`). Both halves take the id from `TBD_PlayerIdentity.GetArmaId`, so
 * the bytes match; unlinked players still match nobody, which
 * `TBD_ResultsPayload.LogIdentityCensus` prints each round.
 */

//! End-of-round reporter: stage watch, one registration and one results revision per round.
//! @authority server
class TBD_ResultsReporter
{
	static const string CH_RESULTS = "Results"; //!< log channel: `grep '\[TBD\]\[Results\]' console.log`
	protected static const int POLL_MS = 1000; //!< stage poll period, in milliseconds; bounds how late `ended_at` can be

	protected static bool s_bArmed; //!< true while the stage poll runs
	protected static TBD_EGameStage s_LastStage; //!< stage seen by the last poll; default LOADING
	protected static bool s_bSawLive; //!< the round went LIVE; LOBBY straight to END is no round and reports nothing
	protected static bool s_bReported; //!< this round's results are queued
	protected static string s_sStartedAtUtc; //!< JSON key `started_at`, RFC 3339 UTC
	protected static string s_sEndedAtUtc; //!< JSON key `ended_at`, RFC 3339 UTC
	protected static string s_sSourceMatchId; //!< JSON key `source_match_id`, fixed when the round goes LIVE

	//! Start watching this world's round. Called from `TBD_MissionLoader.ParseMissionJson` once a
	//! valid mission document exists, a server-only path. Idempotent across a scenario restart:
	//! statics outlive a world, so `Remove` before `CallLater` keeps exactly one poll, and
	//! `ScriptCallQueue.Remove` cancels by function. Does nothing on a client.
	//! @authority server
	static void Arm()
	{
		if (TBD_Authority.IsClient())
			return;

		ResetRound();

		ScriptCallQueue queue = GetGame().GetCallqueue();
		queue.Remove(Tick);
		queue.CallLater(Tick, POLL_MS, true);
		s_bArmed = true;

		// Prints the UTC timestamp on every boot, so its RFC 3339 shape is visible in each log.
		TBD_Log.Kv(CH_RESULTS, "armed", string.Format("utcNow=%1 backend=%2 event='%3'",
			TBD_BackendText.UtcNowIso8601(), TBD_BackendText.DescribeBackend("(none)"), TBD_DeployedMission.GetEventId()));
	}

	//! Stop the poll and reset the round. Called when the world is not a
	//! framework world; `Arm()` re-arming keeps a stale poll from surviving a world.
	static void Shutdown()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		s_bArmed = false;
		ResetRound();
	}

	//! Clear every per-round field back to LOADING with no report.
	protected static void ResetRound()
	{
		s_LastStage = TBD_EGameStage.LOADING;
		s_bSawLive = false;
		s_bReported = false;
		s_sStartedAtUtc = string.Empty;
		s_sEndedAtUtc = string.Empty;
		s_sSourceMatchId = string.Empty;
	}

	//! Poll the replicated stage and forward a change to `OnStageChanged`. A world that is not a
	//! framework world disarms the poll.
	protected static void Tick()
	{
		if (!TBD_FrameworkManager.IsFrameworkWorld())
		{
			Shutdown();
			return;
		}

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		TBD_EGameStage stage = fm.GetStage();
		if (stage == s_LastStage)
			return;

		s_LastStage = stage;
		OnStageChanged(stage);
	}

	//! React to a stage change: LOADING flushes the event recording and resets the round, LIVE
	//! starts a fresh round with a fresh source match id and its recording, and the first END or
	//! DEBRIEF after LIVE flushes the recording and reports once. Idempotent; does
	//! nothing on a client.
	//! @param stage the new stage
	//! @authority server
	static void OnStageChanged(TBD_EGameStage stage)
	{
		if (TBD_Authority.IsClient())
			return;

		// A restart back through LOADING is a new round, not a continuation of the old one.
		if (stage == TBD_EGameStage.LOADING)
		{
			TBD_MatchEventRecorder.EndRound();
			ResetRound();
			return;
		}

		if (stage == TBD_EGameStage.LIVE)
		{
			// Re-entering LIVE (admin rewind) starts a fresh round with a fresh match id.
			s_bSawLive = true;
			s_bReported = false;
			s_sStartedAtUtc = TBD_BackendText.UtcNowIso8601();
			s_sSourceMatchId = TBD_ResultsPayload.BuildSourceMatchId(s_sStartedAtUtc);
			TBD_Log.Kv(CH_RESULTS, "round-start", string.Format("sourceMatchId='%1' startedAt=%2",
				s_sSourceMatchId, s_sStartedAtUtc));
			TBD_MatchRegistration.BeginRound(s_sSourceMatchId, s_sStartedAtUtc);
			TBD_MatchEventRecorder.BeginRound();
			return;
		}

		if (stage != TBD_EGameStage.END && stage != TBD_EGameStage.DEBRIEF)
			return;

		if (s_bReported)
			return;

		// No round ran, so nothing is reported, silently: an admin walking a fresh server through
		// the stages creates no match row and no alarming log line.
		if (!s_bSawLive)
			return;

		s_bReported = true;
		s_sEndedAtUtc = TBD_BackendText.UtcNowIso8601();
		TBD_MatchEventRecorder.EndRound();
		Report();
	}

	//! Tally the round, log it and the identity census, and queue its results revision.
	protected static void Report()
	{
		string winner;
		int contesting;
		int stillAlive;
		TBD_FactionElimination.CountSurvivors(winner, contesting, stillAlive);

		string outcome = TBD_ResultsPayload.ResolveOutcome(winner, contesting);

		array<ref TBD_MatchPlayerLine> lines = new array<ref TBD_MatchPlayerLine>();
		int resolvedDurable;
		int resolvedSynthetic;
		int unresolved;
		int unslotted;
		int deaths;
		TBD_ResultsPayload.CollectPlayers(s_sSourceMatchId, lines, resolvedDurable, resolvedSynthetic, unresolved, unslotted, deaths);

		TBD_Log.Kv(CH_RESULTS, "round-end", string.Format(
			"outcome=%1 winner='%2' contesting=%3 stillAlive=%4 players=%5 deaths=%6",
			outcome, winner, contesting, stillAlive, lines.Count(), deaths));

		TBD_ResultsPayload.LogIdentityCensus(resolvedDurable, resolvedSynthetic, unresolved, unslotted);

		int revision = TBD_MatchResultsRevision.Enqueue(s_sSourceMatchId, outcome, s_sStartedAtUtc, s_sEndedAtUtc, winner, lines);
		TBD_Log.Kv(CH_RESULTS, "results-queued", string.Format("sourceMatchId='%1' revision=%2 backend=%3",
			s_sSourceMatchId, revision, TBD_BackendText.DescribeBackend("(none)")));
	}
}
