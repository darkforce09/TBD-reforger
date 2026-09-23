//! Reports ended player lives to the platform: `POST /api/v1/game-runtime/sessions/{sessionId}/
//! deployments/{occupancyId}/end`, one report per life TBD_DeploymentAuthorization closes.
//!
//! A bounded retry queue. At most CAPACITY reports wait at once (the one in flight included), and
//! they are delivered one at a time in the order the lives ended. A report that gets no answer, a
//! timeout or a server-side failure goes back to the head and is retried with exponential backoff;
//! a report the platform rejects outright (unknown life or session, rejected credential) is dropped
//! with an ERROR. When the queue is full, the oldest waiting report is dropped with an ERROR naming
//! the life it could not report; the platform then closes that life when its runtime session ends.
//!
//! Repeating a report is harmless: the platform ends exactly the named life, answers a repeat with
//! how that life ended, and never ends a newer life in the same slot. Each report names the session
//! its life belongs to, so it stays deliverable after that session, or the world, has ended.
//! @authority server

//! One ended life waiting to be reported.
class TBD_DeploymentEndReport
{
	string m_sRuntimeSessionId;
	string m_sOccupancyId;
	string m_sArmaId;
	//! Empty when the report closes a life whose slot this server never learned.
	string m_sOrbatSlotId;
	//! Why this server ended the life; for the log only.
	string m_sReason;
	int m_iFailures;
	//! `TBD_GameRuntimeHttp.NowMs()` from which the report may be sent.
	int m_iNotBeforeMs;
}

//! An end-of-life report on its way to the platform.
class TBD_DeploymentEndCall : TBD_GameRuntimeCall
{
	ref TBD_DeploymentEndReport m_Report;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_DeploymentEndQueue.OnCallAnswered(this, answer);
	}
}

class TBD_DeploymentEndQueue
{
	//! Greppable channel shared with the rest of the deployment flow.
	protected static const string CH_DEPLOYMENT = "Deployment";

	//! One report per concurrent life at a round's end on the largest server, with headroom.
	static const int CAPACITY = 256;

	protected static const int RETRY_BASE_MS = 2000;
	protected static const int RETRY_CAP_MS = 60000;
	protected static const int PUMP_MS = 1000;

	protected static ref array<ref TBD_DeploymentEndReport> s_aQueue;
	protected static ref TBD_DeploymentEndCall s_InFlight;
	protected static bool s_bTicking;

	//------------------------------------------------------------------------------------------------
	//! Queue the end of one life. Delivery starts at once when nothing else is in flight.
	static void Enqueue(string runtimeSessionId, string occupancyId, string armaId, string orbatSlotId, string reason)
	{
		if (runtimeSessionId.IsEmpty() || occupancyId.IsEmpty())
			return;

		if (!s_aQueue)
			s_aQueue = new array<ref TBD_DeploymentEndReport>();

		if (Pending() >= CAPACITY && !s_aQueue.IsEmpty())
		{
			TBD_DeploymentEndReport dropped = s_aQueue[0];
			s_aQueue.RemoveOrdered(0);
			TBD_Log.Error(CH_DEPLOYMENT, string.Format("end-of-life queue full (%1) - DROPPED the oldest report: occupancy=%2 session=%3 reason='%4'. The platform keeps that life open until its runtime session ends.",
				CAPACITY, dropped.m_sOccupancyId, dropped.m_sRuntimeSessionId, dropped.m_sReason));
		}

		TBD_DeploymentEndReport report = new TBD_DeploymentEndReport();
		report.m_sRuntimeSessionId = runtimeSessionId;
		report.m_sOccupancyId = occupancyId;
		report.m_sArmaId = armaId;
		report.m_sOrbatSlotId = orbatSlotId;
		report.m_sReason = reason;
		report.m_iNotBeforeMs = TBD_GameRuntimeHttp.NowMs();
		s_aQueue.Insert(report);

		TBD_Log.Kv(CH_DEPLOYMENT, "life-end-queued", string.Format("occupancy=%1 session=%2 reason='%3' pending=%4",
			occupancyId, runtimeSessionId, reason, Pending()));

		StartTicking();
		Pump();
	}

