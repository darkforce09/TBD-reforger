/**
 * @file TBD_DeploymentRequest.c
 * @brief One deployment request record and the platform's decision as read back.
 *
 * Role: carries what TBD_DeploymentAuthorization asks the platform for, and the answer.
 * Position: built by TBD_DeploymentAuthorization, sent and settled by TBD_DeploymentRequestQueue.
 * State: none beyond the record fields.  Invariants: a request keeps its player life id across
 * every retry, so a repeat returns the decision the platform already recorded.
 */

//! One spawn attempt of one player into one event roster slot.
class TBD_DeploymentRequest
{
	int m_iPlayerId; //!< the requesting player
	int m_iConnectionEpoch; //!< connection epoch of the player when the request was made (TBD_ConnectionEpochs)
	string m_sArmaId; //!< the player's game identity sent as `arma_id`
	string m_sSlotUid; //!< the mission slot uid the player deploys into
	string m_sEventMissionId; //!< the event mission sent as `event_mission_id`
	string m_sOrbatSlotId; //!< the event roster slot sent as `orbat_slot_id`
	string m_sPlayerLifeId; //!< `player_life_id`, chosen once per spawn attempt and kept for every retry
	string m_sRuntimeSessionId; //!< The runtime session the request was last sent to; empty before the first send.
	bool m_bAbandoned; //!< nobody waits for the decision (left, changed seat, round over); a sent request is still resolved so an opened life is ended
	bool m_bSent; //!< At least one attempt left this server.
	bool m_bUntrackedLifeEnded; //!< The recovery from PLAYER_ALREADY_DEPLOYED has been used once for this request.
	int m_iFailures; //!< failed attempts so far; drives the backoff
	int m_iNotBeforeMs; //!< `TBD_GameRuntimeHttp.NowMs()` from which the request may be sent.
}

//! `POST .../deployments` answer: `allowed` with the opened life, or `denied` with a reason.
//! Field names are the JSON keys; keys this client does not read are ignored.
//! @contract game-runtime-deployment.schema.json#/oneOf
class TBD_DeploymentDecisionStruct
{
	string decision; //!< JSON `decision`: `allowed` or `denied`
	string occupancy_id; //!< JSON `occupancy_id`: the opened life's occupancy
	string player_life_id; //!< JSON `player_life_id`: echo of the requested life id
	string authorized_by; //!< JSON `authorized_by`: what allowed the deployment
	string reason; //!< JSON `reason`: machine reason of a denial
	string message; //!< JSON `message`: human reason of a denial
}
