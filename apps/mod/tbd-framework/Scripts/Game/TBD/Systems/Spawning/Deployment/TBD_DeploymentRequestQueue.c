/**
 * @file TBD_DeploymentRequestQueue.c
 * @brief Asks the platform for deployment decisions, one request at a time, retried until decided.
 *
 * Role: delivers `POST /api/v1/game-runtime/sessions/{sessionId}/deployments` with
 * `{event_mission_id, orbat_slot_id, arma_id, player_life_id}` and hands every answer to
 * TBD_DeploymentAuthorization.  Position: fed by TBD_DeploymentAuthorization; sends to the session
 * TBD_RuntimeSession holds through TBD_GameRuntimeHttp.
 * State: the bounded queue, the request in flight and the pump flag (static).
 * Invariants: a request keeps its `player_life_id`, so a repeat returns the recorded decision and
 * never opens a second life; no answer, a timeout or a server failure retries with exponential
 * backoff; a player's requests are sent in order; a request waits while TBD_DeploymentEndQueue
 * still has to report an ended life of the same player or slot in the same session; a request waits
 * while no session is held, and a session change decides waiting requests afresh and drops
 * abandoned ones.
 * @authority server
 */

//! A deployment request on its way to the platform.
class TBD_DeploymentRequestCall : TBD_GameRuntimeCall
{
	ref TBD_DeploymentRequest m_Request; //!< the request this call delivers

	//! Hand the answer to the queue.
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_DeploymentRequestQueue.OnCallAnswered(this, answer);
	}
}

//! The deployment request queue.
class TBD_DeploymentRequestQueue
{
	protected static const string CH_DEPLOYMENT = "Deployment"; //!< log channel shared with the deployment flow

	static const int CAPACITY = 256; //!< Far beyond one request per player on the largest server.

	protected static const int RETRY_BASE_MS = 2000; //!< first retry delay (ms); doubles per failure
	protected static const int RETRY_CAP_MS = 30000; //!< longest retry delay (ms)
	protected static const int PUMP_MS = 1000; //!< pump period (ms) while requests wait

	protected static ref array<ref TBD_DeploymentRequest> s_aQueue; //!< waiting requests, oldest first
	protected static ref TBD_DeploymentRequestCall s_InFlight; //!< the request being delivered; null when none
	protected static bool s_bTicking; //!< true while the pump repeats

	//! Queue `request` (dropping the oldest when full) and try to send at once.
	static void Enqueue(notnull TBD_DeploymentRequest request)
	{
		if (!s_aQueue)
			s_aQueue = new array<ref TBD_DeploymentRequest>();

		if (Pending() >= CAPACITY)
			DropOldest();

		request.m_iNotBeforeMs = TBD_GameRuntimeHttp.NowMs();
		s_aQueue.Insert(request);
		StartTicking();
		Pump();
	}

	//! The request this player still waits on, queued or in flight, or null.
	static TBD_DeploymentRequest FindWaitingFor(int playerId)
	{
		TBD_DeploymentRequest sent = InFlightRequest();
		if (sent && !sent.m_bAbandoned && sent.m_iPlayerId == playerId)
			return sent;

		if (!s_aQueue)
			return null;

		foreach (TBD_DeploymentRequest request : s_aQueue)
		{
			if (request && !request.m_bAbandoned && request.m_iPlayerId == playerId)
				return request;
		}

		return null;
	}

	//! Nobody waits for this request any more. One that never left this server is dropped; one that
	//! may have reached the platform stays until it is resolved.
	static void Abandon(notnull TBD_DeploymentRequest request)
	{
		request.m_bAbandoned = true;

		if (request.m_bSent || request == InFlightRequest() || !s_aQueue)
			return;

		int index = s_aQueue.Find(request);
		if (index >= 0)
			s_aQueue.RemoveOrdered(index);
	}

	//! Abandon every request `playerId` waits on.
	static void AbandonFor(int playerId)
	{
		TBD_DeploymentRequest waiting = FindWaitingFor(playerId);
		while (waiting)
		{
			Abandon(waiting);
			waiting = FindWaitingFor(playerId);
		}
	}

