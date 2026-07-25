//! T-181.13.1 — the THIN end-of-round results POST.
//!
//! ══ WHAT "THIN" MEANS HERE ═════════════════════════════════════════════════════════════════
//! It is an operator instruction, not laziness. `TBD_MOD_DESIGN.md` §6 defers full AAR /
//! statistics recording in the operator's own words: *"that's also the AAR, which is not easy to
//! do… we have to record everything. That's very complex. I don't feel like we have the time."*
//! So there is deliberately NO kill tracking, NO longest-kill measurement and NO
//! vehicle-destruction counting in this slice, and none is faked.
//!
//! This sends only what the mod already knows with no new machinery:
//!   * outcome + winning faction — recomputed from the same `TBD_SpawnManager` primitives the
//!     stage machine's own win evaluator uses (`CountClaimedForFaction` / `CountAliveForFaction`);
//!   * started_at / ended_at, terrain, mission id — already on hand;
//!   * per player: `arma_id`, `role_played` from the assigned slot, and `deaths` = 0 or 1, which
//!     ONE LIFE makes exactly knowable.
//!
//! `kills`, `team_kills`, `longest_kill_m`, `vehicles_destroyed`, `is_command`, `command_win` and
//! `aar_replay_url` are **OMITTED FROM THE PAYLOAD ENTIRELY** rather than sent as zeros. Every one
//! of them is `#[serde(default)]` on the backend so the row still writes, and an absent field says
//! "not measured" where a `0` would claim "measured, and it was none". A zero you can defend beats
//! a statistic you cannot.
//!
//! ══ ⚠ ATTENDANCE IS INERT UNTIL T-181.35 LANDS ═════════════════════════════════════════════
//! The endpoint marks attendance, recomputes user stats and refreshes the leaderboard — but all
//! three hang off `SELECT discord_id FROM users WHERE arma_id = $1`
//! (`apps/website/api/src/handlers/telemetry.rs:238`), and `users.arma_id` is written by exactly
//! two things: the dev seed, and `POST /api/v1/ingest/link-confirm` — the game server confirming a
//! player's link code — **which this mod does not implement**. There is no `#tbd link` command.
//!
//! So in production no player has an `arma_id`, this POST returns 200 with a `match_id`, the match
//! and per-player rows are written, and attendance / stats / leaderboard all silently do NOTHING.
//! That is filed as T-181.35 and is not this slice's to fix. What IS this slice's job is to make
//! the no-op visible: `LogIdentityCensus` prints, once per round, how many players resolved to a
//! durable identity versus not, and says out loud that a resolved identity is still not a LINKED
//! one. See `TBD_PlayerIdentity` for the shared accessor T-181.35 must reuse verbatim.
//!
//! ══ FAILURE BEHAVIOUR ══════════════════════════════════════════════════════════════════════
//! The round must end correctly whether or not the backend is reachable. Everything here is
//! callqueue/REST-callback driven and nothing blocks the stage machine: a failed POST is a logged
//! warning plus a bounded retry (`MAX_ATTEMPTS`, idempotent because `source_match_id` is stable for
//! the round), and then it gives up. `TBD_BackendConfig` may be absent entirely — that is a LEGAL
//! state on a local/PIE host, logged at NORMAL and not retried, never an error.
//!
//! ══ HOW IT LEARNS THE ROUND ENDED ══════════════════════════════════════════════════════════
//! By polling `TBD_FrameworkManager.GetStage()` once a second. This slice owns
//! `Scripts/Game/TBD/Backend/**` and `TBD_FrameworkManager.c` belongs to another lane, so it does
//! not write into it — the same call the lobby watcher makes for the same reason
//! (`TBD_LobbyStage` in TBD_LobbyController.c). `OnStageChanged` below is public and
//! side-effect-complete precisely so that replacing the poll is a ONE-LINE hook next to the
//! existing `TBD_RadioBridgeStub.OnStageChanged(stage)` in `TBD_FrameworkManager.SetStage`:
//!
//!     TBD_ResultsReporter.OnStageChanged(stage);
//!
//! Until that lands, the poll costs one enum compare per second and is at most one second late —
//! which cannot change the numbers, because nothing mutates claimed/alive counts after END.
//!
//! @route POST /api/v1/ingest/match-results (service-token tier; `X-Service-Token`)
//! @authority server — only the server has the mission document, the slot map and the identities.
class TBD_ResultsReporter
{
	//! Greppable channel for this subsystem: `grep '\[TBD\]\[Results\]' console.log`.
	//! Kept local rather than added to `TBD_Log`'s vocabulary because `Core/TBD_Log.c` is outside
	//! this slice's ownership — see the slice report.
	protected static const string CH_RESULTS = "Results";

