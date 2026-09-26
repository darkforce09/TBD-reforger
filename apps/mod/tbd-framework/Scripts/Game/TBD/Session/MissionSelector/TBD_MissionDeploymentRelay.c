/**
 * @file TBD_MissionDeploymentRelay.c
 * @brief An in-game admin's mission selection, relayed to the platform as a deployment request.
 *
 * Role: posts the selection to `POST /api/v1/game-runtime/deployments` (mod_runtime credential)
 * and tells the admin the platform's decision in words.  Position: `#tbd mission <n>`
 * (TBD_AdminCommands) and TBD_RpcAsk_SelectMission call RequestByNumber with an entry of
 * TBD_DeployableMissionList; TBD_MissionDeploymentRelayCall brings the answer back to OnAnswered;
 * replies go to TBD_PlayerChat and TBD_AdminAudit.
 * State: none; each request travels in its own call object.  Invariants: nothing restarts here,
 * since an accepted deployment runs when its fleet command does; a request without a readable
 * game identity is not sent; selecting the mission this world runs for an event keeps that event
 * mission, and any other selection deploys without one.
 */

//! The accepted deployment (202), as far as the admin is told about it. Field names are the JSON keys.
//! @contract mission-deployment.schema.json#/definitions/MissionDeployment
class TBD_RelayedDeploymentStruct
{
	string id; //!< JSON `id`: the deployment id
	string mission_title; //!< JSON `mission_title`
	string transition; //!< JSON `transition`: scenario_restart or host_restart
	string state; //!< JSON `state`
	int bound_slots; //!< JSON `bound_slots`: seats bound to the event mission
}

//! A relayed deployment request on its way to the platform.
class TBD_MissionDeploymentRelayCall : TBD_GameRuntimeCall
{
	int m_iAdminPlayerId; //!< the admin who asked
	string m_sTitle; //!< the mission title, for the reply

	//! Hand the platform's answer to TBD_MissionDeploymentRelay.OnAnswered.
	//! @param answer the platform's answer
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_MissionDeploymentRelay.OnAnswered(this, answer);
	}
}

//! Relays an admin's mission selection to the platform and words the decision.
//! @authority server
class TBD_MissionDeploymentRelay
{
	//! Ask the platform to deploy mission `number` of the list on behalf of admin `adminPlayerId`.
	//! The platform's decision reaches the admin in chat later.
	//! @param adminPlayerId the requesting admin
	//! @param number the 1-based list number
	//! @return the immediate reply: the request is on its way, or why it was not sent
	//! @route POST /api/v1/game-runtime/deployments
	static string RequestByNumber(int adminPlayerId, int number)
	{
		TBD_DeployableMissionStruct entry = TBD_DeployableMissionList.GetEntryByNumber(number);
		if (!entry)
			return string.Format("TBD: no mission #%1 - '#tbd missions' lists them.", number);

		string armaId = TBD_PlayerIdentity.GetArmaId(adminPlayerId);
		if (armaId.IsEmpty())
			return "TBD: this server cannot read your game identity, so the platform cannot tell who asks - nothing was deployed.";

		string body = string.Format("{\"mission_id\":\"%1\",\"artifact_id\":\"%2\",\"requested_by_arma_id\":\"%3\"",
			TBD_BackendText.JsonEscape(entry.mission_id), TBD_BackendText.JsonEscape(entry.artifact_id), TBD_BackendText.JsonEscape(armaId));

		string eventMissionId;
		if (entry.mission_id == TBD_DeployedMission.GetMissionId())
			eventMissionId = TBD_DeployedMission.GetEventMissionId();

		if (!eventMissionId.IsEmpty())
			body += string.Format(",\"event_mission_id\":\"%1\"", TBD_BackendText.JsonEscape(eventMissionId));

		body += "}";

		TBD_MissionDeploymentRelayCall call = new TBD_MissionDeploymentRelayCall();
		call.m_iAdminPlayerId = adminPlayerId;
		call.m_sTitle = entry.title;

		string failure;
		if (!TBD_GameRuntimeHttp.Post(call, TBD_GameRuntimeHttp.ROUTE_PREFIX + "/deployments", body, failure))
			return "TBD: the deployment request could not be sent (" + failure + ") - nothing was deployed.";

		TBD_Log.Kv(TBD_DeployableMissionList.CH_MISSIONS, "deployment-requested", string.Format("admin=%1 mission=%2 artifact=%3 eventMission=%4 title='%5'",
			adminPlayerId, entry.mission_id, entry.artifact_id, eventMissionId, entry.title));
		TBD_AdminAudit.Record(string.Format("%1 asked the platform to deploy '%2' (#%3)", TBD_AdminService.Label(adminPlayerId), entry.title, number), false);

		string reply = string.Format("TBD: asking the platform to deploy '%1' [%2]", entry.title, entry.terrain_key);
		if (!eventMissionId.IsEmpty())
			reply += " for the running event mission";

		return reply + "...";
	}