	//------------------------------------------------------------------------------------------------
	//! True while a report of `runtimeSessionId` for the player `armaId`, or for the slot
	//! `orbatSlotId`, has not been delivered. A deployment request waits on this, so the platform
	//! closes the old life before it is asked to open the next one.
	static bool HasPendingFor(string runtimeSessionId, string armaId, string orbatSlotId)
	{
		if (s_InFlight && s_InFlight.m_Report && Concerns(s_InFlight.m_Report, runtimeSessionId, armaId, orbatSlotId))
			return true;

		if (!s_aQueue)
			return false;

		foreach (TBD_DeploymentEndReport report : s_aQueue)
		{
			if (report && Concerns(report, runtimeSessionId, armaId, orbatSlotId))
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool Concerns(notnull TBD_DeploymentEndReport report, string runtimeSessionId, string armaId, string orbatSlotId)
	{
		if (report.m_sRuntimeSessionId != runtimeSessionId)
			return false;

		if (!armaId.IsEmpty() && report.m_sArmaId == armaId)
			return true;

		return !orbatSlotId.IsEmpty() && report.m_sOrbatSlotId == orbatSlotId;
	}

	//------------------------------------------------------------------------------------------------
	//! Reports waiting or in flight.
	protected static int Pending()
	{
		int count = 0;
		if (s_aQueue)
			count = s_aQueue.Count();

		if (s_InFlight)
			count++;

		return count;
	}

	//------------------------------------------------------------------------------------------------
	//! One repeating tick while reports wait, for the backoff schedule. Statics outlive a world, and
	//! so does this tick: a report queued as a world ends is still delivered in the next one.
	protected static void StartTicking()
	{
		if (s_bTicking)
			return;

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		s_bTicking = true;
		queue.CallLater(Pump, PUMP_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	protected static void StopTicking()
	{
		if (!s_bTicking)
			return;

		s_bTicking = false;
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Pump);
	}

	//------------------------------------------------------------------------------------------------
	//! Send the head report when it is due and nothing is in flight.
	protected static void Pump()
	{
		if (s_InFlight)
			return;

		if (!s_aQueue || s_aQueue.IsEmpty())
		{
			StopTicking();
			return;
		}

		TBD_DeploymentEndReport report = s_aQueue[0];
		if (!TBD_GameRuntimeHttp.IsDue(report.m_iNotBeforeMs))
			return;

		s_aQueue.RemoveOrdered(0);

		TBD_DeploymentEndCall call = new TBD_DeploymentEndCall();
		call.m_Report = report;

		// In flight before sending, so an answer can never find the call unrecorded.
		s_InFlight = call;
		string failure;
		string path = string.Format("%1/sessions/%2/deployments/%3/end", TBD_GameRuntimeHttp.ROUTE_PREFIX,
			report.m_sRuntimeSessionId, report.m_sOccupancyId);
		if (TBD_GameRuntimeHttp.Post(call, path, "{}", failure))
			return;

		s_InFlight = null;
		RetryLater(report, failure);
	}

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_DeploymentEndCall with the answer to the report in flight. A report its watchdog
	//! answered may have been admitted, and repeating it is harmless, so a TRANSIENT answer sends it
	//! again.
	static void OnCallAnswered(notnull TBD_DeploymentEndCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (call != s_InFlight)
			return;

		s_InFlight = null;
		TBD_DeploymentEndReport report = call.m_Report;

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			TBD_Log.Kv(CH_DEPLOYMENT, "life-end-reported", string.Format("occupancy=%1 session=%2 %3",
				report.m_sOccupancyId, report.m_sRuntimeSessionId, answer.m_sDetail));
		}
		else if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
		{
			RetryLater(report, answer.m_sDetail);
		}
		else
		{
			TBD_Log.Error(CH_DEPLOYMENT, string.Format("the platform rejected the end of occupancy=%1 session=%2 (%3) - report DROPPED",
				report.m_sOccupancyId, report.m_sRuntimeSessionId, answer.m_sDetail));
		}

		Pump();
	}

	//------------------------------------------------------------------------------------------------
	//! Back to the head of the queue with exponential backoff.
	protected static void RetryLater(notnull TBD_DeploymentEndReport report, string detail)
	{
		report.m_iFailures++;
		int delay = TBD_GameRuntimeHttp.BackoffMs(report.m_iFailures, RETRY_BASE_MS, RETRY_CAP_MS);
		report.m_iNotBeforeMs = TBD_GameRuntimeHttp.NowMs() + delay;
		s_aQueue.InsertAt(report, 0);

		TBD_Log.Warn(CH_DEPLOYMENT, string.Format("end of occupancy=%1 not delivered (%2) - attempt %3, retrying in %4 ms (%5 pending)",
			report.m_sOccupancyId, detail, report.m_iFailures, delay, Pending()));
	}
}
