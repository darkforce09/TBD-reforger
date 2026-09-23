//! Closing the runtime session of a world that has ended: an offline heartbeat, then
//! `POST /api/v1/game-runtime/sessions/{sessionId}/end`, which ends the session and every player
//! life still open in it. The platform marks a server offline by itself only when a session
//! expires, so a clean shutdown reports it first.
//!
//! Sessions close one at a time, in the order they were handed over. TBD_RuntimeSession starts the
//! next world's session only while `IsClosing()` is false, so the platform sees the old session end
//! before the new one starts. A failed step is logged and not repeated: the platform expires a
//! silent session on its own, and the next start supersedes it.
//! @authority server

//! A session being closed.
class TBD_ClosingRuntimeSession
{
	string m_sSessionId;
	int m_iGeneration;
	//! The last heartbeat sequence sent in the session.
	int m_iSequence;
	bool m_bReportedOffline;
}

//! The offline heartbeat or the end request of the session being closed.
class TBD_RuntimeSessionClosingCall : TBD_GameRuntimeCall
{
	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_RuntimeSessionClosing.OnCallAnswered(this, answer);
	}
}

class TBD_RuntimeSessionClosing
{
	//! The runtime session's greppable channel.
	protected static const string CH_RUNTIME = "Runtime";

	protected static ref array<ref TBD_ClosingRuntimeSession> s_aSessions;
	protected static ref TBD_RuntimeSessionClosingCall s_InFlight;

	//------------------------------------------------------------------------------------------------
	//! Close `sessionId`, whose last heartbeat carried `lastSequence`.
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

	//------------------------------------------------------------------------------------------------
	//! True while a session is still being closed.
	static bool IsClosing()
	{
		if (s_InFlight)
			return true;

		if (!s_aSessions)
			return false;

		return s_aSessions.Count() > 0;
	}

	//------------------------------------------------------------------------------------------------
	//! The next step of the first session: its offline heartbeat, then its end.
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

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_RuntimeSessionClosingCall with the answer to the step in flight.
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

	//------------------------------------------------------------------------------------------------
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
