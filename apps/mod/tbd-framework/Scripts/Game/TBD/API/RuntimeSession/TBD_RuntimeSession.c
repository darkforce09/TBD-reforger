/**
 * @file TBD_RuntimeSession.c
 * @brief This server's game runtime on the platform: one session per world, fenced by generation.
 *
 * Role: starts the session once the world has decided what it runs, reporting the loaded artifact
 * (`TBD_LoadedArtifactReport`) so the start confirms a mission deployment, then sends a heartbeat
 * every `heartbeat_interval_seconds` with the generation, a strictly increasing sequence and the
 * readings of `TBD_RuntimeStatusReadings`.  Position: started and stopped by
 * `TBD_RuntimeSessionLifecycle`; sends through `TBD_GameRuntimeHttp`; `Stop` hands the session to
 * `TBD_RuntimeSessionClosing`; `GetSessionId` is what deployment authorisation, ended lives and
 * fleet command claims address.
 * State: world counter, running and terminated flags, the session id, generation, sequence,
 * interval, failure count and the one call in flight; server statics.  Invariants: one start or
 * heartbeat is in flight at a time; a world's session starts only after the previous world's has
 * closed; an answer for a world or session other than the current one is discarded. A rejected
 * artifact report (422 `UNKNOWN_ARTIFACT`, 400) is an error and the session starts without one.
 * Starting supersedes the server's previous session with every player life open in it. A refused
 * heartbeat steers the loop: STALE_SEQUENCE continues past `last_sequence`, STALE_GENERATION takes
 * the session's generation, RUNTIME_SESSION_ENDED `expired` starts a new session, and `superseded`,
 * `credential_revoked` or `ended_by_runtime` stop the loop instead of competing. No answer, a
 * timeout or a server failure backs off and retries, never stopping the loop. Without a usable
 * machine credential the world holds no session and looks again every minute.
 */

//! The runtime session loop: start, heartbeats, refusals, retry.
//! @authority server
class TBD_RuntimeSession
{
	protected static const string CH_RUNTIME = "Runtime"; //!< log channel: `grep '\[TBD\]\[Runtime\]' console.log`
	protected static const int DEFAULT_HEARTBEAT_INTERVAL_S = 15; //!< heartbeat interval until the platform states its own, in seconds
	protected static const int RETRY_BASE_MS = 2000; //!< first retry delay, in milliseconds
	protected static const int RETRY_CAP_MS = 60000; //!< retry delay ceiling, in milliseconds
	protected static const int WAIT_POLL_MS = 1000; //!< re-check period of a start waiting on the previous closing or the loaded artifact, in milliseconds
	protected static const int CREDENTIAL_POLL_MS = 60000; //!< re-check period without a usable credential, in milliseconds; the platform loops re-read the config

	protected static int s_iWorld; //!< bumped by `Start` and `Stop`; an answer for another world is stale
	protected static bool s_bRunning; //!< between `Start` and `Stop`
	protected static bool s_bTerminated; //!< this world holds no session any more: credential refused, or another runtime owns the session
	protected static string s_sSessionId; //!< the session held, or empty
	protected static int s_iGeneration; //!< the held session's generation, or 0
	protected static int s_iSequence; //!< the last heartbeat sequence sent in `s_sSessionId`
	protected static int s_iHeartbeatIntervalMs; //!< heartbeat period, in milliseconds
	protected static int s_iFailures; //!< consecutive start or heartbeat attempts without an admitted answer
	protected static bool s_bHeartbeatAdmitted; //!< a heartbeat of this session was admitted
	protected static bool s_bArtifactReportRejected; //!< the platform rejected this world's artifact report; the session starts without one
	protected static bool s_bWaitForArtifactLogged; //!< the wait for the loaded artifact is logged once per world
	protected static ref TBD_RuntimeSessionCall s_InFlight; //!< the one start or heartbeat awaiting its answer, or null

	//! The session this runtime holds.
	//! @return the session id, or empty while it holds none
	static string GetSessionId()
	{
		if (!s_bRunning || s_bTerminated)
			return string.Empty;

		return s_sSessionId;
	}

	//! The generation of the session held.
	//! @return the generation, or 0 without a session
	static int GetGeneration()
	{
		if (GetSessionId().IsEmpty())
			return 0;

		return s_iGeneration;
	}

	//! True while this world holds a session or is still getting one. False before the game starts,
	//! after it ends, without a usable machine credential, and once the loop has stopped.
	//! @return true while a session is held or being started
	static bool CanHoldSession()
	{
		return s_bRunning && !s_bTerminated && TBD_GameRuntimeHttp.IsConfigured();
	}

