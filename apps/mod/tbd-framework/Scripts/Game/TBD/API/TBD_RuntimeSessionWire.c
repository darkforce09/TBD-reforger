//! The wire of the runtime session loop (TBD_RuntimeSession): the start answer, and the call that
//! carries a start or a heartbeat with the world and session it was sent for.
//! @authority server

//! `POST /api/v1/game-runtime/sessions` response. Field names are the JSON keys.
class TBD_StartedRuntimeSessionStruct
{
	string runtime_session_id;
	string server_id;
	int generation;
	string started_at;
	int heartbeat_interval_seconds;
	int expires_after_seconds;

	//------------------------------------------------------------------------------------------------
	//! The started session in `body`, or null when the body is not a usable start answer.
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

//! What a runtime-session call asks for.
enum TBD_ERuntimeSessionRequest
{
	START,
	HEARTBEAT,
}

//! A start or heartbeat request, with the world and session it was sent for.
class TBD_RuntimeSessionCall : TBD_GameRuntimeCall
{
	TBD_ERuntimeSessionRequest m_eRequest;
	int m_iWorld;
	string m_sSessionId;
	//! A start that reported a loaded artifact.
	bool m_bReportedArtifact;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_RuntimeSession.OnCallAnswered(this, answer);
	}
}
