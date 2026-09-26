/**
 * @file TBD_ResultsReporter.c
 * @brief The thin end-of-round results POST to `/api/v1/ingest/match-results`.
 *
 * Role: watches the round's stage, and once a round that went LIVE reaches END or DEBRIEF, posts
 * one results report with a bounded, idempotent retry.  Position: armed by
 * `TBD_MissionLoader.ParseMissionJson` next to `TBD_IdentityLink.Arm()`; polls
 * `TBD_FrameworkManager.GetStage()` once a second; `TBD_ResultsPayload` builds the body.
 * State: round flags, timestamps, the source match id, the built payload and the attempt count;
 * server statics that outlive a world.  Invariants: the report carries only what the mod knows
 * exactly (outcome, winning faction, times, terrain, mission id, and per player `arma_id`, role
 * and one-life `deaths`); kills, longest kill and vehicle counts are sent as zeros inside a
 * complete `counters` block, never measured; the round ends correctly whether or not the backend
 * answers, and a missing `TBD_BackendConfig` is a legal local state, logged and not retried.
 *
 * IDENTITY LINKING SHIPS: attendance, the user-stat recompute and the leaderboard refresh join on
 * `users.arma_id`, written by `TBD_IdentityLink` (`#tbd link <code>`, POST
 * `/api/v1/ingest/link-confirm`). Both halves take the id from `TBD_PlayerIdentity.GetArmaId`, so
 * the bytes match; unlinked players still match nobody, which
 * `TBD_ResultsPayload.LogIdentityCensus` prints each round.
 */

//! End-of-round results reporter: stage watch, one report per round, bounded retry.
//! @authority server
class TBD_ResultsReporter
{
	static const string CH_RESULTS = "Results"; //!< log channel: `grep '\[TBD\]\[Results\]' console.log`
	protected static const string INGEST_PATH = "/api/v1/ingest/match-results"; //!< backend route, service-token tier
	protected static const int POLL_MS = 1000; //!< stage poll period, in milliseconds; bounds how late `ended_at` can be
	protected static const int MAX_ATTEMPTS = 3; //!< POST attempts per round before giving up
	protected static const int RETRY_BASE_MS = 5000; //!< retry delay per failed attempt, in milliseconds (linear backoff)

	protected static bool s_bArmed; //!< true while the stage poll runs
	protected static TBD_EGameStage s_LastStage; //!< stage seen by the last poll; default LOADING
	protected static bool s_bSawLive; //!< the round went LIVE; LOBBY straight to END is no round and sends nothing
	protected static bool s_bReported; //!< this round's report is built and sending
	protected static string s_sStartedAtUtc; //!< JSON key `started_at`, RFC 3339 UTC
	protected static string s_sEndedAtUtc; //!< JSON key `ended_at`, RFC 3339 UTC
	protected static string s_sSourceMatchId; //!< JSON key `source_match_id`; the idempotency key, fixed when the round goes LIVE
	protected static string s_sPayload; //!< the body, built once at END and re-sent verbatim on every retry
	protected static int s_iAttempt; //!< attempts made for this round's report
	protected static ref RestCallback s_RestCallback; //!< callback of the outstanding POST, held so it outlives the call

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

