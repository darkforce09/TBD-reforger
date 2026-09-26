/**
 * @file TBD_TelemetryDelivery.c
 * @brief Sends the telemetry queue to the API, one request at a time, and settles every answer.
 *
 * Role: posts the queue's head entry through `TBD_GameRuntimeHttp` and applies the spec's outcome
 * table: acknowledge, send the match's registration first, drop, or keep and back off.
 * Position: pumped once per beat by `TBD_RuntimeHeartbeat.TickServer`; drains `TBD_TelemetryQueue`;
 * hands registration answers to `TBD_MatchRegistration`.
 * State: the call in flight, the consecutive failure count and the retry clock; server statics.
 * Invariants: at most one request is in flight; an answer settles only the call it belongs to;
 * 2xx and 409 `STALE_REVISION` acknowledge; 409 `MATCH_NOT_REGISTERED` sends the match's
 * registration first, once per entry; every other 409 and every other client error drop the entry,
 * counted and logged at ERROR; 401 and 403 keep it, because a credential fix must still deliver;
 * no answer, 429 and 5xx keep it; a kept entry retries after `BACKOFF_BASE_MS`, doubling to
 * `BACKOFF_CAP_MS`; the body and the credential are never logged.
 */

//! The one telemetry request in flight; hands its answer to `TBD_TelemetryDelivery`.
class TBD_TelemetryDeliveryCall : TBD_GameRuntimeCall
{
	int m_iEntryId; //!< id of the queue entry this call sends

	//! Forward the answer to the delivery loop.
	//! @param answer the classified answer
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_TelemetryDelivery.OnAnswer(this, answer);
	}
}

//! Serial sender of the durable telemetry queue.
//! @authority server
class TBD_TelemetryDelivery
{
	static const int BACKOFF_BASE_MS = 2000; //!< delay after the first kept failure, in milliseconds
	static const int BACKOFF_CAP_MS = 60000; //!< longest delay between attempts, in milliseconds

	protected static ref TBD_TelemetryDeliveryCall s_Call; //!< the call in flight, or null
	protected static int s_iFailures; //!< consecutive attempts that kept their entry
	protected static int s_iNotBeforeMs; //!< `TBD_GameRuntimeHttp.NowMs` before which nothing is sent while backing off

	//! One heartbeat beat: queue the round's registration once a runtime session exists, then send
	//! the head entry when nothing is in flight and no backoff is pending. Does nothing on a client.
	//! @authority server
	static void Tick()
	{
		if (TBD_Authority.IsClient())
			return;

		TBD_MatchRegistration.EnqueueWhenSessionHeld();

		if (s_Call)
			return;

		if (s_iFailures > 0 && !TBD_GameRuntimeHttp.IsDue(s_iNotBeforeMs))
			return;

		SendHead();
	}

	//! Post the queue's head entry to its route.
	//! @route POST /api/v1/ingest/matches
	//! @route POST /api/v1/ingest/match-results
	//! @route POST /api/v1/ingest/match-events
	//! @authority server
	protected static void SendHead()
	{
		TBD_TelemetryQueueEntry entry = TBD_TelemetryQueue.Head();
		if (!entry)
			return;

		TBD_TelemetryDeliveryCall call = new TBD_TelemetryDeliveryCall();
		call.m_iEntryId = entry.m_iId;

		// In flight before sending, so an answer can never find the call unrecorded.
		s_Call = call;
		TBD_TelemetryQueue.SetInFlight(entry.m_iId);

		string failure;
		if (TBD_GameRuntimeHttp.Post(call, entry.m_sRoute, entry.m_sBody, failure))
			return;

		s_Call = null;
		TBD_TelemetryQueue.SetInFlight(0);
		Keep(entry, "not sent: " + failure);
	}

