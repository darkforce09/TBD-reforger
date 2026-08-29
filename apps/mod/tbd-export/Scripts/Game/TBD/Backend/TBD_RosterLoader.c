//! Game roster response: identityId -> slotId assignments for an event.
//!
//! The keys are camelCase on the wire (not the platform's usual snake_case) because
//! `JsonLoadContext` binds JSON keys onto these field names, and a key this class does not
//! declare is silently invisible rather than an error. The backend renames to match - see
//! `ingest_event_roster` in `apps/website/api/src/handlers/events.rs`.
//!
//! `assignments` is keyed on `users.arma_id`, which is the SAME string
//! `TBD_SpawnManager.PlayerBindKey` produces and `TBD_PlayerIdentity.GetArmaId` puts on the
//! wire at link time - the raw engine identity uuid. If those two ever drift apart the map
//! matches nobody and every player falls silently to round-robin seating, which is exactly
//! the failure T-181.51 fixed; change the shape in `TBD_PlayerIdentity`, never here.
//!
//! The VALUE is the compiled mission slot's `uid` (the durable editor slot id), which is what
//! `TBD_MissionLoader.GetSlotById` resolves. It is not the website's `orbat_slots` UUID - the
//! backend pairs the two before answering, because that UUID exists nowhere in the compiled
//! document and would resolve to null, i.e. round-robin again.
class TBD_RosterResponseStruct
{
	string eventId;
	//! Informational only (this file never reads it): the backend fills it only when the
	//! event holds exactly one mission, since the roster covers the whole event.
	string missionId;
	ref map<string, string> assignments;
}

//! Fetches and caches the event roster for slot enforcement.
//! @route GET /api/v1/ingest/events/{id}/roster (service-token tier; `X-Service-Token`)
class TBD_RosterLoader
{
	//! `%1` = the configured event id. Under `/ingest/` with the other service-token routes;
	//! the member-tier `/event-missions/{emid}/orbat` is not usable here because it answers
	//! per event MISSION and is scoped to the calling user's own registration state.
	protected static const string ROSTER_PATH = "/api/v1/ingest/events/%1/roster";

	protected static ref map<string, string> s_IdentityToSlot;
	protected static bool s_Loaded;
	protected static bool s_LoadInFlight;
	protected static ref RestCallback s_RestCallback;
	//! A5 - how the loader settled ("loaded"/"unconfigured"/"failed"/"timeout"/"empty"),
	//! for the deterministic `[TBD][Spawn] roster settled=...` breadcrumb.
	protected static string s_SettleReason = "pending";

	//------------------------------------------------------------------------------------------------
	static bool IsLoaded()
	{
		return s_Loaded;
	}

	//------------------------------------------------------------------------------------------------
	static string GetSettleReason()
	{
		return s_SettleReason;
	}

	//------------------------------------------------------------------------------------------------
	static int GetAssignmentCount()
	{
		if (!s_IdentityToSlot)
			return 0;
		return s_IdentityToSlot.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! A5 - deadline force-settle (stage machine hit its 2 s roster budget with the
	//! fetch still in flight). Mirrors the error path: empty assignments, round-robin.
	//! A late REST response after this is ignored for determinism (s_Loaded guards).
	static void ForceSettle()
	{
		if (s_Loaded)
			return;
		s_IdentityToSlot = new map<string, string>();
		s_Loaded = true;
		s_SettleReason = "timeout";
		Print("[TBD] RosterLoader: settle deadline hit with fetch in flight - round-robin slots only.", LogLevel.WARNING);
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
			Print("[TBD] RosterLoader: eventId not configured - using round-robin slot assignment.", LogLevel.WARNING);
			s_Loaded = true;
			s_SettleReason = "unconfigured";
			s_IdentityToSlot = new map<string, string>();
			return;
		}

		if (TBD_BackendConfig.GetBackendUrl().IsEmpty() || TBD_BackendConfig.GetServerToken().IsEmpty())
		{
			Print("[TBD] RosterLoader: backend not configured - round-robin slots only.", LogLevel.WARNING);
			s_Loaded = true;
			s_SettleReason = "unconfigured";
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
		// T-181.51 - the game-server tier is `X-Service-Token`, NOT an Authorization bearer
		// (`ServiceAuth`, apps/website/api/src/middleware/auth.rs, reads only that header). This
		// pointed at `/api/game/events/{id}/roster`, a route that has never existed, so the 404
		// masked the auth bug underneath: fixing the URL alone would have turned it into a 401.
		// Same "Key,Value,Key,Value" comma form the three working loaders use.
		ctx.SetHeaders(string.Format("X-Service-Token,%1,Accept,application/json", token));

		string path = string.Format(ROSTER_PATH, eventId);
		Print("[TBD] Fetching roster for event " + eventId);
		ctx.GET(s_RestCallback, path);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnFetchSuccess(RestCallback cb)
	{
		// A5: a response landing after ForceSettle must not mutate settled state.
		if (s_Loaded)
			return;
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
		// silently. Warn loudly on an eventId mismatch (don't drop - the fetch URL already keys
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
		s_SettleReason = "loaded";
		Print(string.Format("[TBD] Roster loaded (%1 assignments).", s_IdentityToSlot.Count()));
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnFetchError(RestCallback cb)
	{
		// A5: a response landing after ForceSettle must not mutate settled state.
		if (s_Loaded)
			return;
		s_LoadInFlight = false;
		s_IdentityToSlot = new map<string, string>();
		s_Loaded = true;
		s_SettleReason = "failed";
		Print("[TBD] RosterLoader: fetch failed - round-robin slots only.", LogLevel.WARNING);
	}
}