	//! Tell the admin the platform's decision and audit it.
	//! @param call the request, carrying the admin and the title
	//! @param answer the platform's answer
	static void OnAnswered(notnull TBD_MissionDeploymentRelayCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		string text;
		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			TBD_RelayedDeploymentStruct deployment = new TBD_RelayedDeploymentStruct();
			JsonLoadContext context = new JsonLoadContext();
			if (context.LoadFromString(answer.m_sBody))
				context.ReadValue("", deployment);

			text = string.Format("TBD: deployment of '%1' ACCEPTED (%2) - the server restarts into it when the platform's deployment command runs; nothing restarts before that.",
				call.m_sTitle, deployment.transition);
			TBD_Log.Kv(TBD_DeployableMissionList.CH_MISSIONS, "deployment-accepted", string.Format("admin=%1 deployment=%2 transition=%3 state=%4 boundSlots=%5",
				call.m_iAdminPlayerId, deployment.id, deployment.transition, deployment.state, deployment.bound_slots));
		}
		else
		{
			text = string.Format("TBD: deployment of '%1' REFUSED - %2", call.m_sTitle, Explain(answer));
			TBD_Log.Warn(TBD_DeployableMissionList.CH_MISSIONS, string.Format("deployment-refused admin=%1 title='%2' - %3", call.m_iAdminPlayerId, call.m_sTitle, answer.m_sDetail));
		}

		TBD_AdminAudit.Record(string.Format("%1: %2", TBD_AdminService.Label(call.m_iAdminPlayerId), text), answer.m_eOutcome != TBD_EGameRuntimeOutcome.SUCCESS);
		TBD_PlayerChat.Tell(call.m_iAdminPlayerId, text);
	}

	//! The platform's refusal in words the admin can act on, with its code.
	//! @param answer a refused or unreachable answer
	//! @return the sentence
	protected static string Explain(notnull TBD_GameRuntimeAnswer answer)
	{
		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
			return "the platform could not be reached (" + answer.m_sDetail + "); nothing was deployed - try again.";

		string code = answer.m_sErrorCode;
		string words = WordsFor(code);
		if (!words.IsEmpty())
			return string.Format("%1 (%2)", words, code);

		if (answer.m_eCode == HttpCode.HTTP_CODE_404)
			return "the platform does not know that mission. '#tbd refresh', then pick again.";

		string status = typename.EnumToString(HttpCode, answer.m_eCode);
		if (!code.IsEmpty())
			status += " " + code;

		return string.Format("the platform refused it (%1).", status);
	}

	//! What a refusal code means for the admin.
	//! @param code the platform's error code
	//! @return the words, or empty for a code without its own
	protected static string WordsFor(string code)
	{
		if (code == "IDENTITY_NOT_LINKED")
			return "your game identity is not linked to a TBD platform account. Link it with '#tbd link' first.";
		if (code == "NOT_AN_ADMINISTRATOR")
			return "your TBD platform account is not an administrator, so it cannot deploy missions.";
		if (code == "DEPLOYMENT_IN_PROGRESS")
			return "another deployment to this server is in flight. Wait until it finishes.";
		if (code == "ARTIFACT_NOT_APPROVED")
			return "that mission is not live with this approved version any more. '#tbd refresh', then pick again.";
		if (code == "EVENT_MISSION_NOT_ON_SERVER")
			return "the running event mission is not scheduled on this server with that mission.";
		if (code == "SERVER_INACTIVE")
			return "this server is deactivated on the platform.";
		if (code == "MODPACK_MISMATCH")
			return "the mission is compiled for another modpack than this server requires.";
		if (code == "TERRAIN_NOT_RUNNABLE")
			return "the fleet has no scenario registered for the mission's terrain.";
		if (code == "ORBAT_ARTIFACT_MISMATCH")
			return "the event's ORBAT seats do not match the mission's slots one to one.";

		return string.Empty;
	}
}
