/**
 * @file TBD_RuntimeSessionCall.c
 * @brief One runtime-session request in flight: a start or a heartbeat.
 *
 * Role: remembers the world and session a request was sent for, so a stale answer is recognised.
 * Position: created by `TBD_RuntimeSession.Send`, sent through `TBD_GameRuntimeHttp`, and handed
 * back to `TBD_RuntimeSession.OnCallAnswered`.
 * State: the request's own fields.  Invariants: exactly one answer reaches `OnAnswered` per sent
 * call.
 */

//! What a runtime-session call asks for.
enum TBD_ERuntimeSessionRequest
{
	START, //!< `POST /api/v1/game-runtime/sessions`
	HEARTBEAT, //!< `POST /api/v1/game-runtime/sessions/{id}/heartbeats`
}

//! A start or heartbeat request, with the world and session it was sent for.
class TBD_RuntimeSessionCall : TBD_GameRuntimeCall
{
	TBD_ERuntimeSessionRequest m_eRequest; //!< start or heartbeat
	int m_iWorld; //!< the world counter it was sent in; another world's answer is stale
	string m_sSessionId; //!< the session a heartbeat addresses; empty for a start
	bool m_bReportedArtifact; //!< a start that reported a loaded artifact

	//! Hand the answer to `TBD_RuntimeSession.OnCallAnswered`.
	//! @authority server
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_RuntimeSession.OnCallAnswered(this, answer);
	}
}
