/**
 * @file TBD_MatchEventBatch.c
 * @brief The `MatchEventBatch` envelope around detailed events, queued for delivery.
 *
 * Role: wraps 1 to `EVENTS_MAX` event objects of one match in the batch envelope and queues it.
 * Position: the enqueue entry point of detailed-event capture; each event object carries a
 * `sequence` from `TBD_TelemetryQueue.ReserveEventSequences`; queues through
 * `TBD_TelemetryQueue.EnqueueEventBatch`.
 * State: none.  Invariants: a batch holds 1 to `EVENTS_MAX` events, the API's limit, so a batch is
 * never refused for its size; the events are placed in the order given, which is their capture
 * order; each event object is already a complete JSON `MatchEvent`.
 */

//! Envelope builder and enqueuer of detailed event batches.
//! @authority server
class TBD_MatchEventBatch
{
	static const int EVENTS_MAX = 500; //!< events per batch at most, the API's `EVENT_BATCH_TOO_LARGE` bound

	//! Queue `events` of `sourceMatchId` as one batch.
	//! @param events 1 to `EVENTS_MAX` JSON `MatchEvent` objects in capture order
	//! @return false, with nothing queued, for an empty or oversized batch or a write that failed
	static bool Enqueue(string sourceMatchId, notnull array<string> events)
	{
		if (events.IsEmpty() || events.Count() > EVENTS_MAX)
		{
			TBD_Log.Error(TBD_TelemetryQueue.CH_TELEMETRY, string.Format("event batch of %1 events for '%2' not queued: outside 1..%3",
				events.Count(), sourceMatchId, EVENTS_MAX));
			return false;
		}

		return TBD_TelemetryQueue.EnqueueEventBatch(sourceMatchId, BuildBody(sourceMatchId, events));
	}

	//! The batch envelope `{"source_match_id":..,"events":[..]}`.
	//! @return the JSON body
	//! @contract match-telemetry.schema.json#/definitions/MatchEventBatch
	static string BuildBody(string sourceMatchId, notnull array<string> events)
	{
		string json = string.Format("{\"source_match_id\":\"%1\",\"events\":[", TBD_BackendText.JsonEscape(sourceMatchId));
		foreach (int i, string eventObject : events)
		{
			if (i > 0)
				json += ",";
			json += eventObject;
		}

		json += "]}";
		return json;
	}
}
