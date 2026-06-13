//! Minimal parsed mission header — expanded in Phase 1 as loader hardens.
class TBD_MissionMetaStruct
{
	string id;
	string name;
	string terrain;
}

class TBD_MissionDocumentStruct
{
	string schemaVersion;
	ref TBD_MissionMetaStruct meta;
}

//! Loads Mission JSON from backend REST or $profile fallback.
class TBD_MissionLoader
{
	protected static ref TBD_MissionDocumentStruct s_Mission;
	protected static string s_RawJson;
	protected static bool s_Loaded;
	protected static bool s_LoadInFlight;

	protected static ref RestCallback s_RestCallback;

	//------------------------------------------------------------------------------------------------
	static bool IsLoaded()
	{
		return s_Loaded;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_MissionDocumentStruct GetMission()
	{
		return s_Mission;
	}

	//------------------------------------------------------------------------------------------------
	static string GetRawJson()
	{
		return s_RawJson;
	}

	//------------------------------------------------------------------------------------------------
	//! Entry point: tries REST when backend config exists, else file only.
	static void BeginLoad()
	{
		if (s_Loaded || s_LoadInFlight)
			return;

		TBD_BackendConfig.Load();
		string missionId = TBD_BackendConfig.GetMissionId();
		if (missionId.IsEmpty())
		{
			Print("[TBD] missionId not configured — cannot load mission.", LogLevel.ERROR);
			return;
		}

		if (!TBD_BackendConfig.GetBackendUrl().IsEmpty() && !TBD_BackendConfig.GetServerToken().IsEmpty())
		{
			s_LoadInFlight = true;
			FetchFromBackend(missionId);
			return;
		}

		if (LoadFromProfileFile(missionId))
			s_Loaded = true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void FetchFromBackend(string missionId)
	{
		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			Print("[TBD] RestApi unavailable — trying profile fallback.", LogLevel.WARNING);
			s_LoadInFlight = false;
			if (LoadFromProfileFile(missionId))
				s_Loaded = true;
			return;
		}

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext ctx = rest.GetContext(baseUrl);
		if (!ctx)
		{
			Print("[TBD] RestContext failed for " + baseUrl, LogLevel.ERROR);
			s_LoadInFlight = false;
			if (LoadFromProfileFile(missionId))
				s_Loaded = true;
			return;
		}

		s_RestCallback = new RestCallback();
		s_RestCallback.SetOnSuccess(OnBackendFetchSuccess);
		s_RestCallback.SetOnError(OnBackendFetchError);

		string token = TBD_BackendConfig.GetServerToken();
		ctx.SetHeaders(string.Format("Authorization, Bearer %1,Accept,application/json", token));

		string path = string.Format("/api/missions/%1/compiled", missionId);
		Print("[TBD] Fetching mission " + missionId + " from " + baseUrl + path);
		ctx.GET(s_RestCallback, path);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnBackendFetchSuccess(RestCallback cb)
	{
		s_LoadInFlight = false;
		string data = cb.GetData();
		if (data.IsEmpty())
		{
			Print("[TBD] Backend returned empty mission body.", LogLevel.ERROR);
			TryProfileFallbackAfterRestFailure();
			return;
		}

		if (!ParseMissionJson(data))
		{
			TryProfileFallbackAfterRestFailure();
			return;
		}

		string missionId = TBD_BackendConfig.GetMissionId();
		CacheToProfile(missionId, data);
		s_Loaded = true;
		Print("[TBD] Mission loaded from backend: " + s_Mission.meta.name);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnBackendFetchError(RestCallback cb)
	{
		s_LoadInFlight = false;
		Print("[TBD] Backend mission fetch failed — trying profile fallback.", LogLevel.WARNING);
		TryProfileFallbackAfterRestFailure();
	}

	//------------------------------------------------------------------------------------------------
	protected static void TryProfileFallbackAfterRestFailure()
	{
		string missionId = TBD_BackendConfig.GetMissionId();
		if (LoadFromProfileFile(missionId))
		{
			s_Loaded = true;
			Print("[TBD] Mission loaded from profile fallback.");
		}
		else
		{
			Print("[TBD] Mission load failed (REST + profile). Server stays in LOADING.", LogLevel.ERROR);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static bool LoadFromProfileFile(string missionId)
	{
		string path = string.Format("$profile:missions/%1.json", missionId);
		if (!FileIO.FileExists(path))
		{
			Print("[TBD] Profile mission file missing: " + path, LogLevel.ERROR);
			return false;
		}

		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
			return false;

		string data;
		handle.Read(data, 8 * 1024 * 1024);
		handle.Close();

		return ParseMissionJson(data);
	}

	//------------------------------------------------------------------------------------------------
	protected static bool ParseMissionJson(string data)
	{
		s_RawJson = data;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.ImportFromString(data))
		{
			Print("[TBD] Mission JSON parse failed.", LogLevel.ERROR);
			return false;
		}

		s_Mission = new TBD_MissionDocumentStruct();
		if (!ctx.ReadValue("", s_Mission))
		{
			Print("[TBD] Mission JSON schema mismatch (meta block).", LogLevel.ERROR);
			s_Mission = null;
			return false;
		}

		if (!s_Mission.meta || s_Mission.meta.id.IsEmpty())
		{
			Print("[TBD] Mission JSON missing meta.id.", LogLevel.ERROR);
			s_Mission = null;
			return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void CacheToProfile(string missionId, string data)
	{
		string dir = "$profile:missions";
		if (!FileIO.MakeDirectory(dir))
		{
			// May already exist — not fatal.
		}

		string path = string.Format("%1/%2.json", dir, missionId);
		FileHandle handle = FileIO.OpenFile(path, FileMode.WRITE);
		if (!handle)
		{
			Print("[TBD] Could not cache mission to " + path, LogLevel.WARNING);
			return;
		}

		handle.Write(data);
		handle.Close();
		Print("[TBD] Cached mission to " + path);
	}
}
