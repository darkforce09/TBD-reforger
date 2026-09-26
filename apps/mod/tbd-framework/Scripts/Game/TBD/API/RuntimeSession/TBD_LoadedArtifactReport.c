/**
 * @file TBD_LoadedArtifactReport.c
 * @brief What this world's runtime session reports loading when it starts.
 *
 * Role: holds the decided mission artifact, by id and the SHA-256 of the exact bytes loaded, or
 * none, and builds the session start body.  Position: decided by `TBD_DeployedMission` once per
 * world; read by `TBD_RuntimeSession` for `POST /api/v1/game-runtime/sessions`; reset by
 * `TBD_RuntimeSession.Stop` and `TBD_DeployedMission`.
 * State: the decided flag, artifact id and digest; server statics.  Invariants: a deployment is
 * confirmed by the first artifact report of a session started after it was requested, so the
 * report names only what this world really runs; the session starts only once the report is
 * decided, and a report is cleared the moment its world is about to be replaced.
 */

//! The decided loaded-artifact report of this world.
//! @authority server
class TBD_LoadedArtifactReport
{
	protected static bool s_bDecided; //!< this world has decided what it runs
	protected static string s_sArtifactId; //!< JSON key `loaded_artifact_id`, or empty
	protected static string s_sArtifactSha256; //!< JSON key `loaded_artifact_sha256`, hex digest of the loaded bytes

	//! Nothing decided: a new world is loading, or the running one is about to be replaced.
	static void Reset()
	{
		s_bDecided = false;
		s_sArtifactId = string.Empty;
		s_sArtifactSha256 = string.Empty;
	}

	//! This world runs `artifactId`, whose exact bytes hash to `sha256`; a waiting start goes now.
	//! @authority server
	static void DeclareLoaded(string artifactId, string sha256)
	{
		s_sArtifactId = artifactId;
		s_sArtifactSha256 = sha256;
		s_bDecided = true;
		TBD_RuntimeSession.OnLoadedArtifactDecided();
	}

	//! This world runs no mission artifact; a waiting start goes now.
	//! @authority server
	static void DeclareNone()
	{
		s_sArtifactId = string.Empty;
		s_sArtifactSha256 = string.Empty;
		s_bDecided = true;
		TBD_RuntimeSession.OnLoadedArtifactDecided();
	}

	//! Whether this world has decided what it runs.
	//! @return true once `DeclareLoaded` or `DeclareNone` ran since the last `Reset`
	static bool IsDecided()
	{
		return s_bDecided;
	}

	//! Whether the decided report names an artifact.
	//! @return true for a decided, non-empty artifact id
	static bool NamesArtifact()
	{
		return s_bDecided && !s_sArtifactId.IsEmpty();
	}

	//! The report for log lines.
	//! @return `artifact=<id> sha256=<digest>`, or `artifact=none`
	static string Describe()
	{
		if (!NamesArtifact())
			return "artifact=none";

		return string.Format("artifact=%1 sha256=%2", s_sArtifactId, s_sArtifactSha256);
	}

	//! The session start body.
	//! @return both fields together, or `{}` when the world runs no artifact
	static string BuildStartBody()
	{
		if (!NamesArtifact())
			return "{}";

		return string.Format("{\"loaded_artifact_id\":\"%1\",\"loaded_artifact_sha256\":\"%2\"}",
			TBD_BackendText.JsonEscape(s_sArtifactId), TBD_BackendText.JsonEscape(s_sArtifactSha256));
	}
}
