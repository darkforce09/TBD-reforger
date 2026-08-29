//! One mission entry from the backend mission list.
//!
//! Field names are the wire keys - Enfusion's `JsonLoadContext` binds JSON keys onto class
//! fields BY NAME, so `slotCount` is camelCase on the backend too
//! (`IngestMissionListEntry` in `apps/website/api/src/handlers/missions.rs` renames it
//! explicitly). A key this class does not declare is not an error there, it is invisible -
//! a snake_case `slot_count` would parse to 0 for every mission with nothing logged.
//!
//! `terrain` is the compiled document's `meta.terrain` slug ("everon"), because
//! `TBD_FrameworkManager.SelectMissionByNumber` compares it to the loaded mission's terrain
//! and feeds it to `TBD_ScenarioRouter.GetScenarioForTerrain`.
//! @route GET /api/v1/ingest/missions
class TBD_MissionListEntry
{
	string id;
	string name;
	string terrain;
	int slotCount;
}

//! Wrapper for the {"missions":[...], "count":N} list response.
class TBD_MissionListStruct
{
	ref array<ref TBD_MissionListEntry> missions;
	int count;
}

//! Fetches the available-mission list from the backend for the in-game admin
//! browser. Mirrors the REST pattern in TBD_MissionLoader.
//!
//! @route GET /api/v1/ingest/missions (service-token tier; `X-Service-Token`)
class TBD_MissionListLoader
{
	//! The service-token mission list. This is NOT `/api/v1/missions`: that route is member
	//! tier and OWNER-SCOPED (it takes an `AuthUser` and filters on the caller's discord id),
	//! and a game server has no "me". T-181.51 added this unscoped sibling next to the other
	//! `/ingest/` routes for exactly that reason.
	protected static const string LIST_PATH = "/api/v1/ingest/missions";

	protected static ref array<ref TBD_MissionListEntry> s_Entries;
	protected static bool s_Loaded;
	protected static bool s_InFlight;
	protected static ref RestCallback s_Callback;

	//------------------------------------------------------------------------------------------------
	static bool IsLoaded()
	{
		return s_Loaded;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsLoading()
	{
		return s_InFlight;
	}

	//------------------------------------------------------------------------------------------------
	static array<ref TBD_MissionListEntry> GetEntries()
	{
		return s_Entries;
	}

	//------------------------------------------------------------------------------------------------
	static int Count()
	{
		if (!s_Entries)
			return 0;
		return s_Entries.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! 1-based lookup to match the numbered list shown to admins; null if out of range.
	static TBD_MissionListEntry GetEntryByNumber(int number)
	{
		int index = number - 1;
		if (!s_Entries || index < 0 || index >= s_Entries.Count())
			return null;
		return s_Entries[index];
	}

	//------------------------------------------------------------------------------------------------
	//! Begins a refresh from the backend. Returns false if unconfigured or already in flight.
	static bool Refresh()
	{
		if (s_InFlight)
			return false;

		TBD_BackendConfig.Load();
		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		string token = TBD_BackendConfig.GetServerToken();
		if (baseUrl.IsEmpty() || token.IsEmpty())
		{
			Print("[TBD] MissionList: backend not configured.", LogLevel.ERROR);
			return false;
		}

		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			Print("[TBD] MissionList: RestApi unavailable.", LogLevel.ERROR);
			return false;
		}

		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext ctx = rest.GetContext(baseUrl);
		if (!ctx)
		{
			Print("[TBD] MissionList: RestContext failed for " + baseUrl, LogLevel.ERROR);
			return false;
		}

		s_Callback = new RestCallback();
		s_Callback.SetOnSuccess(OnSuccess);
		s_Callback.SetOnError(OnError);
		// T-181.51 - the game-server tier is `X-Service-Token`, NOT an Authorization bearer
		// (`ServiceAuth`, apps/website/api/src/middleware/auth.rs, reads only that header). This
		// sent `Authorization: Bearer` against a route that never existed, so the 404 hid the
		// auth bug underneath it: fixing the URL alone would have turned it into a 401. Same
		// "Key,Value,Key,Value" comma form (no space) the three working loaders use -
		// TBD_MissionLoader, TBD_IdentityLink, TBD_ResultsReporter.
		ctx.SetHeaders(string.Format("X-Service-Token,%1,Accept,application/json", token));

		s_InFlight = true;
		Print("[TBD] MissionList: fetching " + baseUrl + LIST_PATH);
		ctx.GET(s_Callback, LIST_PATH);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnSuccess(RestCallback cb)
	{
		s_InFlight = false;
		string data = cb.GetData();
		if (data.IsEmpty())
		{
			Print("[TBD] MissionList: empty response.", LogLevel.ERROR);
			return;
		}
		ParseList(data);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnError(RestCallback cb)
	{
		s_InFlight = false;
		Print("[TBD] MissionList: fetch failed.", LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	protected static bool ParseList(string data)
	{
		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(data))
		{
			Print("[TBD] MissionList: JSON parse failed.", LogLevel.ERROR);
			return false;
		}

		TBD_MissionListStruct doc = new TBD_MissionListStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print("[TBD] MissionList: JSON schema mismatch.", LogLevel.ERROR);
			return false;
		}

		s_Entries = doc.missions;
		if (!s_Entries)
			s_Entries = new array<ref TBD_MissionListEntry>();

		s_Loaded = true;
		Print(string.Format("[TBD] MissionList: %1 missions available.", s_Entries.Count()));
		return true;
	}
}