	//! The answer to a delivery call: settle its entry, then send the next one at once when the
	//! entry left the queue or a registration was put ahead of it.
	//! @param call the answered call; any call other than the one in flight is ignored
	//! @param answer the classified answer
	//! @authority server
	static void OnAnswer(notnull TBD_TelemetryDeliveryCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (call != s_Call)
			return;

		s_Call = null;
		TBD_TelemetryQueue.SetInFlight(0);

		TBD_TelemetryQueueEntry entry = TBD_TelemetryQueue.Find(call.m_iEntryId);
		if (!entry)
			return;

		if (Settle(entry, answer))
		{
			s_iFailures = 0;
			SendHead();
		}
	}

	//! Apply the outcome table to `entry`.
	//! @return true when the queue moved on (acknowledged, dropped or registration put first);
	//! false when the entry is kept for a later attempt
	protected static bool Settle(notnull TBD_TelemetryQueueEntry entry, notnull TBD_GameRuntimeAnswer answer)
	{
		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			if (entry.m_eKind == TBD_ETelemetryEntryKind.REGISTRATION)
				TBD_MatchRegistration.OnRegistered(entry.m_sSourceMatchId, answer.m_sBody);

			TBD_TelemetryQueue.Acknowledge(entry.m_iId);
			return true;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.REFUSED)
			return SettleRefusal(entry, answer);

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.PERMANENT)
		{
			// A credential fix must still deliver what was queued under the refused credential.
			if (answer.m_eCode == HttpCode.HTTP_CODE_401 || answer.m_eCode == HttpCode.HTTP_CODE_403)
			{
				Keep(entry, "credential refused: " + answer.m_sDetail);
				return false;
			}

			TBD_TelemetryQueue.Drop(entry.m_iId, answer.m_sDetail);
			return true;
		}

		Keep(entry, answer.m_sDetail);
		return false;
	}

	//! A 409 with `details.code`: `STALE_REVISION` acknowledges, `MATCH_NOT_REGISTERED` puts the
	//! match's registration first, every other code drops.
	//! @return true when the queue moved on
	protected static bool SettleRefusal(notnull TBD_TelemetryQueueEntry entry, notnull TBD_GameRuntimeAnswer answer)
	{
		string code = answer.m_sErrorCode;
		if (code == "STALE_REVISION")
		{
			// The API holds a newer revision of this match; this one is superseded.
			TBD_TelemetryQueue.Acknowledge(entry.m_iId);
			return true;
		}

		if (code != "MATCH_NOT_REGISTERED")
		{
			TBD_TelemetryQueue.Drop(entry.m_iId, answer.m_sDetail);
			return true;
		}

		if (!entry.m_bRegistrationResent && TBD_TelemetryQueue.PutRegistrationFirst(entry.m_sSourceMatchId))
		{
			entry.m_bRegistrationResent = true;
			TBD_Log.Kv(TBD_TelemetryQueue.CH_TELEMETRY, "registration-first", entry.Describe());
			return true;
		}

		// The round's registration is still waiting for a runtime session, or was already sent
		// ahead of this entry: wait and ask again.
		if (entry.m_bRegistrationResent || TBD_MatchRegistration.IsAwaitingSession(entry.m_sSourceMatchId))
		{
			Keep(entry, answer.m_sDetail);
			return false;
		}

		TBD_TelemetryQueue.Drop(entry.m_iId, "MATCH_NOT_REGISTERED and no registration of the match is known");
		return true;
	}

	//! Keep `entry` for a later attempt and back off.
	//! @param why the failure, logged
	protected static void Keep(notnull TBD_TelemetryQueueEntry entry, string why)
	{
		s_iFailures++;
		int delay = TBD_GameRuntimeHttp.BackoffMs(s_iFailures, BACKOFF_BASE_MS, BACKOFF_CAP_MS);
		s_iNotBeforeMs = TBD_GameRuntimeHttp.NowMs() + delay;
		TBD_Log.Warn(TBD_TelemetryQueue.CH_TELEMETRY, string.Format("kept %1 failures=%2 retry_in_ms=%3 -- %4",
			entry.Describe(), s_iFailures, delay, why));
	}
}
