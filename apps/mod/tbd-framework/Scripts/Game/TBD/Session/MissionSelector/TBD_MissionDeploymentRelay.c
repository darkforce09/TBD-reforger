//! An in-game administrator's mission selection, relayed to the platform as a deployment request:
//! `POST /api/v1/game-runtime/deployments` {mission_id, artifact_id, event_mission_id?,
//! requested_by_arma_id} (mod_runtime credential). The platform decides: the admin's game identity
//! must be linked to a platform administrator, and the selection is validated like a deployment
//! made on the website. Nothing restarts here. An accepted deployment runs when its fleet command
//! does - a `load_mission` this runtime executes (TBD_FleetLoadMissionAction), or a restart by the
//! host agent - and the admin is told the outcome in chat: accepted, or the refusal in words.
//!
//! Selecting the mission this world runs for an event keeps the event: the request names the
//! running deployment's event mission, so restarting the event's mission keeps its seats and
//! reservations. Any other selection deploys without an event.
//! @authority server

//! The accepted deployment (202), as far as the admin is told about it. Field names are the JSON keys.
class TBD_RelayedDeploymentStruct
{
	string id;
	string mission_title;
	string transition;
	string state;
	int bound_slots;
}

//! A relayed deployment request on its way to the platform.
class TBD_MissionDeploymentRelayCall : TBD_GameRuntimeCall
{
	int m_iAdminPlayerId;
	string m_sTitle;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_MissionDeploymentRelay.OnAnswered(this, answer);
	}
}

class TBD_MissionDeploymentRelay
{
	//------------------------------------------------------------------------------------------------
	//! Ask the platform to deploy mission `number` of the list on behalf of admin `adminPlayerId`.
	//! Returns the immediate reply; the platform's decision reaches the admin in chat.
	static string RequestByNumber(int adminPlayerId, int number)
	{
		TBD_DeployableMissionStruct entry = TBD_DeployableMissionList.GetEntryByNumber(number);
		if (!entry)
			return string.Format("TBD: no mission #%1 - '#tbd missions' lists them.", number);

		string armaId = TBD_PlayerIdentity.GetArmaId(adminPlayerId);
		if (armaId.IsEmpty())
			return "TBD: this server cannot read your game identity, so the platform cannot tell who asks - nothing was deployed.";

		string body = string.Format("{\"mission_id\":\"%1\",\"artifact_id\":\"%2\",\"requested_by_arma_id\":\"%3\"",
			TBD_GameRuntimeHttp.JsonEscape(entry.mission_id), TBD_GameRuntimeHttp.JsonEscape(entry.artifact_id), TBD_GameRuntimeHttp.JsonEscape(armaId));

		string eventMissionId;
		if (entry.mission_id == TBD_DeployedMission.GetMissionId())
			eventMissionId = TBD_DeployedMission.GetEventMissionId();

		if (!eventMissionId.IsEmpty())
			body += string.Format(",\"event_mission_id\":\"%1\"", TBD_GameRuntimeHttp.JsonEscape(eventMissionId));

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

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_MissionDeploymentRelayCall with the platform's decision.
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

	//------------------------------------------------------------------------------------------------
	//! The platform's refusal in words the admin can act on, with its code.
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

	//------------------------------------------------------------------------------------------------
	//! What a refusal code means for the admin, or empty for a code without its own words.
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