	//! A request against `sessionId` was refused because that session has ended. Acted on only while
	//! it is still the session this runtime holds.
	//! @param endReason the platform's `end_reason`
	//! @authority server
	static void ReportSessionEnded(string sessionId, string endReason)
	{
		if (sessionId.IsEmpty() || sessionId != GetSessionId())
			return;

		TBD_Log.Kv(CH_RUNTIME, "session-ended-reported", string.Format("session=%1 endReason=%2", sessionId, endReason));
		OnSessionEnded(endReason);
	}

	//! Begin this world's session. Called once the game starts, on the authority of a framework
	//! world. A session still recorded from an earlier world is superseded by the new start. Does
	//! nothing on a client.
	//! @authority server
	static void Start()
	{
		if (TBD_Authority.IsClient())
			return;

		if (!TBD_BackendConfig.Get())
			TBD_BackendConfig.Load();

		s_iWorld++;
		s_bRunning = true;
		s_bTerminated = false;
		s_bArtifactReportRejected = false;
		s_bWaitForArtifactLogged = false;
		ForgetSession();

		// Without a credential the loop waits for one (see Step); local and PIE hosts carry none, a
		// legal state.
		if (TBD_GameRuntimeHttp.IsConfigured())
			TBD_Log.Kv(CH_RUNTIME, "starting", "backend=" + TBD_GameRuntimeHttp.DescribeBackend());
		else
			TBD_Log.Event(CH_RUNTIME, "no runtime session until a machine credential is configured - backend=" + TBD_GameRuntimeHttp.DescribeBackend());

		ScheduleStep(0);
	}

	//! The world is ending: its session is reported offline and ended, which ends every player life
	//! still open in it.
	//! @authority server
	static void Stop()
	{
		if (!s_bRunning)
			return;

		s_bRunning = false;
		s_iWorld++;

		if (!s_sSessionId.IsEmpty() && !s_bTerminated)
			TBD_RuntimeSessionClosing.Close(s_sSessionId, s_iGeneration, s_iSequence);

		ForgetSession();

		// What this world ran is no report for the next one.
		TBD_LoadedArtifactReport.Reset();
	}

	//! This world has decided what it runs (`TBD_LoadedArtifactReport`): a start waiting on it goes now.
	//! @authority server
	static void OnLoadedArtifactDecided()
	{
		if (s_bRunning && !s_bTerminated && s_sSessionId.IsEmpty() && !s_InFlight)
			ScheduleStep(0);
	}

	//! Drop the held session's id, generation, sequence, interval and failure count.
	protected static void ForgetSession()
	{
		s_sSessionId = string.Empty;
		s_iGeneration = 0;
		s_iSequence = 0;
		s_iHeartbeatIntervalMs = DEFAULT_HEARTBEAT_INTERVAL_S * 1000;
		s_iFailures = 0;
		s_bHeartbeatAdmitted = false;
	}