	//! Abandon every request: queued ones that never left this server are dropped, a sent one stays
	//! until it is resolved.
	static void AbandonAll()
	{
		TBD_DeploymentRequest sent = InFlightRequest();
		if (sent)
			sent.m_bAbandoned = true;

		if (!s_aQueue)
			return;

		for (int i = s_aQueue.Count() - 1; i >= 0; i--)
		{
			TBD_DeploymentRequest request = s_aQueue[i];
			if (!request)
				continue;

			request.m_bAbandoned = true;
			if (!request.m_bSent)
				s_aQueue.RemoveOrdered(i);
		}
	}

	//! The request in flight, or null.
	protected static TBD_DeploymentRequest InFlightRequest()
	{
		if (!s_InFlight)
			return null;

		return s_InFlight.m_Request;
	}

	//! Requests waiting or in flight.
	protected static int Pending()
	{
		int count = 0;
		if (s_aQueue)
			count = s_aQueue.Count();

		if (s_InFlight)
			count++;

		return count;
	}

	//! Make room: the oldest abandoned request goes first, else the oldest request, whose player is
	//! told to deploy again.
	protected static void DropOldest()
	{
		if (!s_aQueue || s_aQueue.IsEmpty())
			return;

		int index = 0;
		for (int i = 0; i < s_aQueue.Count(); i++)
		{
			if (s_aQueue[i] && s_aQueue[i].m_bAbandoned)
			{
				index = i;
				break;
			}
		}

		TBD_DeploymentRequest dropped = s_aQueue[index];
		s_aQueue.RemoveOrdered(index);
		if (!dropped)
			return;

		TBD_Log.Error(CH_DEPLOYMENT, string.Format("deployment request queue full (%1) - DROPPED player=%2 slot=%3 life=%4 abandoned=%5",
			CAPACITY, dropped.m_iPlayerId, dropped.m_sSlotUid, dropped.m_sPlayerLifeId, dropped.m_bAbandoned));

		if (!dropped.m_bAbandoned)
			TBD_DeploymentAuthorization.OnUndecided(dropped, "too many deployment requests are waiting for the platform - deploy again in a moment.");
	}

	//! One repeating tick while requests wait, for the backoff schedule and for the runtime session
	//! to become available.
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

