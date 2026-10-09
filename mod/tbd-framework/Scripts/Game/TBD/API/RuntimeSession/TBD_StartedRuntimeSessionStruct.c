/**
 * @file TBD_StartedRuntimeSessionStruct.c
 * @brief The answer to `POST /api/v1/game-runtime/sessions`: the session this server now holds.
 *
 * Role: parses and validates the start answer.  Position: read by `TBD_RuntimeSession` when a
 * session start succeeds.
 * State: none beyond each answer's fields.  Invariants: a parsed answer has a non-empty
 * `runtime_session_id` and a generation of at least 1; anything else parses to null.
 */

//! `POST /api/v1/game-runtime/sessions` response. Field names are the JSON keys.
//! @contract game-runtime-session.schema.json#
class TBD_StartedRuntimeSessionStruct
{
	string runtime_session_id; //!< JSON key `runtime_session_id`; the session this server now holds
	string server_id; //!< JSON key `server_id`
	int generation; //!< JSON key `generation`; the per-server fence, at least 1
	string started_at; //!< JSON key `started_at`, RFC 3339 UTC
	int heartbeat_interval_seconds; //!< JSON key `heartbeat_interval_seconds`, in seconds
	int expires_after_seconds; //!< JSON key `expires_after_seconds`; silence after which the session expires, in seconds

	//! The started session in `body`.
	//! @return the answer, or null when the body is not a usable start answer
	static TBD_StartedRuntimeSessionStruct Parse(string body)
	{
		if (body.IsEmpty())
			return null;

		JsonLoadContext context = new JsonLoadContext();
		if (!context.LoadFromString(body))
			return null;

		TBD_StartedRuntimeSessionStruct started = new TBD_StartedRuntimeSessionStruct();
		if (!context.ReadValue("", started))
			return null;

		if (started.runtime_session_id.IsEmpty() || started.generation < 1)
			return null;

		return started;
	}
}