	//! Stop the poll, drop any pending retry and reset the round. Called when the world is not a
	//! framework world; `Arm()` re-arming keeps a stale poll from surviving a world.
	static void Shutdown()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			queue.Remove(Tick);
			queue.Remove(SendAttempt);
		}

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
		s_sPayload = string.Empty;
		s_iAttempt = 0;
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

	//! React to a stage change: LOADING resets the round, LIVE starts a fresh round with a fresh
	//! source match id, and the first END or DEBRIEF after LIVE reports once. Idempotent; does
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
			ResetRound();
			return;
		}

		if (stage == TBD_EGameStage.LIVE)
		{
			// Re-entering LIVE (admin rewind) starts a fresh round with a fresh match id.
			s_bSawLive = true;
			s_bReported = false;
			s_iAttempt = 0;
			s_sPayload = string.Empty;
			s_sStartedAtUtc = TBD_BackendText.UtcNowIso8601();
			s_sSourceMatchId = TBD_ResultsPayload.BuildSourceMatchId(s_sStartedAtUtc);
			TBD_Log.Kv(CH_RESULTS, "round-start", string.Format("sourceMatchId='%1' startedAt=%2",
				s_sSourceMatchId, s_sStartedAtUtc));
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
		Report();
	}

	//! Build the payload once, log it and the identity census, then start the bounded send.
	protected static void Report()
	{
		string winner = string.Empty;
		int contesting = 0;
		int stillAlive = 0;
		TBD_ResultsPayload.ResolveWinner(winner, contesting, stillAlive);

		string outcome = TBD_ResultsPayload.ResolveOutcome(winner, contesting);

		array<string> playerRows = new array<string>();
		int resolvedDurable = 0;
		int resolvedSynthetic = 0;
		int unresolved = 0;
		int unslotted = 0;
		int deaths = 0;
		TBD_ResultsPayload.CollectPlayers(playerRows, resolvedDurable, resolvedSynthetic, unresolved, unslotted, deaths);

		s_sPayload = TBD_ResultsPayload.BuildPayload(s_sSourceMatchId, s_sStartedAtUtc, s_sEndedAtUtc, outcome, winner, playerRows);

		TBD_Log.Kv(CH_RESULTS, "round-end", string.Format(
			"outcome=%1 winner='%2' contesting=%3 stillAlive=%4 players=%5 deaths=%6",
			outcome, winner, contesting, stillAlive, playerRows.Count(), deaths));

		TBD_ResultsPayload.LogIdentityCensus(resolvedDurable, resolvedSynthetic, unresolved, unslotted);

		// The payload itself, once, so an operator sees exactly what left the server; it survives
		// a backend that never answers.
		TBD_Log.Event(CH_RESULTS, "payload " + s_sPayload);

		s_iAttempt = 0;
		SendAttempt();
	}

	//! One POST attempt with the server's `X-Service-Token`. No backend configured logs a legal
	//! local state and stops; a missing `RestApi` or context retries. Never blocks and never
	//! touches the stage machine.
	//! @route POST /api/v1/ingest/match-results
	//! @authority server
	protected static void SendAttempt()
	{
		s_iAttempt++;

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		string token = TBD_BackendConfig.GetServerToken();
		if (baseUrl.IsEmpty() || token.IsEmpty())
		{
			// A legal state on a local or PIE host, logged at normal level so it neither alarms an
			// operator nor trips the world-boot error triage.
			TBD_Log.Event(CH_RESULTS, "not reported -- no backend configured (backendUrl/serverToken empty). This is a legal state on a local host.");
			return;
		}

		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			Retry("RestApi unavailable");
			return;
		}

		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext ctx = rest.GetContext(baseUrl);
		if (!ctx)
		{
			Retry(string.Format("RestContext failed for %1", baseUrl));
			return;
		}

		s_RestCallback = new RestCallback();
		s_RestCallback.SetOnSuccess(OnSendSuccess);
		s_RestCallback.SetOnError(OnSendError);

		// Content-Type is required: the handler's Axum `Json<MatchResultsInput>` extractor rejects
		// a body without `application/json` with a 400 before the handler runs.
		ctx.SetHeaders(string.Format("X-Service-Token,%1,Content-Type,application/json,Accept,application/json", token));

		TBD_Log.Kv(CH_RESULTS, "post", string.Format("attempt=%1/%2 url=%3%4 bytes=%5",
			s_iAttempt, MAX_ATTEMPTS, baseUrl, INGEST_PATH, s_sPayload.Length()));

		ctx.POST(s_RestCallback, INGEST_PATH, s_sPayload);
	}

	//! Success callback: logs the attempt and the backend's answer.
	protected static void OnSendSuccess(RestCallback cb)
	{
		TBD_Log.Kv(CH_RESULTS, "posted", string.Format("attempt=%1 response=%2", s_iAttempt, cb.GetData()));
	}

	//! Error callback: retries, logging the body, where the backend says why
	//! (`{"error":"invalid outcome"}`); empty on a transport failure.
	protected static void OnSendError(RestCallback cb)
	{
		Retry(string.Format("backend rejected or unreachable, response='%1'", cb.GetData()));
	}

	//! Schedule the next attempt after `RETRY_BASE_MS * attempts`, or give up after `MAX_ATTEMPTS`.
	//! Idempotent: the payload is byte-identical each time and the backend upserts the match on
	//! `source_match_id`, so a retry after an unseen response creates no second match row.
	//! @param why the failure, logged
	protected static void Retry(string why)
	{
		if (s_iAttempt >= MAX_ATTEMPTS)
		{
			TBD_Log.Warn(CH_RESULTS, string.Format(
				"GIVING UP after %1 attempt(s) -- %2. The round is unaffected; the payload is above in this log and the endpoint is idempotent on source_match_id='%3', so it can be replayed by hand.",
				s_iAttempt, why, s_sSourceMatchId));
			return;
		}

		int delay = RETRY_BASE_MS * s_iAttempt;
		TBD_Log.Warn(CH_RESULTS, string.Format("attempt=%1/%2 failed (%3) -- retrying in %4 ms",
			s_iAttempt, MAX_ATTEMPTS, why, delay));

		GetGame().GetCallqueue().CallLater(SendAttempt, delay, false);
	}
}
