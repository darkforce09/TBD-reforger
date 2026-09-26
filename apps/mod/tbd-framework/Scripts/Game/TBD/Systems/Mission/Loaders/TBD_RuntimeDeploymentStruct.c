/**
 * @file TBD_RuntimeDeploymentStruct.c
 * @brief The deployment a server runs, as the platform describes it and as the artifact cache
 * records it.
 *
 * Role: the `GET /api/v1/game-runtime/deployment` answer (the deployment in flight, else the latest
 * confirmed one) and the cache's `identity.json`: which deployment, catalog mission, artifact with
 * its SHA-256 and size, terrain and scenario, and, for a platform event, the event and event
 * mission.  Position: parsed by `TBD_DeployedMission`; read and written by
 * `TBD_MissionArtifactCache`.
 * State: none; plain data.  Invariants: field names are the JSON keys; `Parse` refuses an answer
 * with no deployment or artifact id, a digest that is not 64 lowercase hex characters, or a size
 * outside 1..`MISSION_FILE_MAX_BYTES`.
 */

//! One runtime deployment. `event_id` and `event_mission_id` are absent without an event.
//! @contract mission-deployment.schema.json#/definitions/RuntimeDeployment
//! @authority server
class TBD_RuntimeDeploymentStruct
{
	string deployment_id;    //!< JSON `deployment_id`: the deployment.
	string state;            //!< JSON `state`: the deployment state.
	string mission_id;       //!< JSON `mission_id`: the catalog mission.
	string artifact_id;      //!< JSON `artifact_id`: the compiled artifact to run.
	string artifact_sha256;  //!< JSON `artifact_sha256`: the artifact's SHA-256, 64 lowercase hex characters.
	int artifact_bytes;      //!< JSON `artifact_bytes`: the artifact's size in bytes, 1..`MISSION_FILE_MAX_BYTES`.
	string terrain_key;      //!< JSON `terrain_key`: the terrain the mission runs on.
	string scenario_id;      //!< JSON `scenario_id`: the mission header the server boots.
	string event_id;         //!< JSON `event_id`: the platform event; empty without one.
	string event_mission_id; //!< JSON `event_mission_id`: the event mission; empty without an event.

	//! The deployment in `body`.
	//! @param body the answer or identity file text
	//! @param problem set to why the deployment cannot be used
	//! @return the deployment, or null with `problem` set
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
