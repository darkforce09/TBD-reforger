//! Game roster response: identityId → slotId assignments for an event.
class TBD_RosterResponseStruct
{
	string eventId;
	string missionId;
	ref map<string, string> assignments;
}

//! Fetches and caches GET /api/game/events/{eventId}/roster for slot enforcement.
//! @route GET /api/game/events/{eventId}/roster
class TBD_RosterLoader
{
	protected static ref map<string, string> s_IdentityToSlot;
	protected static bool s_Loaded;
	protected static bool s_LoadInFlight;
	protected static ref RestCallback s_RestCallback;

	//------------------------------------------------------------------------------------------------
	static bool IsLoaded()
	{
		return s_Loaded;
	}

	//------------------------------------------------------------------------------------------------
	static string GetSlotForIdentity(string identityId)
	{
		if (!s_Loaded || !s_IdentityToSlot || identityId.IsEmpty())
			return string.Empty;

		string slotId;
		if (s_IdentityToSlot.Find(identityId, slotId))
			return slotId;
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	static void BeginLoad()
	{
		if (s_Loaded || s_LoadInFlight)
			return;

		TBD_BackendConfig.Load();
		string eventId = TBD_BackendConfig.GetEventId();
		if (eventId.IsEmpty())
		{
			Print("[TBD] RosterLoader: eventId not configured — using round-robin slot assignment.", LogLevel.WARNING);
			s_Loaded = true;
			s_IdentityToSlot = new map<string, string>();
			return;
		}

		if (TBD_BackendConfig.GetBackendUrl().IsEmpty() || TBD_BackendConfig.GetServerToken().IsEmpty())
		{
			Print("[TBD] RosterLoader: backend not configured — round-robin slots only.", LogLevel.WARNING);
			s_Loaded = true;
			s_IdentityToSlot = new map<string, string>();
			return;
		}

		s_LoadInFlight = true;
		FetchFromBackend(eventId);
	}

	//------------------------------------------------------------------------------------------------
	protected static void FetchFromBackend(string eventId)
	{
		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			s_LoadInFlight = false;
			s_Loaded = true;
			s_IdentityToSlot = new map<string, string>();
			return;
		}

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext ctx = rest.GetContext(baseUrl);
		if (!ctx)
		{
			s_LoadInFlight = false;
			s_Loaded = true;
			s_IdentityToSlot = new map<string, string>();
			return;
		}

		s_RestCallback = new RestCallback();
		s_RestCallback.SetOnSuccess(OnFetchSuccess);
		s_RestCallback.SetOnError(OnFetchError);

		string token = TBD_BackendConfig.GetServerToken();
		ctx.SetHeaders(string.Format("Authorization, Bearer %1,Accept,application/json", token));

		string path = string.Format("/api/game/events/%1/roster", eventId);
		Print("[TBD] Fetching roster for event " + eventId);
		ctx.GET(s_RestCallback, path);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnFetchSuccess(RestCallback cb)
	{
		s_LoadInFlight = false;
		s_IdentityToSlot = new map<string, string>();

		string data = cb.GetData();
		if (data.IsEmpty())
		{
			Print("[TBD] RosterLoader: empty roster response.", LogLevel.WARNING);
			s_Loaded = true;
			return;
		}

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(data))
		{
			Print("[TBD] RosterLoader: JSON parse failed.", LogLevel.ERROR);
			s_Loaded = true;
			return;
		}

		ref TBD_RosterResponseStruct roster = new TBD_RosterResponseStruct();
		if (!ctx.ReadValue("", roster))
		{
			Print("[TBD] RosterLoader: schema mismatch.", LogLevel.ERROR);
			s_Loaded = true;
			return;
		}

		// Defensive (T-122 M12): a roster fetched for a different event must not be trusted
		// silently. Warn loudly on an eventId mismatch (don't drop — the fetch URL already keys
		// on the configured event, so this guards a backend/proxy mix-up, not normal flow).
		string expectedEventId = TBD_BackendConfig.GetEventId();
		if (!roster.eventId.IsEmpty() && !expectedEventId.IsEmpty() && roster.eventId != expectedEventId)
			Print(string.Format("[TBD] RosterLoader: WARNING roster eventId '%1' != configured '%2'", roster.eventId, expectedEventId), LogLevel.WARNING);

		if (roster.assignments)
		{
			foreach (string identityId, string slotId : roster.assignments)
			{
				s_IdentityToSlot.Insert(identityId, slotId);
			}
		}

		s_Loaded = true;
		Print(string.Format("[TBD] Roster loaded (%1 assignments).", s_IdentityToSlot.Count()));
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnFetchError(RestCallback cb)
	{
		s_LoadInFlight = false;
		s_IdentityToSlot = new map<string, string>();
		s_Loaded = true;
		Print("[TBD] RosterLoader: fetch failed — round-robin slots only.", LogLevel.WARNING);
	}
}
