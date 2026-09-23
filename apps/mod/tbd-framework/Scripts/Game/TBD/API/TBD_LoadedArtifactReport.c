//! What this world's runtime session reports loading when it starts
//! (`POST /api/v1/game-runtime/sessions`): a mission artifact, by id and the SHA-256 of the exact
//! bytes loaded, or nothing.
//!
//! A mission deployment is confirmed by the first artifact report of a session started after it
//! was requested, so the report must name only what this world really runs. The session therefore
//! starts only once the report is decided (TBD_DeployedMission decides it, once per world), and a
//! report is cleared the moment its world is about to be replaced.
//! @authority server
class TBD_LoadedArtifactReport
{
	protected static bool s_bDecided;
	protected static string s_sArtifactId;
	protected static string s_sArtifactSha256;

	//------------------------------------------------------------------------------------------------
	//! Nothing decided: a new world is loading, or the running one is about to be replaced.
	static void Reset()
	{
		s_bDecided = false;
		s_sArtifactId = string.Empty;
		s_sArtifactSha256 = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! This world runs `artifactId`, whose exact bytes hash to `sha256`.
	static void DeclareLoaded(string artifactId, string sha256)
	{
		s_sArtifactId = artifactId;
		s_sArtifactSha256 = sha256;
		s_bDecided = true;
		TBD_RuntimeSession.OnLoadedArtifactDecided();
	}

	//------------------------------------------------------------------------------------------------
	//! This world runs no mission artifact.
	static void DeclareNone()
	{
		s_sArtifactId = string.Empty;
		s_sArtifactSha256 = string.Empty;
		s_bDecided = true;
		TBD_RuntimeSession.OnLoadedArtifactDecided();
	}

	//------------------------------------------------------------------------------------------------
	static bool IsDecided()
	{
		return s_bDecided;
	}

	//------------------------------------------------------------------------------------------------
	//! True when the decided report names an artifact.
	static bool NamesArtifact()
	{
		return s_bDecided && !s_sArtifactId.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! For log lines: `artifact=<id> sha256=<digest>`, or `artifact=none`.
	static string Describe()
	{
		if (!NamesArtifact())
			return "artifact=none";

		return string.Format("artifact=%1 sha256=%2", s_sArtifactId, s_sArtifactSha256);
	}

	//------------------------------------------------------------------------------------------------
	//! The session start body: both fields together, or `{}` when the world runs no artifact.
	static string BuildStartBody()
	{
		if (!NamesArtifact())
			return "{}";

		return string.Format("{\"loaded_artifact_id\":\"%1\",\"loaded_artifact_sha256\":\"%2\"}",
			TBD_GameRuntimeHttp.JsonEscape(s_sArtifactId), TBD_GameRuntimeHttp.JsonEscape(s_sArtifactSha256));
	}
}
