//! Server-side backend connection settings. Copy backend.example.json to
//! $profile:TBD_BackendConfig.json on dedicated hosts.
//!
//! Two secrets authenticate two route families:
//!   * `serverToken` - the shared `X-Service-Token` of link confirmation and match results;
//!   * `machineCredential` - this server's own `mod_runtime` machine credential
//!     (`tbdm_<32 hex>_<64 hex>`, issued by an administrator for this server), sent as
//!     `Authorization: Bearer` to every `/api/v1/game-runtime/` and `/api/v1/fleet-executor/`
//!     route: the deployed mission and its artifact, the runtime session and its heartbeats, the
//!     event roster, deployment authorization and ended lives, fleet commands, the deployable
//!     mission list and in-game deployment requests.
//! Neither is ever logged. The mission and its event are not configured here: the server runs the
//! mission deployed to it on the platform (TBD_DeployedMission).
class TBD_BackendConfigStruct
{
	string backendUrl;
	string serverToken;
	string machineCredential;
}

class TBD_BackendConfig
{
	protected static ref TBD_BackendConfigStruct s_Config;
	protected static string s_ConfigPath = "$profile:TBD_BackendConfig.json";

	//------------------------------------------------------------------------------------------------
	static bool Load()
	{
		s_Config = new TBD_BackendConfigStruct();

		if (!FileIO.FileExists(s_ConfigPath))
		{
			Print("[TBD] Missing backend config at " + s_ConfigPath + " - no platform connection until configured.", LogLevel.WARNING);
			return false;
		}

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromFile(s_ConfigPath))
		{
			Print("[TBD] Failed to read backend config.", LogLevel.ERROR);
			return false;
		}

		if (!ctx.ReadValue("", s_Config))
		{
			Print("[TBD] Failed to parse backend config.", LogLevel.ERROR);
			return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Re-read the profile file while the server runs. The settings in force are replaced only by a
	//! file that reads and parses, so a file caught half-written changes nothing.
	static bool Reload()
	{
		if (!FileIO.FileExists(s_ConfigPath))
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromFile(s_ConfigPath))
			return false;

		TBD_BackendConfigStruct reloaded = new TBD_BackendConfigStruct();
		if (!ctx.ReadValue("", reloaded))
			return false;

		s_Config = reloaded;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_BackendConfigStruct Get()
	{
		return s_Config;
	}

	//------------------------------------------------------------------------------------------------
	static string GetBackendUrl()
	{
		if (!s_Config)
			return string.Empty;
		return s_Config.backendUrl;
	}

	//------------------------------------------------------------------------------------------------
	static string GetServerToken()
	{
		if (!s_Config)
			return string.Empty;
		return s_Config.serverToken;
	}

	//------------------------------------------------------------------------------------------------
	//! This server's `mod_runtime` machine credential without surrounding whitespace, or empty.
	static string GetMachineCredential()
	{
		if (!s_Config)
			return string.Empty;
		return s_Config.machineCredential.Trim();
	}

	//------------------------------------------------------------------------------------------------
	//! Ensures s_Config exists (loads from disk on first use, else empty struct).
	protected static void EnsureConfig()
	{
		if (!s_Config)
			Load();
		if (!s_Config)
			s_Config = new TBD_BackendConfigStruct();
	}

	//------------------------------------------------------------------------------------------------
	//! Persists the current config back to $profile so it survives a scenario reload.
	protected static bool Save()
	{
		if (!s_Config)
			return false;

		JsonSaveContext ctx = new JsonSaveContext();
		ctx.WriteValue("", s_Config);
		if (!ctx.SaveToFile(s_ConfigPath))
		{
			Print("[TBD] Failed to write backend config to " + s_ConfigPath, LogLevel.ERROR);
			return false;
		}
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Repoints the backend URL (and optionally the server token), then persists.
	static bool SetBackend(string backendUrl, string serverToken = string.Empty)
	{
		EnsureConfig();
		s_Config.backendUrl = backendUrl;
		if (!serverToken.IsEmpty())
			s_Config.serverToken = serverToken;
		return Save();
	}
}
