//! The records of one deployment request: what TBD_DeploymentAuthorization asks the platform for,
//! and the platform's decision as TBD_DeploymentRequestQueue reads it.

//! One spawn attempt of one player into one event roster slot.
class TBD_DeploymentRequest
{
	int m_iPlayerId;
	int m_iConnectionEpoch; //!< TBD_SpawnManager's connection epoch of the player when the request was made.
	string m_sArmaId;
	string m_sSlotUid;
	string m_sEventMissionId;
	string m_sOrbatSlotId;
	//! Chosen once per spawn attempt and kept for every retry of it, so a repeated request returns
	//! the decision the platform already recorded.
	string m_sPlayerLifeId;
	string m_sRuntimeSessionId; //!< The runtime session the request was last sent to; empty before the first send.
	//! Nobody waits for this decision any more (the player left, changed seat, or the round ended).
	//! A request that may already have reached the platform is still resolved, so a life the
	//! platform opened for it is ended.
	bool m_bAbandoned;
	bool m_bSent; //!< At least one attempt left this server.
	bool m_bUntrackedLifeEnded; //!< The recovery from PLAYER_ALREADY_DEPLOYED has been used once for this request.
	int m_iFailures;
	int m_iNotBeforeMs; //!< `TBD_GameRuntimeHttp.NowMs()` from which the request may be sent.
}

//! `POST .../deployments` answer: `allowed` with the opened life, or `denied` with a reason.
//! Field names are the JSON keys; keys this client does not read are ignored.
class TBD_DeploymentDecisionStruct
{
	string decision;
	string occupancy_id;
	string player_life_id;
	string authorized_by;
	string reason;
	string message;
}