	protected static const string INGEST_PATH = "/api/v1/ingest/match-results";

	//! One enum compare per tick. Fast enough to be honest about `ended_at` without being a load.
	protected static const int POLL_MS = 1000;

	//! Bounded, and bounded small: the round is over, nobody is waiting, and an unbounded retry
	//! against a dead backend would run until the world is torn down.
	protected static const int MAX_ATTEMPTS = 3;
	protected static const int RETRY_BASE_MS = 5000;

	protected static bool s_bArmed;
	protected static TBD_EGameStage s_LastStage;

	//! A round only exists once it went LIVE. An admin taking a server from LOBBY to END has not
	//! run a round and must not create a match row.
	protected static bool s_bSawLive;
	protected static bool s_bReported;

	protected static string s_sStartedAtUtc;
	protected static string s_sEndedAtUtc;
	protected static string s_sSourceMatchId;

	//! Built once at END and re-sent verbatim on every retry, so a retry is genuinely idempotent
	//! (the backend upserts on `source_match_id`) rather than a second, slightly different report.
	protected static string s_sPayload;
	protected static int s_iAttempt;

	protected static ref RestCallback s_RestCallback;

	//------------------------------------------------------------------------------------------------
	//! Start watching this world's round. Called from `TBD_MissionLoader.ParseMissionJson` once a
	//! valid mission document exists, which is the earliest moment a results report could mean
	//! anything — and, conveniently, a server-only path.
	//!
	//! Idempotent, and safe across a scenario restart: statics outlive a world inside one process
	//! (measured landmine), so `Remove` before `CallLater` guarantees exactly one live tick rather
	//! than one per world. `ScriptCallQueue.Remove` cancels BY FUNCTION, which is exactly right
	//! here because there is one instance of this tick per process.
	static void Arm()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		ResetRound();

		ScriptCallQueue queue = GetGame().GetCallqueue();
		queue.Remove(Tick);
		queue.CallLater(Tick, POLL_MS, true);
		s_bArmed = true;

