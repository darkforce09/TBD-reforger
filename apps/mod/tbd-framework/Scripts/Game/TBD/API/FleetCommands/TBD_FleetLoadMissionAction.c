//! The effect of the `load_mission` fleet command: a mission deployment on the terrain this runtime
//! already runs. Once the platform has admitted `executing` (TBD_FleetCommandExecution):
//!   1. the deployment in effect is read (`GET /api/v1/game-runtime/deployment`); it must be the
//!      command's deployment, with the command's artifact and SHA-256;
//!   2. the artifact is fetched and the SHA-256 of its bytes verified
//!      (TBD_MissionArtifactVerification);
//!   3. the verified bytes are written to the profile cache under the deployment's identity
//!      (TBD_MissionArtifactCache);
//!   4. `succeeded` is reported with outcome {artifact_id, restart_requested: true};
//!   5. once that report is recorded, this world's runtime session is closed and the scenario
//!      restarts in-process (`GameStateTransitions.RequestScenarioRestart`). The next world's boot
//!      (TBD_DeployedMission) reads the same deployment, takes its artifact from the cache under the
//!      same id and SHA-256, and starts a session reporting it, which confirms the deployment.
//! A failure in steps 1 to 3 is reported `failed` with its reason, and nothing restarts.
//! @authority server

//! The deployment read of a `load_mission` command.
class TBD_LoadMissionDeploymentReadCall : TBD_GameRuntimeCall
{
	string m_sCommandId;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_FleetLoadMissionAction.OnDeploymentAnswered(this, answer);
	}
}

//! The artifact verification of a `load_mission` command.
class TBD_LoadMissionArtifactVerification : TBD_MissionArtifactVerification
{
	string m_sCommandId;
	ref TBD_RuntimeDeploymentStruct m_Identity;

	//------------------------------------------------------------------------------------------------
	override void OnFinished()
	{
		TBD_FleetLoadMissionAction.OnVerificationFinished(m_sCommandId);
	}
}

class TBD_FleetLoadMissionAction
{
	protected static ref TBD_LoadMissionArtifactVerification s_Verification;

	//------------------------------------------------------------------------------------------------
	static void Start(notnull TBD_FleetCommand command)
	{
		TBD_LoadMissionDeploymentReadCall call = new TBD_LoadMissionDeploymentReadCall();
		call.m_sCommandId = command.m_sCommandId;

		string failure;
		if (!TBD_GameRuntimeHttp.Get(call, TBD_GameRuntimeHttp.ROUTE_PREFIX + "/deployment", failure))
			TBD_FleetCommandExecution.Fail(command, "the deployment could not be read: " + failure);
	}

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_LoadMissionDeploymentReadCall with the platform's answer.
	static void OnDeploymentAnswered(notnull TBD_LoadMissionDeploymentReadCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_FleetCommand command = CurrentCommand(call.m_sCommandId);
		if (!command)
			return;

		if (answer.m_eOutcome != TBD_EGameRuntimeOutcome.SUCCESS)
		{
			TBD_FleetCommandExecution.Fail(command, "the deployment could not be read (" + answer.m_sDetail + ")");
			return;
		}

		string problem;
		TBD_RuntimeDeploymentStruct deployment = TBD_RuntimeDeploymentStruct.Parse(answer.m_sBody, problem);
		if (!deployment)
		{
			TBD_FleetCommandExecution.Fail(command, "the deployment answer is unusable: " + problem);
			return;
		}

		string deploymentId = command.Argument("deployment_id");
		string artifactId = command.Argument("artifact_id");
		string sha256 = command.Argument("artifact_sha256");
		if (deployment.deployment_id != deploymentId || deployment.artifact_id != artifactId || deployment.artifact_sha256 != sha256)
		{
			TBD_FleetCommandExecution.Fail(command, string.Format("the deployment in effect is %1 (artifact %2), not the command's %3 (artifact %4)",
				deployment.deployment_id, deployment.artifact_id, deploymentId, artifactId));
			return;
		}

		s_Verification = new TBD_LoadMissionArtifactVerification();
		s_Verification.m_sCommandId = command.m_sCommandId;
		s_Verification.m_Identity = deployment;
		s_Verification.FetchFromPlatform(artifactId, sha256, deployment.artifact_bytes);
	}

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_LoadMissionArtifactVerification when it ends. Acted on in the next frame, once the
	//! verification's own call stack has unwound.
	static void OnVerificationFinished(string commandId)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.CallLater(SettleVerification, 0, false, commandId);
	}

	//------------------------------------------------------------------------------------------------
	protected static void SettleVerification(string commandId)
	{
		TBD_LoadMissionArtifactVerification verification = s_Verification;
		TBD_FleetCommand command = CurrentCommand(commandId);
		if (!command || !verification || verification.m_sCommandId != commandId)
			return;

		s_Verification = null;

		if (verification.m_eResult != TBD_EArtifactVerificationResult.VERIFIED)
		{
			TBD_FleetCommandExecution.Fail(command, verification.m_sFailure);
			return;
		}

		string failure;
		if (!TBD_MissionArtifactCache.Store(verification.m_sDocument, verification.m_Identity, failure))
		{
			TBD_FleetCommandExecution.Fail(command, "the verified artifact could not be written to the profile cache: " + failure);
			return;
		}

		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "load-mission-cached", string.Format("command=%1 deployment=%2 %3",
			commandId, verification.m_Identity.deployment_id, verification.Describe()));
		TBD_FleetCommandExecution.Succeed(command, string.Format("{\"artifact_id\":\"%1\",\"restart_requested\":true}",
			TBD_GameRuntimeHttp.JsonEscape(verification.m_sArtifactId)), true);
	}

	//------------------------------------------------------------------------------------------------
	//! The success is recorded: close this world's session and restart the scenario in-process. A
	//! world that already ended since the claim is not restarted again; the next boot reads the same
	//! deployment.
	static void RestartScenario(notnull TBD_FleetCommand command)
	{
		string artifactId = command.Argument("artifact_id");
		if (!TBD_FleetCommandPoller.IsCurrentWorld(command.m_iWorld))
		{
			TBD_Log.Event(TBD_FleetCommandPoller.CH_FLEET, string.Format("command=%1: the world that claimed it has ended already - no second restart; the next boot loads artifact %2",
				command.m_sCommandId, artifactId));
			return;
		}

		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "restart", string.Format("command=%1 deployment=%2 artifact=%3 - closing this world's runtime session and restarting the scenario in-process",
			command.m_sCommandId, command.Argument("deployment_id"), artifactId));
		TBD_PlayerChat.TellEveryone("TBD: the server restarts the scenario now to load the newly deployed mission.");

		TBD_FleetCommandPoller.Stop();
		TBD_RuntimeSession.Stop();
		GameStateTransitions.RequestScenarioRestart();
	}

	//------------------------------------------------------------------------------------------------
	//! The `load_mission` command being executed under `commandId`, or null.
	protected static TBD_FleetCommand CurrentCommand(string commandId)
	{
		TBD_FleetCommand command = TBD_FleetCommandExecution.GetCurrent();
		if (!command || command.m_sCommandId != commandId || command.m_sAction != "load_mission")
			return null;

		if (command.m_eStage != TBD_EFleetCommandStage.EFFECT)
			return null;

		return command;
	}
}
