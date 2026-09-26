/**
 * @file TBD_MatchRegistration.c
 * @brief Registers each round with the API when it goes LIVE and keeps the match id it answers.
 *
 * Role: builds the `MatchRegistration` body of the current round and queues it once this runtime
 * holds a session; records the answered `match_id`, which every heartbeat reports as
 * `current_match_id`.  Position: `BeginRound` is called by `TBD_ResultsReporter` when a round goes
 * LIVE; `EnqueueWhenSessionHeld` and `OnRegistered` by `TBD_TelemetryDelivery`; the heartbeat field
 * is read by `TBD_RuntimeStatusReadings`; the source match id by `TBD_MatchEventRecorder`.
 * State: the current round's source match id, start time, mission, event and terrain, whether
 * its registration still waits for a session, and the answered match id; server statics.
 * Invariants: one registration
 * per round, queued before any report of the round leaves the queue behind it; a round whose
 * runtime holds no session yet is registered with the first session this process holds;
 * `mission_id` and `event_id` are sent only as UUIDs and `terrain` only when known, since a
 * refused registration would lose every report of the match.
 */

//! The registration answer. Field names are the JSON keys.
//! @contract match-telemetry.schema.json#/definitions/MatchRegistrationAnswer
class TBD_MatchRegistrationAnswerStruct
{
	string match_id; //!< JSON `match_id`: the API's id of the registered match
	bool registered; //!< JSON `registered`: true when this answer created the match
}

//! Registration of the current round and the match id it answered.
//! @authority server
class TBD_MatchRegistration
{
	protected static string s_sSourceMatchId; //!< the current round's source match id, or empty before the first round
	protected static string s_sStartedAtUtc; //!< the current round's `started_at`, RFC 3339 UTC
	protected static string s_sMissionId; //!< the deployed mission when the round went LIVE, or empty
	protected static string s_sEventId; //!< the deployed event when the round went LIVE, or empty
	protected static string s_sTerrain; //!< the mission header's terrain when the round went LIVE, or empty
	protected static bool s_bAwaitingSession; //!< the current round's registration waits for a runtime session
	protected static string s_sMatchId; //!< the API's `match_id` of the current round, or empty until answered

	//! Start registering a round that just went LIVE; the previous round stops being current.
	//! @param sourceMatchId the round's source match id
	//! @param startedAtUtc the round's start, RFC 3339 UTC
	//! @authority server
	static void BeginRound(string sourceMatchId, string startedAtUtc)
	{
		s_sSourceMatchId = sourceMatchId;
		s_sStartedAtUtc = startedAtUtc;
		s_sMissionId = TBD_DeployedMission.GetMissionId();
		s_sEventId = TBD_DeployedMission.GetEventId();
		s_sTerrain = MissionTerrain();
		s_sMatchId = string.Empty;
		s_bAwaitingSession = true;
		EnqueueWhenSessionHeld();
	}

	//! Queue the current round's registration once this runtime holds a session; does nothing
	//! when it is queued already or no session is held yet.
	//! @authority server
	static void EnqueueWhenSessionHeld()
	{
		if (!s_bAwaitingSession)
			return;

		string sessionId = TBD_RuntimeSession.GetSessionId();
		if (sessionId.IsEmpty())
			return;

		s_bAwaitingSession = false;
		string body = BuildBody(sessionId);
		TBD_TelemetryQueue.EnqueueRegistration(s_sSourceMatchId, body);
		TBD_Log.Kv(TBD_TelemetryQueue.CH_TELEMETRY, "registration-queued", string.Format("source='%1' session=%2",
			s_sSourceMatchId, sessionId));
	}

	//! The current round's source match id, which names its registration and every report of it.
	//! @return the id, or empty before this process's first round
	static string GetSourceMatchId()
	{
		return s_sSourceMatchId;
	}

	//! Whether `sourceMatchId` is the current round and its registration still waits for a session.
	//! @return true while the registration is not queued yet
	static bool IsAwaitingSession(string sourceMatchId)
	{
		return s_bAwaitingSession && sourceMatchId == s_sSourceMatchId;
	}

	//! Record the API's answer to the registration of `sourceMatchId`; only the current round's
	//! match id is kept.
	//! @param answerBody the `MatchRegistrationAnswer` body
	static void OnRegistered(string sourceMatchId, string answerBody)
	{
		TBD_MatchRegistrationAnswerStruct answer = new TBD_MatchRegistrationAnswerStruct();
		JsonLoadContext context = new JsonLoadContext();
		if (!context.LoadFromString(answerBody) || !context.ReadValue("", answer) || !TBD_BackendText.IsUuid(answer.match_id))
		{
			TBD_Log.Warn(TBD_TelemetryQueue.CH_TELEMETRY, string.Format("registration of '%1' acknowledged without a readable match_id: %2",
				sourceMatchId, TBD_GameRuntimeAnswer.LoggableBody(answerBody)));
			return;
		}

		TBD_Log.Kv(TBD_TelemetryQueue.CH_TELEMETRY, "registered", string.Format("source='%1' match_id=%2 new=%3",
			sourceMatchId, answer.match_id, answer.registered));

		if (sourceMatchId == s_sSourceMatchId)
			s_sMatchId = answer.match_id;
	}

	//! The heartbeat's `current_match_id` member: the current round's match id, or an empty value,
	//! which clears the stored match, while a round's registration is unanswered.
	//! @return `,"current_match_id":"<id>"`, or empty before this process's first round
	static string BuildHeartbeatField()
	{
		if (s_sSourceMatchId.IsEmpty())
			return string.Empty;

		return string.Format(",\"current_match_id\":\"%1\"", s_sMatchId);
	}

	//! The terrain key from the mission header.
	//! @return the key, or empty without a mission
	static string MissionTerrain()
	{
		TBD_MissionDocumentStruct mission = TBD_MissionLoader.GetMission();
		if (!mission || !mission.meta)
			return string.Empty;

		return mission.meta.terrain;
	}

	//! The registration body of the current round, hand-built so its bytes are fixed by code; the
	//! same body is resent after `MATCH_NOT_REGISTERED`, which the API takes as a repeat.
	//! @param sessionId the runtime session the round ran in
	//! @return the JSON body
	//! @contract match-telemetry.schema.json#/definitions/MatchRegistration
	protected static string BuildBody(string sessionId)
	{
		string json = string.Format("{\"source_match_id\":\"%1\"", TBD_BackendText.JsonEscape(s_sSourceMatchId));
		json += string.Format(",\"runtime_session_id\":\"%1\"", TBD_BackendText.JsonEscape(sessionId));
		json += string.Format(",\"started_at\":\"%1\"", s_sStartedAtUtc);

		if (TBD_BackendText.IsUuid(s_sMissionId))
			json += string.Format(",\"mission_id\":\"%1\"", s_sMissionId);

		if (TBD_BackendText.IsUuid(s_sEventId))
			json += string.Format(",\"event_id\":\"%1\"", s_sEventId);

		if (!s_sTerrain.IsEmpty())
			json += string.Format(",\"terrain\":\"%1\"", TBD_BackendText.JsonEscape(s_sTerrain));

		json += "}";
		return json;
	}
}