	//! Schedule `Step` after `delayMs`; one step is pending per process, and scheduling again
	//! replaces it.
	protected static void ScheduleStep(int delayMs)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		queue.Remove(Step);
		queue.CallLater(Step, delayMs, false);
	}

	//! Start the session, or send its next heartbeat.
	protected static void Step()
	{
		if (s_InFlight || !s_bRunning || s_bTerminated)
			return;

		if (!TBD_GameRuntimeHttp.IsConfigured())
		{
			ScheduleStep(CREDENTIAL_POLL_MS);
			return;
		}

		if (!s_sSessionId.IsEmpty())
		{
			SendHeartbeat();
			return;
		}

		// A start supersedes the server's current session: the previous world's ends first.
		if (TBD_RuntimeSessionClosing.IsClosing())
		{
			ScheduleStep(WAIT_POLL_MS);
			return;
		}

		// The start reports what this world runs, so it waits until the world has decided that.
		if (!TBD_LoadedArtifactReport.IsDecided())
		{
			if (!s_bWaitForArtifactLogged)
			{
				s_bWaitForArtifactLogged = true;
				TBD_Log.Event(CH_RUNTIME, "session start waits until this world's mission artifact has loaded (or no mission is decided)");
			}

			ScheduleStep(WAIT_POLL_MS);
			return;
		}

		bool reportArtifact = TBD_LoadedArtifactReport.NamesArtifact() && !s_bArtifactReportRejected;
		string body = "{}";
		if (reportArtifact)
			body = TBD_LoadedArtifactReport.BuildStartBody();

		Send(TBD_ERuntimeSessionRequest.START, TBD_GameRuntimeHttp.ROUTE_PREFIX + "/sessions", body, reportArtifact);
	}

	//! Send the next heartbeat of the held session.
	protected static void SendHeartbeat()
	{
		s_iSequence++;
		string body = string.Format("{\"generation\":%1,\"sequence\":%2,", s_iGeneration, s_iSequence);
		body += TBD_RuntimeStatusReadings.BuildOnlineFields();
		body += "}";

		string path = TBD_GameRuntimeHttp.ROUTE_PREFIX + "/sessions/" + s_sSessionId + "/heartbeats";
		Send(TBD_ERuntimeSessionRequest.HEARTBEAT, path, body, false);
	}

	//! Record the call in flight and POST it; a send that cannot start retries later.
	//! @param reportedArtifact the start body names a loaded artifact
	//! @route POST /api/v1/game-runtime/sessions
	//! @route POST /api/v1/game-runtime/sessions/{id}/heartbeats
	//! @authority server
	protected static void Send(TBD_ERuntimeSessionRequest request, string path, string body, bool reportedArtifact)
	{
		TBD_RuntimeSessionCall call = new TBD_RuntimeSessionCall();
		call.m_eRequest = request;
		call.m_iWorld = s_iWorld;
		call.m_sSessionId = s_sSessionId;
		call.m_bReportedArtifact = reportedArtifact;

		// In flight before sending, so an answer can never find the call unrecorded.
		s_InFlight = call;
		string failure;
		if (TBD_GameRuntimeHttp.Post(call, path, body, failure))
			return;

		s_InFlight = null;
		RetryLater(typename.EnumToString(TBD_ERuntimeSessionRequest, request), failure);
	}

	//! Called by `TBD_RuntimeSessionCall` with the answer to the start or heartbeat in flight; an
	//! answer to any other call is ignored.
	//! @authority server
	static void OnCallAnswered(notnull TBD_RuntimeSessionCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (call != s_InFlight)
			return;

		s_InFlight = null;
		if (call.m_eRequest == TBD_ERuntimeSessionRequest.START)
			SettleStart(call, answer);
		else
			SettleHeartbeat(call, answer);
	}

	//! Settle a start answer: hold the started session, close one started for a world that has
	//! stopped, retry a transient failure, restart without the artifact report the platform
	//! rejected, or stop the loop on any other refusal.
	protected static void SettleStart(notnull TBD_RuntimeSessionCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		bool current = call.m_iWorld == s_iWorld && CanHoldSession();

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			TBD_StartedRuntimeSessionStruct started = TBD_StartedRuntimeSessionStruct.Parse(answer.m_sBody);
			if (!started)
			{
				TBD_Log.Error(CH_RUNTIME, "session start answered with an unreadable body: " + answer.m_sDetail);
				if (current)
					RetryLater("session start", "unreadable answer");
				else
					ScheduleStep(0);
				return;
			}

			if (!current)
			{
				// Started for a world that has stopped since: closed now rather than left holding the
				// server's session until it expires.
				TBD_RuntimeSessionClosing.Close(started.runtime_session_id, started.generation, 0);
				ScheduleStep(0);
				return;
			}

			s_sSessionId = started.runtime_session_id;
			s_iGeneration = started.generation;
			s_iSequence = 0;
			s_iFailures = 0;
			s_bHeartbeatAdmitted = false;
			if (started.heartbeat_interval_seconds > 0)
				s_iHeartbeatIntervalMs = started.heartbeat_interval_seconds * 1000;

			string reported = "artifact=none";
			if (call.m_bReportedArtifact)
				reported = TBD_LoadedArtifactReport.Describe();

			TBD_Log.Kv(CH_RUNTIME, "session-started", string.Format("session=%1 generation=%2 heartbeatSeconds=%3 expiresAfterSeconds=%4 %5",
				s_sSessionId, s_iGeneration, started.heartbeat_interval_seconds, started.expires_after_seconds, reported));
			ScheduleStep(0);
			return;
		}

		if (!current)
		{
			ScheduleStep(0);
			return;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
		{
			RetryLater("session start", answer.m_sDetail);
			return;
		}

		// The platform rejects the artifact report itself: the session starts without one rather than
		// leave the server without a session.
		if (call.m_bReportedArtifact && (answer.m_eCode == HttpCode.HTTP_CODE_422 || answer.m_eCode == HttpCode.HTTP_CODE_400))
		{
			s_bArtifactReportRejected = true;
			TBD_Log.Error(CH_RUNTIME, string.Format("the platform rejected this world's artifact report (%1; %2) - starting the session WITHOUT a loaded artifact, so it confirms no deployment and event seats are refused as not in the loaded mission",
				TBD_LoadedArtifactReport.Describe(), answer.m_sDetail));
			ScheduleStep(0);
			return;
		}

		Terminate("the platform refused to start a runtime session for this server's machine credential (" + answer.m_sDetail + ")");
	}

	//! Settle a heartbeat answer: schedule the next on success, steer on a refusal, retry a
	//! transient failure, start a new session for an unknown one, stop on a rejected credential,
	//! and back off on a malformed request.
	protected static void SettleHeartbeat(notnull TBD_RuntimeSessionCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		// The session this answer is about is not the one held now.
		if (call.m_iWorld != s_iWorld || call.m_sSessionId != GetSessionId())
		{
			ScheduleStep(0);
			return;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			if (!s_bHeartbeatAdmitted || s_iFailures > 0)
				TBD_Log.Kv(CH_RUNTIME, "heartbeat-admitted", string.Format("session=%1 sequence=%2 afterFailures=%3", s_sSessionId, s_iSequence, s_iFailures));

			s_bHeartbeatAdmitted = true;
			s_iFailures = 0;
			ScheduleStep(s_iHeartbeatIntervalMs);
			return;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.REFUSED)
		{
			SettleHeartbeatRefusal(answer.m_Refusal, answer.m_sDetail);
			return;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
		{
			RetryLater("heartbeat", answer.m_sDetail);
			return;
		}

		if (answer.m_eCode == HttpCode.HTTP_CODE_404)
		{
			TBD_Log.Warn(CH_RUNTIME, string.Format("the platform does not know session=%1 (%2) - starting a new session", s_sSessionId, answer.m_sDetail));
			ForgetSession();
			ScheduleStep(0);
			return;
		}

		if (answer.m_eCode == HttpCode.HTTP_CODE_401 || answer.m_eCode == HttpCode.HTTP_CODE_403)
		{
			Terminate("the platform rejected this server's machine credential (" + answer.m_sDetail + ")");
			return;
		}

		// A body the platform could not read is this client's defect; the loop keeps trying, backing off.
		TBD_Log.Error(CH_RUNTIME, "heartbeat rejected as malformed (" + answer.m_sDetail + ")");
		RetryLater("heartbeat", "rejected as malformed");
	}

	//! Act on a 409 fence refusal of a heartbeat (see the file header).
	protected static void SettleHeartbeatRefusal(notnull TBD_GameRuntimeRefusalDetails refusal, string detail)
	{
		if (refusal.code == "STALE_SEQUENCE" && refusal.last_sequence >= s_iSequence)
		{
			s_iSequence = refusal.last_sequence;
			TBD_Log.Kv(CH_RUNTIME, "stale-sequence", string.Format("session=%1 lastAdmitted=%2 next=%3", s_sSessionId, refusal.last_sequence, s_iSequence + 1));
			ScheduleStep(0);
			return;
		}

		if (refusal.code == "STALE_GENERATION" && refusal.generation >= 1 && refusal.generation != s_iGeneration)
		{
			TBD_Log.Error(CH_RUNTIME, string.Format("session=%1 is generation %2, not the recorded %3 - continuing with the session's own generation",
				s_sSessionId, refusal.generation, s_iGeneration));
			s_iGeneration = refusal.generation;
			ScheduleStep(0);
			return;
		}

		if (refusal.code == "RUNTIME_SESSION_ENDED")
		{
			OnSessionEnded(refusal.end_reason);
			return;
		}

		RetryLater("heartbeat", "unhandled refusal " + detail);
	}

	//! The session held has ended on the platform, and with it every player life open in it.
	protected static void OnSessionEnded(string endReason)
	{
		string ended = s_sSessionId;

		if (endReason == "expired")
		{
			TBD_Log.Warn(CH_RUNTIME, string.Format("session=%1 expired on the platform (no heartbeat admitted in time; its player lives ended with it) - starting a new session", ended));
			ForgetSession();
			ScheduleStep(0);
			return;
		}

		string why = string.Format("the session ended on the platform with end_reason '%1'", endReason);
		if (endReason == "superseded")
			why = "another runtime started a session for this server (superseded)";
		else if (endReason == "credential_revoked")
			why = "this server's machine credential was revoked (credential_revoked) - issue a new mod_runtime credential and restart the server";
		else if (endReason == "ended_by_runtime")
			why = "the session was ended through its end route by a call this world did not make (ended_by_runtime)";

		Terminate(string.Format("%1; session=%2", why, ended));
	}

	//! Stop holding a session for the rest of this world.
	protected static void Terminate(string why)
	{
		s_bTerminated = true;
		ForgetSession();
		TBD_Log.Error(CH_RUNTIME, "runtime session loop STOPPED - " + why + ". No more heartbeats and no competing session from this runtime; deployments into event seats cannot be authorized until the server restarts.");
	}

	//! Back off exponentially and try again; the loop never stops on a failure without an answer.
	protected static void RetryLater(string what, string detail)
	{
		s_iFailures++;
		int delay = TBD_GameRuntimeHttp.BackoffMs(s_iFailures, RETRY_BASE_MS, RETRY_CAP_MS);
		TBD_Log.Warn(CH_RUNTIME, string.Format("%1 not admitted (%2) - attempt %3, retrying in %4 ms", what, detail, s_iFailures, delay));
		ScheduleStep(delay);
	}
}
