/**
 * @file TBD_RuntimeSessionClosing.c
 * @brief Closes the runtime session of a world that has ended: offline heartbeat, then end.
 *
 * Role: reports each handed-over session offline and then ends it, which ends every player life
 * still open in it.  Position: fed by `TBD_RuntimeSession.Stop` and by a start answered for a
 * world that has stopped; `TBD_RuntimeSession` starts the next session only while `IsClosing()`
 * is false; posts through `TBD_GameRuntimeHttp`.
 * State: the queue of sessions being closed and the one call in flight; server statics.
 * Invariants: sessions close one at a time in hand-over order, so the platform sees the old
 * session end before the new one starts; the platform marks a server offline by itself only when
 * a session expires, so a clean shutdown reports it first; a failed step is logged and not
 * repeated, because the platform expires a silent session and the next start supersedes it.
 */

//! A session being closed.
class TBD_ClosingRuntimeSession
{
	string m_sSessionId; //!< the session to close
	int m_iGeneration; //!< its generation
	int m_iSequence; //!< the last heartbeat sequence sent in the session
	bool m_bReportedOffline; //!< the offline heartbeat step is done; the end request is next
}

//! The offline heartbeat or the end request of the session being closed.
class TBD_RuntimeSessionClosingCall : TBD_GameRuntimeCall
{
	//! Hand the answer to `TBD_RuntimeSessionClosing.OnCallAnswered`.
	//! @authority server
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_RuntimeSessionClosing.OnCallAnswered(this, answer);
	}
}

//! Serial closer of ended runtime sessions.
//! @authority server
class TBD_RuntimeSessionClosing
{
	protected static const string CH_RUNTIME = "Runtime"; //!< the runtime session's log channel

	protected static ref array<ref TBD_ClosingRuntimeSession> s_aSessions; //!< sessions still to close, in hand-over order
	protected static ref TBD_RuntimeSessionClosingCall s_InFlight; //!< the step awaiting its answer, or null

	//! Queue `sessionId`, whose last heartbeat carried `lastSequence`, for closing; an empty id
	//! does nothing.
	//! @authority server
	static void Close(string sessionId, int generation, int lastSequence)
	{
		if (sessionId.IsEmpty())
			return;

		if (!s_aSessions)
			s_aSessions = new array<ref TBD_ClosingRuntimeSession>();

		TBD_ClosingRuntimeSession closing = new TBD_ClosingRuntimeSession();
		closing.m_sSessionId = sessionId;
		closing.m_iGeneration = generation;
		closing.m_iSequence = lastSequence;
		s_aSessions.Insert(closing);

		TBD_Log.Kv(CH_RUNTIME, "closing", "session=" + sessionId);
		Next();
	}

	//! Whether a session is still being closed.
	//! @return true while a step is in flight or a session is queued
	static bool IsClosing()
	{
		if (s_InFlight)
			return true;

		if (!s_aSessions)
			return false;

		return s_aSessions.Count() > 0;
	}

	//! The next step of the first session: its offline heartbeat, then its end. A step that cannot
	//! be sent settles as TRANSIENT at once.
	//! @route POST /api/v1/game-runtime/sessions/{id}/heartbeats
	//! @route POST /api/v1/game-runtime/sessions/{id}/end
	//! @authority server
	protected static void Next()
	{
		if (s_InFlight || !s_aSessions || s_aSessions.IsEmpty())
			return;

		TBD_ClosingRuntimeSession closing = s_aSessions[0];
		string path = TBD_GameRuntimeHttp.ROUTE_PREFIX + "/sessions/" + closing.m_sSessionId;
		string body = "{}";
		if (closing.m_bReportedOffline)
		{
			path += "/end";
		}
		else
		{
			closing.m_iSequence++;
			path += "/heartbeats";
			body = string.Format("{\"generation\":%1,\"sequence\":%2,", closing.m_iGeneration, closing.m_iSequence);
			body += TBD_RuntimeStatusReadings.BuildOfflineFields();
			body += "}";
		}

		// In flight before sending, so an answer can never find the call unrecorded.
		TBD_RuntimeSessionClosingCall call = new TBD_RuntimeSessionClosingCall();
		s_InFlight = call;
		string failure;
		if (TBD_GameRuntimeHttp.Post(call, path, body, failure))
			return;

		s_InFlight = null;
		Settle(closing, TBD_EGameRuntimeOutcome.TRANSIENT, failure);
	}

	//! Called by `TBD_RuntimeSessionClosingCall` with the answer to the step in flight; an answer to
	//! any other call is ignored.
	//! @authority server
	static void OnCallAnswered(notnull TBD_RuntimeSessionClosingCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (call != s_InFlight)
			return;

		s_InFlight = null;
		if (!s_aSessions || s_aSessions.IsEmpty())
			return;

		TBD_ClosingRuntimeSession closing = s_aSessions[0];
		Settle(closing, answer.m_eOutcome, answer.m_sDetail);
	}

	//! Log a step's outcome and move on: after the offline heartbeat to the end request, after the
	//! end request to the next session.
	protected static void Settle(notnull TBD_ClosingRuntimeSession closing, TBD_EGameRuntimeOutcome outcome, string detail)
	{
		if (!closing.m_bReportedOffline)
		{
			if (outcome == TBD_EGameRuntimeOutcome.SUCCESS)
				TBD_Log.Kv(CH_RUNTIME, "reported-offline", "session=" + closing.m_sSessionId);
			else
				TBD_Log.Warn(CH_RUNTIME, string.Format("offline report for session=%1 failed (%2) - ending the session anyway", closing.m_sSessionId, detail));

			closing.m_bReportedOffline = true;
			Next();
			return;
		}

		if (outcome == TBD_EGameRuntimeOutcome.SUCCESS)
			TBD_Log.Kv(CH_RUNTIME, "session-ended", string.Format("session=%1 %2", closing.m_sSessionId, detail));
		else
			TBD_Log.Warn(CH_RUNTIME, string.Format("ending session=%1 failed (%2) - the platform expires it once no heartbeat arrives, or supersedes it at the next start",
				closing.m_sSessionId, detail));

		s_aSessions.RemoveOrdered(0);
		Next();
	}
}
