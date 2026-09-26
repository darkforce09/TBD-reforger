/**
 * @file TBD_BackendConfig.c
 * @brief The server's backend connection settings, read from `$profile:TBD_BackendConfig.json`.
 *
 * Role: loads, reloads, repoints and persists the backend URL and the machine credential.
 * Position: loaded by `TBD_DeployedMission` at boot and reloaded by it and `TBD_RosterLoader`;
 * repointed by `#tbd backend` in `TBD_AdminCommands`; read by `TBD_GameRuntimeHttp`,
 * and `TBD_BackendText`.
 * State: the settings in force, a server static.  Invariants: `machineCredential` is this server's
 * `mod_runtime` credential (`tbdm_<32 hex>_<64 hex>`), sent as `Authorization: Bearer` to every
 * `/api/v1/game-runtime/`, `/api/v1/fleet-executor/` and `/api/v1/ingest/` route and never logged;
 * `#tbd backend` repoints only the URL, so the credential changes only in the profile file; a reload
 * replaces the settings only with a file that reads and parses. The mission is not configured
 * here: the server runs the mission deployed to it on the platform (`TBD_DeployedMission`).
 */

//! The profile file's shape; the field names are its JSON keys.
class TBD_BackendConfigFile
{
	string backendUrl; //!< JSON key `backendUrl`; base URL of the platform, empty when unset
	string machineCredential; //!< JSON key `machineCredential`; `mod_runtime` bearer credential, never logged
}

//! Static access to the backend settings in force.
//! @authority server
class TBD_BackendConfig
{
	protected static ref TBD_BackendConfigFile s_Config; //!< the settings in force, or null before the first load
	protected static string s_ConfigPath = "$profile:TBD_BackendConfig.json"; //!< profile file path

	//! Load the profile file, replacing the settings with an empty set first. A missing file warns
	//! (no platform connection until configured); an unreadable or unparsable file logs an error.
	//! @return true when the file loaded and parsed
	static bool Load()
	{
		s_Config = new TBD_BackendConfigFile();

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

	//! Re-read the profile file while the server runs. The settings in force are replaced only by a
	//! file that reads and parses, so a file caught half-written changes nothing.
	//! @return true when the settings were replaced
	static bool Reload()
	{
		if (!FileIO.FileExists(s_ConfigPath))
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromFile(s_ConfigPath))
			return false;

		TBD_BackendConfigFile reloaded = new TBD_BackendConfigFile();
		if (!ctx.ReadValue("", reloaded))
			return false;

		s_Config = reloaded;
		return true;
	}

	//! The settings in force.
	//! @return the settings, or null before the first load
	static TBD_BackendConfigFile Get()
	{
		return s_Config;
	}

	//! The backend base URL.
	//! @return the URL, or empty when unset or not loaded
	static string GetBackendUrl()
	{
		if (!s_Config)
			return string.Empty;
		return s_Config.backendUrl;
	}

	//! This server's `mod_runtime` machine credential without surrounding whitespace.
	//! @return the credential, or empty when unset or not loaded
	static string GetMachineCredential()
	{
		if (!s_Config)
			return string.Empty;
		return s_Config.machineCredential.Trim();
	}

	//! Make sure settings exist: load from disk on first use, else start from an empty set.
	protected static void EnsureConfig()
	{
		if (!s_Config)
			Load();
		if (!s_Config)
			s_Config = new TBD_BackendConfigFile();
	}

	//! Write the settings in force back to the profile file so they survive a scenario reload.
	//! @return false without settings or when the write fails (logged)
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

	//! Repoint the backend URL, keeping the machine credential, then persist.
	//! @param backendUrl the new base URL
	//! @return true when the settings were written
	static bool SetBackend(string backendUrl)
	{
		EnsureConfig();
		s_Config.backendUrl = backendUrl;
		return Save();
	}
}
