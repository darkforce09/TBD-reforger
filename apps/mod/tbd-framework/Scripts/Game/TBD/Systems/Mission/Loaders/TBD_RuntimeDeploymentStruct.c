//! The deployment a server runs, as the platform describes it and as the local artifact cache
//! records it: which deployment, which catalog mission, which artifact with which SHA-256 and size,
//! which terrain and scenario, and - when it runs for a platform event - the event and event mission.
//! @authority server

//! `GET /api/v1/game-runtime/deployment` answer (the deployment in flight, else the latest confirmed
//! one), and the identity file of TBD_MissionArtifactCache. Field names are the JSON keys;
//! `event_id` and `event_mission_id` are absent without an event.
class TBD_RuntimeDeploymentStruct
{
	string deployment_id;
	string state;
	string mission_id;
	string artifact_id;
	string artifact_sha256;
	int artifact_bytes;
	string terrain_key;
	string scenario_id;
	string event_id;
	string event_mission_id;

	//------------------------------------------------------------------------------------------------
	//! The deployment in `body`, or null with `problem` saying why it cannot be used.
	static TBD_RuntimeDeploymentStruct Parse(string body, out string problem)
	{
		problem = string.Empty;
		JsonLoadContext context = new JsonLoadContext();
		TBD_RuntimeDeploymentStruct deployment = new TBD_RuntimeDeploymentStruct();
		if (body.IsEmpty() || !context.LoadFromString(body) || !context.ReadValue("", deployment))
		{
			problem = "not readable JSON: " + TBD_GameRuntimeAnswer.LoggableBody(body);
			return null;
		}

		if (deployment.deployment_id.IsEmpty() || deployment.artifact_id.IsEmpty())
		{
			problem = "it names no deployment_id or artifact_id";
			return null;
		}

		if (!TBD_Sha256.IsHexDigest(deployment.artifact_sha256))
		{
			problem = "its artifact_sha256 is not 64 lowercase hex characters";
			return null;
		}

		if (deployment.artifact_bytes < 1 || deployment.artifact_bytes > TBD_MissionLoader.MISSION_FILE_MAX_BYTES)
		{
			problem = string.Format("its artifact_bytes %1 is outside 1..%2", deployment.artifact_bytes, TBD_MissionLoader.MISSION_FILE_MAX_BYTES);
			return null;
		}

		return deployment;
	}
}