		// Also the runtime proof that the UTC clock works and that the timestamp shape is the
		// RFC 3339 the backend's `DateTime<Utc>` parses. The compile probe proved the symbols
		// resolve; only a boot can show the VALUE, and this puts it in every boot's log.
		TBD_Log.Kv(CH_RESULTS, "armed", string.Format("utcNow=%1 backend=%2 event='%3'",
			UtcNowIso8601(), DescribeBackend(), TBD_BackendConfig.GetEventId()));
	}

	//------------------------------------------------------------------------------------------------
	//! Stop watching and drop any pending retry. Must be called on world teardown if this ever
	//! gains a component host; today `Arm()` re-arming is what keeps a stale tick from surviving.
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Poll the replicated stage. Self-healing: a world that is not a framework world disarms the
	//! tick, and a stage that has gone back to LOADING is a new round.
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

	//------------------------------------------------------------------------------------------------
	//! The single entry point for "the round changed phase" as far as results are concerned.
	//! Public, idempotent and side-effect-complete so wiring it to `TBD_FrameworkManager.SetStage`
	//! is one line and the poll above can then be deleted.
	//! @authority server
	static void OnStageChanged(TBD_EGameStage stage)
	{
		if (RplSession.Mode() == RplMode.Client)
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
			s_sStartedAtUtc = UtcNowIso8601();
			s_sSourceMatchId = BuildSourceMatchId(s_sStartedAtUtc);
			TBD_Log.Kv(CH_RESULTS, "round-start", string.Format("sourceMatchId='%1' startedAt=%2",
				s_sSourceMatchId, s_sStartedAtUtc));
			return;
		}

		if (stage != TBD_EGameStage.END && stage != TBD_EGameStage.DEBRIEF)
			return;

		if (s_bReported)
			return;

		// Nothing ran, so there is nothing to report. Silent on purpose: an admin walking a fresh
		// server through the stages should not manufacture a match row, nor a log line implying
		// something went wrong.
		if (!s_bSawLive)
			return;

		s_bReported = true;
		s_sEndedAtUtc = UtcNowIso8601();
		Report();
	}

	//------------------------------------------------------------------------------------
	// REPORT
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Build the payload once, log it, then start the (bounded) send.
	protected static void Report()
	{
		string winner = string.Empty;
		int contesting = 0;
		int stillAlive = 0;
		ResolveWinner(winner, contesting, stillAlive);

		string outcome = ResolveOutcome(winner, contesting);

		array<string> playerRows = new array<string>();
		int resolvedDurable = 0;
		int resolvedSynthetic = 0;
		int unresolved = 0;
		int unslotted = 0;
		int deaths = 0;
		CollectPlayers(playerRows, resolvedDurable, resolvedSynthetic, unresolved, unslotted, deaths);

		s_sPayload = BuildPayload(outcome, winner, playerRows);

		TBD_Log.Kv(CH_RESULTS, "round-end", string.Format(
			"outcome=%1 winner='%2' contesting=%3 stillAlive=%4 players=%5 deaths=%6",
			outcome, winner, contesting, stillAlive, playerRows.Count(), deaths));

		LogIdentityCensus(resolvedDurable, resolvedSynthetic, unresolved, unslotted);

		// The payload itself, once, so an operator can see exactly what left the server without
		// running a proxy. It is the only artifact that survives a backend that never answers.
		TBD_Log.Event(CH_RESULTS, "payload " + s_sPayload);

		s_iAttempt = 0;
		SendAttempt();
	}

	//------------------------------------------------------------------------------------------------
	//! Who won, evaluated exactly the way `TBD_FrameworkManager.TickWinConditions` evaluates it —
	//! same two public `TBD_SpawnManager` primitives, same guards.
	//!
	//! Recomputed rather than read off the stage machine because that file is another lane's and
	//! records the winner nowhere a caller can reach (it Prints it and drops it). The counts are
	//! stable after END: `TBD_SpawnManager.OnStageChanged` does nothing on END, and nothing else
	//! mutates the claimed-slot map or the dead set once the round is over, so the poll's
	//! sub-second lag cannot change the answer.
	protected static void ResolveWinner(out string winner, out int contesting, out int stillAlive)
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

	//------------------------------------------------------------------------------------------------
	//! `matches.outcome` is `"" | success | failure | aborted | pending` and **anything else is a
	//! 400** (telemetry.rs, ingest_match_results), so this only ever returns a member of that set.
	//!
	//! The mapping is deliberately conservative. `success` is claimed ONLY when the mission itself
	//! declared `faction_eliminated` and the survivor arithmetic actually resolved to one side —
	//! i.e. the round reached the conclusion its author wrote. Everything else is `aborted`: an
	//! admin ended it, both sides were wiped, or fewer than two sides were ever fielded. TBD events
	//! are PvP, so `failure` has no side-independent meaning and is never sent; who actually won is
	//! carried by `winning_faction`, not by this field.
	protected static string ResolveOutcome(string winner, int contesting)
	{
		if (winner.IsEmpty())
			return "aborted";

		if (contesting < 2)
			return "aborted";

		if (!TBD_MissionLoader.HasEndTrigger("faction_eliminated"))
			return "aborted";

		return "success";
	}

	//------------------------------------------------------------------------------------------------
	//! One JSON row per participating player.
	//!
	//! The participant set is "connected AND holding a claimed slot". Two honest limitations, both
	//! inherent rather than lazy:
	//!   * a player who died and then DISCONNECTED is not reported — the engine stops answering the
	//!     identity lookup once a player is torn down, so there is no `arma_id` left to report them
	//!     under. Their seat still counts toward the winner arithmetic above (TBD_SpawnManager
	//!     keeps departed seats), which is where it actually matters.
	//!   * a player with no engine identity is DROPPED, not sent with an empty `arma_id`. The
	//!     backend's dedupe index is `(match_id, arma_id, source_event_id)`, so several empty ids
	//!     would collapse into a single row, and `users.arma_id` is UNIQUE, so one empty-string user
	//!     would absorb all of them. Dropping is the only non-corrupting option; the count is
	//!     logged by `LogIdentityCensus`.
	protected static void CollectPlayers(notnull array<string> outRows, out int durable, out int synthetic,
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

		string sourceEventId = TBD_BackendConfig.GetEventId();

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

			// ONE LIFE is what makes this exactly knowable: a player is dead or they are not, and
			// there is no second death to miss. It is the only per-player statistic this slice
			// claims, and the only one it can defend.
			int playerDeaths = 0;
			if (sm.IsPlayerDead(playerId))
			{
				playerDeaths = 1;
				deaths++;
			}

			outRows.Insert(BuildPlayerRow(armaId, slot.role, playerDeaths, sourceEventId));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Say plainly, once per round, whether this POST can possibly do anything.
	//!
	//! "Resolved" here means the ENGINE gave us an identity — NOT that the backend knows it. Until
	//! T-181.35 ships `#tbd link`, nothing ever writes `users.arma_id` in production, so even a
	//! fully durable census resolves to zero users. Saying so every round is the difference between
	//! a visible no-op and a silent one.
	protected static void LogIdentityCensus(int durable, int synthetic, int unresolved, int unslotted)
	{
		TBD_Log.Kv(CH_RESULTS, "identities", string.Format(
			"sent=%1 durable=%2 synthetic=%3 noIdentity=%4 unslotted=%5",
			durable + synthetic, durable, synthetic, unresolved, unslotted));

		if (synthetic > 0)
		{
			TBD_Log.Warn(CH_RESULTS, string.Format(
				"%1 player(s) reported under a NAME-DERIVED identity (vanilla's 00bbbddd- fallback, listen/hosted host). Those ids are not durable — a rename makes a new person and a shared name makes one. Run events on a dedicated server.",
				synthetic));
		}

		if (unresolved > 0)
		{
			TBD_Log.Warn(CH_RESULTS, string.Format(
				"%1 slotted player(s) had NO engine identity and were dropped from the report. On a dedicated server this means the backend identity service is not configured.",
				unresolved));
		}

		TBD_Log.Event(CH_RESULTS,
			"NOTE: attendance, user-stat recompute and leaderboard refresh stay INERT until T-181.35 lands. The backend joins on users.arma_id, which only POST /api/v1/ingest/link-confirm writes, and this mod does not implement it yet — so the POST will succeed and match nobody.");
	}

	//------------------------------------------------------------------------------------
	// SEND
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! One attempt. Never throws, never blocks, never touches the stage machine.
	protected static void SendAttempt()
	{
		s_iAttempt++;

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		string token = TBD_BackendConfig.GetServerToken();
		if (baseUrl.IsEmpty() || token.IsEmpty())
		{
			// LEGAL STATE, not an error: a local/PIE host has no backend config at all. Logged at
			// NORMAL so it neither alarms an operator nor trips the world-boot error triage.
			TBD_Log.Event(CH_RESULTS, "not reported — no backend configured (backendUrl/serverToken empty). This is a legal state on a local host.");
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

		// Content-Type is NOT optional: the handler takes an Axum `Json<MatchResultsInput>`
		// extractor, which rejects a body without `application/json` before the handler ever runs —
		// the request would come back 400 "match and players are required" with a perfectly valid
		// payload. Same X-Service-Token tier the mission fetch already uses (TBD_MissionLoader).
		ctx.SetHeaders(string.Format("X-Service-Token,%1,Content-Type,application/json,Accept,application/json", token));

		TBD_Log.Kv(CH_RESULTS, "post", string.Format("attempt=%1/%2 url=%3%4 bytes=%5",
			s_iAttempt, MAX_ATTEMPTS, baseUrl, INGEST_PATH, s_sPayload.Length()));

		ctx.POST(s_RestCallback, INGEST_PATH, s_sPayload);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnSendSuccess(RestCallback cb)
	{
		TBD_Log.Kv(CH_RESULTS, "posted", string.Format("attempt=%1 response=%2", s_iAttempt, cb.GetData()));
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnSendError(RestCallback cb)
	{
		// The body is where the backend says WHY (e.g. `{"error":"invalid outcome"}`), and a 400
		// that only ever logged "failed" is exactly the kind of dead end this program keeps
		// finding. It may be empty on a transport failure; that is still information.
		Retry(string.Format("backend rejected or unreachable, response='%1'", cb.GetData()));
	}

	//------------------------------------------------------------------------------------------------
	//! Bounded retry. Idempotent by construction: the payload is byte-identical each time and the
	//! backend upserts the match on `source_match_id`, so a retry after a response we never saw
	//! cannot create a second match row.
	protected static void Retry(string why)
	{
		if (s_iAttempt >= MAX_ATTEMPTS)
		{
			TBD_Log.Warn(CH_RESULTS, string.Format(
				"GIVING UP after %1 attempt(s) — %2. The round is unaffected; the payload is above in this log and the endpoint is idempotent on source_match_id='%3', so it can be replayed by hand.",
				s_iAttempt, why, s_sSourceMatchId));
			return;
		}

		int delay = RETRY_BASE_MS * s_iAttempt;
		TBD_Log.Warn(CH_RESULTS, string.Format("attempt=%1/%2 failed (%3) — retrying in %4 ms",
			s_iAttempt, MAX_ATTEMPTS, why, delay));

		GetGame().GetCallqueue().CallLater(SendAttempt, delay, false);
	}

	//------------------------------------------------------------------------------------
	// PAYLOAD
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! The wire body, hand-built.
	//!
	//! Hand-built rather than via `JsonSaveContext` for two reasons that both come down to being
	//! able to defend the bytes: a save context serialises every declared field, so the omissions
	//! that make this report honest (`kills`, `longest_kill_m`, …) would come back as zeros; and
	//! its exact output shape is a RUNTIME property this lane cannot observe, whereas this function
	//! is fully determined by code that compiles.
	//!
	//! Assembled in steps, never one long `+` chain: a chain of 9 fields is a measured
	//! `Formula too complex`, whose SECOND diagnostic is a misleading `Incompatible parameter`.
	protected static string BuildPayload(string outcome, string winner, notnull array<string> playerRows)
	{
		string json = "{\"match\":{";
		json += string.Format("\"source_match_id\":\"%1\"", JsonEscape(s_sSourceMatchId));
		json += string.Format(",\"event_id\":\"%1\"", JsonEscape(TBD_BackendConfig.GetEventId()));

		// The mod's configured missionId. In production this is the mission's UUID — the same id
		// `GET /api/v1/missions/{id}/compiled` is fetched with, and that route hard-requires a UUID
		// (`Uuid::parse_str` else 400, handlers/missions.rs). On a local `$profile:` fallback boot
		// it is a content-hash id like `msn_8f3a2c`, which `parse_uuid_opt` correctly drops to NULL
		// — there is no mission row for it, so a NULL is the truthful answer.
		json += string.Format(",\"mission_id\":\"%1\"", JsonEscape(TBD_BackendConfig.GetMissionId()));
		json += string.Format(",\"terrain\":\"%1\"", JsonEscape(GetTerrain()));
		json += string.Format(",\"started_at\":\"%1\"", s_sStartedAtUtc);
		json += string.Format(",\"ended_at\":\"%1\"", s_sEndedAtUtc);
		json += string.Format(",\"outcome\":\"%1\"", outcome);
		json += string.Format(",\"winning_faction\":\"%1\"", JsonEscape(winner));
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

	//------------------------------------------------------------------------------------------------
	//! One `players[]` entry. Only the four fields the mod can defend; everything else is absent on
	//! purpose (see the class header).
	protected static string BuildPlayerRow(string armaId, string role, int deaths, string sourceEventId)
	{
		string row = "{";
		row += string.Format("\"arma_id\":\"%1\"", JsonEscape(armaId));
		row += string.Format(",\"role_played\":\"%1\"", JsonEscape(role));
		row += string.Format(",\"deaths\":%1", deaths);
		row += string.Format(",\"source_event_id\":\"%1\"", JsonEscape(sourceEventId));
		row += "}";
		return row;
	}

	//------------------------------------------------------------------------------------------------
	//! Terrain key from the mission header, or empty. The backend allowlists `everon|arland|custom`
	//! and stores NULL for anything else, so an unexpected key degrades rather than 400s.
	protected static string GetTerrain()
	{
		TBD_MissionDocumentStruct mission = TBD_MissionLoader.GetMission();
		if (!mission || !mission.meta)
			return string.Empty;

		return mission.meta.terrain;
	}

	//------------------------------------------------------------------------------------------------
	//! Stable, unique-per-round idempotency key. Computed ONCE when the round goes LIVE and reused
	//! by every retry — recomputing it would defeat the upsert and create duplicate match rows.
	//! `GetTickCount()` disambiguates two rounds of the same mission starting in the same second.
	protected static string BuildSourceMatchId(string startedAtUtc)
	{
		string missionId = TBD_BackendConfig.GetMissionId();
		if (missionId.IsEmpty())
			missionId = "unknown-mission";

		return string.Format("%1@%2#%3", missionId, startedAtUtc, System.GetTickCount());
	}

	//------------------------------------------------------------------------------------------------
	//! Backend description for the armed line, without ever printing the service token.
	protected static string DescribeBackend()
	{
		string url = TBD_BackendConfig.GetBackendUrl();
		if (url.IsEmpty())
			return "(none)";

		if (TBD_BackendConfig.GetServerToken().IsEmpty())
			return url + " (NO TOKEN)";

		return url;
	}

	//------------------------------------------------------------------------------------
	// PRIMITIVES
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! RFC 3339 UTC, which is what the backend's `DateTime<Utc>` (serde/chrono) parses:
	//! `2026-07-25T16:31:28Z`.
	//!
	//! Proven, not assumed (T-181.13.1 compile probe; negative control
	//! `System.GetYearMonthDayUTCZZ` -> `Undefined function`): `System.GetYearMonthDayUTC` and
	//! `System.GetHourMinuteSecondUTC` both resolve against the real dedicated-server script API.
	//! `Arm()` prints the resulting value on every boot so the FORMAT is proved by a run, not by a
	//! compile — measured `utcNow=2026-07-25T14:41:45Z` on a host running UTC+2.
	//!
	//! Date and clock are two separate reads, so a call that straddles midnight could pair
	//! tomorrow's date with 23:59:59. Left alone deliberately: it is a one-second-per-day window on
	//! a field used for match ordering, and the fix (re-read and compare) costs more complexity than
	//! the defect is worth.
	protected static string UtcNowIso8601()
	{
		int year, month, day;
		int hour, minute, second;
		System.GetYearMonthDayUTC(year, month, day);
		System.GetHourMinuteSecondUTC(hour, minute, second);

		string date = string.Format("%1-%2-%3", year, Pad2(month), Pad2(day));
		string time = string.Format("%1:%2:%3", Pad2(hour), Pad2(minute), Pad2(second));
		return date + "T" + time + "Z";
	}

	//------------------------------------------------------------------------------------------------
	//! Zero-pad to two digits — `string.Format` has no width specifier, and RFC 3339 is fixed-width.
	//! No ternary: Enforce Script has none (`cond ? a : b` fails with `Broken expression`).
	protected static string Pad2(int value)
	{
		if (value < 10)
			return string.Format("0%1", value);

		return string.Format("%1", value);
	}

	//------------------------------------------------------------------------------------------------
	//! Make a string safe to sit inside a JSON double-quoted scalar.
	//!
	//! Load-bearing: `role_played`, `winning_faction` and `terrain` all come from the authored
	//! mission document, so a role label containing a quote or a backslash would otherwise emit
	//! malformed JSON and the whole round's report would 400.
	//!
	//! TWO measured landmines in five lines:
	//!   * `string.Replace()` MUTATES IN PLACE and returns a COUNT — `s = s.Replace(a, b)` does not
	//!     compile. The calls below are statements, and their return value is deliberately unused.
	//!   * because it mutates in place, `string escaped = value;` risks mutating the CALLER's
	//!     string (here: a live field of the parsed mission document). `string.Format("%1", value)`
	//!     is used instead of a plain assignment because it unambiguously produces a fresh string.
	//!
	//! Backslash must be escaped FIRST or it would double-escape the backslashes this very function
	//! introduces for the quotes. Control characters are folded to spaces rather than `\uXXXX`
	//! escaped: no field on this wire is allowed to carry a newline anyway, and a space keeps the
	//! payload one greppable line in the log.
	protected static string JsonEscape(string value)
	{
		string escaped = string.Format("%1", value);
		escaped.Replace("\\", "\\\\");
		escaped.Replace("\"", "\\\"");
		escaped.Replace("\n", " ");
		escaped.Replace("\r", " ");
		escaped.Replace("\t", " ");
		return escaped;
	}
}
