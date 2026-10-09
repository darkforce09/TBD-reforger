/**
 * @file TBD_TelemetryQueueEntry.c
 * @brief One queued telemetry report, its kind and route, and the queue's reading for heartbeats.
 *
 * Role: the in-memory form of one entry of the durable telemetry queue and of the queue reading a
 * heartbeat carries.  Position: built by `TBD_TelemetryQueue` on enqueue and by
 * `TBD_TelemetryQueueStorage` on reload; sent by `TBD_TelemetryDelivery`; the reading is read by
 * `TBD_RuntimeStatusReadings`.
 * State: each entry's own fields.  Invariants: an entry's id is unique for the life of the queue
 * directory and never reused; the route of a kind is fixed (`RouteOf`); the body is the exact bytes
 * sent, built once at enqueue.
 */

//! What a queued report is; decides its route and how the queue's capacity treats it.
enum TBD_ETelemetryEntryKind
{
	REGISTRATION, //!< `POST /api/v1/ingest/matches`; counts against the reserved capacity
	RESULTS,      //!< `POST /api/v1/ingest/match-results`; counts against the reserved capacity
	EVENT_BATCH,  //!< `POST /api/v1/ingest/match-events`; dropped first when the queue is full
}

//! One report waiting for the API's acknowledgement.
class TBD_TelemetryQueueEntry
{
	static const string ROUTE_MATCHES = "/api/v1/ingest/matches"; //!< registration route
	static const string ROUTE_MATCH_RESULTS = "/api/v1/ingest/match-results"; //!< results revision route
	static const string ROUTE_MATCH_EVENTS = "/api/v1/ingest/match-events"; //!< event batch route

	int m_iId; //!< entry id, from the queue state's next entry id; names the entry file
	TBD_ETelemetryEntryKind m_eKind; //!< what the entry reports
	string m_sSourceMatchId; //!< the server-scoped source match id the report is about
	string m_sRoute; //!< the path the body is posted to
	string m_sBody; //!< the JSON body, sent verbatim on every attempt
	int m_iEnqueuedAt; //!< enqueue time, in Unix epoch seconds
	bool m_bRegistrationResent; //!< this entry already had its match's registration sent ahead of it; in memory only

	//! The route an entry of `kind` is posted to.
	//! @return the path
	static string RouteOf(TBD_ETelemetryEntryKind kind)
	{
		if (kind == TBD_ETelemetryEntryKind.REGISTRATION)
			return ROUTE_MATCHES;

		if (kind == TBD_ETelemetryEntryKind.RESULTS)
			return ROUTE_MATCH_RESULTS;

		return ROUTE_MATCH_EVENTS;
	}

	//! Whether the entry counts against the capacity reserved for registrations and results.
	//! @return true for a registration or a results revision
	bool IsReserved()
	{
		return m_eKind != TBD_ETelemetryEntryKind.EVENT_BATCH;
	}

	//! The entry for a log line: id, kind and source match id; never the body.
	//! @return `id=<n> kind=<KIND> source='<id>'`
	string Describe()
	{
		return string.Format("id=%1 kind=%2 source='%3'", m_iId,
			typename.EnumToString(TBD_ETelemetryEntryKind, m_eKind), m_sSourceMatchId);
	}
}

//! The queue reading a heartbeat reports as `telemetry_queue`.
//! @contract match-telemetry.schema.json#/definitions/TelemetryQueueReading
class TBD_TelemetryQueueStatsStruct
{
	int backlog; //!< JSON `backlog`: entries waiting, the one in flight included
	int capacity; //!< JSON `capacity`: entries the queue holds at most
	int dropped_total; //!< JSON `dropped_total`: entries ever dropped, over restarts
	int oldest_age_seconds; //!< JSON `oldest_age_seconds`: age of the oldest waiting entry; 0 when empty

	//! The reading as a JSON object.
	//! @return `{"backlog":..,"capacity":..,"dropped_total":..,"oldest_age_seconds":..}`
	//! @contract match-telemetry.schema.json#/definitions/TelemetryQueueReading
	string ToJson()
	{
		return string.Format("{\"backlog\":%1,\"capacity\":%2,\"dropped_total\":%3,\"oldest_age_seconds\":%4}",
			backlog, capacity, dropped_total, oldest_age_seconds);
	}
}
