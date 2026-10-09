/**
 * @file TBD_TelemetryQueueState.c
 * @brief The durable counters of the telemetry queue: entry ids, drops and per-match counters.
 *
 * Role: the record kept in the queue's two alternating state slots: the next entry id, the drop
 * total and, per source match, the last results revision, the last event sequence and the
 * registration body.  Position: owned by `TBD_TelemetryQueue`; saved and loaded by
 * `TBD_TelemetryQueueStorage`.
 * State: the fields below.  Invariants: `serial` grows by one per save, so the valid slot with the
 * higher serial is the latest; a match's results revision and event sequence only grow; at most
 * `MATCHES_MAX` matches are kept, and a match is forgotten only while no queued entry names it;
 * field names are the JSON keys of the slot record.
 */

//! The counters of one source match.
class TBD_TelemetryMatchCounters
{
	string source_match_id; //!< JSON `source_match_id`: the server-scoped source match id
	int results_revision; //!< JSON `results_revision`: the last results revision issued; 0 before the first
	int event_sequence; //!< JSON `event_sequence`: the last event sequence issued; 0 before the first
	string registration_body; //!< JSON `registration_body`: the registration sent for the match, resent on `MATCH_NOT_REGISTERED`
}

//! The record of one state slot.
class TBD_TelemetryQueueState
{
	static const int MATCHES_MAX = 16; //!< matches whose counters are kept at most

	int serial; //!< JSON `serial`: save count; the higher valid slot wins on reload
	int next_entry_id; //!< JSON `next_entry_id`: the id the next entry receives; starts at 1
	int dropped_total; //!< JSON `dropped_total`: entries ever dropped
	ref array<ref TBD_TelemetryMatchCounters> matches; //!< JSON `matches`: per-match counters, oldest first

	//! An empty state: ids from 1, nothing dropped, no matches.
	void TBD_TelemetryQueueState()
	{
		next_entry_id = 1;
		matches = new array<ref TBD_TelemetryMatchCounters>();
	}

	//! The counters of `sourceMatchId`.
	//! @return the counters, or null when the match is not kept
	TBD_TelemetryMatchCounters FindMatch(string sourceMatchId)
	{
		if (!matches)
			return null;

		foreach (TBD_TelemetryMatchCounters counters : matches)
		{
			if (counters && counters.source_match_id == sourceMatchId)
				return counters;
		}

		return null;
	}

	//! The counters of `sourceMatchId`, created when missing. Creating one past `MATCHES_MAX`
	//! forgets the oldest match no queued entry names.
	//! @param queuedMatchIds the source match ids the queue's entries name
	//! @return the counters
	TBD_TelemetryMatchCounters EnsureMatch(string sourceMatchId, notnull array<string> queuedMatchIds)
	{
		TBD_TelemetryMatchCounters counters = FindMatch(sourceMatchId);
		if (counters)
			return counters;

		if (!matches)
			matches = new array<ref TBD_TelemetryMatchCounters>();

		if (matches.Count() >= MATCHES_MAX)
			ForgetOldestUnqueued(queuedMatchIds);

		counters = new TBD_TelemetryMatchCounters();
		counters.source_match_id = sourceMatchId;
		matches.Insert(counters);
		return counters;
	}

	//! Forget the oldest match that no queued entry names; when every kept match is still queued,
	//! nothing is forgotten and the list grows past `MATCHES_MAX` until entries drain.
	protected void ForgetOldestUnqueued(notnull array<string> queuedMatchIds)
	{
		for (int i = 0; i < matches.Count(); i++)
		{
			TBD_TelemetryMatchCounters counters = matches[i];
			if (!counters || queuedMatchIds.Find(counters.source_match_id) < 0)
			{
				matches.RemoveOrdered(i);
				return;
			}
		}
	}

	//! The slot record: one JSON line, hand-built so its bytes are fixed by code.
	//! @return the JSON object
	string ToJson()
	{
		string json = string.Format("{\"serial\":%1,\"next_entry_id\":%2,\"dropped_total\":%3,\"matches\":[",
			serial, next_entry_id, dropped_total);

		if (matches)
		{
			bool first = true;
			foreach (TBD_TelemetryMatchCounters counters : matches)
			{
				if (!counters)
					continue;

				if (!first)
					json += ",";
				first = false;

				json += string.Format("{\"source_match_id\":\"%1\",\"results_revision\":%2,\"event_sequence\":%3",
					TBD_BackendText.JsonEscape(counters.source_match_id), counters.results_revision, counters.event_sequence);
				json += string.Format(",\"registration_body\":\"%1\"}", TBD_BackendText.JsonEscape(counters.registration_body));
			}
		}

		json += "]}";
		return json;
	}

	//! A state read from a slot record.
	//! @return the state, or null when the record does not parse
	static TBD_TelemetryQueueState FromJson(string record)
	{
		JsonLoadContext context = new JsonLoadContext();
		if (!context.LoadFromString(record))
			return null;

		TBD_TelemetryQueueState state = new TBD_TelemetryQueueState();
		if (!context.ReadValue("", state))
			return null;

		if (!state.matches)
			state.matches = new array<ref TBD_TelemetryMatchCounters>();

		if (state.next_entry_id < 1)
			state.next_entry_id = 1;

		return state;
	}
}
