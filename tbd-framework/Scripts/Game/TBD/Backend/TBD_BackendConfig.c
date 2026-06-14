//! Server-side backend connection settings. Copy backend.example.json to
//! $profile:TBD_BackendConfig.json on dedicated hosts.
class TBD_BackendConfigStruct
{
	string backendUrl;
	string serverToken;
	string missionId;
	string eventId;
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
			Print("[TBD] Missing backend config at " + s_ConfigPath + " — REST loader disabled until configured.", LogLevel.WARNING);
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
	static string GetMissionId()
	{
		if (!s_Config)
			return string.Empty;
		return s_Config.missionId;
	}

	//------------------------------------------------------------------------------------------------
	static string GetEventId()
	{
		if (!s_Config)
			return string.Empty;
		return s_Config.eventId;
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

		SCR_JsonSaveContext ctx = new SCR_JsonSaveContext();
		ctx.WriteValue("", s_Config);
		if (!ctx.SaveToFile(s_ConfigPath))
		{
			Print("[TBD] Failed to write backend config to " + s_ConfigPath, LogLevel.ERROR);
			return false;
		}
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Sets the active missionId and persists it (the loader re-reads it after a scenario reload).
	static bool SetMissionId(string missionId)
	{
		EnsureConfig();
		s_Config.missionId = missionId;
		return Save();
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