	//! Stop the pump.
	protected static void StopTicking()
	{
		if (!s_bTicking)
			return;

		s_bTicking = false;
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Pump);
	}

	//! Send the first request that may go now, when nothing is in flight.
	protected static void Pump()
	{
		if (s_InFlight)
			return;

		if (!s_aQueue || s_aQueue.IsEmpty())
		{
			StopTicking();
			return;
		}

		if (!TBD_RuntimeSession.CanHoldSession())
		{
			FailAll("this server holds no platform runtime session, so event seats cannot be authorized - tell an admin.");
			return;
		}

		string session = TBD_RuntimeSession.GetSessionId();
		if (session.IsEmpty())
			return;

		for (int i = 0; i < s_aQueue.Count(); i++)
		{
			TBD_DeploymentRequest request = s_aQueue[i];
			if (!request)
			{
				s_aQueue.RemoveOrdered(i);
				i--;
				continue;
			}

			// Sent to a session that has ended since, and that session ended every life it held.
			if (IsPinnedElsewhere(request, session))
			{
				if (request.m_bAbandoned)
				{
					TBD_Log.Kv(CH_DEPLOYMENT, "request-dropped", string.Format("player=%1 life=%2 - abandoned, and its session %3 has ended",
						request.m_iPlayerId, request.m_sPlayerLifeId, request.m_sRuntimeSessionId));
					s_aQueue.RemoveOrdered(i);
					i--;
					continue;
				}

				request.m_sRuntimeSessionId = string.Empty;
			}

			if (!TBD_GameRuntimeHttp.IsDue(request.m_iNotBeforeMs))
				continue;

			if (HasEarlierRequestOf(i, request.m_sArmaId))
				continue;

			if (TBD_DeploymentEndQueue.HasPendingFor(session, request.m_sArmaId, request.m_sOrbatSlotId))
				continue;

			s_aQueue.RemoveOrdered(i);
			Send(request, session);
			return;
		}
	}

	//! Sent to a runtime session other than the current `session`.
	protected static bool IsPinnedElsewhere(notnull TBD_DeploymentRequest request, string session)
	{
		return !request.m_sRuntimeSessionId.IsEmpty() && request.m_sRuntimeSessionId != session;
	}

	//! True when a request of `armaId` waits ahead of `index`.
	protected static bool HasEarlierRequestOf(int index, string armaId)
	{
		for (int i = 0; i < index; i++)
		{
			if (s_aQueue[i] && s_aQueue[i].m_sArmaId == armaId)
				return true;
		}

		return false;
	}

	//! No request can be decided: the waiting players are refused, the abandoned requests dropped.
	protected static void FailAll(string sentence)
	{
		array<ref TBD_DeploymentRequest> failed = s_aQueue;
		s_aQueue = new array<ref TBD_DeploymentRequest>();

		foreach (TBD_DeploymentRequest request : failed)
		{
			if (request && !request.m_bAbandoned)
				TBD_DeploymentAuthorization.OnUndecided(request, sentence);
		}
	}

	//! Send `request` to `session` and record it in flight.
	//! @route POST /api/v1/game-runtime/sessions/{sessionId}/deployments
	protected static void Send(notnull TBD_DeploymentRequest request, string session)
	{
		string body = "{";
		body += string.Format("\"event_mission_id\":\"%1\"", TBD_BackendText.JsonEscape(request.m_sEventMissionId));
		body += string.Format(",\"orbat_slot_id\":\"%1\"", TBD_BackendText.JsonEscape(request.m_sOrbatSlotId));
		body += string.Format(",\"arma_id\":\"%1\"", TBD_BackendText.JsonEscape(request.m_sArmaId));
		body += string.Format(",\"player_life_id\":\"%1\"", TBD_BackendText.JsonEscape(request.m_sPlayerLifeId));
		body += "}";

		TBD_DeploymentRequestCall call = new TBD_DeploymentRequestCall();
		call.m_Request = request;

		TBD_Log.Kv(CH_DEPLOYMENT, "deployment-request", string.Format("player=%1 slot=%2 orbatSlot=%3 life=%4 session=%5 attempt=%6",
			request.m_iPlayerId, request.m_sSlotUid, request.m_sOrbatSlotId, request.m_sPlayerLifeId, session, request.m_iFailures + 1));

		// In flight, and addressed to its session, before sending, so an answer can never find the
		// request unrecorded. Nothing sent leaves the request as it was.
		bool sentBefore = request.m_bSent;
		string sessionBefore = request.m_sRuntimeSessionId;
		s_InFlight = call;
		request.m_sRuntimeSessionId = session;
		request.m_bSent = true;

		string failure;
		string path = TBD_GameRuntimeHttp.ROUTE_PREFIX + "/sessions/" + session + "/deployments";
		if (TBD_GameRuntimeHttp.Post(call, path, body, failure))
			return;

		s_InFlight = null;
		request.m_bSent = sentBefore;
		request.m_sRuntimeSessionId = sessionBefore;
		RetryLater(request, failure);
	}

	//! Called by TBD_DeploymentRequestCall with the answer to the request in flight. A request its
	//! watchdog answered may have been decided, and the same life id returns that decision, so a
	//! TRANSIENT answer sends it again.
	static void OnCallAnswered(notnull TBD_DeploymentRequestCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (call != s_InFlight)
			return;

		s_InFlight = null;
		TBD_DeploymentRequest request = call.m_Request;

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
			SettleDecision(request, answer);
		else if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.REFUSED)
			SettleRefusal(request, answer);
		else if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
			RetryLater(request, answer.m_sDetail);
		else
			SettleRejection(request, answer.m_sDetail);

		Pump();
	}

	//! Apply a 2xx answer: allowed (for this request's life), denied (possibly requeued at the head)
	//! or unreadable (retried, or undecided for an unknown decision).
	protected static void SettleDecision(notnull TBD_DeploymentRequest request, notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_DeploymentDecisionStruct decision = ParseDecision(answer.m_sBody);
		if (!decision)
		{
			TBD_Log.Error(CH_DEPLOYMENT, "unreadable deployment decision: " + answer.m_sDetail);
			RetryLater(request, "unreadable decision");
			return;
		}

		if (decision.decision == "allowed")
		{
			// The platform echoes the life it recorded; any other id is not this request's answer.
			if (decision.occupancy_id.IsEmpty() || decision.player_life_id != request.m_sPlayerLifeId)
			{
				TBD_Log.Error(CH_DEPLOYMENT, string.Format("allowed decision does not name life %1: %2", request.m_sPlayerLifeId, answer.m_sDetail));
				RetryLater(request, "decision for another life");
				return;
			}

			TBD_DeploymentAuthorization.OnAllowed(request, decision.occupancy_id, decision.authorized_by);
			return;
		}

		if (decision.decision == "denied")
		{
			if (TBD_DeploymentAuthorization.OnDenied(request, decision.reason, decision.message, decision.occupancy_id))
			{
				request.m_iNotBeforeMs = TBD_GameRuntimeHttp.NowMs();
				s_aQueue.InsertAt(request, 0);
			}
			return;
		}

		TBD_Log.Error(CH_DEPLOYMENT, "deployment answer carries an unknown decision: " + answer.m_sDetail);
		TBD_DeploymentAuthorization.OnUndecided(request, "the TBD platform answered this server with something it cannot read - tell an admin.");
	}

	//! Read `body` as a decision, or null when it is empty or not a decision.
	protected static TBD_DeploymentDecisionStruct ParseDecision(string body)
	{
		if (body.IsEmpty())
			return null;

		JsonLoadContext context = new JsonLoadContext();
		if (!context.LoadFromString(body))
			return null;

		TBD_DeploymentDecisionStruct decision = new TBD_DeploymentDecisionStruct();
		if (!context.ReadValue("", decision))
			return null;

		return decision;
	}

	//! A 409 fence refusal. A deployment is refused only once its runtime session has ended; the
	//! platform ended every life of that session with it.
	protected static void SettleRefusal(notnull TBD_DeploymentRequest request, notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_GameRuntimeRefusalDetails refusal = answer.m_Refusal;
		if (!refusal || refusal.code != "RUNTIME_SESSION_ENDED")
		{
			SettleRejection(request, answer.m_sDetail);
			return;
		}

		TBD_RuntimeSession.ReportSessionEnded(request.m_sRuntimeSessionId, refusal.end_reason);

		if (request.m_bAbandoned)
		{
			TBD_Log.Kv(CH_DEPLOYMENT, "request-dropped", string.Format("player=%1 life=%2 - abandoned, and its session has ended (%3)",
				request.m_iPlayerId, request.m_sPlayerLifeId, refusal.end_reason));
			return;
		}

		request.m_sRuntimeSessionId = string.Empty;
		RetryLater(request, "runtime session ended (" + refusal.end_reason + ") - asking again in the next session");
	}

	//! A client-error answer: the platform will not decide this request, however often it is sent.
	protected static void SettleRejection(notnull TBD_DeploymentRequest request, string detail)
	{
		TBD_Log.Error(CH_DEPLOYMENT, string.Format("the platform rejected the deployment request of player=%1 slot=%2 life=%3 (%4)",
			request.m_iPlayerId, request.m_sSlotUid, request.m_sPlayerLifeId, detail));
		TBD_DeploymentAuthorization.OnUndecided(request, "the TBD platform rejected this server's deployment request - tell an admin.");
	}

	//! Back to the head of the queue with exponential backoff and the same life id.
	protected static void RetryLater(notnull TBD_DeploymentRequest request, string detail)
	{
		request.m_iFailures++;
		int delay = TBD_GameRuntimeHttp.BackoffMs(request.m_iFailures, RETRY_BASE_MS, RETRY_CAP_MS);
		request.m_iNotBeforeMs = TBD_GameRuntimeHttp.NowMs() + delay;
		s_aQueue.InsertAt(request, 0);

		TBD_Log.Warn(CH_DEPLOYMENT, string.Format("deployment request of player=%1 slot=%2 undecided (%3) - attempt %4, asking again in %5 ms with life %6",
			request.m_iPlayerId, request.m_sSlotUid, detail, request.m_iFailures, delay, request.m_sPlayerLifeId));
	}
}
